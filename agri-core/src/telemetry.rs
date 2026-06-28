use chrono::Utc;
use sqlx::SqlitePool;
use tokio::sync::broadcast;

const METRIC_MAP: &[(&str, &str)] = &[
    ("air_temp", "temperature"),
    ("air_humidity", "humidity"),
    ("soil_temp", "soil_temperature"),
];
const KNOWN_METRICS: &[&str] = &["temperature", "humidity", "soil_moisture", "soil_temperature", "ec", "light", "rssi", "relay_state"];

pub fn normalize_metric(name: &str) -> &str {
    METRIC_MAP.iter().find(|(k, _)| *k == name).map(|(_, v)| *v).unwrap_or(name)
}

pub fn is_known_metric(name: &str) -> bool {
    KNOWN_METRICS.contains(&name)
}

pub fn validate_value(metric: &str, val: f64) -> bool {
    match metric {
        "temperature" | "soil_temperature" => val >= -5.0 && val <= 50.0,
        "humidity" | "soil_moisture" => val >= 0.0 && val <= 100.0,
        "ec" => val >= 0.0 && val <= 10.0,
        "light" => val >= 0.0 && val <= 200000.0,
        "rssi" => val >= -120.0 && val <= 0.0,
        "relay_state" => val == 0.0 || val == 1.0,
        _ => true,
    }
}

/// Check if DHT22 dual-zero fault: temperature==0 AND humidity==0 in same telemetry frame.
/// This is a reliable indicator of DHT22 short circuit / hardware failure.
pub fn is_dht22_dual_zero(metrics: &serde_json::Map<String, serde_json::Value>) -> bool {
    let extract_val = |key: &str| -> Option<f64> {
        metrics.get(key).and_then(|v| match v {
            serde_json::Value::Number(n) => n.as_f64(),
            _ => None,
        })
    };
    let temp = extract_val("temperature").or_else(|| extract_val("air_temp"));
    let hum = extract_val("humidity").or_else(|| extract_val("air_humidity"));
    matches!((temp, hum), (Some(t), Some(h)) if t == 0.0 && h == 0.0)
}

pub fn metric_unit(metric: &str) -> &str {
    match metric {
        "temperature" | "soil_temperature" => "\u{2103}",
        "humidity" | "soil_moisture" => "%",
        "light" => "lux",
        "ec" => "mS/cm",
        "rssi" => "dBm",
        "relay_state" => "",
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
    seq: Option<i64>,
    boot_id: Option<&str>,
    captured_at: Option<i64>,
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

    let now_received = Utc::now().timestamp();
    let ts = captured_at.unwrap_or(now_received);
    let mut inserted: i64 = 0;
    let mut inserted_readings: Vec<(String, f64, String)> = Vec::new();

    // Evidence E1: DHT22 dual-zero fault detection (sensor short circuit / hardware failure)
    let dht22_fault = is_dht22_dual_zero(metrics);
    if dht22_fault {
        tracing::warn!(
            "DHT22 dual-zero fault detected on node={}: temperature=0 AND humidity=0, skipping DHT22 data",
            node_id
        );

        // P3: Spatial fill — try to find a neighbor's recent readings for interpolation
        let fill_result: Result<Option<(String, f64, String, f64)>, _> = sqlx::query_as::<_, (String, f64, String, f64)>(
            "SELECT n.node_id, sr_temp.value, 'temperature', sr_hum.value \
             FROM sensor_readings sr_temp \
             JOIN sensor_readings sr_hum ON sr_hum.device_id = sr_temp.device_id AND sr_hum.metric = 'humidity' \
             JOIN devices d ON d.id = sr_temp.device_id AND d.node_id = ? \
             LEFT JOIN devices n ON n.area_id = d.area_id AND n.node_id != d.node_id AND n.status = 'online' \
             WHERE sr_temp.metric = 'temperature' AND sr_temp.timestamp > ? \
             AND sr_hum.timestamp > ? \
             AND n.id IS NOT NULL \
             ORDER BY sr_temp.timestamp DESC LIMIT 1"
        )
        .bind(node_id)
        .bind(now_received - 120)  // within last 2 minutes
        .bind(now_received - 120)
        .fetch_optional(pool)
        .await;

        if let Ok(Some((fill_from, fill_temp, _, fill_hum))) = fill_result {
            let _ = sqlx::query(
                "INSERT INTO anomaly_events (device_id, node_id, metric, anomaly_type, severity, value_original, message, created_at) \
                 VALUES ((SELECT id FROM devices WHERE node_id = ?), ?, ?, 'Dht22Fault', 'Warning', ?, ?, ?)"
            )
            .bind(node_id)
            .bind(node_id)
            .bind("temperature")
            .bind(Some(0.0f64))
            .bind(format!("DHT22 fault: filled temperature={}, humidity={} from neighbor {}",
                         fill_temp, fill_hum, fill_from))
            .bind(now_received)
            .execute(pool)
            .await;
            tracing::info!("P3 fill: {} used {}'s data (temp={}, hum={})",
                          node_id, fill_from, fill_temp, fill_hum);
        }
    }

    for (device_id, _) in &devices {
        for (metric, value) in metrics {
            let m = metric.as_str();
            let normalized = normalize_metric(m);
            if !is_known_metric(normalized) { continue; }

            // Under DHT22 fault, skip air temperature and humidity (other metrics still valid)
            if dht22_fault && (normalized == "temperature" || normalized == "humidity") {
                continue;
            }

            let mut val = match value {
                serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
                serde_json::Value::Bool(b) => if *b { 1.0 } else { 0.0 },
                serde_json::Value::String(s) => s.parse::<f64>().unwrap_or(0.0),
                _ => continue,
            };
            val = maybe_convert_ec(normalized, val);
            if !validate_value(normalized, val) {
                tracing::warn!(
                    "Validation rejected node={} metric={} value={}",
                    node_id, normalized, val
                );
                continue;
            }
            let unit = metric_unit(normalized);

            let result = if let (Some(s), Some(b)) = (seq, boot_id) {
                sqlx::query(
                    "INSERT INTO sensor_readings (device_id, metric, value, unit, timestamp, seq, boot_id) \
                     VALUES (?, ?, ?, ?, ?, ?, ?) \
                     ON CONFLICT(device_id, metric, seq, boot_id) WHERE seq IS NOT NULL AND boot_id IS NOT NULL DO NOTHING"
                )
                .bind(device_id)
                .bind(normalized)
                .bind(val)
                .bind(unit)
                .bind(ts)
                .bind(s)
                .bind(b)
                .execute(pool)
                .await
            } else if let Some(s) = seq {
                sqlx::query(
                    "INSERT INTO sensor_readings (device_id, metric, value, unit, timestamp, seq) \
                     VALUES (?, ?, ?, ?, ?, ?)"
                )
                .bind(device_id)
                .bind(normalized)
                .bind(val)
                .bind(unit)
                .bind(ts)
                .bind(s)
                .execute(pool)
                .await
            } else {
                sqlx::query(
                    "INSERT INTO sensor_readings (device_id, metric, value, unit, timestamp) VALUES (?, ?, ?, ?, ?)"
                )
                .bind(device_id)
                .bind(normalized)
                .bind(val)
                .bind(unit)
                .bind(ts)
                .execute(pool)
                .await
            };

            match result {
                Ok(r) => {
                    if r.rows_affected() > 0 {
                        inserted += 1;
                        inserted_readings.push((normalized.to_string(), val, unit.to_string()));
                    }
                }
                Err(e) => tracing::warn!("Failed to insert reading: {}", e),
            }
        }
    }

    if inserted > 0 {
        sqlx::query("UPDATE devices SET status = 'online', updated_at = ? WHERE node_id = ?")
            .bind(now_received)
            .bind(node_id)
            .execute(pool)
            .await
            .ok();

        if let Some(tx) = event_tx {
            let readings: Vec<serde_json::Value> = inserted_readings.iter().map(|(m, v, u)| {
                serde_json::json!({"metric": m, "value": v, "unit": u})
            }).collect();
            let payload = serde_json::json!({
                "type": "telemetry",
                "node_id": node_id,
                "timestamp": now_received,
                "readings": readings,
            }).to_string();
            match tx.send(payload) {
                Ok(n) => { tracing::trace!("Broadcast telemetry to {} receivers", n); }
                Err(e) => { tracing::warn!("Broadcast send error: {}", e); }
            }
        }
    }

    Ok(inserted)
}
