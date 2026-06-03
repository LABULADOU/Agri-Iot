use sqlx::Row;
use crate::state::AppState;
use agri_core::ai::emergency::{check_emergency, WeatherAlertInput};
use agri_core::models::{Rule, WeatherData};
use agri_mqtt::client::publish_command;
use anyhow::Result;
use chrono::{Timelike, Utc};
use std::time::Duration;
use tokio::time::interval;
use tracing::info;

pub async fn start(state: AppState) -> Result<()> {
    info!("Rule engine started");

    refresh_rules_cache(&state).await?;

    let mut interval_timer = interval(Duration::from_secs(5));
    let mut last_minute_refresh: Option<u32> = None; // 记录上次刷新规则的分钟数

    loop {
        interval_timer.tick().await;

        if let Err(e) = evaluate_rules(&state).await {
            tracing::warn!("Rule evaluation error: {}", e);
        }

        // 每分钟的第0秒刷新规则缓存（避免每秒检查）
        let now = Utc::now();
        let current_minute = now.minute();
        if now.second() == 0 && Some(current_minute) != last_minute_refresh {
            if let Err(e) = refresh_rules_cache(&state).await {
                tracing::warn!("Rule cache refresh error: {}", e);
            }
            last_minute_refresh = Some(current_minute);
        }
    }
}

async fn refresh_rules_cache(state: &AppState) -> Result<()> {
    let rules = sqlx::query_as::<_, Rule>(
        "SELECT id, name, enabled, trigger_type, conditions, actions, schedule, priority, auto_execute, created_at FROM rules WHERE enabled = 1",
    )
    .fetch_all(&state.pool)
    .await?;

    let mut cache = state.rules_cache.lock().await;
    *cache = rules;
    info!("Rules cache refreshed: {} rules loaded", cache.len());
    Ok(())
}

async fn evaluate_rules(state: &AppState) -> Result<()> {
    // Step 1: 检查紧急情况
    let weather = sqlx::query_as::<_, WeatherData>(
        "SELECT * FROM weather_data ORDER BY timestamp DESC LIMIT 1"
    )
    .fetch_optional(&state.pool)
    .await?;

    if let Some(w) = weather {
        let input = WeatherAlertInput {
            wind_speed_kmh: w.wind_speed,
            precipitation_mm_per_hour: w.precipitation,
            temperature_celsius: w.temperature,
            snow_probability: w.snow_probability,
            humidity: w.humidity,
        };
        let mut ctx = state.emergency_ctx.lock().await;
        let output = check_emergency(&input, &mut ctx, "all");

        if !output.emergencies.is_empty() {
            for emergency in &output.emergencies {
                let action = agri_core::ai::emergency::get_emergency_action(emergency);
                info!(
                    "EMERGENCY triggered: {:?} — {}",
                    emergency.emergency_type, emergency.message
                );

                // 写入 command_log
                let device_type = &action.device_type;
                let cmd = &action.command;
                let payload = serde_json::json!({
                    "emergency": true,
                    "emergency_type": format!("{:?}", emergency.emergency_type),
                    "command": cmd,
                    "target_percent": action.target_percent,
                });

                let _ = sqlx::query(
                    "INSERT INTO command_log (device_id, command, payload, status, created_at)
                     VALUES ('emergency', ?, ?, 'pending', datetime('now'))"
                )
                .bind(cmd)
                .bind(payload.to_string())
                .execute(&state.pool)
                .await;

                // 如果有 MQTT 客户端，直接发送紧急命令
                if let Some(client) = state.mqtt_client.lock().await.as_ref() {
                    let cmd_id = uuid::Uuid::new_v4().to_string();
                    let _ = publish_command(client, device_type, &cmd_id, &payload.to_string()).await;
                }

                // 广播 SSE 事件
                let _ = state.event_tx.send(serde_json::json!({
                    "type": "emergency",
                    "emergency_type": format!("{:?}", emergency.emergency_type),
                    "message": emergency.message,
                    "pauses_auto_mode": output.pauses_auto_mode,
                }).to_string());
            }

            // Step 2: 如果紧急情况要求暂停自动模式，跳过规则评估
            if output.pauses_auto_mode {
                info!("Emergency pauses auto mode — skipping rule evaluation");
                return Ok(());
            }
        }
    }

    // Step 3: 更新设备在线状态（用于 SystemFailure 检测）
    {
        let devices = sqlx::query(
            "SELECT id, updated_at FROM devices WHERE status = 'online'"
        )
        .fetch_all(&state.pool)
        .await?;
        let mut ctx = state.emergency_ctx.lock().await;
        for row in devices {
            let device_id: String = row.try_get(0)?;
            let updated_at: i64 = row.try_get(1)?;
            let dt = chrono::DateTime::from_timestamp(updated_at, 0)
                .unwrap_or_else(|| Utc::now());
            ctx.track_device(&device_id, dt);
        }
    }

    // Step 3.5: 设备离线检测 — 超过5分钟无数据标记为 offline
    {
        let cutoff = Utc::now().timestamp() - 300;
        let affected = sqlx::query(
            "UPDATE devices SET status = 'offline' WHERE status = 'online' AND updated_at < ?"
        )
        .bind(cutoff)
        .execute(&state.pool)
        .await?;
        if affected.rows_affected() > 0 {
            info!("{} device(s) marked offline due to timeout", affected.rows_affected());
        }
    }

    // Step 4: 正常规则评估
    let rules = state.rules_cache.lock().await.clone();
    for rule in rules {
        if !rule.enabled {
            continue;
        }

        match rule.trigger_type {
            agri_core::models::TriggerType::Condition => {
                evaluate_condition_rule(state, &rule).await?;
            }
            agri_core::models::TriggerType::Schedule => {
                evaluate_schedule_rule(state, &rule).await?;
            }
        }
    }

    Ok(())
}

async fn evaluate_condition_rule(state: &AppState, rule: &agri_core::models::Rule) -> Result<()> {
    if let Some(conditions) = rule.conditions.get("conditions").and_then(|c| c.as_array()) {
        let mut all_met = true;

        for condition in conditions {
            let metric = condition["metric"].as_str().unwrap_or("");
            let operator = condition["operator"].as_str().unwrap_or("");
            let threshold = condition["value"].as_f64().unwrap_or(0.0);

            let latest = sqlx::query(
                "SELECT value FROM sensor_readings WHERE metric = ? ORDER BY timestamp DESC LIMIT 1",
            )
            .bind(metric)
            .fetch_optional(&state.pool)
            .await?;

            if let Some(row) = latest {
                let value: f64 = row.try_get(0)?;
                let met = match operator {
                    ">" => value > threshold,
                    ">=" => value >= threshold,
                    "<" => value < threshold,
                    "<=" => value <= threshold,
                    "==" => (value - threshold).abs() < 0.001,
                    _ => false,
                };

                if !met {
                    all_met = false;
                    break;
                }
            } else {
                all_met = false;
                break;
            }
        }

        if all_met {
            trigger_actions(state, rule).await?;
        }
    }

    Ok(())
}

async fn evaluate_schedule_rule(state: &AppState, rule: &agri_core::models::Rule) -> Result<()> {
    if let Some(schedule) = &rule.schedule {
        if let Some(time_str) = schedule.strip_prefix("at ") {
            let now = Utc::now();
            let current_time = format!("{:02}:{:02}", now.hour(), now.minute());

            // 只在整秒时触发，避免重复执行
            if time_str == current_time && now.second() == 0 {
                // 检查是否在今天已经执行过（简单去重）
                let _last_execution_key = format!("rule_{}_last_exec", rule.id);
                let _last_exec_minute = now.minute();

                // 这里可以添加更复杂的去重逻辑，比如使用Redis或数据库记录
                // 目前简化为每分钟最多执行一次
                trigger_actions(state, rule).await?;
                info!("Scheduled rule '{}' triggered at {}", rule.name, current_time);
            }
        }
    }

    Ok(())
}

async fn trigger_actions(state: &AppState, rule: &agri_core::models::Rule) -> Result<()> {
    // 紧急规则：立即执行，跳过队列
    if rule.priority > 0 || rule.auto_execute {
        info!("EMERGENCY rule '{}' triggered, executing immediately", rule.name);
    }

    if let Some(actions) = rule.actions.get("actions").and_then(|a| a.as_array()) {
        for action in actions {
            let device_id = action["device_id"].as_str().unwrap_or("");
            let command = action["command"].as_str().unwrap_or("");
            let params = action["params"].clone();

            let device: Option<(String, String)> = sqlx::query_as("SELECT id, node_id FROM devices WHERE id = ?")
                .bind(device_id)
                .fetch_optional(&state.pool)
                .await?;

            if let Some((dev_id, node_id)) = device {
                let cmd_id = uuid::Uuid::new_v4().to_string();
                let payload = serde_json::json!({
                    "command": command,
                    "params": params
                })
                .to_string();

                // 紧急预案：直接通过MQTT发送，不写队列
                if rule.priority > 0 || rule.auto_execute {
                    if let Some(client) = state.mqtt_client.lock().await.as_ref() {
                        if let Err(e) = publish_command(client, &node_id, &cmd_id, &payload).await {
                            tracing::warn!("Failed to publish emergency command: {}", e);
                        } else {
                            info!("Emergency command sent to device {}: {}", dev_id, command);
                        }
                    }
                } else {
                    // 普通规则：写入command_log队列
                    let now = Utc::now().timestamp();
                    sqlx::query(
                        "INSERT INTO command_log (device_id, command, payload, status, created_at) VALUES (?, ?, ?, 'pending', ?)"
                    )
                    .bind(&dev_id)
                    .bind(command)
                    .bind(&payload)
                    .bind(now)
                    .execute(&state.pool)
                    .await?;
                    info!("Rule '{}' queued action for device {}", rule.name, dev_id);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agri_core::models::{Rule, TriggerType};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    async fn create_test_db() -> sqlx::SqlitePool {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();

        sqlx::query(
            "CREATE TABLE rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                trigger_type TEXT NOT NULL,
                conditions TEXT,
                actions TEXT,
                schedule TEXT,
                priority INTEGER NOT NULL DEFAULT 0,
                auto_execute INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE sensor_readings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL,
                metric TEXT NOT NULL,
                value REAL NOT NULL,
                unit TEXT NOT NULL DEFAULT '',
                timestamp INTEGER NOT NULL
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE devices (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                node_id TEXT NOT NULL,
                device_type TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'offline',
                config TEXT,
                area_id TEXT,
                comfort_config TEXT,
                capabilities TEXT NOT NULL DEFAULT '[\"sensor\"]',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE command_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL,
                command TEXT NOT NULL,
                payload TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at INTEGER NOT NULL
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE weather_data (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                area_id TEXT,
                source TEXT NOT NULL,
                temperature REAL,
                humidity REAL,
                wind_speed REAL,
                wind_direction TEXT,
                precipitation REAL,
                snow_probability REAL,
                uv_index REAL,
                forecast_hour INTEGER,
                timestamp INTEGER NOT NULL
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    fn make_state(pool: sqlx::SqlitePool) -> AppState {
        let (tx, _) = tokio::sync::broadcast::channel(256);
        AppState {
            pool,
            mqtt_client: Arc::new(Mutex::new(None)),
            rules_cache: Arc::new(Mutex::new(Vec::new())),
            event_tx: tx,
            obsidian_vault_path: None,
            emergency_ctx: Arc::new(Mutex::new(
                agri_core::ai::emergency::EmergencyContext::new()
            )),
        }
    }

    #[test]
    fn test_trigger_type_condition() {
        let trigger = TriggerType::Condition;
        match trigger {
            TriggerType::Condition => (),
            _ => panic!("Expected Condition"),
        }
    }

    #[test]
    fn test_trigger_type_schedule() {
        let trigger = TriggerType::Schedule;
        match trigger {
            TriggerType::Schedule => (),
            _ => panic!("Expected Schedule"),
        }
    }

    #[tokio::test]
    async fn test_refresh_rules_cache_empty() {
        let pool = create_test_db().await;
        let state = make_state(pool);

        let result = refresh_rules_cache(&state).await;
        assert!(result.is_ok());
        assert_eq!(state.rules_cache.lock().await.len(), 0);
    }

    #[tokio::test]
    async fn test_refresh_rules_cache_with_enabled_rule() {
        let pool = create_test_db().await;
        let state = make_state(pool);

        sqlx::query(
            "INSERT INTO rules (id, name, enabled, trigger_type, conditions, actions, schedule, priority, auto_execute, created_at)
             VALUES ('550e8400-e29b-41d4-a716-446655440000', '高温告警', 1, 'condition', '{}', '{}', NULL, 0, 0, 1000000)"
        )
        .execute(&state.pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO rules (id, name, enabled, trigger_type, conditions, actions, schedule, priority, auto_execute, created_at)
             VALUES ('550e8400-e29b-41d4-a716-446655440001', '定时浇水', 0, 'schedule', '{}', '{}', 'at 08:00', 0, 0, 1000000)"
        )
        .execute(&state.pool)
        .await
        .unwrap();

        refresh_rules_cache(&state).await.unwrap();
        let cache = state.rules_cache.lock().await;
        assert_eq!(cache.len(), 1);
        assert_eq!(cache[0].name, "高温告警");
    }

    #[tokio::test]
    async fn test_evaluate_condition_met_queues_action() {
        let pool = create_test_db().await;
        let state = make_state(pool);

        let rule = Rule {
            id: uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap().into(),
            name: "高温告警".to_string(),
            enabled: true,
            trigger_type: TriggerType::Condition,
            conditions: serde_json::json!({"conditions": [{"metric": "temperature", "operator": ">", "value": 30.0}]}).into(),
            actions: serde_json::json!({"actions": [{"device_id": "dev-001", "command": "alarm_on", "params": {}}]}).into(),
            schedule: None,
            priority: 0,
            auto_execute: false,
            created_at: chrono::DateTime::from_timestamp(1000000, 0).unwrap(),
        };

        sqlx::query("INSERT INTO devices (id, name, node_id, device_type, status, created_at, updated_at) VALUES ('dev-001', 'device1', 'node-001', 'actuator', 'online', 1000000, 1000000)")
            .execute(&state.pool).await.unwrap();

        sqlx::query("INSERT INTO sensor_readings (device_id, metric, value, unit, timestamp) VALUES ('dev-001', 'temperature', 35.0, '℃', 1000000)")
            .execute(&state.pool).await.unwrap();

        evaluate_condition_rule(&state, &rule).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM command_log")
            .fetch_one(&state.pool).await.unwrap();
        assert_eq!(count.0, 1, "Condition met should queue 1 action");
    }

    #[tokio::test]
    async fn test_evaluate_condition_not_met_skips_action() {
        let pool = create_test_db().await;
        let state = make_state(pool);

        let rule = Rule {
            id: uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap().into(),
            name: "高温告警".to_string(),
            enabled: true,
            trigger_type: TriggerType::Condition,
            conditions: serde_json::json!({"conditions": [{"metric": "temperature", "operator": ">", "value": 30.0}]}).into(),
            actions: serde_json::json!({"actions": [{"device_id": "dev-001", "command": "alarm_on", "params": {}}]}).into(),
            schedule: None,
            priority: 0,
            auto_execute: false,
            created_at: chrono::DateTime::from_timestamp(1000000, 0).unwrap(),
        };

        sqlx::query("INSERT INTO sensor_readings (device_id, metric, value, unit, timestamp) VALUES ('dev-001', 'temperature', 25.0, '℃', 1000000)")
            .execute(&state.pool).await.unwrap();

        evaluate_condition_rule(&state, &rule).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM command_log")
            .fetch_one(&state.pool).await.unwrap();
        assert_eq!(count.0, 0, "Condition not met should not queue action");
    }

    #[tokio::test]
    async fn test_evaluate_condition_no_data_skips_action() {
        let pool = create_test_db().await;
        let state = make_state(pool);

        let rule = Rule {
            id: uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap().into(),
            name: "高温告警".to_string(),
            enabled: true,
            trigger_type: TriggerType::Condition,
            conditions: serde_json::json!({"conditions": [{"metric": "temperature", "operator": ">", "value": 30.0}]}).into(),
            actions: serde_json::json!({"actions": [{"device_id": "dev-001", "command": "alarm_on", "params": {}}]}).into(),
            schedule: None,
            priority: 0,
            auto_execute: false,
            created_at: chrono::DateTime::from_timestamp(1000000, 0).unwrap(),
        };

        evaluate_condition_rule(&state, &rule).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM command_log")
            .fetch_one(&state.pool).await.unwrap();
        assert_eq!(count.0, 0, "No sensor data should not queue action");
    }

    #[tokio::test]
    async fn test_evaluate_condition_multiple_conditions_all_met() {
        let pool = create_test_db().await;
        let state = make_state(pool);

        let rule = Rule {
            id: uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap().into(),
            name: "温湿度告警".to_string(),
            enabled: true,
            trigger_type: TriggerType::Condition,
            conditions: serde_json::json!({
                "conditions": [
                    {"metric": "temperature", "operator": ">", "value": 25.0},
                    {"metric": "humidity", "operator": "<", "value": 80.0}
                ]
            }).into(),
            actions: serde_json::json!({"actions": [{"device_id": "dev-001", "command": "ventilate", "params": {}}]}).into(),
            schedule: None,
            priority: 0,
            auto_execute: false,
            created_at: chrono::DateTime::from_timestamp(1000000, 0).unwrap(),
        };

        sqlx::query("INSERT INTO devices (id, name, node_id, device_type, status, created_at, updated_at) VALUES ('dev-001', 'device1', 'node-001', 'actuator', 'online', 1000000, 1000000)")
            .execute(&state.pool).await.unwrap();
        sqlx::query("INSERT INTO sensor_readings (device_id, metric, value, unit, timestamp) VALUES ('dev-001', 'temperature', 30.0, '℃', 1000000)")
            .execute(&state.pool).await.unwrap();
        sqlx::query("INSERT INTO sensor_readings (device_id, metric, value, unit, timestamp) VALUES ('dev-001', 'humidity', 60.0, '%', 1000000)")
            .execute(&state.pool).await.unwrap();

        evaluate_condition_rule(&state, &rule).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM command_log")
            .fetch_one(&state.pool).await.unwrap();
        assert_eq!(count.0, 1, "All conditions met should queue 1 action");
    }

    #[tokio::test]
    async fn test_evaluate_multi_operator_types() {
        let pool = create_test_db().await;
        let state = make_state(pool);

        // "==" operator with value close to threshold
        let rule_eq = Rule {
            id: uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap().into(),
            name: "精确匹配".to_string(),
            enabled: true,
            trigger_type: TriggerType::Condition,
            conditions: serde_json::json!({"conditions": [{"metric": "temp", "operator": "==", "value": 30.0}]}).into(),
            actions: serde_json::json!({"actions": [{"device_id": "dev-001", "command": "ok", "params": {}}]}).into(),
            schedule: None,
            priority: 0,
            auto_execute: false,
            created_at: chrono::DateTime::from_timestamp(1000000, 0).unwrap(),
        };

        sqlx::query("INSERT INTO devices (id, name, node_id, device_type, status, created_at, updated_at) VALUES ('dev-001', 'd', 'n', 'actuator', 'online', 1000000, 1000000)")
            .execute(&state.pool).await.unwrap();
        sqlx::query("INSERT INTO sensor_readings (device_id, metric, value, unit, timestamp) VALUES ('dev-001', 'temp', 30.001, '', 1000000)")
            .execute(&state.pool).await.unwrap();

        evaluate_condition_rule(&state, &rule_eq).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM command_log")
            .fetch_one(&state.pool).await.unwrap();
        assert_eq!(count.0, 0, "30.001 not within 0.001 of 30.0");
    }

    #[tokio::test]
    async fn test_trigger_actions_device_not_found_skips() {
        let pool = create_test_db().await;
        let state = make_state(pool);

        let rule = Rule {
            id: uuid::Uuid::new_v4().into(),
            name: "操作不存在设备".to_string(),
            enabled: true,
            trigger_type: TriggerType::Condition,
            conditions: serde_json::json!({}).into(),
            actions: serde_json::json!({"actions": [{"device_id": "nonexistent", "command": "turn_on", "params": {}}]}).into(),
            schedule: None,
            priority: 0,
            auto_execute: false,
            created_at: chrono::DateTime::from_timestamp(1000000, 0).unwrap(),
        };

        trigger_actions(&state, &rule).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM command_log")
            .fetch_one(&state.pool).await.unwrap();
        assert_eq!(count.0, 0, "No action queued for unknown device");
    }

    #[tokio::test]
    async fn test_trigger_actions_priority_rule_no_queue() {
        let pool = create_test_db().await;
        let state = make_state(pool);

        sqlx::query("INSERT INTO devices (id, name, node_id, device_type, status, created_at, updated_at) VALUES ('dev-001', 'd', 'n', 'actuator', 'online', 1000000, 1000000)")
            .execute(&state.pool).await.unwrap();

        let rule = Rule {
            id: uuid::Uuid::new_v4().into(),
            name: "紧急规则".to_string(),
            enabled: true,
            trigger_type: TriggerType::Condition,
            conditions: serde_json::json!({}).into(),
            actions: serde_json::json!({"actions": [{"device_id": "dev-001", "command": "emergency_stop", "params": {}}]}).into(),
            schedule: None,
            priority: 1,
            auto_execute: true,
            created_at: chrono::DateTime::from_timestamp(1000000, 0).unwrap(),
        };

        // priority > 0 会走 MQTT 路径（跳过 command_log），这里没有 mqtt client 所以不会有日志
        trigger_actions(&state, &rule).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM command_log")
            .fetch_one(&state.pool).await.unwrap();
        assert_eq!(count.0, 0, "Priority rule skips command_log queue");
    }

    #[tokio::test]
    async fn test_evaluate_rules_disabled_skipped() {
        let pool = create_test_db().await;
        let state = make_state(pool);

        sqlx::query("INSERT INTO sensor_readings (device_id, metric, value, unit, timestamp) VALUES ('dev-001', 'temp', 99.0, '', 1000000)")
            .execute(&state.pool).await.unwrap();

        {
            let mut cache = state.rules_cache.lock().await;
            cache.push(Rule {
                id: uuid::Uuid::new_v4().into(),
                name: "禁用规则".to_string(),
                enabled: false,
                trigger_type: TriggerType::Condition,
                conditions: serde_json::json!({"conditions": [{"metric": "temp", "operator": ">", "value": 0.0}]}).into(),
                actions: serde_json::json!({"actions": [{"device_id": "dev-001", "command": "alarm", "params": {}}]}).into(),
                schedule: None,
                priority: 0,
                auto_execute: false,
                created_at: chrono::DateTime::from_timestamp(1000000, 0).unwrap(),
            });
        }

        evaluate_rules(&state).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM command_log")
            .fetch_one(&state.pool).await.unwrap();
        assert_eq!(count.0, 0, "Disabled rule should not trigger action");
    }
}
