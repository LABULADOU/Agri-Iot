use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ApprovalLevel {
    Critical,
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeoutAction {
    ConservativeExec,
    Skip,
    Stack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalPolicy {
    pub level: ApprovalLevel,
    pub wait_secs: u32,
    pub on_timeout: TimeoutAction,
}

impl ApprovalPolicy {
    pub fn from_level(level: ApprovalLevel) -> Self {
        match level {
            ApprovalLevel::Critical => Self {
                level,
                wait_secs: 0,
                on_timeout: TimeoutAction::ConservativeExec,
            },
            ApprovalLevel::High => Self {
                level,
                wait_secs: 120,
                on_timeout: TimeoutAction::ConservativeExec,
            },
            ApprovalLevel::Normal => Self {
                level,
                wait_secs: 600,
                on_timeout: TimeoutAction::Skip,
            },
            ApprovalLevel::Low => Self {
                level,
                wait_secs: 0,
                on_timeout: TimeoutAction::Skip,
            },
        }
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.wait_secs as u64)
    }

    pub fn requires_confirmation(&self) -> bool {
        self.level != ApprovalLevel::Critical && self.wait_secs > 0
    }
}

pub struct ApprovalGate {
    pub policy: ApprovalPolicy,
    pub escalation_chain: Option<crate::decision::notification::escalator::EscalationChain>,
}

impl ApprovalGate {
    pub fn new(policy: ApprovalPolicy) -> Self {
        Self { policy, escalation_chain: None }
    }

    pub fn with_escalation(mut self, chain: crate::decision::notification::escalator::EscalationChain) -> Self {
        self.escalation_chain = Some(chain);
        self
    }
}
