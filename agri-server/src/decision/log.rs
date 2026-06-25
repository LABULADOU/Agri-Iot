use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DecisionLogEntry {
    pub id: i64,
    pub flow_name: String,
    pub node_id: String,
    pub trigger: String,
    pub outcome: String,
    pub detail: Option<String>,
    pub created_at: i64,
}

// TODO(decision): decision_log CRUD 已实现，但无人调用。接入时机：Stage::process() 输出决策后记录
pub async fn write_log(pool: &SqlitePool, entry: &DecisionLogEntry) -> Result<i64> {
    let result = sqlx::query(
        "INSERT INTO decision_log (flow_name, node_id, trigger, outcome, detail, created_at) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&entry.flow_name)
    .bind(&entry.node_id)
    .bind(&entry.trigger)
    .bind(&entry.outcome)
    .bind(&entry.detail)
    .bind(entry.created_at)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn query_log(pool: &SqlitePool, node_id: &str, limit: i64) -> Result<Vec<DecisionLogEntry>> {
    let rows = sqlx::query_as::<_, DecisionLogEntry>(
        "SELECT id, flow_name, node_id, trigger, outcome, detail, created_at FROM decision_log WHERE node_id = ? ORDER BY created_at DESC LIMIT ?"
    )
    .bind(node_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn query_recent(pool: &SqlitePool, minutes: i64) -> Result<Vec<DecisionLogEntry>> {
    let cutoff = chrono::Utc::now().timestamp() - minutes * 60;
    let rows = sqlx::query_as::<_, DecisionLogEntry>(
        "SELECT id, flow_name, node_id, trigger, outcome, detail, created_at FROM decision_log WHERE created_at > ? ORDER BY created_at DESC"
    )
    .bind(cutoff)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
