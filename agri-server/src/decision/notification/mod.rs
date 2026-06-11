pub mod router;
pub mod escalator;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationMsg {
    pub title: String,
    pub body: String,
    pub urgency: Urgency,
    pub node_id: Option<String>,
    pub flow_name: Option<String>,
    pub action_required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Urgency {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChannelType {
    Push,
    DingTalk,
    WeCom,
    SMS,
    VoiceCall,
    Email,
}

#[async_trait]
pub trait Notifier: Send + Sync {
    fn channel_type(&self) -> ChannelType;
    async fn notify(&self, msg: &NotificationMsg) -> Result<()>;
}

pub struct NotificationDispatch {
    notifiers: Vec<Box<dyn Notifier>>,
}

impl NotificationDispatch {
    pub fn new() -> Self {
        Self { notifiers: Vec::new() }
    }

    pub fn register(&mut self, n: Box<dyn Notifier>) {
        self.notifiers.push(n);
    }

    pub async fn dispatch(&self, msg: &NotificationMsg, channels: &[ChannelType]) {
        for notifier in &self.notifiers {
            if channels.contains(&notifier.channel_type()) {
                if let Err(e) = notifier.notify(msg).await {
                    tracing::warn!("[{}] notify failed: {}", msg.title, e);
                }
            }
        }
    }

    pub async fn dispatch_all(&self, msg: &NotificationMsg) {
        for notifier in &self.notifiers {
            if let Err(e) = notifier.notify(msg).await {
                tracing::warn!("[{}] notify failed: {}", msg.title, e);
            }
        }
    }
}
