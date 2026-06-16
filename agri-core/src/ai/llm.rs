use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// LLM Provider 封装（通过 OpenAI-compatible API 调用）
pub struct LlmProvider {
    api_key: String,
    api_url: String,
    model: String,
    temperature: f64,
    max_tokens: u64,
    client: reqwest::Client,
}

impl LlmProvider {
    pub fn new(api_key: &str, model: &str, temperature: f64, max_tokens: u64) -> Self {
        let api_url = std::env::var("LLM_API_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".into());
        Self {
            api_key: api_key.to_string(),
            api_url,
            model: model.to_string(),
            temperature,
            max_tokens,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .build()
                .expect("Failed to build reqwest client"),
        }
    }

    /// 从环境变量初始化
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("LLM_API_KEY")
            .context("LLM_API_KEY not set")?;
        let model = std::env::var("LLM_MODEL")
            .unwrap_or_else(|_| "gpt-4o-mini".into());
        let temperature = std::env::var("LLM_TEMPERATURE")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.3);
        let max_tokens = std::env::var("LLM_MAX_TOKENS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(4096);
        Ok(Self::new(&api_key, &model, temperature, max_tokens))
    }

    /// Chat completion（自然语言响应）
    pub async fn chat(&self, system: &str, user: &str) -> Result<String> {
        self.chat_with_history(system, &[] as &[HistoryMessage], user).await
    }

    /// Chat with conversation history
    pub async fn chat_with_history(
        &self,
        system: &str,
        history: &[HistoryMessage],
        user: &str,
    ) -> Result<String> {
        let mut messages = Vec::with_capacity(2 + history.len());
        messages.push(Message { role: "system".into(), content: system.to_string() });

        for msg in history {
            messages.push(Message {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }

        messages.push(Message { role: "user".into(), content: user.to_string() });

        let body = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: self.temperature,
            max_tokens: self.max_tokens as u64,
            response_format: None,
        };

        let resp = self.client
            .post(format!("{}/chat/completions", self.api_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .context("LLM API request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("LLM API error {}: {}", status, text);
        }

        let parsed: ChatResponse = resp
            .json()
            .await
            .context("LLM API response parse failed")?;

        parsed.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .context("LLM returned empty choices")
    }

    /// JSON mode chat completion（结构化输出）
    pub async fn chat_json<T: for<'de> Deserialize<'de>>(
        &self,
        system: &str,
        user: &str,
    ) -> Result<T> {
        let body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                Message { role: "system".into(), content: system.to_string() },
                Message { role: "user".into(), content: user.to_string() },
            ],
            temperature: self.temperature,
            max_tokens: self.max_tokens as u64,
            response_format: Some(ResponseFormat {
                type_field: "json_object".into(),
            }),
        };

        let resp = self.client
            .post(format!("{}/chat/completions", self.api_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .context("LLM JSON API request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("LLM JSON API error {}: {}", status, text);
        }

        let parsed: ChatResponse = resp
            .json()
            .await
            .context("LLM JSON API response parse failed")?;

        let raw = parsed.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .context("LLM returned empty choices")?;

        // 清理可能的 markdown 代码块标记
        let cleaned = raw
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        serde_json::from_str(cleaned)
            .context(format!(
                "LLM JSON parse failed (first 200 chars): {}",
                &cleaned[..cleaned.len().min(200)]
            ))
    }
}

// ========== OpenAI Chat API 数据结构 ==========

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f64,
    max_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    type_field: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct Usage {
    total_tokens: u32,
}

// ========== 业务数据结构 ==========

/// LLM 决策输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmDecision {
    pub overall_score: f64,
    pub scores: MetricScores,
    pub assessment: String,
    pub action: Option<LlmAction>,
    pub risk_flags: Vec<String>,
    pub knowledge_gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricScores {
    pub soil_temp: f64,
    pub soil_moisture: f64,
    pub ec: f64,
    pub air_temp: f64,
    pub air_humidity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub device: String,
    pub target_percent: f64,
    pub reason: String,
}

/// Agent 查询输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub answer: String,
    pub data_sources: Vec<String>,
    pub follow_up_questions: Vec<String>,
}

/// 系统提示词：环境评估
pub const SYSTEM_PROMPT_ASSESS: &str = r#"你是 Agri-Iot AI 农业决策助手。
你的职责是分析温室环境数据，结合作物知识库和气象信息，做出调控决策。

## 决策规则
1. 紧急优先：大风(>40km/h)、大雨(>10mm/h)、降雪已由系统自动处理，你不要重复触发
2. 作物导向：所有评分基于当前作物的最适区间计算
3. 通风优先：首选项是调节通风（0-100%），而非其他设备
4. EC 值仅供参考：施肥决策以人工为主
5. 历史案例参考：优先选择相似情境下成功率高的方案

## JSON 输出格式
{
  "overall_score": <f64 0-100>,
  "scores": { "soil_temp": <f64>, "soil_moisture": <f64>, "ec": <f64>, "air_temp": <f64>, "air_humidity": <f64> },
  "assessment": "<一句话环境评估>",
  "action": null | {
    "type": "ventilation" | "irrigation" | "alert",
    "device": "top_vent" | "side_vent",
    "target_percent": <f64 0-100>,
    "reason": "<决策理由>"
  },
  "risk_flags": ["<病虫害风险>"],
  "knowledge_gaps": ["<建议补充的知识条目>"]
}"#;

/// 系统提示词：Agent 查询
pub const SYSTEM_PROMPT_AGENT: &str = r#"你是 Agri-Iot 农业物联网助手。用户会询问温室状态、历史事件或调控建议。
请用中文回答，简明扼要。你不需要重复输出 JSON，用自然语言回复即可。

可参考的上下文包括：当前传感器读数、气象数据、作物信息、最新调控案例、知识库笔记。
保持回答事实性，不确定的说"数据不足"。"#;
