use agri_core::ai::emergency::EmergencyContext;
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
    pub obsidian_vault_path: Option<String>,
    pub emergency_ctx: Arc<Mutex<EmergencyContext>>,
}

impl AppState {
    pub fn new(pool: SqlitePool, client: rumqttc::AsyncClient) -> Self {
        let (tx, _) = broadcast::channel(256);
        let vault_path = std::env::var("OBSIDIAN_VAULT_PATH").ok();
        Self {
            pool,
            mqtt_client: Arc::new(Mutex::new(Some(client))),
            rules_cache: Arc::new(Mutex::new(Vec::new())),
            event_tx: tx,
            obsidian_vault_path: vault_path,
            emergency_ctx: Arc::new(Mutex::new(EmergencyContext::new())),
        }
    }
}
