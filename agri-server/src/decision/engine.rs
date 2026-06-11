use crate::decision::approval::{ApprovalLevel, ApprovalPolicy, TimeoutAction};
use crate::decision::notification::escalator::EscalationChain;

#[derive(Debug, Clone, PartialEq)]
pub enum Trigger {
    PerTelemetry,
    Timed { interval_secs: u64 },
    OnStateChange,
    Manual,
}

pub struct FlowContext {
    pub node_id: String,
    pub telemetry: Option<serde_json::Value>,
    pub device_state: Option<serde_json::Value>,
    pub crop_profile: Option<serde_json::Value>,
    pub weather: Option<serde_json::Value>,
    pub scores: Option<serde_json::Value>,
    pub outcome: Option<String>,
}

impl FlowContext {
    pub fn new(node_id: &str) -> Self {
        Self {
            node_id: node_id.to_string(),
            telemetry: None,
            device_state: None,
            crop_profile: None,
            weather: None,
            scores: None,
            outcome: None,
        }
    }
}

pub enum StageOut {
    Continue,
    Terminate,
    Error(String),
}

#[async_trait::async_trait]
pub trait Stage: Send + Sync {
    fn name(&self) -> &str;
    async fn process(&self, ctx: &mut FlowContext) -> StageOut;
}

pub struct DecisionFlow {
    pub name: String,
    pub trigger: Trigger,
    pub stages: Vec<Box<dyn Stage>>,
    pub approval: Option<ApprovalPolicy>,
    pub escalation: Option<EscalationChain>,
}

impl DecisionFlow {
    pub fn builder(name: &str) -> DecisionFlowBuilder {
        DecisionFlowBuilder::new(name)
    }
}

pub struct DecisionFlowBuilder {
    name: String,
    trigger: Option<Trigger>,
    stages: Vec<Box<dyn Stage>>,
    approval: Option<ApprovalPolicy>,
    escalation: Option<EscalationChain>,
}

impl DecisionFlowBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            trigger: None,
            stages: Vec::new(),
            approval: None,
            escalation: None,
        }
    }

    pub fn trigger(mut self, t: Trigger) -> Self {
        self.trigger = Some(t);
        self
    }

    pub fn stage(mut self, s: Box<dyn Stage>) -> Self {
        self.stages.push(s);
        self
    }

    pub fn approval(mut self, l: ApprovalLevel) -> Self {
        self.approval = Some(ApprovalPolicy::from_level(l));
        self
    }

    pub fn escalation(mut self, c: EscalationChain) -> Self {
        self.escalation = Some(c);
        self
    }

    pub fn build(self) -> DecisionFlow {
        DecisionFlow {
            name: self.name,
            trigger: self.trigger.unwrap_or(Trigger::Manual),
            stages: self.stages,
            approval: self.approval,
            escalation: self.escalation,
        }
    }
}
