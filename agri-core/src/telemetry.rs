use chrono::Utc;
use sqlx::SqlitePool;
use tokio::sync::broadcast;

const METRIC_MAP: &[(&str, &str)] = &[
    ("air_temp", "temperature"),
    ("air_humidity", "humidity"),
    ("soil_temp", "soil_temperature"),
];
const KNOWN_METRICS: &[&str] = &["temperature", "humidity", "soil_moisture", "soil_temperature", "ec", "light"];

pub fn normalize_metric(name: &str) -> &str {
    METRIC_MAP.iter().find(|(k, _)| *k == name).map(|(_, v)| *v).unwrap_or(name)
}

pub fn is_known_metric(name: &str) -> bool {
    KNOWN_METRICS.contains(&name)
}

pub fn validate_value(metric: &str, val: f64) -> bool {
    match metric {
        "temperature" | "soil_temperature" => val >= -10.0 && val <= 60.0,
        "humidity" | "soil_moisture" => val >= 0.0 && val <= 100.0,
        "ec" => val >= 0.0 && val <= 10.0,
        "light" => val >= 0.0 && val <= 200000.0,
        _ => true,
    }
}

pub fn metric_unit(metric: &str) -> &str {
    match metric {
        "temperature" | "soil_temperature" => "\u{2103}",
        "humidity" | "soil_moisture" => "%",
        "light" => "lux",
        "ec" => "mS/cm",
        _ => "",
    }
}

pub fn maybe_convert_ec(metric: &str, val: f64) -> f64 {
    if metric == "ec" { val / 1000.0 } else { val }
}

pub async fn process_telemetry(
    pool: &SqlitePool,
    node_id: &str,
    metrics: &serde_json::Map<String, serde_json::Value>,
    event_tx: Option<&broadcast::Sender<String>>,
) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
    let devices = sqlx::query_as::<_, (String, String)>(
        "SELECT id, node_id FROM devices WHERE node_id = ?",
    )
    .bind(node_id)
    .fetch_all(pool)
    .await?;

    if devices.is_empty() {
        return Ok(0);
    }

    let now = Utc::now().timestamp();
    let mut inserted: i64 = 0;

    for (device_id, _) in &devices {
        for (metric, value) in metrics {
            let Some(mut val) = value.as_f64() else { continue };
            let m = metric.as_str();
            let normalized = normalize_metric(m);
            if !is_known_metric(normalized) { continue; }
            if !validate_value(normalized, val) { continue; }
            val = maybe_convert_ec(normalized, val);
            let unit = metric_unit(normalized);

            if let Err(e) = sqlx::query(
                "INSERT INTO sensor_readings (device_id, metric, value, unit, timestamp) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(device_id)
            .bind(normalized)
            .bind(val)
            .bind(unit)
            .bind(now)
            .execute(pool)
            .await
            {
                tracing::warn!("Failed to insert reading: {}", e);
            } else {
                inserted += 1;
            }
        }
    }

    if inserted > 0 {
        sqlx::query("UPDATE devices SET status = 'online', updated_at = ? WHERE node_id = ?")
            .bind(now)
            .bind(node_id)
            .execute(pool)
            .await
            .ok();

        if let Some(tx) = event_tx {
            let _ = tx.send(serde_json::json!({
                "type": "telemetry",
                "node_id": node_id,
                "timestamp": now,
            }).to_string());
        }
    }

    Ok(inserted)
}
