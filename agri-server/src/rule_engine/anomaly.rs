use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tokio::sync::broadcast;

/// Evidence types collected by detectors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AnomalyType {
    /// E1: DHT22 dual-zero (handled in telemetry.rs)
    Dht22Fault,
    /// E2: Sudden rate-of-change jump
    RateAnomaly,
    /// E3: Significant deviation from neighbor nodes
    SpatialAnomaly,
    /// E5: A metric that was previously reporting has stopped
    MetricSilent,
}

/// Severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

/// An anomaly event emitted by the detection engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyEvent {
    pub node_id: String,
    pub metric: String,
    pub anomaly_type: AnomalyType,
    pub severity: Severity,
    pub value_original: Option<f64>,
    pub message: String,
    pub timestamp: i64,
}

/// Dedup tracker: prevents alert storms for the same (node_id, metric, type)
struct DedupTracker {
    fired_at: HashMap<(String, String, AnomalyType), i64>,
}

impl DedupTracker {
    fn new() -> Self {
        Self { fired_at: HashMap::new() }
    }

    /// Returns true if this event should be dispatched (not yet fired within cooldown)
    fn should_fire(&mut self, node_id: &str, metric: &str, at: AnomalyType, cooldown_secs: i64) -> bool {
        let key = (node_id.to_string(), metric.to_string(), at);
        let now = Utc::now().timestamp();
        if let Some(last) = self.fired_at.get(&key) {
            if now - *last < cooldown_secs {
                return false;
            }
        }
        self.fired_at.insert(key, now);
        true
    }
}

static DEDUP: OnceLock<Mutex<DedupTracker>> = OnceLock::new();

fn with_dedup<F>(f: F)
where
    F: FnOnce(&mut DedupTracker),
{
    let dedup = DEDUP.get_or_init(|| Mutex::new(DedupTracker::new()));
    if let Ok(mut guard) = dedup.lock() {
        f(&mut *guard);
    }
}

fn dispatch(dedup: &mut DedupTracker, pool: &SqlitePool, event: &AnomalyEvent, tx: &broadcast::Sender<String>) {
    let cooldown = match event.severity {
        Severity::Critical => 300,  // 5 min
        Severity::Warning => 600,   // 10 min
        Severity::Info => 1800,     // 30 min
    };
    if !dedup.should_fire(&event.node_id, &event.metric, event.anomaly_type.clone(), cooldown) {
        return;
    }

    // Persist to anomaly_events table
    let at_str = format!("{:?}", event.anomaly_type);
    let sev_str = format!("{:?}", event.severity);
    let _ = sqlx::query(
        "INSERT INTO anomaly_events (device_id, node_id, metric, anomaly_type, severity, value_original, message, created_at) \
         VALUES ((SELECT id FROM devices WHERE node_id = ?), ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&event.node_id)
    .bind(&event.node_id)
    .bind(&event.metric)
    .bind(&at_str)
    .bind(&sev_str)
    .bind(event.value_original)
    .bind(&event.message)
    .bind(event.timestamp)
    .execute(pool);

    // Broadcast SSE event
    let payload = serde_json::json!({
        "type": "anomaly",
        "node_id": event.node_id,
        "metric": event.metric,
        "anomaly_type": at_str,
        "severity": sev_str,
        "value_original": event.value_original,
        "message": event.message,
        "timestamp": event.timestamp,
    }).to_string();
    match tx.send(payload) {
        Ok(n) => tracing::info!("Anomaly broadcast to {} receivers: {}", n, event.message),
        Err(e) => tracing::warn!("Anomaly broadcast error: {}", e),
    }
}

/// Main entry: run all anomaly detectors. Called every 60s from rule engine timer.
pub async fn run_anomaly_detection(pool: &SqlitePool, event_tx: &broadcast::Sender<String>) {
    // E5: Metric silence detection
    if let Err(e) = detect_metric_silence(pool, event_tx).await {
        tracing::warn!("metric_silence check failed: {}", e);
    }
    // E2 + E3: Rate anomaly + spatial cross-check
    if let Err(e) = detect_rate_and_spatial(pool, event_tx).await {
        tracing::warn!("rate/spatial check failed: {}", e);
    }
}

/// E5: Detect metrics that have stopped reporting.
/// A device is "online" but one of its metrics hasn't been seen in >10 minutes.
async fn detect_metric_silence(
    pool: &SqlitePool,
    event_tx: &broadcast::Sender<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let now = Utc::now().timestamp();
    let one_hour_ago = now - 3600;
    let silence_cutoff = now - 600; // 10 min

    // Find online devices with metrics that have gone silent
    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT d.node_id, sr.metric, MAX(sr.timestamp) as last_seen \
         FROM sensor_readings sr \
         JOIN devices d ON d.id = sr.device_id \
         WHERE d.status = 'online' \
         AND sr.timestamp > ? \
         AND sr.metric IN ('temperature', 'humidity', 'soil_moisture', 'soil_temperature', 'ec') \
         GROUP BY d.node_id, sr.metric \
         HAVING MAX(sr.timestamp) < ?"
    )
    .bind(one_hour_ago)
    .bind(silence_cutoff)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(());
    }

    with_dedup(|dedup| {
        for (node_id, metric, last_seen) in &rows {
            let event = AnomalyEvent {
                node_id: node_id.clone(),
                metric: metric.clone(),
                anomaly_type: AnomalyType::MetricSilent,
                severity: Severity::Warning,
                value_original: None,
                message: format!(
                    "{} has not reported '{}' in {} minutes (device is still online)",
                    node_id, metric, (now - last_seen) / 60
                ),
                timestamp: now,
            };
            dispatch(dedup, pool, &event, event_tx);
        }
    });

    Ok(())
}

/// E2 + E3: Detect rate-of-change anomalies and spatial cross-check.
///
/// E2: If a single reading jumped abnormally compared to prior value.
/// E3: Cross-reference with neighbor nodes in same area.
/// Together they distinguish sensor faults from real environmental events.
async fn detect_rate_and_spatial(
    pool: &SqlitePool,
    event_tx: &broadcast::Sender<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let now = Utc::now().timestamp();
    let recent = now - 120; // look at last 2 minutes
    // Find devices with area_id (spatial check needs neighbors in same area)
    let devices: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, node_id, area_id FROM devices WHERE status = 'online'"
    )
    .fetch_all(pool)
    .await?;

    // Build area → [node_id] map for spatial lookups
    let mut area_nodes: HashMap<String, Vec<String>> = HashMap::new();
    for (_id, node_id, area_id) in &devices {
        if let Some(aid) = area_id {
            area_nodes.entry(aid.clone()).or_default().push(node_id.clone());
        }
    }

    for (device_id, node_id, area_id) in &devices {
        // E2: Rate anomaly — check temperature and humidity for sudden jumps
        for metric in &["temperature", "humidity"] {
            // Get most recent reading
            let current: Option<(f64, i64)> = sqlx::query_as(
                "SELECT value, timestamp FROM sensor_readings \
                 WHERE device_id = ? AND metric = ? AND timestamp > ? \
                 ORDER BY timestamp DESC LIMIT 1"
            )
            .bind(device_id)
            .bind(metric)
            .bind(recent)
            .fetch_optional(pool)
            .await?;

            if let Some((cur_val, cur_ts)) = current {
                // Get the reading before it
                let prev: Option<(f64,)> = sqlx::query_as(
                    "SELECT value FROM sensor_readings \
                     WHERE device_id = ? AND metric = ? AND timestamp < ? \
                     ORDER BY timestamp DESC LIMIT 1"
                )
                .bind(device_id)
                .bind(metric)
                .bind(cur_ts)
                .fetch_optional(pool)
                .await?;

                if let Some((prev_val,)) = prev {
                    let delta = (cur_val - prev_val).abs();
                    let (rate_threshold, spatial_threshold) = match *metric {
                        "temperature" => (2.0, 1.0),  // 2°C/10s = anomaly, 1°C from neighbor = spatial
                        "humidity" => (5.0, 5.0),     // 5%/10s = anomaly, 5% from neighbor = spatial
                        _ => continue,
                    };

                    if delta >= rate_threshold {
                        // Rate anomaly detected → check spatial (E3)
                        let mut spatial_anomaly = false;
                        if let Some(aid) = area_id {
                            if let Some(neighbors) = area_nodes.get(aid) {
                                let mut neighbor_vals: Vec<f64> = Vec::new();
                                for nid in neighbors {
                                    if *nid == *node_id { continue; }
                                    let nv: Option<(f64,)> = sqlx::query_as(
                                        "SELECT sr.value FROM sensor_readings sr \
                                         JOIN devices d ON d.id = sr.device_id \
                                         WHERE d.node_id = ? AND sr.metric = ? AND sr.timestamp > ? \
                                         ORDER BY sr.timestamp DESC LIMIT 1"
                                    )
                                    .bind(nid)
                                    .bind(metric)
                                    .bind(recent)
                                    .fetch_optional(pool)
                                    .await
                                    .unwrap_or(None);
                                    if let Some((v,)) = nv {
                                        neighbor_vals.push(v);
                                    }
                                }
                                if !neighbor_vals.is_empty() {
                                    let neighbor_median = median(&mut neighbor_vals);
                                    if (cur_val - neighbor_median).abs() >= spatial_threshold {
                                        spatial_anomaly = true;
                                    }
                                }
                            }
                        }

                        // Determine severity and message based on spatial check
                        let (at, sev, msg) = if spatial_anomaly {
                            // Rate + Spatial → likely sensor fault, not real event
                            (AnomalyType::SpatialAnomaly, Severity::Warning,
                             format!("{} {} jumped {:.1}→{:.1} (Δ={:.1}) and deviates from neighbor — possible sensor fault",
                                     node_id, metric, prev_val, cur_val, delta))
                        } else {
                            // Rate only → could be real environmental event (spraying, irrigation)
                            (AnomalyType::RateAnomaly, Severity::Info,
                             format!("{} {} changed {:.1}→{:.1} (Δ={:.1}), neighbors agree — likely real event",
                                     node_id, metric, prev_val, cur_val, delta))
                        };

                        with_dedup(|dedup| {
                            dispatch(dedup, pool, &AnomalyEvent {
                                node_id: node_id.clone(),
                                metric: metric.to_string(),
                                anomaly_type: at,
                                severity: sev,
                                value_original: Some(cur_val),
                                message: msg,
                                timestamp: now,
                            }, event_tx);
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

fn median(vals: &mut Vec<f64>) -> f64 {
    if vals.is_empty() { return 0.0; }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let len = vals.len();
    if len % 2 == 0 {
        (vals[len / 2 - 1] + vals[len / 2]) / 2.0
    } else {
        vals[len / 2]
    }
}
