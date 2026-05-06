use agri_core::models::Rule;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub mqtt_client: Arc<Mutex<Option<rumqttc::AsyncClient>>>,
    pub rules_cache: Arc<Mutex<Vec<Rule>>>,
}

impl AppState {
    pub fn new(pool: SqlitePool, client: rumqttc::AsyncClient) -> Self {
        Self {
            pool,
            mqtt_client: Arc::new(Mutex::new(Some(client))),
            rules_cache: Arc::new(Mutex::new(Vec::new())),
        }
    }
}
