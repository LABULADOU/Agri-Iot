// ==================== 每日用工成本 ====================
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::response;
use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/labor/records", get(list_records).post(create_record))
        .route("/api/v1/labor/records/:id", get(get_record).put(update_record).delete(delete_record))
        .route("/api/v1/labor/summary", get(get_summary))
        .with_state(state)
}

// ==================== Records ====================

#[derive(Debug, Deserialize)]
pub struct CreateLaborRequest {
    pub log_date: String,
    pub area_id: Option<String>,
    pub category: String,
    pub worker: Option<String>,
    pub worker_count: Option<i64>,
    pub work_hours: Option<f64>,
    pub rate: f64,
    pub paid_status: Option<String>,
    pub operator: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLaborRequest {
    pub log_date: Option<String>,
    pub area_id: Option<String>,
    pub category: Option<String>,
    pub worker: Option<String>,
    pub worker_count: Option<i64>,
    pub work_hours: Option<f64>,
    pub rate: Option<f64>,
    pub paid_status: Option<String>,
    pub operator: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListRecordsQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub area_id: Option<String>,
    pub category: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SummaryQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub area_id: Option<String>,
}

fn compute_amount(work_hours: f64, worker_count: i64, rate: f64) -> f64 {
    (work_hours * worker_count as f64 * rate * 100.0).round() / 100.0
}

async fn create_record(
    State(state): State<AppState>,
    Json(req): Json<CreateLaborRequest>,
) -> impl IntoResponse {
    if req.log_date.is_empty() || req.category.is_empty() {
        return response::bad_request("log_date and category are required");
    }
    if req.rate < 0.0 {
        return response::bad_request("rate cannot be negative");
    }
    let id = Uuid::new_v4();
    let now = Utc::now().timestamp();
    let worker_count = req.worker_count.unwrap_or(1).max(1);
    let work_hours = req.work_hours.unwrap_or(0.0).max(0.0);
    let amount = compute_amount(work_hours, worker_count, req.rate);

    let result = sqlx::query(
        "INSERT INTO labor_records (id, log_date, area_id, category, worker, worker_count, work_hours, rate, amount, paid_status, operator, notes, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(&req.log_date)
    .bind(&req.area_id)
    .bind(&req.category)
    .bind(req.worker.as_deref().unwrap_or(""))
    .bind(worker_count)
    .bind(work_hours)
    .bind(req.rate)
    .bind(amount)
    .bind(req.paid_status.as_deref().unwrap_or("unpaid"))
    .bind(req.operator.as_deref().unwrap_or(""))
    .bind(req.notes.as_deref().unwrap_or(""))
    .bind(now)
    .bind(now)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({"id": id.to_string(), "amount": amount}))).into_response(),
        Err(e) => response::internal_err(e),
    }
}

async fn get_record(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let row = sqlx::query(
        "SELECT id, log_date, area_id, category, worker, worker_count, work_hours, rate, amount, paid_status, operator, notes, created_at, updated_at FROM labor_records WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await;

    match row {
        Ok(Some(r)) => Json(row_to_json(&r)).into_response(),
        Ok(None) => response::not_found(Some("Labor record not found")),
        Err(e) => response::internal_err(e),
    }
}

async fn list_records(
    State(state): State<AppState>,
    Query(q): Query<ListRecordsQuery>,
) -> impl IntoResponse {
    let page = q.page.unwrap_or(1).max(1);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * limit;

    let mut sql = String::from(
        "SELECT id, log_date, area_id, category, worker, worker_count, work_hours, rate, amount, paid_status, operator, notes, created_at, updated_at FROM labor_records WHERE 1=1"
    );
    let mut binds: Vec<String> = Vec::new();
    if let Some(ref area_id) = q.area_id {
        sql.push_str(" AND area_id = ?");
        binds.push(area_id.clone());
    }
    if let Some(ref date_from) = q.date_from {
        sql.push_str(" AND log_date >= ?");
        binds.push(date_from.clone());
    }
    if let Some(ref date_to) = q.date_to {
        sql.push_str(" AND log_date <= ?");
        binds.push(date_to.clone());
    }
    if let Some(ref category) = q.category {
        sql.push_str(" AND category = ?");
        binds.push(category.clone());
    }
    sql.push_str(" ORDER BY log_date DESC, created_at DESC LIMIT ? OFFSET ?");
    binds.push(limit.to_string());
    binds.push(offset.to_string());

    let mut query = sqlx::query(&sql);
    for b in &binds {
        query = query.bind(b);
    }
    let total = sqlx::query_scalar::<_, i64>(
        &format!("SELECT COUNT(*) FROM labor_records WHERE {}", sql.split(" WHERE ").nth(1).map(|s| s.split(" ORDER BY").next().unwrap_or(s)).unwrap_or("1=1")),
    )
    .fetch_one(&state.pool)
    .await;

    match query.fetch_all(&state.pool).await {
        Ok(rows) => Json(serde_json::json!({
            "records": rows.iter().map(row_to_json).collect::<Vec<_>>(),
            "page": page,
            "limit": limit,
            "total": total.unwrap_or(0),
        })).into_response(),
        Err(e) => response::internal_err(e),
    }
}

async fn update_record(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateLaborRequest>,
) -> impl IntoResponse {
    let now = Utc::now().timestamp();
    // 重算 amount：合并现有值 + 新值
    let existing = sqlx::query(
        "SELECT work_hours, worker_count, rate FROM labor_records WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await;

    let (wh, wc, rate) = match existing {
        Ok(Some(r)) => (
            req.work_hours.unwrap_or(r.try_get::<f64, _>("work_hours").unwrap_or(0.0)),
            req.worker_count.unwrap_or(r.try_get::<i64, _>("worker_count").unwrap_or(1)).max(1),
            req.rate.unwrap_or(r.try_get::<f64, _>("rate").unwrap_or(0.0)),
        ),
        Ok(None) => return response::not_found(Some("Labor record not found")),
        Err(e) => return response::internal_err(e),
    };
    let amount = compute_amount(wh, wc, rate);

    let result = sqlx::query(
        "UPDATE labor_records SET log_date = COALESCE(?, log_date), area_id = COALESCE(?, area_id), category = COALESCE(?, category), worker = COALESCE(?, worker), worker_count = ?, work_hours = ?, rate = ?, amount = ?, paid_status = COALESCE(?, paid_status), operator = COALESCE(?, operator), notes = COALESCE(?, notes), updated_at = ? WHERE id = ?",
    )
    .bind(&req.log_date)
    .bind(&req.area_id)
    .bind(&req.category)
    .bind(&req.worker)
    .bind(wc)
    .bind(wh)
    .bind(rate)
    .bind(amount)
    .bind(&req.paid_status)
    .bind(&req.operator)
    .bind(&req.notes)
    .bind(now)
    .bind(&id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({"message": "Labor record updated", "amount": amount})).into_response(),
        Err(e) => response::internal_err(e),
    }
}

async fn delete_record(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM labor_records WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(_) => Json(serde_json::json!({"message": "Labor record deleted"})).into_response(),
        Err(e) => response::internal_err(e),
    }
}

// ==================== Summary ====================

async fn get_summary(
    State(state): State<AppState>,
    Query(q): Query<SummaryQuery>,
) -> impl IntoResponse {
    let mut base = "WHERE 1=1".to_string();
    let mut binds: Vec<String> = Vec::new();
    if let Some(ref area_id) = q.area_id {
        base.push_str(" AND area_id = ?");
        binds.push(area_id.clone());
    }
    if let Some(ref date_from) = q.date_from {
        base.push_str(" AND log_date >= ?");
        binds.push(date_from.clone());
    }
    if let Some(ref date_to) = q.date_to {
        base.push_str(" AND log_date <= ?");
        binds.push(date_to.clone());
    }

    let totalsql = format!(
        "SELECT COALESCE(SUM(amount), 0) AS total_amount, COALESCE(SUM(worker_count), 0) AS total_worker_days, COUNT(DISTINCT log_date) AS total_workdays, COUNT(*) AS record_count FROM labor_records {}",
        base
    );
    let by_date_sql = format!(
        "SELECT log_date, COALESCE(SUM(amount), 0) AS amount, COALESCE(SUM(worker_count), 0) AS worker_count FROM labor_records {} GROUP BY log_date ORDER BY log_date",
        base
    );
    let by_cat_sql = format!(
        "SELECT category, COALESCE(SUM(amount), 0) AS amount, COUNT(*) AS count FROM labor_records {} GROUP BY category ORDER BY amount DESC",
        base
    );

    let mut totals_q = sqlx::query(&totalsql);
    let mut by_date_q = sqlx::query(&by_date_sql);
    let mut by_cat_q = sqlx::query(&by_cat_sql);
    for b in &binds {
        totals_q = totals_q.bind(b);
        by_date_q = by_date_q.bind(b);
        by_cat_q = by_cat_q.bind(b);
    }

    match (
        totals_q.fetch_one(&state.pool).await,
        by_date_q.fetch_all(&state.pool).await,
        by_cat_q.fetch_all(&state.pool).await,
    ) {
        (Ok(total), Ok(by_date), Ok(by_cat)) => {
            Json(serde_json::json!({
                "summary": {
                    "total_amount": total.try_get::<f64, _>("total_amount").unwrap_or(0.0),
                    "total_worker_days": total.try_get::<i64, _>("total_worker_days").unwrap_or(0),
                    "total_workdays": total.try_get::<i64, _>("total_workdays").unwrap_or(0),
                    "record_count": total.try_get::<i64, _>("record_count").unwrap_or(0),
                    "by_date": by_date.iter().map(|r| serde_json::json!({
                        "log_date": r.try_get::<String, _>("log_date").unwrap_or_default(),
                        "amount": r.try_get::<f64, _>("amount").unwrap_or(0.0),
                        "worker_count": r.try_get::<i64, _>("worker_count").unwrap_or(0),
                    })).collect::<Vec<_>>(),
                    "by_category": by_cat.iter().map(|r| serde_json::json!({
                        "category": r.try_get::<String, _>("category").unwrap_or_default(),
                        "amount": r.try_get::<f64, _>("amount").unwrap_or(0.0),
                        "count": r.try_get::<i64, _>("count").unwrap_or(0),
                    })).collect::<Vec<_>>(),
                }
            })).into_response()
        }
        _ => response::internal_err("Failed to aggregate labor summary"),
    }
}

// ==================== Helpers ====================

fn row_to_json(r: &sqlx::sqlite::SqliteRow) -> serde_json::Value {
    serde_json::json!({
        "id": r.try_get::<String, _>("id").unwrap_or_default(),
        "log_date": r.try_get::<String, _>("log_date").unwrap_or_default(),
        "area_id": r.try_get::<Option<String>, _>("area_id").unwrap_or(None),
        "category": r.try_get::<String, _>("category").unwrap_or_default(),
        "worker": r.try_get::<String, _>("worker").unwrap_or_default(),
        "worker_count": r.try_get::<i64, _>("worker_count").unwrap_or(1),
        "work_hours": r.try_get::<f64, _>("work_hours").unwrap_or(0.0),
        "rate": r.try_get::<f64, _>("rate").unwrap_or(0.0),
        "amount": r.try_get::<f64, _>("amount").unwrap_or(0.0),
        "paid_status": r.try_get::<String, _>("paid_status").unwrap_or_default(),
        "operator": r.try_get::<String, _>("operator").unwrap_or_default(),
        "notes": r.try_get::<String, _>("notes").unwrap_or_default(),
        "created_at": r.try_get::<i64, _>("created_at").unwrap_or(0),
        "updated_at": r.try_get::<i64, _>("updated_at").unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, http::StatusCode};
    use sqlx::{Executor, SqlitePool};
    use tower::ServiceExt;

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        for path in [
            concat!(env!("CARGO_MANIFEST_DIR"), "/../agri-core/migrations/001_init.sql"),
            concat!(env!("CARGO_MANIFEST_DIR"), "/../agri-core/migrations/015_labor.sql"),
        ] {
            let schema = std::fs::read_to_string(path).unwrap();
            for stmt in schema.split(';').filter(|s| !s.trim().is_empty()) {
                sqlx::query(stmt).execute(&pool).await.unwrap();
            }
        }
        pool
    }

    fn test_state(pool: SqlitePool) -> AppState {
        let (tx, _rx) = tokio::sync::broadcast::channel(16);
        AppState {
            pool,
            event_tx: tx,
            mqtt_client: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
            rules_cache: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
            obsidian_vault_path: None,
            emergency_ctx: std::sync::Arc::new(tokio::sync::Mutex::new(agri_core::ai::emergency::EmergencyContext::new())),
            telemetry_limiter: std::sync::Arc::new(crate::rate_limiter::RateLimiter::new(60, 1)),
        }
    }

    #[tokio::test]
    async fn create_record_computes_amount_and_reads_back() {
        let pool = setup_db().await;
        let app = create_router(test_state(pool.clone())).into_service();

        let res = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/labor/records")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"log_date":"2026-08-20","area_id":"a1","category":"采收","worker_count":3,"work_hours":8,"rate":20.0,"operator":"张三","notes":"整棚采收"}"#))
                .unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        // amount = 3 * 8 * 20 = 480
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["amount"], 480.0);

        let res = app.clone().oneshot(
            Request::builder().method("GET").uri(format!("/api/v1/labor/records/{}", parsed["id"].as_str().unwrap())).body(Body::empty()).unwrap(),
        ).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let rec: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(rec["category"], "采收");
        assert_eq!(rec["amount"], 480.0);
        assert_eq!(rec["worker_count"], 3);
        assert_eq!(rec["paid_status"], "unpaid");
    }

    #[tokio::test]
    async fn update_record_recomputes_amount() {
        let pool = setup_db().await;
        let app = create_router(test_state(pool.clone())).into_service();

        let res = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/labor/records")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"log_date":"2026-08-20","category":"采收","worker_count":3,"work_hours":8,"rate":20.0}"#))
                .unwrap(),
        ).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let id = parsed["id"].as_str().unwrap().to_string();
        assert_eq!(parsed["amount"], 480.0);

        // 工时改为 5，其他人不增收 rate → 3*5*20=300
        let res = app.oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/labor/records/{}", id))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"work_hours":5}"#))
                .unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["amount"], 300.0);
    }

    #[tokio::test]
    async fn delete_record_removes_row() {
        let pool = setup_db().await;
        let app = create_router(test_state(pool.clone())).into_service();

        let res = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/labor/records")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"log_date":"2026-08-20","category":"采收","worker_count":1,"work_hours":8,"rate":20.0}"#))
                .unwrap(),
        ).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let id = parsed["id"].as_str().unwrap().to_string();

        let res = app.oneshot(
            Request::builder().method("DELETE").uri(format!("/api/v1/labor/records/{}", id)).body(Body::empty()).unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM labor_records").fetch_one(&pool).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn summary_aggregates_by_date_and_category() {
        let pool = setup_db().await;
        let app = create_router(test_state(pool.clone())).into_service();

        for payload in [
            r#"{"log_date":"2026-08-01","category":"采收","worker_count":2,"work_hours":8,"rate":20.0}"#,
            r#"{"log_date":"2026-08-01","category":"打药","worker_count":1,"work_hours":4,"rate":25.0}"#,
            r#"{"log_date":"2026-08-02","category":"采收","worker_count":3,"work_hours":8,"rate":20.0}"#,
        ] {
            let res = app.clone().oneshot(
                Request::builder().method("POST").uri("/api/v1/labor/records").header("content-type", "application/json").body(Body::from(payload)).unwrap(),
            ).await.unwrap();
            assert_eq!(res.status(), StatusCode::CREATED);
        }

        // 8-01: 2*8*20=320 打药 1*4*25=100 → 420；8-02: 3*8*20=480；total 900
        let res = app.oneshot(
            Request::builder().method("GET").uri("/api/v1/labor/summary?date_from=2026-08-01&date_to=2026-08-02").body(Body::empty()).unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let s = &parsed["summary"];
        assert_eq!(s["total_amount"], 900.0);
        assert_eq!(s["total_workdays"], 2);
        assert_eq!(s["total_worker_days"], 6);
        assert_eq!(s["by_date"].as_array().unwrap().len(), 2);
        assert_eq!(s["by_date"][0]["amount"], 420.0);
        assert_eq!(s["by_category"][0]["category"], "采收");
        assert_eq!(s["by_category"][0]["amount"], 800.0);
    }

    #[tokio::test]
    async fn create_record_rejects_negative_rate() {
        let pool = setup_db().await;
        let app = create_router(test_state(pool)).into_service();

        let res = app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/labor/records")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"log_date":"2026-08-20","category":"采收","rate":-5.0}"#))
                .unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn list_records_paginates() {
        let pool = setup_db().await;
        let app = create_router(test_state(pool.clone())).into_service();

        for d in ["2026-08-01", "2026-08-02", "2026-08-03"] {
            let res = app.clone().oneshot(
                Request::builder().method("POST").uri("/api/v1/labor/records").header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"log_date":"{}","category":"采收","rate":20.0}}"#, d))).unwrap(),
            ).await.unwrap();
            assert_eq!(res.status(), StatusCode::CREATED);
        }

        let res = app.oneshot(
            Request::builder().method("GET").uri("/api/v1/labor/records?limit=2&page=2").body(Body::empty()).unwrap(),
        ).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["records"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["total"], 3);
    }
}