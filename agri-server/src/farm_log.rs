use agri_core::models::{FarmOpStatus, FarmOperation, FarmOpTemplate, JsonValue, UuidText};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put, delete},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::response;
use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/farm/operations", get(list_operations).post(create_operation))
        .route("/api/v1/farm/operations/:id", get(get_operation).put(update_operation).delete(delete_operation))
        .route("/api/v1/farm/templates", get(list_templates).post(create_template))
        .route("/api/v1/farm/templates/:id", put(update_template).delete(delete_template))
        .with_state(state)
}

// ==================== Operations ====================

#[derive(Debug, Deserialize)]
pub struct ListOperationsQuery {
    pub area_id: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub category: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

async fn list_operations(
    State(state): State<AppState>,
    Query(q): Query<ListOperationsQuery>,
) -> impl IntoResponse {
    let page = q.page.unwrap_or(1).max(1);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * limit;

    let mut sql = String::from(
        "SELECT id, area_id, log_date, log_time, category, content, operator, status, weather, crop_status, notes, details, created_at, updated_at FROM farm_operations WHERE 1=1"
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

    sql.push_str(" ORDER BY log_date DESC, log_time DESC LIMIT ? OFFSET ?");
    binds.push(limit.to_string());
    binds.push(offset.to_string());

    let mut query = sqlx::query_as::<_, FarmOperation>(&sql);
    for b in &binds {
        query = query.bind(b);
    }

    match query.fetch_all(&state.pool).await {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.into_iter().map(op_to_json).collect();
            Json(serde_json::json!({"operations": result, "page": page, "limit": limit})).into_response()
        }
        Err(e) => response::internal_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateOperationRequest {
    pub area_id: String,
    pub log_date: String,
    pub log_time: Option<String>,
    pub category: String,
    pub content: String,
    pub operator: Option<String>,
    pub weather: Option<String>,
    pub crop_status: Option<String>,
    pub notes: Option<String>,
    pub details: Option<serde_json::Value>,
}

async fn create_operation(
    State(state): State<AppState>,
    Json(req): Json<CreateOperationRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4();
    let now = Utc::now().timestamp();
    let details = serde_json::to_string(&req.details.unwrap_or(serde_json::json!({}))).unwrap_or_default();

    let result = sqlx::query(
        "INSERT INTO farm_operations (id, area_id, log_date, log_time, category, content, operator, status, weather, crop_status, notes, details, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(&req.area_id)
    .bind(&req.log_date)
    .bind(req.log_time.as_deref().unwrap_or(""))
    .bind(&req.category)
    .bind(&req.content)
    .bind(req.operator.as_deref().unwrap_or(""))
    .bind("completed")
    .bind(req.weather.as_deref().unwrap_or(""))
    .bind(req.crop_status.as_deref().unwrap_or(""))
    .bind(req.notes.as_deref().unwrap_or(""))
    .bind(&details)
    .bind(now)
    .bind(now)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({"id": id.to_string(), "message": "Operation created"}))).into_response(),
        Err(e) => response::internal_err(e),
    }
}

async fn get_operation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let row = sqlx::query_as::<_, FarmOperation>(
        "SELECT id, area_id, log_date, log_time, category, content, operator, status, weather, crop_status, notes, details, created_at, updated_at FROM farm_operations WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await;

    match row {
        Ok(Some(op)) => Json(op_to_json(op)).into_response(),
        Ok(None) => response::not_found(Some("Operation not found")),
        Err(e) => response::internal_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateOperationRequest {
    pub log_time: Option<String>,
    pub content: Option<String>,
    pub operator: Option<String>,
    pub status: Option<String>,
    pub weather: Option<String>,
    pub crop_status: Option<String>,
    pub notes: Option<String>,
    pub details: Option<serde_json::Value>,
}

async fn update_operation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateOperationRequest>,
) -> impl IntoResponse {
    let now = Utc::now().timestamp();
    let details = req.details.map(|d| serde_json::to_string(&d).unwrap_or_default());

    let result = sqlx::query(
        "UPDATE farm_operations SET log_time = COALESCE(?, log_time), content = COALESCE(?, content), operator = COALESCE(?, operator), status = COALESCE(?, status), weather = COALESCE(?, weather), crop_status = COALESCE(?, crop_status), notes = COALESCE(?, notes), details = COALESCE(?, details), updated_at = ? WHERE id = ?",
    )
    .bind(&req.log_time)
    .bind(&req.content)
    .bind(&req.operator)
    .bind(&req.status)
    .bind(&req.weather)
    .bind(&req.crop_status)
    .bind(&req.notes)
    .bind(&details)
    .bind(now)
    .bind(&id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({"message": "Operation updated"})).into_response(),
        Err(e) => response::internal_err(e),
    }
}

async fn delete_operation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM farm_operations WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(_) => Json(serde_json::json!({"message": "Operation deleted"})).into_response(),
        Err(e) => response::internal_err(e),
    }
}

// ==================== Templates ====================

#[derive(Debug, Deserialize)]
pub struct ListTemplatesQuery {
    pub category: Option<String>,
}

async fn list_templates(
    State(state): State<AppState>,
    Query(q): Query<ListTemplatesQuery>,
) -> impl IntoResponse {
    let mut sql = String::from(
        "SELECT id, name, category, details, sort_order, created_at FROM farm_operation_templates"
    );
    if q.category.is_some() {
        sql.push_str(" WHERE category = ?");
    }
    sql.push_str(" ORDER BY sort_order ASC, name ASC");

    let mut query = sqlx::query_as::<_, FarmOpTemplate>(&sql);
    if let Some(ref cat) = q.category {
        query = query.bind(cat);
    }

    match query.fetch_all(&state.pool).await {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.into_iter().map(|t| {
                serde_json::json!({
                    "id": t.id.to_string(),
                    "name": t.name,
                    "category": t.category,
                    "details": t.details,
                    "sort_order": t.sort_order,
                    "created_at": t.created_at.timestamp(),
                })
            }).collect();
            Json(result).into_response()
        }
        Err(e) => response::internal_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub category: String,
    pub details: Option<serde_json::Value>,
    pub sort_order: Option<i32>,
}

async fn create_template(
    State(state): State<AppState>,
    Json(req): Json<CreateTemplateRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4();
    let now = Utc::now().timestamp();
    let details = serde_json::to_string(&req.details.unwrap_or(serde_json::json!({}))).unwrap_or_default();

    let result = sqlx::query(
        "INSERT INTO farm_operation_templates (id, name, category, details, sort_order, created_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(&req.name)
    .bind(&req.category)
    .bind(&details)
    .bind(req.sort_order.unwrap_or(0))
    .bind(now)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({"id": id.to_string(), "message": "Template created"}))).into_response(),
        Err(e) => response::internal_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateTemplateRequest {
    pub name: Option<String>,
    pub details: Option<serde_json::Value>,
    pub sort_order: Option<i32>,
}

async fn update_template(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTemplateRequest>,
) -> impl IntoResponse {
    let details = req.details.map(|d| serde_json::to_string(&d).unwrap_or_default());

    let result = sqlx::query(
        "UPDATE farm_operation_templates SET name = COALESCE(?, name), details = COALESCE(?, details), sort_order = COALESCE(?, sort_order) WHERE id = ?",
    )
    .bind(&req.name)
    .bind(&details)
    .bind(req.sort_order)
    .bind(&id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({"message": "Template updated"})).into_response(),
        Err(e) => response::internal_err(e),
    }
}

async fn delete_template(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM farm_operation_templates WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(_) => Json(serde_json::json!({"message": "Template deleted"})).into_response(),
        Err(e) => response::internal_err(e),
    }
}

// ==================== Helpers ====================

fn op_to_json(op: FarmOperation) -> serde_json::Value {
    let status_str = match op.status {
        FarmOpStatus::Planned => "planned",
        FarmOpStatus::InProgress => "in_progress",
        FarmOpStatus::Completed => "completed",
        FarmOpStatus::Cancelled => "cancelled",
    };
    serde_json::json!({
        "id": op.id.to_string(),
        "area_id": op.area_id.to_string(),
        "log_date": op.log_date,
        "log_time": op.log_time,
        "category": op.category,
        "content": op.content,
        "operator": op.operator,
        "status": status_str,
        "weather": op.weather,
        "crop_status": op.crop_status,
        "notes": op.notes,
        "details": op.details,
        "created_at": op.created_at.timestamp(),
        "updated_at": op.updated_at.timestamp(),
    })
}
