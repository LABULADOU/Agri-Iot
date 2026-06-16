pub mod engine;
pub mod registry;
pub mod log;
pub mod approval;
pub mod notification;

use crate::decision::approval::ApprovalLevel;
use crate::state::AppState;
use anyhow::Result;
use engine::{DecisionFlow, Trigger};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

mod stages {
    pub mod llm_stage;
}

pub struct DecisionEngine {
    flows: Vec<DecisionFlow>,
    state_registry: Arc<Mutex<registry::StateRegistry>>,
    pool: sqlx::SqlitePool,
}

impl DecisionEngine {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self {
            flows: Vec::new(),
            state_registry: Arc::new(Mutex::new(registry::StateRegistry::new())),
            pool,
        }
    }

    pub fn register(&mut self, flow: DecisionFlow) {
        info!("Decision flow registered: {} (trigger={:?})", flow.name, flow.trigger);
        self.flows.push(flow);
    }

    pub fn state_registry(&self) -> Arc<Mutex<registry::StateRegistry>> {
        self.state_registry.clone()
    }

    pub fn pool(&self) -> &sqlx::SqlitePool {
        &self.pool
    }
}

pub async fn start(state: AppState) -> Result<()> {
    info!("Decision engine starting");

    let _reg = registry::StateRegistry::new();

    let _t1 = engine::DecisionFlow::builder("emergency_flow")
        .trigger(Trigger::PerTelemetry)
        .build();
    let _t2 = engine::DecisionFlow::builder("state_flow")
        .trigger(Trigger::OnStateChange)
        .build();

    // Tier 3: LLM 评估管线（1800s = 30 分钟）
    let mut t3 = engine::DecisionFlow::builder("assess_flow")
        .trigger(Trigger::Timed { interval_secs: 1800 })
        .approval(ApprovalLevel::Normal);

    if let Some(llm) = stages::llm_stage::create_llm_stage(&state) {
        t3 = t3.stage(Box::new(llm));
        info!("[decision] LlmStage registered for assess_flow");
    } else {
        info!("[decision] LlmStage skipped (LLM not configured), assess_flow runs without AI");
    }

    let _t3 = t3.build();

    info!("Decision flows registered: emergency(state), state(state), assess(1800s)");

    info!("Decision engine started");

    let pool = state.pool.clone();
    let event_tx = state.event_tx.clone();
    let event_rx = event_tx.subscribe();

    tokio::spawn(async move {
        let mut rx = event_rx;
        loop {
            match rx.recv().await {
                Ok(data) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                        if v.get("type").and_then(|t| t.as_str()) == Some("telemetry") {
                            let node_id = v.get("node_id").and_then(|n| n.as_str()).unwrap_or("");
                            let _ = (node_id, &pool);
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("Decision broadcast lagged by {}", n);
                }
                Err(_) => break,
            }
        }
    });

    Ok(())
}
