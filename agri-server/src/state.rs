use agri_core::models::Rule;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub mqtt_client: Arc<Mutex<Option<rumqttc::AsyncClient>>>,
    pub rules_cache: Arc<Mutex<Vec<Rule>>>,
    pub event_tx: broadcast::Sender<String>,
}

impl AppState {
    pub fn new(pool: SqlitePool, client: rumqttc::AsyncClient) -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            pool,
            mqtt_client: Arc::new(Mutex::new(Some(client))),
            rules_cache: Arc::new(Mutex::new(Vec::new())),
            event_tx: tx,
        }
    }
}
