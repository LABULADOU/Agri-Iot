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
        .route("/api/v1/yield/harvests", get(list_harvests).post(create_harvest))
        .route("/api/v1/yield/harvests/:id", get(get_harvest).put(update_harvest).delete(delete_harvest))
        .route("/api/v1/yield/analysis", get(get_analysis))
        .with_state(state)
}

// ==================== Harvests ====================

fn harvest_to_json(row: &sqlx::sqlite::SqliteRow) -> serde_json::Value {
    serde_json::json!({
        "id": row.try_get::<String, _>("id").unwrap_or_default(),
        "area_id": row.try_get::<String, _>("area_id").unwrap_or_default(),
        "area_name": row.try_get::<Option<String>, _>("area_name").unwrap_or(None).unwrap_or_default(),
        "crop_batch_id": row.try_get::<Option<String>, _>("crop_batch_id").unwrap_or(None).unwrap_or_default(),
        "crop_name": row.try_get::<Option<String>, _>("crop_name").unwrap_or(None).unwrap_or_default(),
        "harvest_date": row.try_get::<String, _>("harvest_date").unwrap_or_default(),
        "quantity": row.try_get::<f64, _>("quantity").unwrap_or(0.0),
        "unit": row.try_get::<String, _>("unit").unwrap_or_default(),
        "grade": row.try_get::<String, _>("grade").unwrap_or_default(),
        "price": row.try_get::<f64, _>("price").unwrap_or(0.0),
        "amount": row.try_get::<f64, _>("amount").unwrap_or(0.0),
        "operator": row.try_get::<String, _>("operator").unwrap_or_default(),
        "notes": row.try_get::<String, _>("notes").unwrap_or_default(),
        "created_at": row.try_get::<i64, _>("created_at").unwrap_or(0),
    })
}

const HARVEST_SELECT: &str = "SELECT h.*, a.name AS area_name, c.name AS crop_name FROM harvests h LEFT JOIN areas a ON a.id = h.area_id LEFT JOIN crop_batches cb ON cb.id = h.crop_batch_id LEFT JOIN crops c ON c.id = cb.crop_id";

#[derive(Debug, Deserialize)]
pub struct ListHarvestsQuery {
    pub area_id: Option<String>,
    pub crop_batch_id: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

async fn list_harvests(
    State(state): State<AppState>,
    Query(q): Query<ListHarvestsQuery>,
) -> impl IntoResponse {
    let page = q.page.unwrap_or(1).max(1);
    let limit = q.limit.unwrap_or(50).clamp(1, 500);
    let offset = (page - 1) * limit;

    let mut sql = String::from(HARVEST_SELECT);
    sql.push_str(" WHERE 1=1");
    let mut binds: Vec<String> = Vec::new();

    if let Some(ref area_id) = q.area_id {
        sql.push_str(" AND h.area_id = ?");
        binds.push(area_id.clone());
    }
    if let Some(ref crop_batch_id) = q.crop_batch_id {
        sql.push_str(" AND h.crop_batch_id = ?");
        binds.push(crop_batch_id.clone());
    }
    if let Some(ref date_from) = q.date_from {
        sql.push_str(" AND h.harvest_date >= ?");
        binds.push(date_from.clone());
    }
    if let Some(ref date_to) = q.date_to {
        sql.push_str(" AND h.harvest_date <= ?");
        binds.push(date_to.clone());
    }

    sql.push_str(" ORDER BY h.harvest_date DESC, h.created_at DESC LIMIT ? OFFSET ?");
    binds.push(limit.to_string());
    binds.push(offset.to_string());

    let mut query = sqlx::query(&sql);
    for b in &binds {
        query = query.bind(b);
    }

    match query.fetch_all(&state.pool).await {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.iter().map(harvest_to_json).collect();
            Json(serde_json::json!({"harvests": result, "page": page, "limit": limit})).into_response()
        }
        Err(e) => response::internal_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateHarvestRequest {
    pub area_id: String,
    pub crop_batch_id: Option<String>,
    pub harvest_date: String,
    pub quantity: f64,
    pub unit: Option<String>,
    pub grade: Option<String>,
    pub price: Option<f64>,
    pub operator: Option<String>,
    pub notes: Option<String>,
}

async fn create_harvest(
    State(state): State<AppState>,
    Json(req): Json<CreateHarvestRequest>,
) -> impl IntoResponse {
    if req.quantity <= 0.0 {
        return response::bad_request("Quantity must be positive");
    }
    let id = Uuid::new_v4();
    let now = Utc::now().timestamp();
    let price = req.price.unwrap_or(0.0).max(0.0);
    let amount = price * req.quantity;

    let result = sqlx::query(
        "INSERT INTO harvests (id, area_id, crop_batch_id, harvest_date, quantity, unit, grade, price, amount, operator, notes, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(&req.area_id)
    .bind(&req.crop_batch_id)
    .bind(&req.harvest_date)
    .bind(req.quantity)
    .bind(req.unit.as_deref().unwrap_or("kg"))
    .bind(req.grade.as_deref().unwrap_or(""))
    .bind(price)
    .bind(amount)
    .bind(req.operator.as_deref().unwrap_or(""))
    .bind(req.notes.as_deref().unwrap_or(""))
    .bind(now)
    .bind(now)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({"id": id.to_string(), "amount": amount, "message": "Harvest created"}))).into_response(),
        Err(e) => response::internal_err(e),
    }
}

async fn get_harvest(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let sql = format!("{} WHERE h.id = ?", HARVEST_SELECT);
    let row = sqlx::query(&sql)
        .bind(&id)
        .fetch_optional(&state.pool)
        .await;

    match row {
        Ok(Some(r)) => Json(harvest_to_json(&r)).into_response(),
        Ok(None) => response::not_found(Some("Harvest not found")),
        Err(e) => response::internal_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateHarvestRequest {
    pub harvest_date: Option<String>,
    pub quantity: Option<f64>,
    pub unit: Option<String>,
    pub grade: Option<String>,
    pub price: Option<f64>,
    pub operator: Option<String>,
    pub notes: Option<String>,
}

async fn update_harvest(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateHarvestRequest>,
) -> impl IntoResponse {
    let now = Utc::now().timestamp();
    let row = sqlx::query("SELECT quantity, price FROM harvests WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await;

    let (old_qty, old_price) = match row {
        Ok(Some(r)) => (
            r.try_get::<f64, _>("quantity").unwrap_or(0.0),
            r.try_get::<f64, _>("price").unwrap_or(0.0),
        ),
        Ok(None) => return response::not_found(Some("Harvest not found")),
        Err(e) => return response::internal_err(e),
    };

    let qty = req.quantity.unwrap_or(old_qty);
    let price = req.price.unwrap_or(old_price);
    let amount = price * qty;

    let result = sqlx::query(
        "UPDATE harvests SET harvest_date = COALESCE(?, harvest_date), quantity = ?, unit = COALESCE(?, unit), grade = COALESCE(?, grade), price = ?, amount = ?, operator = COALESCE(?, operator), notes = COALESCE(?, notes), updated_at = ? WHERE id = ?",
    )
    .bind(&req.harvest_date)
    .bind(qty)
    .bind(&req.unit)
    .bind(&req.grade)
    .bind(price)
    .bind(amount)
    .bind(&req.operator)
    .bind(&req.notes)
    .bind(now)
    .bind(&id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({"message": "Harvest updated", "amount": amount})).into_response(),
        Err(e) => response::internal_err(e),
    }
}

async fn delete_harvest(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM harvests WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(_) => Json(serde_json::json!({"message": "Harvest deleted"})).into_response(),
        Err(e) => response::internal_err(e),
    }
}

// ==================== Analysis ====================

#[derive(Debug, Deserialize)]
pub struct AnalysisQuery {
    pub area_id: Option<String>,
    pub crop_id: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

async fn get_analysis(
    State(state): State<AppState>,
    Query(q): Query<AnalysisQuery>,
) -> impl IntoResponse {
    let mut binds: Vec<String> = Vec::new();

    // 1. yield summary
    let mut ysql = String::from(
        "SELECT h.area_id, a.name AS area_name, COALESCE(SUM(h.quantity), 0) AS total_quantity, COALESCE(SUM(h.amount), 0) AS total_amount, COUNT(*) AS harvest_count FROM harvests h LEFT JOIN areas a ON a.id = h.area_id WHERE 1=1"
    );
    if let Some(ref area_id) = q.area_id {
        ysql.push_str(" AND h.area_id = ?");
        binds.push(area_id.clone());
    }
    if let Some(ref date_from) = q.date_from {
        ysql.push_str(" AND h.harvest_date >= ?");
        binds.push(date_from.clone());
    }
    if let Some(ref date_to) = q.date_to {
        ysql.push_str(" AND h.harvest_date <= ?");
        binds.push(date_to.clone());
    }
    ysql.push_str(" GROUP BY h.area_id");

    let mut yield_q = sqlx::query(&ysql);
    for b in &binds {
        yield_q = yield_q.bind(b);
    }

    // 2. daily yield trend
    let mut tsql = String::from(
        "SELECT h.harvest_date, COALESCE(SUM(h.quantity), 0) AS quantity, COALESCE(SUM(h.amount), 0) AS amount FROM harvests h WHERE 1=1"
    );
    let mut tbinds: Vec<String> = Vec::new();
    if let Some(ref area_id) = q.area_id {
        tsql.push_str(" AND h.area_id = ?");
        tbinds.push(area_id.clone());
    }
    if let Some(ref date_from) = q.date_from {
        tsql.push_str(" AND h.harvest_date >= ?");
        tbinds.push(date_from.clone());
    }
    if let Some(ref date_to) = q.date_to {
        tsql.push_str(" AND h.harvest_date <= ?");
        tbinds.push(date_to.clone());
    }
    tsql.push_str(" GROUP BY h.harvest_date ORDER BY h.harvest_date");

    let mut trend_q = sqlx::query(&tsql);
    for b in &tbinds {
        trend_q = trend_q.bind(b);
    }

    // 3. grade distribution
    let mut gsql = String::from(
        "SELECT h.grade, COALESCE(SUM(h.quantity), 0) AS quantity, COALESCE(SUM(h.amount), 0) AS amount FROM harvests h WHERE h.grade != ''"
    );
    let mut gbinds: Vec<String> = Vec::new();
    if let Some(ref area_id) = q.area_id {
        gsql.push_str(" AND h.area_id = ?");
        gbinds.push(area_id.clone());
    }
    if let Some(ref date_from) = q.date_from {
        gsql.push_str(" AND h.harvest_date >= ?");
        gbinds.push(date_from.clone());
    }
    if let Some(ref date_to) = q.date_to {
        gsql.push_str(" AND h.harvest_date <= ?");
        gbinds.push(date_to.clone());
    }
    gsql.push_str(" GROUP BY h.grade");

    let mut grade_q = sqlx::query(&gsql);
    for b in &gbinds {
        grade_q = grade_q.bind(b);
    }

    // 4. operational cost estimate from farm logs (fertilizer + pesticide items matched against inventory prices)
    let mut csql = String::from(
        "SELECT fo.category, fo.details, fo.area_id FROM farm_operations fo WHERE fo.category IN ('施肥', '打药') AND fo.details != '{}'"
    );
    let mut cbinds: Vec<String> = Vec::new();
    if let Some(ref area_id) = q.area_id {
        csql.push_str(" AND fo.area_id = ?");
        cbinds.push(area_id.clone());
    }
    if let Some(ref date_from) = q.date_from {
        csql.push_str(" AND fo.log_date >= ?");
        cbinds.push(date_from.clone());
    }
    if let Some(ref date_to) = q.date_to {
        csql.push_str(" AND fo.log_date <= ?");
        cbinds.push(date_to.clone());
    }

    let mut cost_q = sqlx::query(&csql);
    for b in &cbinds {
        cost_q = cost_q.bind(b);
    }

    let (yield_res, trend_res, grade_res, ops_res) = tokio::join!(
        yield_q.fetch_all(&state.pool),
        trend_q.fetch_all(&state.pool),
        grade_q.fetch_all(&state.pool),
        cost_q.fetch_all(&state.pool),
    );

    match (yield_res, trend_res, grade_res, ops_res) {
        (Ok(yrows), Ok(trows), Ok(grows), Ok(orows)) => {
            let mut input_cost = 0.0f64;
            let mut op_counts: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();

            for row in &orows {
                let category: String = row.try_get("category").unwrap_or_default();
                *op_counts.entry(category.clone()).or_insert(serde_json::json!(0)) =
                    serde_json::json!(op_counts.get(&category).and_then(|v| v.as_i64()).unwrap_or(0) + 1);

                let details: Option<String> = row.try_get("details").unwrap_or(None);
                if let Some(details) = details {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&details) {
                        if let Some(items) = v.get("items").and_then(|i| i.as_array()) {
                            for item in items {
                                let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("");
                                let amount = item.get("amount").and_then(|a| a.as_str())
                                    .and_then(|s| s.split_whitespace().next())
                                    .and_then(|s| s.parse::<f64>().ok())
                                    .unwrap_or(0.0);
                                input_cost += estimate_item_cost(&state, name, amount).await;
                            }
                        }
                    }
                }
            }

            let areas: Vec<serde_json::Value> = yrows.iter().map(|r| serde_json::json!({
                "area_id": r.try_get::<String, _>("area_id").unwrap_or_default(),
                "area_name": r.try_get::<Option<String>, _>("area_name").unwrap_or(None).unwrap_or_default(),
                "total_quantity": r.try_get::<f64, _>("total_quantity").unwrap_or(0.0),
                "total_amount": r.try_get::<f64, _>("total_amount").unwrap_or(0.0),
                "harvest_count": r.try_get::<i64, _>("harvest_count").unwrap_or(0),
            })).collect();

            let trend: Vec<serde_json::Value> = trows.iter().map(|r| serde_json::json!({
                "date": r.try_get::<String, _>("harvest_date").unwrap_or_default(),
                "quantity": r.try_get::<f64, _>("quantity").unwrap_or(0.0),
                "amount": r.try_get::<f64, _>("amount").unwrap_or(0.0),
            })).collect();

            let grades: Vec<serde_json::Value> = grows.iter().map(|r| serde_json::json!({
                "grade": r.try_get::<String, _>("grade").unwrap_or_default(),
                "quantity": r.try_get::<f64, _>("quantity").unwrap_or(0.0),
                "amount": r.try_get::<f64, _>("amount").unwrap_or(0.0),
            })).collect();

            let total_quantity: f64 = areas.iter().map(|a| a["total_quantity"].as_f64().unwrap_or(0.0)).sum();
            let total_revenue: f64 = areas.iter().map(|a| a["total_amount"].as_f64().unwrap_or(0.0)).sum();

            Json(serde_json::json!({
                "yield": {
                    "total_quantity": total_quantity,
                    "total_revenue": total_revenue,
                    "total_harvests": areas.iter().map(|a| a["harvest_count"].as_i64().unwrap_or(0)).sum::<i64>(),
                    "input_cost_estimate": input_cost,
                    "net_profit_estimate": total_revenue - input_cost,
                },
                "areas": areas,
                "trend": trend,
                "grades": grades,
                "operations": op_counts,
            })).into_response()
        }
        _ => response::internal_err("Failed to aggregate yield analysis"),
    }
}

async fn estimate_item_cost(state: &AppState, name: &str, amount: f64) -> f64 {
    if name.is_empty() || amount <= 0.0 {
        return 0.0;
    }
    let row = sqlx::query("SELECT price FROM inventory_items WHERE name = ? ORDER BY created_at DESC LIMIT 1")
        .bind(name)
        .fetch_optional(&state.pool)
        .await;
    match row {
        Ok(Some(r)) => r.try_get::<f64, _>("price").unwrap_or(0.0) * amount,
        Ok(None) => 0.0,
        Err(_) => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, http::StatusCode};
    use sqlx::{Executor, SqlitePool};
    use tower::ServiceExt;

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let schemas = [
            concat!(env!("CARGO_MANIFEST_DIR"), "/../agri-core/migrations/001_init.sql"),
            concat!(env!("CARGO_MANIFEST_DIR"), "/../agri-core/migrations/009_farm_operations.sql"),
            concat!(env!("CARGO_MANIFEST_DIR"), "/../agri-core/migrations/012_inventory.sql"),
            concat!(env!("CARGO_MANIFEST_DIR"), "/../agri-core/migrations/013_yield.sql"),
            concat!(env!("CARGO_MANIFEST_DIR"), "/../agri-core/migrations/015_labor.sql"),
        ];
        for path in schemas {
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
    async fn harvest_crud_and_analysis() {
        let pool = setup_db().await;
        let app = create_router(test_state(pool.clone())).into_service();

        // seed area + crop + batch
        pool.execute("INSERT INTO areas (id, name, created_at) VALUES ('a1', '主大棚', 0)").await.unwrap();
        pool.execute("INSERT INTO crops (id, name, comfort_config, created_at) VALUES ('c1', '番茄', '{}', 0)").await.unwrap();
        pool.execute("INSERT INTO crop_batches (id, area_id, crop_id, plant_date, status, created_at) VALUES ('b1', 'a1', 'c1', 0, 'active', 0)").await.unwrap();

        // create harvests
        for (date, qty, price) in [("2026-08-01", 50.0, 4.0), ("2026-08-02", 30.0, 5.0), ("2026-08-02", 20.0, 6.0)] {
            let res = app.clone().oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/yield/harvests")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"area_id":"a1","crop_batch_id":"b1","harvest_date":"{}","quantity":{},"price":{},"grade":"A"}}"#, date, qty, price)))
                    .unwrap(),
            ).await.unwrap();
            assert_eq!(res.status(), StatusCode::CREATED);
        }

        // list with filter
        let res = app.clone().oneshot(
            Request::builder().method("GET").uri("/api/v1/yield/harvests?date_from=2026-08-02").body(Body::empty()).unwrap(),
        ).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(list["harvests"].as_array().unwrap().len(), 2);

        // analysis: total 100kg, revenue 50*4+30*5+20*6 = 470
        let res = app.clone().oneshot(
            Request::builder().method("GET").uri("/api/v1/yield/analysis?area_id=a1").body(Body::empty()).unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let analysis: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(analysis["yield"]["total_quantity"], 100.0);
        assert_eq!(analysis["yield"]["total_revenue"], 470.0);
        assert_eq!(analysis["yield"]["total_harvests"], 3);
        assert_eq!(analysis["trend"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn analysis_accounts_for_input_costs() {
        let pool = setup_db().await;
        let app = create_router(test_state(pool.clone())).into_service();

        pool.execute("INSERT INTO areas (id, name, created_at) VALUES ('a1', '主大棚', 0)").await.unwrap();
        // inventory item to price matched inputs
        pool.execute("INSERT INTO inventory_items (id, name, category, unit, price, stock, created_at, updated_at) VALUES ('i1', '尿素', 'fertilizer', 'kg', 3.0, 100, 0, 0)").await.unwrap();
        // farm op with fertilizer details (urea 10kg)
        pool.execute("INSERT INTO farm_operations (id, area_id, log_date, category, content, details, created_at, updated_at) VALUES ('o1', 'a1', '2026-08-01', '施肥', '施用尿素', '{\"items\":[{\"name\":\"尿素\",\"amount\":\"10 kg\"}]}', 0, 0)").await.unwrap();
        // harvest
        let res = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/yield/harvests")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"area_id":"a1","harvest_date":"2026-08-03","quantity":100,"price":5.0}"#))
                .unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let res = app.clone().oneshot(
            Request::builder().method("GET").uri("/api/v1/yield/analysis?area_id=a1").body(Body::empty()).unwrap(),
        ).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let analysis: serde_json::Value = serde_json::from_slice(&body).unwrap();
        // input cost = 10kg * 3.0 = 30, revenue = 500
        assert_eq!(analysis["yield"]["input_cost_estimate"], 30.0);
        assert_eq!(analysis["yield"]["net_profit_estimate"], 470.0);
        assert_eq!(analysis["operations"]["施肥"], 1);
    }

    #[tokio::test]
    async fn analysis_accounts_for_labor_costs() {
        let pool = setup_db().await;
        // seed labor_records table (GREEN provides 015_labor.sql)
        let app = create_router(test_state(pool.clone())).into_service();

        pool.execute("INSERT INTO areas (id, name, created_at) VALUES ('a1', '主大棚', 0)").await.unwrap();
        // labor: 8-01 采收 3人8h @20 = 480；8-02 打药 1人4h @25 = 100 → labor total 580
        pool.execute("INSERT INTO labor_records (id, log_date, area_id, category, worker, worker_count, work_hours, rate, amount, paid_status, operator, notes, created_at, updated_at) VALUES ('l1', '2026-08-01', 'a1', '采收', '', 3, 8, 20.0, 480.0, 'unpaid', '', '', 0, 0)").await.unwrap();
        pool.execute("INSERT INTO labor_records (id, log_date, area_id, category, worker, worker_count, work_hours, rate, amount, paid_status, operator, notes, created_at, updated_at) VALUES ('l2', '2026-08-02', 'a1', '打药', '', 1, 4, 25.0, 100.0, 'unpaid', '', '', 0, 0)").await.unwrap();
        // harvest: 100kg @5 = 500
        let res = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/yield/harvests")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"area_id":"a1","harvest_date":"2026-08-03","quantity":100,"price":5.0}"#))
                .unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let res = app.clone().oneshot(
            Request::builder().method("GET").uri("/api/v1/yield/analysis?area_id=a1").body(Body::empty()).unwrap(),
        ).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let analysis: serde_json::Value = serde_json::from_slice(&body).unwrap();
        // revenue 500 - input 0 - labor 580 = -80
        assert_eq!(analysis["yield"]["labor_cost_estimate"], 580.0);
        assert_eq!(analysis["yield"]["net_profit_estimate"], -80.0);
    }
}