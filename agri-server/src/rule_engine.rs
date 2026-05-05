use sqlx::Row;
use crate::state::AppState;
use agri_mqtt::client::publish_command;
use anyhow::Result;
use chrono::{Timelike, Utc};
use std::time::Duration;
use tokio::time::interval;
use tracing::info;

pub async fn start(state: AppState) -> Result<()> {
    info!("Rule engine started");

    refresh_rules_cache(&state).await?;

    let mut interval = interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        if let Err(e) = evaluate_rules(&state).await {
            tracing::warn!("Rule evaluation error: {}", e);
        }

        if Utc::now().timestamp() % 60 < 5 {
            if let Err(e) = refresh_rules_cache(&state).await {
                tracing::warn!("Rule cache refresh error: {}", e);
            }
        }
    }
}

async fn refresh_rules_cache(state: &AppState) -> Result<()> {
    let rules = sqlx::query_as::<_, (String, String, i64, String, String, String, Option<String>, i64)>(
        "SELECT id, name, enabled, trigger_type, conditions, actions, schedule, created_at FROM rules WHERE enabled = 1",
    )
    .fetch_all(&state.pool)
    .await?;

    let mut cache = state.rules_cache.lock().await;
    *cache = rules
        .into_iter()
        .map(|r| agri_core::models::Rule {
            id: uuid::Uuid::parse_str(&r.0).unwrap_or_default(),
            name: r.1,
            enabled: r.2 == 1,
            trigger_type: match r.3.as_str() {
                "schedule" => agri_core::models::TriggerType::Schedule,
                _ => agri_core::models::TriggerType::Condition,
            },
            conditions: serde_json::from_str(&r.4).unwrap_or_default(),
            actions: serde_json::from_str(&r.5).unwrap_or_default(),
            schedule: r.6,
            created_at: chrono::DateTime::from_timestamp(r.7, 0).unwrap_or_default(),
        })
        .collect();

    info!("Rules cache refreshed: {} rules loaded", cache.len());
    Ok(())
}

async fn evaluate_rules(state: &AppState) -> Result<()> {
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

            if time_str == current_time && now.second() == 0 {
                trigger_actions(state, rule).await?;
            }
        }
    }

    Ok(())
}

async fn trigger_actions(state: &AppState, rule: &agri_core::models::Rule) -> Result<()> {
    if let Some(actions) = rule.actions.get("actions").and_then(|a| a.as_array()) {
        for action in actions {
            let device_id = action["device_id"].as_str().unwrap_or("");
            let command = action["command"].as_str().unwrap_or("");
            let params = action["params"].clone();

            let device: Option<(String,)> = sqlx::query_as("SELECT node_id FROM devices WHERE id = ?")
                .bind(device_id)
                .fetch_optional(&state.pool)
                .await?;

            if let Some((node_id,)) = device {
                let cmd_id = uuid::Uuid::new_v4().to_string();
                let payload = serde_json::json!({
                    "command": command,
                    "params": params
                })
                .to_string();

                if let Some(client) = state.mqtt_client.lock().await.as_ref() {
                    if let Err(e) = publish_command(client, &node_id, &cmd_id, &payload).await {
                        tracing::warn!("Failed to publish command: {}", e);
                    } else {
                        info!("Rule '{}' triggered action for device {}", rule.name, device_id);
                    }
                }
            }
        }
    }

    Ok(())
}
