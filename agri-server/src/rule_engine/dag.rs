use anyhow::Context;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TbMsgType {
    Telemetry,
    TimerTick,
    DeviceOnline,
    DeviceOffline,
    Emergency,
    Status,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TbMsg {
    pub id: String,
    pub msg_type: TbMsgType,
    pub originator: String,
    pub data: serde_json::Value,
    pub metadata: HashMap<String, String>,
}

impl TbMsg {
    pub fn new(originator: &str, msg_type: TbMsgType, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            msg_type,
            originator: originator.to_string(),
            data,
            metadata: HashMap::new(),
        }
    }

    pub fn get_metric(&self, name: &str) -> Option<f64> {
        if let Some(obj) = self.data.as_object() {
            if let Some(v) = obj.get(name).and_then(|v| v.as_f64()) {
                return Some(v);
            }
        }
        if let Some(readings) = self.data.get("readings").and_then(|r| r.as_array()) {
            for r in readings {
                if r.get("metric").and_then(|m| m.as_str()) == Some(name) {
                    return r.get("value").and_then(|v| v.as_f64());
                }
            }
        }
        None
    }
}

pub struct NodeContext {
    pub pool: sqlx::SqlitePool,
    pub mqtt_client: Arc<Mutex<Option<rumqttc::AsyncClient>>>,
    pub event_tx: tokio::sync::broadcast::Sender<String>,
}

#[async_trait]
pub trait RuleNode: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    async fn on_msg(&self, ctx: &NodeContext, msg: TbMsg) -> Vec<TbMsg>;
}

pub struct RuleChain {
    entry_indices: Vec<usize>,
    nodes: Vec<Box<dyn RuleNode>>,
    edges: Vec<(usize, usize)>,
    ctx: Arc<NodeContext>,
}

impl RuleChain {
    pub fn new(ctx: NodeContext) -> Self {
        Self { nodes: vec![], edges: vec![], entry_indices: vec![], ctx: Arc::new(ctx) }
    }

    pub fn add_node(&mut self, node: Box<dyn RuleNode>) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(node);
        self.rebuild_entry_indices();
        idx
    }

    pub fn add_edge(&mut self, from: usize, to: usize) {
        self.edges.push((from, to));
    }

    fn rebuild_entry_indices(&mut self) {
        let has_incoming: Vec<bool> = (0..self.nodes.len())
            .map(|i| self.edges.iter().any(|(_, to)| *to == i))
            .collect();
        self.entry_indices = (0..self.nodes.len()).filter(|i| !has_incoming[*i]).collect();
    }

    pub async fn process_async(&self, msg: TbMsg) -> anyhow::Result<()> {
        let ctx = &self.ctx;
        let node_count = self.nodes.len();
        anyhow::ensure!(node_count > 0, "empty rule chain");

        let mut stack: Vec<(usize, TbMsg)> = Vec::new();

        for entry_idx in &self.entry_indices {
            if *entry_idx < node_count {
                let results = self.nodes[*entry_idx].on_msg(ctx, msg.clone()).await;
                for out in results {
                    for (from, to) in &self.edges {
                        if *from == *entry_idx && *to < node_count {
                            stack.push((*to, out.clone()));
                        }
                    }
                }
            }
        }

        let mut visited = vec![false; node_count];
        while let Some((idx, m)) = stack.pop() {
            if idx >= node_count || visited[idx] { continue; }
            visited[idx] = true;
            let results = self.nodes[idx].on_msg(ctx, m).await;
            for out in results {
                for (from, to) in &self.edges {
                    if *from == idx && *to < node_count {
                        stack.push((*to, out.clone()));
                    }
                }
            }
        }
        Ok(())
    }
}

pub fn telemetry_to_tbmsg(node_id: &str, data: serde_json::Value) -> TbMsg {
    TbMsg::new(node_id, TbMsgType::Telemetry, data)
}
