use super::{ChannelType, NotificationMsg, Urgency};
use crate::decision::notification::router::Contact;

#[derive(Debug, Clone)]
pub struct EscalationStep {
    pub level: u32,
    pub contacts: Vec<Contact>,
    pub channels: Vec<ChannelType>,
    pub timeout_secs: u32,
}

#[derive(Debug, Clone)]
pub struct EscalationChain {
    pub name: String,
    pub steps: Vec<EscalationStep>,
}

impl EscalationChain {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), steps: Vec::new() }
    }

    pub fn step(mut self, s: EscalationStep) -> Self {
        self.steps.push(s);
        self
    }

    pub fn default_emergency() -> Self {
        Self {
            name: "emergency_default".to_string(),
            steps: vec![
                EscalationStep {
                    level: 1,
                    contacts: vec![],
                    channels: vec![ChannelType::Push],
                    timeout_secs: 30,
                },
                EscalationStep {
                    level: 2,
                    contacts: vec![],
                    channels: vec![ChannelType::SMS],
                    timeout_secs: 120,
                },
                EscalationStep {
                    level: 3,
                    contacts: vec![],
                    channels: vec![ChannelType::VoiceCall],
                    timeout_secs: 0,
                },
            ],
        }
    }
}

// TODO(decision): run_escalation 打印了日志但没有真正发送通知。需接入实际 Notifier 实现（Push/SMS/VoiceCall）
pub async fn run_escalation(chain: &EscalationChain, msg: &NotificationMsg) {
    let urgent = matches!(msg.urgency, Urgency::Critical | Urgency::High);

    for step in &chain.steps {
        let _ = &step.contacts;
        tracing::info!(
            "[escalation] {} step {}: {} channels, {} contacts, timeout={}s",
            chain.name, step.level, step.channels.len(), step.contacts.len(), step.timeout_secs
        );

        if !urgent && step.timeout_secs == 0 {
            break;
        }
    }
}
