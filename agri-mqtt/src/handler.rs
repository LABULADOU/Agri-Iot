use chrono::Utc;
use sqlx::SqlitePool;
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

    info!("Device {} status changed to {}", node_id, db_status);

    Ok(())
}
