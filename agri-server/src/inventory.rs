use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
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
        .route("/api/v1/inventory/items", get(list_items).post(create_item))
        .route("/api/v1/inventory/items/:id", get(get_item).put(update_item).delete(delete_item))
        .route("/api/v1/inventory/items/:id/transactions", get(list_item_transactions))
        .route("/api/v1/inventory/transactions", post(create_transaction))
        .route("/api/v1/inventory/summary", get(get_summary))
        .with_state(state)
}

// ==================== Items ====================

#[derive(Debug, Deserialize)]
pub struct ListItemsQuery {
    pub category: Option<String>,
    pub low_stock: Option<bool>,
    pub search: Option<String>,
}

fn item_to_json(row: &sqlx::sqlite::SqliteRow) -> serde_json::Value {
    serde_json::json!({
        "id": row.try_get::<String, _>("id").unwrap_or_default(),
        "name": row.try_get::<String, _>("name").unwrap_or_default(),
        "category": row.try_get::<String, _>("category").unwrap_or_default(),
        "unit": row.try_get::<String, _>("unit").unwrap_or_default(),
        "price": row.try_get::<f64, _>("price").unwrap_or(0.0),
        "stock": row.try_get::<f64, _>("stock").unwrap_or(0.0),
        "warning_threshold": row.try_get::<f64, _>("warning_threshold").unwrap_or(0.0),
        "low_stock": row.try_get::<f64, _>("stock").unwrap_or(0.0) <= row.try_get::<f64, _>("warning_threshold").unwrap_or(0.0),
        "batch_no": row.try_get::<String, _>("batch_no").unwrap_or_default(),
        "expiry_date": row.try_get::<String, _>("expiry_date").unwrap_or_default(),
        "manufacturer": row.try_get::<String, _>("manufacturer").unwrap_or_default(),
        "specs": row.try_get::<String, _>("specs").unwrap_or_default(),
        "notes": row.try_get::<String, _>("notes").unwrap_or_default(),
        "created_at": row.try_get::<i64, _>("created_at").unwrap_or(0),
        "updated_at": row.try_get::<i64, _>("updated_at").unwrap_or(0),
    })
}

async fn list_items(
    State(state): State<AppState>,
    Query(q): Query<ListItemsQuery>,
) -> impl IntoResponse {
    let mut sql = String::from(
        "SELECT id, name, category, unit, price, stock, warning_threshold, batch_no, expiry_date, manufacturer, specs, notes, created_at, updated_at FROM inventory_items WHERE 1=1"
    );
    let mut binds: Vec<String> = Vec::new();

    if let Some(ref category) = q.category {
        sql.push_str(" AND category = ?");
        binds.push(category.clone());
    }
    if q.low_stock == Some(true) {
        sql.push_str(" AND stock <= warning_threshold");
    }
    if let Some(ref search) = q.search {
        sql.push_str(" AND (name LIKE ? OR manufacturer LIKE ? OR batch_no LIKE ?)");
        let pat = format!("%{}%", search);
        binds.push(pat.clone());
        binds.push(pat.clone());
        binds.push(pat);
    }

    sql.push_str(" ORDER BY created_at DESC");

    let mut query = sqlx::query(&sql);
    for b in &binds {
        query = query.bind(b);
    }

    match query.fetch_all(&state.pool).await {
        Ok(rows) => {
            let items: Vec<serde_json::Value> = rows.iter().map(item_to_json).collect();
            Json(serde_json::json!({"items": items})).into_response()
        }
        Err(e) => response::internal_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateItemRequest {
    pub name: String,
    pub category: String,
    pub unit: Option<String>,
    pub price: Option<f64>,
    pub stock: Option<f64>,
    pub warning_threshold: Option<f64>,
    pub batch_no: Option<String>,
    pub expiry_date: Option<String>,
    pub manufacturer: Option<String>,
    pub specs: Option<String>,
    pub notes: Option<String>,
}

async fn create_item(
    State(state): State<AppState>,
    Json(req): Json<CreateItemRequest>,
) -> impl IntoResponse {
    let valid_categories = ["seed", "fertilizer", "pesticide", "materiel", "other"];
    if !valid_categories.contains(&req.category.as_str()) {
        return response::bad_request("Invalid category, must be one of: seed, fertilizer, pesticide, materiel, other");
    }
    if req.name.trim().is_empty() {
        return response::bad_request("Name is required");
    }

    let id = Uuid::new_v4();
    let now = Utc::now().timestamp();
    let initial_stock = req.stock.unwrap_or(0.0).max(0.0);

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return response::internal_err(e),
    };

    let result = sqlx::query(
        "INSERT INTO inventory_items (id, name, category, unit, price, stock, warning_threshold, batch_no, expiry_date, manufacturer, specs, notes, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(&req.name)
    .bind(&req.category)
    .bind(req.unit.as_deref().unwrap_or(""))
    .bind(req.price.unwrap_or(0.0))
    .bind(initial_stock)
    .bind(req.warning_threshold.unwrap_or(0.0))
    .bind(req.batch_no.as_deref().unwrap_or(""))
    .bind(req.expiry_date.as_deref().unwrap_or(""))
    .bind(req.manufacturer.as_deref().unwrap_or(""))
    .bind(req.specs.as_deref().unwrap_or(""))
    .bind(req.notes.as_deref().unwrap_or(""))
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await;

    if let Err(e) = result {
        let _ = tx.rollback().await;
        return response::internal_err(e);
    }

    if initial_stock > 0.0 {
        let txn_result = sqlx::query(
            "INSERT INTO inventory_transactions (id, item_id, txn_type, quantity, operator, related_type, related_id, note, created_at) VALUES (?, ?, 'in', ?, '', 'initial', '', '初始入库', ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(id.to_string())
        .bind(initial_stock)
        .bind(now)
        .execute(&mut *tx)
        .await;
        if let Err(e) = txn_result {
            let _ = tx.rollback().await;
            return response::internal_err(e);
        }
    }

    match tx.commit().await {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({"id": id.to_string(), "message": "Item created"}))).into_response(),
        Err(e) => response::internal_err(e),
    }
}

async fn get_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let row = sqlx::query(
        "SELECT id, name, category, unit, price, stock, warning_threshold, batch_no, expiry_date, manufacturer, specs, notes, created_at, updated_at FROM inventory_items WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await;

    match row {
        Ok(Some(r)) => Json(item_to_json(&r)).into_response(),
        Ok(None) => response::not_found(Some("Item not found")),
        Err(e) => response::internal_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateItemRequest {
    pub name: Option<String>,
    pub category: Option<String>,
    pub unit: Option<String>,
    pub price: Option<f64>,
    pub warning_threshold: Option<f64>,
    pub batch_no: Option<String>,
    pub expiry_date: Option<String>,
    pub manufacturer: Option<String>,
    pub specs: Option<String>,
    pub notes: Option<String>,
}

async fn update_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateItemRequest>,
) -> impl IntoResponse {
    let now = Utc::now().timestamp();
    let result = sqlx::query(
        "UPDATE inventory_items SET name = COALESCE(?, name), category = COALESCE(?, category), unit = COALESCE(?, unit), price = COALESCE(?, price), warning_threshold = COALESCE(?, warning_threshold), batch_no = COALESCE(?, batch_no), expiry_date = COALESCE(?, expiry_date), manufacturer = COALESCE(?, manufacturer), specs = COALESCE(?, specs), notes = COALESCE(?, notes), updated_at = ? WHERE id = ?",
    )
    .bind(&req.name)
    .bind(&req.category)
    .bind(&req.unit)
    .bind(req.price)
    .bind(req.warning_threshold)
    .bind(&req.batch_no)
    .bind(&req.expiry_date)
    .bind(&req.manufacturer)
    .bind(&req.specs)
    .bind(&req.notes)
    .bind(now)
    .bind(&id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({"message": "Item updated"})).into_response(),
        Err(e) => response::internal_err(e),
    }
}

async fn delete_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM inventory_items WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(_) => Json(serde_json::json!({"message": "Item deleted"})).into_response(),
        Err(e) => response::internal_err(e),
    }
}

// ==================== Transactions ====================

fn txn_to_json(row: &sqlx::sqlite::SqliteRow) -> serde_json::Value {
    serde_json::json!({
        "id": row.try_get::<String, _>("id").unwrap_or_default(),
        "item_id": row.try_get::<String, _>("item_id").unwrap_or_default(),
        "item_name": row.try_get::<Option<String>, _>("item_name").unwrap_or(None).unwrap_or_default(),
        "txn_type": row.try_get::<String, _>("txn_type").unwrap_or_default(),
        "quantity": row.try_get::<f64, _>("quantity").unwrap_or(0.0),
        "operator": row.try_get::<String, _>("operator").unwrap_or_default(),
        "related_type": row.try_get::<String, _>("related_type").unwrap_or_default(),
        "related_id": row.try_get::<String, _>("related_id").unwrap_or_default(),
        "note": row.try_get::<String, _>("note").unwrap_or_default(),
        "created_at": row.try_get::<i64, _>("created_at").unwrap_or(0),
    })
}

async fn list_item_transactions(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<serde_json::Value>,
) -> impl IntoResponse {
    let limit = q.get("limit").and_then(|v| v.as_i64()).unwrap_or(100).clamp(1, 500);
    let offset = q.get("offset").and_then(|v| v.as_i64()).unwrap_or(0).max(0);

    let rows = sqlx::query(
        "SELECT t.*, i.name AS item_name FROM inventory_transactions t LEFT JOIN inventory_items i ON i.id = t.item_id WHERE t.item_id = ? ORDER BY t.created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(&id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await;

    match rows {
        Ok(rows) => {
            let txns: Vec<serde_json::Value> = rows.iter().map(txn_to_json).collect();
            Json(serde_json::json!({"transactions": txns})).into_response()
        }
        Err(e) => response::internal_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTransactionRequest {
    pub item_id: String,
    pub txn_type: String,
    pub quantity: f64,
    pub operator: Option<String>,
    pub related_type: Option<String>,
    pub related_id: Option<String>,
    pub note: Option<String>,
}

async fn create_transaction(
    State(state): State<AppState>,
    Json(req): Json<CreateTransactionRequest>,
) -> impl IntoResponse {
    if !["in", "out", "adjust"].contains(&req.txn_type.as_str()) {
        return response::bad_request("Invalid txn_type, must be: in, out, adjust");
    }
    if req.quantity <= 0.0 {
        return response::bad_request("Quantity must be positive");
    }

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return response::internal_err(e),
    };

    let item = match sqlx::query("SELECT id, name, stock, warning_threshold FROM inventory_items WHERE id = ?")
        .bind(&req.item_id)
        .fetch_optional(&mut *tx)
        .await
    {
        Ok(Some(r)) => r,
        Ok(None) => {
            let _ = tx.rollback().await;
            return response::not_found(Some("Item not found"));
        }
        Err(e) => {
            let _ = tx.rollback().await;
            return response::internal_err(e);
        }
    };

    let current_stock: f64 = item.try_get("stock").unwrap_or(0.0);
    let new_stock = match req.txn_type.as_str() {
        "in" => current_stock + req.quantity,
        "out" => {
            if current_stock < req.quantity {
                let _ = tx.rollback().await;
                return response::err_json(StatusCode::BAD_REQUEST, format!(
                    "Insufficient stock: {} available, {} requested", current_stock, req.quantity
                ));
            }
            current_stock - req.quantity
        }
        _ => req.quantity, // adjust sets absolute value
    };

    let now = Utc::now().timestamp();
    let txn_id = Uuid::new_v4().to_string();

    // adjust 类型 quantity 表示目标库存，流水记录差值（正=增、负=减）
    let recorded_qty = if req.txn_type == "adjust" { new_stock - current_stock } else { req.quantity };

    let result = sqlx::query(
        "INSERT INTO inventory_transactions (id, item_id, txn_type, quantity, operator, related_type, related_id, note, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&txn_id)
    .bind(&req.item_id)
    .bind(&req.txn_type)
    .bind(recorded_qty)
    .bind(req.operator.as_deref().unwrap_or(""))
    .bind(req.related_type.as_deref().unwrap_or(""))
    .bind(req.related_id.as_deref().unwrap_or(""))
    .bind(req.note.as_deref().unwrap_or(""))
    .bind(now)
    .execute(&mut *tx)
    .await;

    if let Err(e) = result {
        let _ = tx.rollback().await;
        return response::internal_err(e);
    }

    let update = sqlx::query("UPDATE inventory_items SET stock = ?, updated_at = ? WHERE id = ?")
        .bind(new_stock)
        .bind(now)
        .bind(&req.item_id)
        .execute(&mut *tx)
        .await;

    if let Err(e) = update {
        let _ = tx.rollback().await;
        return response::internal_err(e);
    }

    match tx.commit().await {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({
            "id": txn_id,
            "message": "Transaction created",
            "new_stock": new_stock,
        }))).into_response(),
        Err(e) => response::internal_err(e),
    }
}

// ==================== Summary ====================

async fn get_summary(State(state): State<AppState>) -> impl IntoResponse {
    let rows = sqlx::query(
        "SELECT category, COUNT(*) AS count, COALESCE(SUM(stock * price), 0) AS total_value, COALESCE(SUM(CASE WHEN stock <= warning_threshold THEN 1 ELSE 0 END), 0) AS low_count FROM inventory_items GROUP BY category",
    )
    .fetch_all(&state.pool)
    .await;

    let total = sqlx::query(
        "SELECT COUNT(*) AS count, COALESCE(SUM(stock * price), 0) AS total_value, COALESCE(SUM(CASE WHEN stock <= warning_threshold THEN 1 ELSE 0 END), 0) AS low_count FROM inventory_items",
    )
    .fetch_one(&state.pool)
    .await;

    match (rows, total) {
        (Ok(rows), Ok(total)) => {
            let categories: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({
                "category": r.try_get::<String, _>("category").unwrap_or_default(),
                "count": r.try_get::<i64, _>("count").unwrap_or(0),
                "total_value": r.try_get::<f64, _>("total_value").unwrap_or(0.0),
                "low_count": r.try_get::<i64, _>("low_count").unwrap_or(0),
            })).collect();

            Json(serde_json::json!({
                "summary": {
                    "total_items": total.try_get::<i64, _>("count").unwrap_or(0),
                    "total_value": total.try_get::<f64, _>("total_value").unwrap_or(0.0),
                    "low_stock_count": total.try_get::<i64, _>("low_count").unwrap_or(0),
                },
                "categories": categories,
            })).into_response()
        }
        _ => response::internal_err("Failed to aggregate inventory summary"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, http::StatusCode};
    use sqlx::SqlitePool;
    use tower::ServiceExt;

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let schema = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../agri-core/migrations/012_inventory.sql")).unwrap();
        for stmt in schema.split(';').filter(|s| !s.trim().is_empty()) {
            sqlx::query(stmt).execute(&pool).await.unwrap();
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
    async fn item_crud_and_transactions() {
        let pool = setup_db().await;
        let app = create_router(test_state(pool)).into_service();

        // create item with initial stock
        let res = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/inventory/items")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"尿素","category":"fertilizer","unit":"kg","price":3.5,"stock":100,"warning_threshold":10}"#))
                .unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let item_id = created["id"].as_str().unwrap().to_string();

        // list
        let res = app.clone().oneshot(
            Request::builder().method("GET").uri("/api/v1/inventory/items").body(Body::empty()).unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(list["items"].as_array().unwrap().len(), 1);
        assert_eq!(list["items"][0]["stock"], 100.0);

        // stock out
        let res = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/inventory/transactions")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"item_id":"{}","txn_type":"out","quantity":30,"operator":"张三"}}"#, item_id)))
                .unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        // verify stock = 70
        let res = app.clone().oneshot(
            Request::builder().method("GET").uri(format!("/api/v1/inventory/items/{}", item_id)).body(Body::empty()).unwrap(),
        ).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let item: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(item["stock"], 70.0);

        // insufficient stock rejected
        let res = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/inventory/transactions")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"item_id":"{}","txn_type":"out","quantity":999}}"#, item_id)))
                .unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        // transactions list
        let res = app.clone().oneshot(
            Request::builder().method("GET").uri(format!("/api/v1/inventory/items/{}/transactions", item_id)).body(Body::empty()).unwrap(),
        ).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let txns: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(txns["transactions"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn low_stock_filter_and_summary() {
        let pool = setup_db().await;
        let app = create_router(test_state(pool)).into_service();

        for (name, cat, stock, threshold) in [
            ("尿素", "fertilizer", 200.0, 50.0),
            ("敌敌畏", "pesticide", 5.0, 20.0),
            ("种子A", "seed", 0.0, 10.0),
        ] {
            app.clone().oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/inventory/items")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"name":"{}","category":"{}","stock":{},"warning_threshold":{}}}"#, name, cat, stock, threshold)))
                    .unwrap(),
            ).await.unwrap();
        }

        // low stock filter
        let res = app.clone().oneshot(
            Request::builder().method("GET").uri("/api/v1/inventory/items?low_stock=true").body(Body::empty()).unwrap(),
        ).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(list["items"].as_array().unwrap().len(), 2);

        // summary
        let res = app.clone().oneshot(
            Request::builder().method("GET").uri("/api/v1/inventory/summary").body(Body::empty()).unwrap(),
        ).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let summary: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(summary["summary"]["total_items"], 3);
        assert_eq!(summary["summary"]["low_stock_count"], 2);
    }

    #[tokio::test]
    async fn invalid_category_rejected() {
        let pool = setup_db().await;
        let app = create_router(test_state(pool)).into_service();
        let res = app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/inventory/items")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"bad","category":"invalid"}"#))
                .unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}