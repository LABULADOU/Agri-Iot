// TODO(decision): LlmStage 实现完整且可用，但需要 mod.rs 的调度循环触发 process()
use crate::decision::engine::{FlowContext, Stage, StageOut};
use crate::state::AppState;
use agri_core::ai::llm::{LlmDecision, LlmProvider, SYSTEM_PROMPT_ASSESS};
use agri_core::ai::retrieval::RetrievalEngine;
use std::sync::Arc;

/// LLM 评估 Stage — Tier 3 定时评估
pub struct LlmStage {
    provider: Arc<LlmProvider>,
    retrieval: Arc<RetrievalEngine>,
}

impl LlmStage {
    pub fn new(provider: Arc<LlmProvider>, retrieval: Arc<RetrievalEngine>) -> Self {
        Self { provider, retrieval }
    }
}

#[async_trait::async_trait]
impl Stage for LlmStage {
    fn name(&self) -> &str {
        "llm_assessment"
    }

    async fn process(&self, ctx: &mut FlowContext) -> StageOut {
        // 1. 构建 RAG 上下文
        let rag = match self.retrieval.build(&ctx.node_id, 5).await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("[llm_stage] retrieval failed: {}", e);
                return StageOut::Continue;
            }
        };

        // 2. 序列化为 LLM 输入
        let context_json = match serde_json::to_string_pretty(&rag) {
            Ok(j) => j,
            Err(e) => {
                tracing::warn!("[llm_stage] context serialize failed: {}", e);
                return StageOut::Continue;
            }
        };

        let user_prompt = format!(
            "请根据以下温室环境数据做出评估和调控决策：\n\n{}",
            context_json
        );

        // 3. 调用 LLM
        let decision: LlmDecision = match self.provider.chat_json(SYSTEM_PROMPT_ASSESS, &user_prompt).await {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("[llm_stage] LLM call failed: {}", e);
                return StageOut::Continue;
            }
        };

        // 4. 填充 ctx
        ctx.scores = Some(serde_json::json!({
            "overall": decision.overall_score,
            "soil_temp": decision.scores.soil_temp,
            "soil_moisture": decision.scores.soil_moisture,
            "ec": decision.scores.ec,
            "air_temp": decision.scores.air_temp,
            "air_humidity": decision.scores.air_humidity,
        }));

        ctx.outcome = Some(serde_json::to_string(&serde_json::json!({
            "assessment": decision.assessment,
            "action": decision.action,
            "risk_flags": decision.risk_flags,
            "knowledge_gaps": decision.knowledge_gaps,
        })).unwrap_or_default());

        tracing::info!(
            "[llm_stage] decision: score={:.1}, action={:?}, risks={:?}",
            decision.overall_score,
            decision.action.as_ref().map(|a| &a.action_type),
            decision.risk_flags,
        );

        StageOut::Continue
    }
}

/// 根据 AppState 创建 LlmStage
pub fn create_llm_stage(state: &AppState) -> Option<LlmStage> {
    let provider = match LlmProvider::from_env() {
        Ok(p) => Arc::new(p),
        Err(e) => {
            tracing::warn!("[llm_stage] LLM not configured (skip): {}", e);
            return None;
        }
    };

    let vault_path = state.obsidian_vault_path.clone().unwrap_or_default();

    let mut retrieval = RetrievalEngine::new(state.pool.clone());
    if !vault_path.is_empty() {
        retrieval = retrieval.with_vault(agri_core::ai::knowledge::ObsidianKnowledge::new(&vault_path));
    }

    Some(LlmStage::new(provider, Arc::new(retrieval)))
}
