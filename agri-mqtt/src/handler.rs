use chrono::Utc;
use sqlx::SqlitePool;
use rumqttc::{Event, Incoming};
use tokio::sync::{broadcast, mpsc};
use tracing::info;

use agri_core::adaptor::{PayloadAdaptor, JsonPayloadAdaptor};
use agri_core::topics;

struct MqttMessage {
    node_id: String,
    topic_type: String,
    payload: Vec<u8>,
}

async fn auto_register_device(
    pool: &SqlitePool,
    node_id: &str,
    capabilities: &[&str],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let exists: bool = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM devices WHERE node_id = ?",
    )
    .bind(node_id)
    .fetch_one(pool)
    .await
    .map(|c| c > 0)
    .unwrap_or(false);

    if !exists {
        let id = uuid::Uuid::new_v4();
        let now = Utc::now().timestamp();
        let caps = serde_json::to_string(capabilities).unwrap_or_else(|_| "[\"sensor\"]".to_string());
        sqlx::query(
            "INSERT INTO devices (id, name, node_id, device_type, status, capabilities, created_at, updated_at) \
             VALUES (?, ?, ?, 'sensor', 'online', ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(node_id)
        .bind(node_id)
        .bind(&caps)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;
        info!("Auto-registered device {} ({})", node_id, id);
    }

    Ok(())
}

pub async fn handle_telemetry(
    pool: &SqlitePool,
    node_id: &str,
    payload: &[u8],
    event_tx: Option<&broadcast::Sender<String>>,
    adaptor: &dyn PayloadAdaptor,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let parsed = adaptor.parse_telemetry(node_id, payload)?;

    auto_register_device(pool, node_id, &["sensor"]).await?;

    let inserted = agri_core::telemetry::process_telemetry(
        pool, &parsed.node_id, &parsed.metrics, event_tx, parsed.seq, parsed.boot_id.as_deref(),
    ).await?;

    if inserted > 0 {
        info!("Stored {} readings for node {} (seq={:?})", inserted, node_id, parsed.seq);
    }

    Ok(())
}

pub async fn handle_gateway_telemetry(
    pool: &SqlitePool,
    gateway_id: &str,
    payload: &[u8],
    event_tx: Option<&broadcast::Sender<String>>,
    adaptor: &dyn PayloadAdaptor,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let devices = adaptor.parse_gateway_telemetry(gateway_id, payload)?;

    for device in &devices {
        auto_register_device(pool, &device.node_id, &["sensor"]).await?;

        let inserted = agri_core::telemetry::process_telemetry(
            pool, &device.node_id, &device.metrics, event_tx, device.seq, device.boot_id.as_deref(),
        ).await?;

        if inserted > 0 {
            info!("[gateway {}] Stored {} readings for sub-device {} (seq={:?})",
                gateway_id, inserted, device.node_id, device.seq);
        }
    }

    Ok(())
}

pub async fn handle_status_change(
    pool: &SqlitePool,
    node_id: &str,
    payload: &[u8],
    adaptor: &dyn PayloadAdaptor,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db_status = adaptor.parse_status(node_id, payload)?;
    let now = Utc::now().timestamp();

    sqlx::query(
        "UPDATE devices SET status = ?, updated_at = ? WHERE node_id = ?",
    )
    .bind(&db_status)
    .bind(now)
    .bind(node_id)
    .execute(pool)
    .await?;

    info!("Device {} status changed to {}", node_id, db_status);

    Ok(())
}

pub async fn start_listener(
    mut eventloop: rumqttc::EventLoop,
    pool: SqlitePool,
    event_tx: Option<broadcast::Sender<String>>,
) {
    let (tx, mut rx) = mpsc::channel::<MqttMessage>(1024);

    let worker_pool = pool.clone();
    let worker_tx = event_tx.clone();
    let worker_adaptor = JsonPayloadAdaptor::new();

    tokio::spawn(async move {
        info!("MQTT worker started");
        while let Some(msg) = rx.recv().await {
            match msg.topic_type.as_str() {
                "telemetry" => {
                    if let Err(e) = handle_telemetry(
                        &worker_pool, &msg.node_id, &msg.payload, worker_tx.as_ref(), &worker_adaptor,
                    ).await {
                        tracing::warn!("Worker telemetry error: {}", e);
                    }
                }
                "gateway_telemetry" => {
                    if let Err(e) = handle_gateway_telemetry(
                        &worker_pool, &msg.node_id, &msg.payload, worker_tx.as_ref(), &worker_adaptor,
                    ).await {
                        tracing::warn!("Worker gateway telemetry error: {}", e);
                    }
                }
                "status" => {
                    if let Err(e) = handle_status_change(
                        &worker_pool, &msg.node_id, &msg.payload, &worker_adaptor,
                    ).await {
                        tracing::warn!("Worker status error: {}", e);
                    }
                }
                _ => {}
            }
        }
        tracing::warn!("MQTT worker channel closed");
    });

    info!("MQTT listener started, waiting for messages");

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Incoming::Publish(p))) => {
                let topic = p.topic;
                let payload = p.payload.to_vec();

                match topics::match_topic(&topic) {
                    Some(topics::TopicMatch::Telemetry { node_id }) => {
                        let msg = MqttMessage {
                            node_id,
                            topic_type: "telemetry".to_string(),
                            payload,
                        };
                        if tx.send(msg).await.is_err() {
                            tracing::warn!("MQTT channel closed, stopping listener");
                            break;
                        }
                    }
                    Some(topics::TopicMatch::GatewayTelemetry { gateway_id }) => {
                        let msg = MqttMessage {
                            node_id: gateway_id,
                            topic_type: "gateway_telemetry".to_string(),
                            payload,
                        };
                        if tx.send(msg).await.is_err() {
                            tracing::warn!("MQTT channel closed, stopping listener");
                            break;
                        }
                    }
                    Some(topics::TopicMatch::Command { node_id: _ }) => {
                        // Commands are published TO devices, not received FROM them
                    }
                    Some(topics::TopicMatch::Status { node_id }) | Some(topics::TopicMatch::GatewayStatus { gateway_id: node_id }) => {
                        let msg = MqttMessage {
                            node_id,
                            topic_type: "status".to_string(),
                            payload,
                        };
                        if tx.send(msg).await.is_err() {
                            tracing::warn!("MQTT channel closed, stopping listener");
                            break;
                        }
                    }
                    None => {}
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

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        sqlx::query(
            "CREATE TABLE devices (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                node_id TEXT NOT NULL UNIQUE,
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
            "CREATE TABLE sensor_readings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL,
                metric TEXT NOT NULL,
                value REAL NOT NULL,
                unit TEXT DEFAULT '',
                timestamp INTEGER NOT NULL,
                seq INTEGER,
                boot_id TEXT
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        let now = Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO devices (id, name, node_id, device_type, status, capabilities, created_at, updated_at) 
             VALUES ('test-device-uuid', 'Test Sensor', 'node-001', 'sensor', 'online', '[\"sensor\"]', ?1, ?1)"
        )
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_handle_telemetry_valid() {
        let pool = setup_test_db().await;
        let adaptor = JsonPayloadAdaptor::new();
        let payload = br#"{"metrics": {"temperature": 25.5, "humidity": 60.0}}"#;
        let result = handle_telemetry(&pool, "node-001", payload, None, &adaptor).await;
        assert!(result.is_ok());

        let readings: Vec<(String, f64)> = sqlx::query_as(
            "SELECT metric, value FROM sensor_readings ORDER BY metric"
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(readings.len(), 2);
        assert_eq!(readings[0].1, 60.0);
        assert_eq!(readings[1].1, 25.5);
    }

    #[tokio::test]
    async fn test_handle_telemetry_invalid_json() {
        let pool = setup_test_db().await;
        let adaptor = JsonPayloadAdaptor::new();
        let payload = b"not a valid json";
        let result = handle_telemetry(&pool, "node-001", payload, None, &adaptor).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_telemetry_auto_register() {
        let pool = setup_test_db().await;
        let adaptor = JsonPayloadAdaptor::new();
        let payload = br#"{"metrics": {"temperature": 25.0}}"#;
        let result = handle_telemetry(&pool, "new-node", payload, None, &adaptor).await;
        assert!(result.is_ok());

        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM devices WHERE node_id = 'new-node'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count.0, 1);
    }

    #[tokio::test]
    async fn test_handle_gateway_telemetry() {
        let pool = setup_test_db().await;
        let adaptor = JsonPayloadAdaptor::new();
        let payload = br#"{"devices": [
            {"node_id": "sub-1", "metrics": {"temperature": 22.0}},
            {"node_id": "sub-2", "metrics": {"humidity": 65.0}}
        ], "seq": 10}"#;
        let result = handle_gateway_telemetry(&pool, "gw-001", payload, None, &adaptor).await;
        assert!(result.is_ok());

        let devices: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM devices WHERE node_id IN ('sub-1', 'sub-2')"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(devices, 2);

        let readings: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sensor_readings")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(readings, 2);
    }

    #[tokio::test]
    async fn test_handle_status_change_online() {
        let pool = setup_test_db().await;
        let adaptor = JsonPayloadAdaptor::new();
        let result = handle_status_change(&pool, "node-001", b"online", &adaptor).await;
        assert!(result.is_ok());

        let status: (String,) = sqlx::query_as(
            "SELECT status FROM devices WHERE node_id = 'node-001'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(status.0, "online");
    }

    #[tokio::test]
    async fn test_handle_status_change_json() {
        let pool = setup_test_db().await;
        let adaptor = JsonPayloadAdaptor::new();
        let result = handle_status_change(&pool, "node-001", br#"{"status": "online"}"#, &adaptor).await;
        assert!(result.is_ok());

        let status: (String,) = sqlx::query_as(
            "SELECT status FROM devices WHERE node_id = 'node-001'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(status.0, "online");
    }

    #[test]
    fn test_topic_matching() {
        use agri_core::topics;
        assert!(matches!(
            topics::match_topic("agri/node/esp32-001/telemetry"),
            Some(topics::TopicMatch::Telemetry { node_id }) if node_id == "esp32-001"
        ));
        assert!(matches!(
            topics::match_topic("agri/gateway/gw-001/telemetry"),
            Some(topics::TopicMatch::GatewayTelemetry { gateway_id }) if gateway_id == "gw-001"
        ));
    }
}
