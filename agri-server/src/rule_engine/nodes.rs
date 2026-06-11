use async_trait::async_trait;
use crate::rule_engine::dag::{NodeContext, RuleNode, TbMsg, TbMsgType};
use agri_mqtt::client::publish_command;

pub struct MsgTypeFilterNode {
    id: String,
    name: String,
    allowed_types: Vec<TbMsgType>,
}

impl MsgTypeFilterNode {
    pub fn new(id: &str, name: &str, allowed_types: Vec<TbMsgType>) -> Self {
        Self { id: id.to_string(), name: name.to_string(), allowed_types }
    }
}

#[async_trait]
impl RuleNode for MsgTypeFilterNode {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.name }

    async fn on_msg(&self, _ctx: &NodeContext, msg: TbMsg) -> Vec<TbMsg> {
        let matched = self.allowed_types.contains(&msg.msg_type);
        tracing::debug!("[DAG] filter {} | type={:?} → {}", self.name, msg.msg_type, if matched { "PASS" } else { "BLOCK" });
        if matched {
            vec![msg]
        } else {
            vec![]
        }
    }
}

pub struct ConditionNode {
    id: String,
    name: String,
    metric: String,
    operator: String,
    threshold: f64,
}

impl ConditionNode {
    pub fn new(id: &str, name: &str, metric: &str, operator: &str, threshold: f64) -> Self {
        Self { id: id.to_string(), name: name.to_string(), metric: metric.to_string(), operator: operator.to_string(), threshold }
    }

    fn evaluate(&self, value: f64) -> bool {
        match self.operator.as_str() {
            ">" => value > self.threshold,
            ">=" => value >= self.threshold,
            "<" => value < self.threshold,
            "<=" => value <= self.threshold,
            "==" => (value - self.threshold).abs() < 0.001,
            _ => false,
        }
    }
}

#[async_trait]
impl RuleNode for ConditionNode {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.name }

    async fn on_msg(&self, _ctx: &NodeContext, msg: TbMsg) -> Vec<TbMsg> {
        if let Some(val) = msg.get_metric(&self.metric) {
            let matched = self.evaluate(val);
            tracing::debug!("[DAG] cond {} | metric={} op={} threshold={} value={} → {}", self.name, self.metric, self.operator, self.threshold, val, if matched { "MATCH" } else { "NO MATCH" });
            if matched {
                return vec![msg];
            }
        } else {
            tracing::debug!("[DAG] cond {} | metric={} not found in msg data", self.name, self.metric);
        }
        vec![]
    }
}

pub struct MultiConditionNode {
    id: String,
    name: String,
    conditions: Vec<(String, String, f64)>,
}

impl MultiConditionNode {
    pub fn new(id: &str, name: &str, conditions: Vec<(String, String, f64)>) -> Self {
        Self { id: id.to_string(), name: name.to_string(), conditions }
    }
}

#[async_trait]
impl RuleNode for MultiConditionNode {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.name }

    async fn on_msg(&self, _ctx: &NodeContext, msg: TbMsg) -> Vec<TbMsg> {
        for (metric, operator, threshold) in &self.conditions {
            if let Some(val) = msg.get_metric(metric) {
                let met = match operator.as_str() {
                    ">" => val > *threshold,
                    ">=" => val >= *threshold,
                    "<" => val < *threshold,
                    "<=" => val <= *threshold,
                    "==" => (val - *threshold).abs() < 0.001,
                    _ => false,
                };
                if !met {
                    tracing::debug!("[DAG] multi_cond {} | {} {} {} → NO MATCH (val={})", self.name, metric, operator, threshold, val);
                    return vec![];
                }
            } else {
                tracing::debug!("[DAG] multi_cond {} | metric {} not found → NO MATCH", self.name, metric);
                return vec![];
            }
        }
        tracing::debug!("[DAG] multi_cond {} | ALL CONDITIONS MATCHED", self.name);
        vec![msg]
    }
}

pub struct ActionNode {
    id: String,
    name: String,
    device_id: String,
    command: String,
    params: serde_json::Value,
    priority: bool,
}

impl ActionNode {
    pub fn new(id: &str, name: &str, device_id: &str, command: &str, params: serde_json::Value, priority: bool) -> Self {
        Self { id: id.to_string(), name: name.to_string(), device_id: device_id.to_string(), command: command.to_string(), params, priority }
    }
}

#[async_trait]
impl RuleNode for ActionNode {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.name }

    async fn on_msg(&self, ctx: &NodeContext, msg: TbMsg) -> Vec<TbMsg> {
        let device: Option<(String, String)> = sqlx::query_as("SELECT id, node_id FROM devices WHERE id = ?")
            .bind(&self.device_id)
            .fetch_optional(&ctx.pool)
            .await
            .unwrap_or(None);

        if let Some((_dev_id, node_id)) = device {
            let cmd_id = uuid::Uuid::new_v4().to_string();
            let payload = serde_json::json!({"command": self.command, "params": self.params}).to_string();

            if self.priority {
                tracing::info!("[DAG] action {} | MQTT publish to {}: {}", self.name, node_id, self.command);
                if let Some(client) = ctx.mqtt_client.lock().await.as_ref() {
                    let _ = publish_command(client, &node_id, &cmd_id, &payload).await;
                }
            } else {
                tracing::info!("[DAG] action {} | DB insert for {}: {}", self.name, node_id, self.command);
                let now = chrono::Utc::now().timestamp();
                let _ = sqlx::query(
                    "INSERT INTO command_log (device_id, command, payload, status, created_at) VALUES (?, ?, ?, 'pending', ?)"
                )
                .bind(&self.device_id)
                .bind(&self.command)
                .bind(&payload)
                .bind(now)
                .execute(&ctx.pool)
                .await;
            }
        } else {
            tracing::warn!("[DAG] action {} | device {} not found in DB", self.name, self.device_id);
        }

        vec![msg]
    }
}

pub struct LogNode {
    id: String,
    name: String,
}

impl LogNode {
    pub fn new(id: &str, name: &str) -> Self { Self { id: id.to_string(), name: name.to_string() } }
}

#[async_trait]
impl RuleNode for LogNode {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.name }

    async fn on_msg(&self, _ctx: &NodeContext, msg: TbMsg) -> Vec<TbMsg> {
        tracing::info!("[DAG] {} | type={:?} | originator={}", self.name, msg.msg_type, msg.originator);
        vec![msg]
    }
}
