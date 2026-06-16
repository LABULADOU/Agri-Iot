use crate::ai::embedding::EmbeddingEngine;
use crate::ai::knowledge::ObsidianKnowledge;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// 观察者：定期维护 embedding 索引
pub struct Observer {
    embed: Arc<Mutex<EmbeddingEngine>>,
    vault: Option<ObsidianKnowledge>,
}

impl Observer {
    pub fn new(_pool: sqlx::SqlitePool, embed: Arc<Mutex<EmbeddingEngine>>) -> Self {
        Self { embed, vault: None }
    }

    pub fn with_vault(mut self, vault: ObsidianKnowledge) -> Self {
        self.vault = Some(vault);
        self
    }

    /// 启动周期性任务
    pub fn start(self) {
        tokio::spawn(async move {
            // 初始延迟 60 秒（等系统启动完成）
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            info!("[observer] starting periodic embedding index");

            loop {
                // 每天凌晨 00:05 执行一次全量索引
                let now = chrono::Utc::now();
                let next = (now + chrono::Duration::days(1))
                    .date_naive()
                    .and_hms_opt(0, 5, 0)
                    .unwrap()
                    .and_utc();
                let delay = (next - now).to_std().unwrap_or(std::time::Duration::from_secs(86400));
                tokio::time::sleep(delay).await;

                info!("[observer] rebuilding embedding index");
                if let Some(ref vault) = self.vault {
                    let embed = self.embed.lock().await;
                    match embed.index_all(vault).await {
                        Ok(n) => info!("[observer] indexed {} notes", n),
                        Err(e) => tracing::warn!("[observer] index failed: {}", e),
                    }
                }
            }
        });
    }
}
