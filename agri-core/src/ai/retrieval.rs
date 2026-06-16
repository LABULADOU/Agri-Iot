use crate::ai::embedding::EmbeddingEngine;
use crate::ai::knowledge::ObsidianKnowledge;
use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// RAG 检索上下文：所有喂给 LLM 的相关信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RagContext {
    pub node_id: String,
    pub current_readings: Vec<MetricReading>,
    pub weather: Option<WeatherBrief>,
    pub crop: Option<CropBrief>,
    pub recent_cases: Vec<CaseBrief>,
    pub knowledge_notes: Vec<KnowledgeNote>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricReading {
    pub metric: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WeatherBrief {
    pub temperature: Option<f64>,
    pub humidity: Option<f64>,
    pub wind_speed: Option<f64>,
    pub precipitation: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CropBrief {
    pub name: String,
    pub variety: Option<String>,
    pub soil_temp_optimal: Option<f64>,
    pub soil_moisture_optimal: Option<f64>,
    pub ec_optimal: Option<f64>,
    pub air_temp_optimal: Option<f64>,
    pub air_humidity_optimal: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CaseBrief {
    pub id: String,
    pub situation: String,
    pub action_taken: String,
    pub outcome: String,
    pub effect_rating: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeNote {
    pub title: String,
    pub note_type: String,
    pub snippet: String,
}

/// RAG 检索引擎
pub struct RetrievalEngine {
    pool: SqlitePool,
    embed: Option<EmbeddingEngine>,
    vault: Option<ObsidianKnowledge>,
}

impl RetrievalEngine {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool, embed: None, vault: None }
    }

    pub fn with_embedding(mut self, embed: EmbeddingEngine) -> Self {
        self.embed = Some(embed);
        self
    }

    pub fn with_vault(mut self, vault: ObsidianKnowledge) -> Self {
        self.vault = Some(vault);
        self
    }

    /// 构建 RAG 上下文：读数 + 天气 + 作物 + 历史案例 + 知识笔记
    pub async fn build(&self, node_id: &str, top_k: usize) -> Result<RagContext> {
        let readings = self.fetch_readings(node_id).await?;
        let weather = self.fetch_weather().await?;
        let crop = self.fetch_crop(node_id).await?;
        let cases = self.fetch_cases(node_id, 5).await?;

        let notes = if let Some(ref embed) = self.embed {
            let query = build_search_query(&readings, &crop, &weather);
            embed.search(&query, top_k).await?
                .into_iter()
                .map(|r| KnowledgeNote {
                    title: r.source_id.unwrap_or_default(),
                    note_type: r.source_type,
                    snippet: r.content.chars().take(300).collect(),
                })
                .collect()
        } else if let Some(ref vault) = self.vault {
            let keyword = crop.as_ref().map(|c| c.name.clone()).unwrap_or_default();
            vault.search(&keyword).ok()
                .unwrap_or_default()
                .into_iter()
                .map(|r| KnowledgeNote {
                    title: r.title,
                    note_type: r.note_type,
                    snippet: r.snippet,
                })
                .collect()
        } else {
            vec![]
        };

        Ok(RagContext {
            node_id: node_id.to_string(),
            current_readings: readings,
            weather,
            crop,
            recent_cases: cases,
            knowledge_notes: notes,
        })
    }

    async fn fetch_readings(&self, node_id: &str) -> Result<Vec<MetricReading>> {
        let rows = sqlx::query_as::<_, (String, f64, String)>(
            "SELECT metric, value, unit FROM sensor_readings
             WHERE device_id IN (SELECT id FROM devices WHERE node_id = ?)
             AND timestamp > datetime('now', '-1 hour')
             ORDER BY timestamp DESC LIMIT 50"
        )
        .bind(node_id)
        .fetch_all(&self.pool)
        .await
        .context("fetch readings failed")?;

        Ok(rows.into_iter()
            .map(|(m, v, u)| MetricReading { metric: m, value: v, unit: u })
            .collect())
    }

    async fn fetch_weather(&self) -> Result<Option<WeatherBrief>> {
        let row = sqlx::query_as::<_, (Option<f64>, Option<f64>, Option<f64>, Option<f64>)>(
            "SELECT temperature, humidity, wind_speed, precipitation
             FROM weather_data ORDER BY timestamp DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await
        .context("fetch weather failed")?;

        Ok(row.map(|(t, h, w, p)| WeatherBrief {
            temperature: t, humidity: h, wind_speed: w, precipitation: p,
        }))
    }

    async fn fetch_crop(&self, node_id: &str) -> Result<Option<CropBrief>> {
        let row = sqlx::query_as::<_, (String, Option<String>, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>)>(
            "SELECT cp.name, cp.variety, cp.soil_temp_optimal, cp.soil_moisture_optimal,
                    cp.ec_optimal, cp.air_temp_optimal, cp.air_humidity_optimal
             FROM crop_profiles cp
             JOIN crop_batches cb ON cb.crop_id = cp.id
             JOIN devices d ON d.area_id = cb.area_id
             WHERE d.node_id = ? AND cb.status = 'active'
             LIMIT 1"
        )
        .bind(node_id)
        .fetch_optional(&self.pool)
        .await
        .context("fetch crop failed")?;

        Ok(row.map(|(n, v, st, sm, ec, at, ah)| CropBrief {
            name: n, variety: v,
            soil_temp_optimal: st, soil_moisture_optimal: sm,
            ec_optimal: ec, air_temp_optimal: at, air_humidity_optimal: ah,
        }))
    }

    async fn fetch_cases(&self, node_id: &str, limit: i64) -> Result<Vec<CaseBrief>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, Option<i64>)>(
            "SELECT cc.id, COALESCE(cc.situation, ''), COALESCE(cc.action_taken, ''),
                    COALESCE(cc.outcome, ''), cc.effect_rating
             FROM control_cases cc
             JOIN devices d ON d.area_id = cc.area_id
             WHERE d.node_id = ? AND cc.outcome IS NOT NULL
             ORDER BY cc.timestamp DESC LIMIT ?"
        )
        .bind(node_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("fetch cases failed")?;

        Ok(rows.into_iter()
            .map(|(id, sit, act, out, eff)| CaseBrief {
                id, situation: sit, action_taken: act, outcome: out, effect_rating: eff,
            })
            .collect())
    }
}

/// 根据上下文构建搜索查询（用于语义检索）
fn build_search_query(readings: &[MetricReading], crop: &Option<CropBrief>, _weather: &Option<WeatherBrief>) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(c) = crop {
        parts.push(format!("作物: {}", c.name));
    }

    for r in readings {
        parts.push(format!("{}: {:.1}", r.metric, r.value));
    }

    if parts.is_empty() {
        "温室环境调控".to_string()
    } else {
        parts.join(", ")
    }
}
