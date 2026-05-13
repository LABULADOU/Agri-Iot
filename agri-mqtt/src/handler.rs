use chrono::Utc;
use sqlx::SqlitePool;
use rumqttc::{Event, Incoming};
use tracing::info;

pub async fn handle_telemetry(
    pool: &SqlitePool,
    node_id: &str,
    payload: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data: serde_json::Value = serde_json::from_str(payload)?;

    if let Some(metrics) = data.get("metrics").and_then(|m| m.as_object()) {
        let now = Utc::now().timestamp();

        let devices = sqlx::query_as::<_, (String, String)>(
            "SELECT id, node_id FROM devices WHERE node_id = ? AND device_type = 'sensor'",
        )
        .bind(node_id)
        .fetch_all(pool)
        .await?;

        for (device_id, _) in devices {
            for (metric, value) in metrics {
                if let Some(val) = value.as_f64() {
                    let unit = match metric.as_str() {
                        "temperature" => "℃",
                        "humidity" => "%",
                        "light" => "lux",
                        "soil_moisture" => "%",
                        _ => "",
                    };

                    sqlx::query(
                        "INSERT INTO sensor_readings (device_id, metric, value, unit, timestamp)
                         VALUES (?, ?, ?, ?, ?)",
                    )
                    .bind(&device_id)
                    .bind(metric)
                    .bind(val)
                    .bind(unit)
                    .bind(now)
                    .execute(pool)
                    .await?;

                    info!(
                        "Stored reading: device={} metric={} value={}{}",
                        device_id, metric, val, unit
                    );
                }
            }
        }
    }

    Ok(())
}

pub async fn handle_status_change(
    pool: &SqlitePool,
    node_id: &str,
    status: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db_status = match status {
        "online" => "online",
        _ => "offline",
    };

    let now = Utc::now().timestamp();

    sqlx::query(
        "UPDATE devices SET status = ?, updated_at = ? WHERE node_id = ?",
    )
    .bind(db_status)
    .bind(now)
    .bind(node_id)
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE sensor_nodes SET status = ?, last_seen = ?, updated_at = ? WHERE id = ?",
    )
    .bind(db_status)
    .bind(now)
    .bind(now)
    .bind(node_id)
    .execute(pool)
    .await?;

    info!("Device {} status changed to {}", node_id, db_status);

    Ok(())
}

pub async fn start_listener(mut eventloop: rumqttc::EventLoop, pool: SqlitePool) {
    info!("MQTT listener started, waiting for messages");

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Incoming::Publish(p))) => {
                let topic = p.topic;
                let payload = String::from_utf8_lossy(&p.payload);

                // 解析主题: agri/node/{node_id}/telemetry 或 agri/node/{node_id}/status
                let parts: Vec<&str> = topic.split('/').collect();
                if parts.len() >= 4 && parts[0] == "agri" && parts[1] == "node" {
                    let node_id = parts[2];
                    let topic_type = parts[3];

                    match topic_type {
                        "telemetry" => {
                            if let Err(e) = handle_telemetry(&pool, node_id, &payload).await {
                                tracing::warn!("Failed to handle telemetry: {}", e);
                            }
                        }
                        "status" => {
                            if let Err(e) = handle_status_change(&pool, node_id, &payload).await {
                                tracing::warn!("Failed to handle status change: {}", e);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("MQTT eventloop error: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    /// 创建测试数据库和表
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        
        // 创建 devices 表
        sqlx::query(
            "CREATE TABLE devices (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                node_id TEXT NOT NULL UNIQUE,
                device_type TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'offline',
                config TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )"
        )
        .execute(&pool)
        .await
        .unwrap();
        
        // 创建 sensor_readings 表
        sqlx::query(
            "CREATE TABLE sensor_readings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL,
                metric TEXT NOT NULL,
                value REAL NOT NULL,
                unit TEXT,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY (device_id) REFERENCES devices(id)
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        // 创建 sensor_nodes 表
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sensor_nodes (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                zone_id TEXT NOT NULL,
                has_irrigation INTEGER NOT NULL DEFAULT 0,
                has_side_vent INTEGER NOT NULL DEFAULT 0,
                has_roof_vent INTEGER NOT NULL DEFAULT 0,
                vent_range TEXT NOT NULL DEFAULT '{\"min\": 0, \"max\": 100}',
                status TEXT NOT NULL DEFAULT 'offline',
                last_seen INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )"
        )
        .execute(&pool)
        .await
        .unwrap();
        
        // 插入测试设备
        let now = Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO devices (id, name, node_id, device_type, status, created_at, updated_at) 
             VALUES ('test-device-uuid', 'Test Sensor', 'node-001', 'sensor', 'online', ?1, ?1)"
        )
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO sensor_nodes (id, name, zone_id, status, created_at, updated_at)
             VALUES ('node-001', 'Test Node', 'zone-001', 'online', ?1, ?1)"
        )
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();
        
        pool
    }

    /// 测试合法的 telemetry JSON 解析
    #[tokio::test]
    async fn test_handle_telemetry_valid_payload() {
        let pool = setup_test_db().await;

        let payload = r#"{"metrics": {"temperature": 25.5, "humidity": 60.0}}"#;
        let result = handle_telemetry(&pool, "node-001", payload).await;

        assert!(result.is_ok(), "合法的 telemetry 应该处理成功");

        // 验证数据是否插入
        let readings: Vec<(String, String, f64)> = sqlx::query_as(
            "SELECT device_id, metric, value FROM sensor_readings ORDER BY metric"
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(readings.len(), 2, "应该插入2条读数");
        assert_eq!(readings[0].1, "humidity");
        assert_eq!(readings[0].2, 60.0);
        assert_eq!(readings[1].1, "temperature");
        assert_eq!(readings[1].2, 25.5);
    }

    /// 测试非法的 JSON 格式（异常捕获）
    #[tokio::test]
    async fn test_handle_telemetry_invalid_json() {
        let pool = setup_test_db().await;
        
        let payload = r#"not a valid json {"metrics": }"#;
        let result = handle_telemetry(&pool, "node-001", payload).await;
        
        assert!(result.is_err(), "非法 JSON 应该返回错误");
    }

    /// 测试缺少 metrics 字段的 JSON
    #[tokio::test]
    async fn test_handle_telemetry_missing_metrics() {
        let pool = setup_test_db().await;
        
        let payload = r#"{"data": {"temperature": 25.5}}"#;
        let result = handle_telemetry(&pool, "node-001", payload).await;
        
        assert!(result.is_ok(), "缺少 metrics 字段不应报错，只是不处理");
        
        // 验证没有数据插入
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sensor_readings")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 0);
    }

    /// 测试设备不在线或不存在的情况
    #[tokio::test]
    async fn test_handle_telemetry_device_not_found() {
        let pool = setup_test_db().await;
        
        let payload = r#"{"metrics": {"temperature": 25.5}}"#;
        let result = handle_telemetry(&pool, "non-existent-node", payload).await;
        
        assert!(result.is_ok(), "设备不存在不应报错，只是不处理");
    }

    /// 测试 handle_status_change - 合法状态
    #[tokio::test]
    async fn test_handle_status_change_online() {
        let pool = setup_test_db().await;
        
        let result = handle_status_change(&pool, "node-001", "online").await;
        assert!(result.is_ok());
        
        let status: (String,) = sqlx::query_as("SELECT status FROM devices WHERE node_id = 'node-001'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status.0, "online");
    }

    /// 测试 handle_status_change - 非法状态转为 offline
    #[tokio::test]
    async fn test_handle_status_change_invalid_status() {
        let pool = setup_test_db().await;
        
        let result = handle_status_change(&pool, "node-001", "error_state").await;
        assert!(result.is_ok());
        
        let status: (String,) = sqlx::query_as("SELECT status FROM devices WHERE node_id = 'node-001'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status.0, "offline", "非法状态应该转为 offline");
    }

    /// 测试数值类型错误（字符串而非数字）
    #[tokio::test]
    async fn test_handle_telemetry_wrong_value_type() {
        let pool = setup_test_db().await;
        
        let payload = r#"{"metrics": {"temperature": "not_a_number"}}"#;
        let result = handle_telemetry(&pool, "node-001", payload).await;
        
        assert!(result.is_ok(), "类型错误不应报错，只是跳过该 metric");
        
        // 验证没有数据插入
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sensor_readings")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 0);
    }
}
