use chrono::Utc;
use sqlx::SqlitePool;
use tokio::sync::broadcast;

const METRIC_MAP: &[(&str, &str)] = &[
    ("air_temp", "temperature"),
    ("air_humidity", "humidity"),
    ("soil_temp", "soil_temperature"),
];
const KNOWN_METRICS: &[&str] = &["temperature", "humidity", "soil_moisture", "soil_temperature", "ec", "light", "rssi", "relay_state", "dht_status"];

pub fn normalize_metric(name: &str) -> &str {
    METRIC_MAP.iter().find(|(k, _)| *k == name).map(|(_, v)| *v).unwrap_or(name)
}

pub fn is_known_metric(name: &str) -> bool {
    KNOWN_METRICS.contains(&name)
}

pub fn validate_value(metric: &str, val: f64) -> bool {
    match metric {
        "temperature" | "soil_temperature" => (-5.0..=50.0).contains(&val),
        "humidity" | "soil_moisture" => (0.0..=100.0).contains(&val),
        "ec" => (0.0..=10.0).contains(&val),
        "light" => (0.0..=200000.0).contains(&val),
        "rssi" => (-120.0..=0.0).contains(&val),
        "relay_state" => val == 0.0 || val == 1.0,
        "dht_status" => val == 0.0 || val == 1.0,
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
        "dht_status" => "",
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
    let mut devices = sqlx::query_as::<_, (String, String)>(
        "SELECT id, node_id FROM devices WHERE node_id = ?",
    )
    .bind(node_id)
    .fetch_all(pool)
    .await?;

    if devices.is_empty() {
        // 自动注册（与 MQTT 入口一致）：首次上报遥测的设备自动创建 sensor 记录
        let id = uuid::Uuid::new_v4();
        let now = Utc::now().timestamp();
        let caps = "[\"sensor\"]";
        let _ = sqlx::query(
            "INSERT INTO devices (id, name, node_id, device_type, status, capabilities, created_at, updated_at) \
             VALUES (?, ?, ?, 'sensor', 'online', ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(node_id)
        .bind(node_id)
        .bind(caps)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;
        tracing::info!("Auto-registered device {} ({}) via HTTP telemetry", node_id, id);
        // 重新查询，确保下方循环拿到新设备 id（否则 inserted 恒为 0）
        devices = sqlx::query_as::<_, (String, String)>(
            "SELECT id, node_id FROM devices WHERE node_id = ?",
        )
        .bind(node_id)
        .fetch_all(pool)
        .await?;
    }

    let now_received = Utc::now().timestamp();
    let ts = captured_at.unwrap_or(now_received);
    let mut inserted: i64 = 0;
    let mut inserted_readings: Vec<(String, f64, String, i64)> = Vec::new();

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
            if m == "last_cmd" {
                tracing::info!("Node {} last_cmd: {:?}", node_id, value);
            }
            if !is_known_metric(normalized) { continue; }

            // Under DHT22 fault, skip air temperature and humidity (other metrics still valid)
            if dht22_fault && (normalized == "temperature" || normalized == "humidity") {
                continue;
            }

            let mut val = match value {
                serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
                serde_json::Value::Bool(b) => if *b { 1.0 } else { 0.0 },
                serde_json::Value::String(s) => {
                    // Handle string status metrics: "failed" → 1.0, others → 0.0
                    if normalized == "dht_status" {
                        if s == "failed" { 1.0 } else { 0.0 }
                    } else {
                        s.parse::<f64>().unwrap_or(0.0)
                    }
                }
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
                        inserted_readings.push((normalized.to_string(), val, unit.to_string(), ts));
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
            let readings: Vec<serde_json::Value> = inserted_readings.iter().map(|(m, v, u, t)| {
                serde_json::json!({"metric": m, "value": v, "unit": u, "timestamp": t})
            }).collect();
            let payload = serde_json::json!({
                "type": "telemetry",
                "node_id": node_id,
                "timestamp": ts,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ===== normalize_metric =====

    #[test]
    fn test_normalize_air_temp() {
        assert_eq!(normalize_metric("air_temp"), "temperature");
    }

    #[test]
    fn test_normalize_air_humidity() {
        assert_eq!(normalize_metric("air_humidity"), "humidity");
    }

    #[test]
    fn test_normalize_soil_temp() {
        assert_eq!(normalize_metric("soil_temp"), "soil_temperature");
    }

    #[test]
    fn test_normalize_known_passthrough() {
        assert_eq!(normalize_metric("temperature"), "temperature");
        assert_eq!(normalize_metric("ec"), "ec");
        assert_eq!(normalize_metric("rssi"), "rssi");
    }

    #[test]
    fn test_normalize_unknown_passthrough() {
        assert_eq!(normalize_metric("unknown_metric"), "unknown_metric");
    }

    // ===== is_known_metric =====

    #[test]
    fn test_is_known_metric_valid() {
        assert!(is_known_metric("temperature"));
        assert!(is_known_metric("humidity"));
        assert!(is_known_metric("soil_moisture"));
        assert!(is_known_metric("soil_temperature"));
        assert!(is_known_metric("ec"));
        assert!(is_known_metric("light"));
        assert!(is_known_metric("rssi"));
        assert!(is_known_metric("relay_state"));
    }

    #[test]
    fn test_is_known_metric_invalid() {
        assert!(!is_known_metric("air_temp"));  // raw name, not normalized
        assert!(!is_known_metric("pressure"));
        assert!(!is_known_metric(""));
    }

    // ===== validate_value =====

    #[test]
    fn test_validate_temperature_boundaries() {
        assert!(validate_value("temperature", -5.0));   // min
        assert!(validate_value("temperature", 50.0));    // max
        assert!(validate_value("temperature", 25.0));    // normal
        assert!(!validate_value("temperature", -5.1));   // below min
        assert!(!validate_value("temperature", 50.1));   // above max
    }

    #[test]
    fn test_validate_humidity_boundaries() {
        assert!(validate_value("humidity", 0.0));
        assert!(validate_value("humidity", 100.0));
        assert!(validate_value("humidity", 65.0));
        assert!(!validate_value("humidity", -0.1));
        assert!(!validate_value("humidity", 100.1));
    }

    #[test]
    fn test_validate_ec() {
        assert!(validate_value("ec", 0.0));
        assert!(validate_value("ec", 10.0));
        assert!(!validate_value("ec", -0.1));
        assert!(!validate_value("ec", 10.1));
    }

    #[test]
    fn test_validate_light() {
        assert!(validate_value("light", 0.0));
        assert!(validate_value("light", 200000.0));
        assert!(!validate_value("light", -1.0));
        assert!(!validate_value("light", 200001.0));
    }

    #[test]
    fn test_validate_rssi() {
        assert!(validate_value("rssi", -120.0));
        assert!(validate_value("rssi", 0.0));
        assert!(validate_value("rssi", -70.0));
        assert!(!validate_value("rssi", -120.1));
        assert!(!validate_value("rssi", 0.1));
    }

    #[test]
    fn test_validate_relay_state() {
        assert!(validate_value("relay_state", 0.0));
        assert!(validate_value("relay_state", 1.0));
        assert!(!validate_value("relay_state", 0.5));
        assert!(!validate_value("relay_state", 2.0));
    }

    #[test]
    fn test_validate_unknown_always_pass() {
        assert!(validate_value("unknown_metric", 99999.0));
    }

    // ===== is_dht22_dual_zero =====

    #[test]
    fn test_dual_zero_with_normalized_keys() {
        let metrics = json!({"temperature": 0.0, "humidity": 0.0}).as_object().unwrap().clone();
        assert!(is_dht22_dual_zero(&metrics));
    }

    #[test]
    fn test_dual_zero_with_raw_keys() {
        let metrics = json!({"air_temp": 0.0, "air_humidity": 0.0}).as_object().unwrap().clone();
        assert!(is_dht22_dual_zero(&metrics));
    }

    #[test]
    fn test_dual_zero_mixed_keys() {
        let metrics = json!({"temperature": 0.0, "air_humidity": 0.0}).as_object().unwrap().clone();
        assert!(is_dht22_dual_zero(&metrics));
    }

    #[test]
    fn test_not_dual_zero_nonzero_temp() {
        let metrics = json!({"temperature": 25.0, "humidity": 0.0}).as_object().unwrap().clone();
        assert!(!is_dht22_dual_zero(&metrics));
    }

    #[test]
    fn test_not_dual_zero_nonzero_hum() {
        let metrics = json!({"temperature": 0.0, "humidity": 60.0}).as_object().unwrap().clone();
        assert!(!is_dht22_dual_zero(&metrics));
    }

    #[test]
    fn test_not_dual_zero_missing_keys() {
        let metrics = json!({"temperature": 0.0}).as_object().unwrap().clone();
        assert!(!is_dht22_dual_zero(&metrics));
    }

    #[test]
    fn test_not_dual_zero_empty() {
        let metrics = json!({}).as_object().unwrap().clone();
        assert!(!is_dht22_dual_zero(&metrics));
    }

    #[test]
    fn test_not_dual_zero_string_values() {
        let metrics = json!({"temperature": "0", "humidity": "0"}).as_object().unwrap().clone();
        assert!(!is_dht22_dual_zero(&metrics));
    }

    // ===== metric_unit =====

    #[test]
    fn test_metric_units() {
        assert_eq!(metric_unit("temperature"), "℃");
        assert_eq!(metric_unit("soil_temperature"), "℃");
        assert_eq!(metric_unit("humidity"), "%");
        assert_eq!(metric_unit("soil_moisture"), "%");
        assert_eq!(metric_unit("light"), "lux");
        assert_eq!(metric_unit("ec"), "mS/cm");
        assert_eq!(metric_unit("rssi"), "dBm");
        assert_eq!(metric_unit("relay_state"), "");
        assert_eq!(metric_unit("unknown"), "");
    }

    // ===== maybe_convert_ec =====

    #[test]
    fn test_maybe_convert_ec() {
        assert_eq!(maybe_convert_ec("ec", 1000.0), 1.0);
        assert_eq!(maybe_convert_ec("ec", 500.0), 0.5);
        assert_eq!(maybe_convert_ec("ec", 0.0), 0.0);
    }

    #[test]
    fn test_maybe_convert_non_ec_passthrough() {
        assert_eq!(maybe_convert_ec("temperature", 25.0), 25.0);
        assert_eq!(maybe_convert_ec("humidity", 60.0), 60.0);
    }
}
