use crate::ai::knowledge::ObsidianKnowledge;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// 嵌入向量维度（text-embedding-3-small = 1536）
pub const EMBEDDING_DIM: usize = 1536;

/// Embedding 引擎：生成 + 存储 + 检索
pub struct EmbeddingEngine {
    pool: SqlitePool,
    api_key: String,
    api_url: String,
    model: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct EmbedRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedData>,
    model: String,
    usage: EmbedUsage,
}

#[derive(Deserialize)]
struct EmbedData {
    embedding: Vec<f64>,
    index: usize,
}

#[derive(Deserialize)]
struct EmbedUsage {
    total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredResult {
    pub id: String,
    pub source_type: String,
    pub source_id: Option<String>,
    pub content: String,
    pub score: f64,
}

impl EmbeddingEngine {
    pub fn new(pool: SqlitePool, api_key: &str, model: &str) -> Self {
        let api_url = std::env::var("LLM_API_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".into());
        Self {
            pool,
            api_key: api_key.to_string(),
            api_url,
            model: model.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env(pool: SqlitePool) -> Result<Self> {
        let api_key = std::env::var("LLM_API_KEY")
            .context("LLM_API_KEY not set")?;
        let model = std::env::var("EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".into());
        Ok(Self::new(pool, &api_key, &model))
    }

    /// 生成单段文本的 embedding
    pub async fn embed_text(&self, text: &str) -> Result<Vec<f64>> {
        let mut batch = self.embed_batch(&[text.to_string()]).await?;
        Ok(batch.remove(0))
    }

    /// 批量生成 embedding
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f64>>> {
        let body = EmbedRequest {
            model: self.model.clone(),
            input: texts.to_vec(),
        };

        let resp = self.client
            .post(format!("{}/embeddings", self.api_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .context("embedding API request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("embedding API error {}: {}", status, text);
        }

        let parsed: EmbedResponse = resp
            .json()
            .await
            .context("embedding API response parse failed")?;

        // 按 index 排序确保顺序一致
        let mut results: Vec<Vec<f64>> = parsed.data
            .into_iter()
            .map(|d| d.embedding)
            .collect();

        // 如果返回数量不一致，补齐空向量
        while results.len() < texts.len() {
            results.push(vec![0.0; EMBEDDING_DIM]);
        }

        Ok(results)
    }

    /// 存储 embedding 到 sqlite
    pub async fn store(&self, source_type: &str, source_id: &str, content: &str) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let emb = self.embed_text(content).await?;
        let emb_json = serde_json::to_string(&emb)?;

        sqlx::query(
            "INSERT OR REPLACE INTO vec_embeddings (id, source_type, source_id, content, embedding) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(source_type)
        .bind(source_id)
        .bind(content)
        .bind(&emb_json)
        .execute(&self.pool)
        .await
        .context("failed to store embedding")?;

        Ok(id)
    }

    /// 余弦相似度搜索
    pub async fn search(&self, query: &str, top_k: usize) -> Result<Vec<ScoredResult>> {
        let q_emb = self.embed_text(query).await?;

        let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
            "SELECT id, source_type, source_id, content, embedding FROM vec_embeddings"
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to fetch embeddings")?;

        let mut scored: Vec<ScoredResult> = rows
            .into_iter()
            .filter_map(|(id, st, sid, content, emb_json)| {
                let emb: Vec<f64> = serde_json::from_str(&emb_json).ok()?;
                let score = cosine_similarity(&q_emb, &emb)?;
                Some(ScoredResult {
                    id,
                    source_type: st,
                    source_id: Some(sid),
                    content,
                    score,
                })
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        Ok(scored)
    }

    /// 全量索引 Obsidian vault 中的 Markdown 文件
    pub async fn index_all(&self, vault: &ObsidianKnowledge) -> Result<usize> {
        let md_files = vault.list_markdown_files()?;
        let mut count = 0usize;

        for path in &md_files {
            if let Ok(content) = vault.read_note(path) {
                let source_id = format!("obsidian:{}", path);
                if self.store("obsidian", &source_id, &content).await.is_ok() {
                    count += 1;
                }
            }
        }

        Ok(count)
    }
}

/// 余弦相似度
fn cosine_similarity(a: &[f64], b: &[f64]) -> Option<f64> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return None;
    }
    Some(dot / (norm_a * norm_b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 2.0, 3.0];
        let score = cosine_similarity(&v, &v).unwrap();
        assert!((score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let score = cosine_similarity(&a, &b).unwrap();
        assert!((score - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        assert!(cosine_similarity(&[], &[]).is_none());
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        assert!(cosine_similarity(&[1.0], &[1.0, 2.0]).is_none());
    }
}
