use agri_core::models::{Area, Crop};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, put},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // 区域 API
        .route("/api/v1/areas", get(list_areas).post(create_area))
        .route("/api/v1/areas/:id", get(get_area).put(update_area).delete(delete_area))
        .route("/api/v1/areas/:id/crop-name", put(update_area_crop_name))
        // 作物 API
        .route("/api/v1/crops", get(list_crops).post(create_crop))
        .route("/api/v1/crops/:id", get(get_crop).put(update_crop).delete(delete_crop))
        // 茬口 API
        .route("/api/v1/crop-batches", get(list_crop_batches).post(create_crop_batch))
        .route("/api/v1/crop-batches/:id", get(get_crop_batch).put(update_crop_batch).delete(delete_crop_batch))
        // 查询茬口下的传感器数据
        .route("/api/v1/crop-batches/:id/readings", get(get_crop_batch_readings))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
pub struct CreateAreaRequest {
    pub name: String,
    pub description: Option<String>,
}

async fn create_area(
    State(state): State<AppState>,
    Json(req): Json<CreateAreaRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4();
    let now = Utc::now().timestamp();
    let result = sqlx::query(
        "INSERT INTO areas (id, name, description, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(&req.name)
    .bind(&req.description)
    .bind(now)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({"id": id.to_string(), "message": "Area created"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn list_areas(State(state): State<AppState>) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, Area>(
        "SELECT id, name, description, created_at FROM areas",
    )
    .fetch_all(&state.pool)
    .await;

    match rows {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|a| {
                    serde_json::json!({
                        "id": a.id,
                        "name": a.name,
                        "description": a.description,
                        "created_at": a.created_at.timestamp(),
                    })
                })
                .collect();
            Json(result).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn get_area(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let row = sqlx::query_as::<_, Area>(
        "SELECT id, name, description, created_at FROM areas WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await;

    match row {
        Ok(Some(a)) => Json(serde_json::json!({
            "id": a.id,
            "name": a.name,
            "description": a.description,
            "created_at": a.created_at.timestamp(),
        }))
        .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Area not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn update_area(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<CreateAreaRequest>,
) -> impl IntoResponse {
    let result = sqlx::query("UPDATE areas SET name = ?, description = ? WHERE id = ?")
        .bind(&req.name)
        .bind(&req.description)
        .bind(&id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({"message": "Area updated"})).into_response(),
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Area not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn delete_area(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM areas WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({"message": "Area deleted"})).into_response(),
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Area not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn update_area_crop_name(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let crop_name = match req.get("crop_name").and_then(|v| v.as_str()) {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "crop_name required"}))).into_response(),
    };

    let batch = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT cb.id, cb.crop_id FROM crop_batches cb WHERE cb.area_id = ? AND cb.status = 'active' LIMIT 1"
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await;

    match batch {
        Ok(Some((_batch_id, Some(crop_id)))) => {
            if let Err(e) = sqlx::query("UPDATE crops SET name = ? WHERE id = ?")
                .bind(&crop_name)
                .bind(&crop_id)
                .execute(&state.pool)
                .await
            {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
            }
            Json(serde_json::json!({"message": "Crop name updated"})).into_response()
        }
        Ok(Some((batch_id, None))) => {
            let new_crop_id = Uuid::new_v4().to_string();
            let now = Utc::now().timestamp();
            let default_config = serde_json::json!({"temperature": {"min": 15, "max": 30}}).to_string();
            if let Err(e) = sqlx::query("INSERT INTO crops (id, name, comfort_config, created_at) VALUES (?, ?, ?, ?)")
                .bind(&new_crop_id)
                .bind(&crop_name)
                .bind(&default_config)
                .bind(now)
                .execute(&state.pool)
                .await
            {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
            }
            if let Err(e) = sqlx::query("UPDATE crop_batches SET crop_id = ? WHERE id = ?")
                .bind(&new_crop_id)
                .bind(&batch_id)
                .execute(&state.pool)
                .await
            {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
            }
            Json(serde_json::json!({"message": "Crop created and linked"})).into_response()
        }
        Ok(None) => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "No active crop batch for this area"}))).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

// ========== Crop API ==========

#[derive(Debug, Deserialize)]
pub struct CreateCropRequest {
    pub name: String,
    pub comfort_config: serde_json::Value,  // { "temperature": {"min": 15, "max": 30}, ... }
}

async fn create_crop(
    State(state): State<AppState>,
    Json(req): Json<CreateCropRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4();
    let now = Utc::now().timestamp();
    let comfort_config_str = req.comfort_config.to_string();
    let result = sqlx::query(
        "INSERT INTO crops (id, name, comfort_config, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(&req.name)
    .bind(&comfort_config_str)
    .bind(now)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({"id": id.to_string(), "message": "Crop created"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn list_crops(State(state): State<AppState>) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, Crop>(
        "SELECT id, name, comfort_config, created_at FROM crops",
    )
    .fetch_all(&state.pool)
    .await;

    match rows {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|c| {
                    serde_json::json!({
                        "id": c.id,
                        "name": c.name,
                        "comfort_config": c.comfort_config,
                        "created_at": c.created_at.timestamp(),
                    })
                })
                .collect();
            Json(result).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn get_crop(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let row = sqlx::query_as::<_, Crop>(
        "SELECT id, name, comfort_config, created_at FROM crops WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await;

    match row {
        Ok(Some(c)) => Json(serde_json::json!({
            "id": c.id,
            "name": c.name,
            "comfort_config": c.comfort_config,
            "created_at": c.created_at.timestamp(),
        }))
        .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Crop not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn update_crop(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<CreateCropRequest>,
) -> impl IntoResponse {
    let comfort_config_str = req.comfort_config.to_string();
    let result = sqlx::query("UPDATE crops SET name = ?, comfort_config = ? WHERE id = ?")
        .bind(&req.name)
        .bind(&comfort_config_str)
        .bind(&id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({"message": "Crop updated"})).into_response(),
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Crop not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn delete_crop(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM crops WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({"message": "Crop deleted"})).into_response(),
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Crop not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

// ========== CropBatch API ==========

#[derive(Debug, Deserialize)]
pub struct CreateCropBatchRequest {
    pub area_id: String,
    pub crop_id: String,
    pub plant_date: i64,
    pub expected_harvest_date: Option<i64>,
}

async fn create_crop_batch(
    State(state): State<AppState>,
    Json(req): Json<CreateCropBatchRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4();
    let now = Utc::now().timestamp();
    let result = sqlx::query(
        "INSERT INTO crop_batches (id, area_id, crop_id, plant_date, expected_harvest_date, status, created_at) VALUES (?, ?, ?, ?, ?, 'active', ?)",
    )
    .bind(id.to_string())
    .bind(&req.area_id)
    .bind(&req.crop_id)
    .bind(req.plant_date)
    .bind(req.expected_harvest_date)
    .bind(now)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({"id": id.to_string(), "message": "Crop batch created"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct CropBatchQuery {
    pub area_id: Option<String>,
    pub status: Option<String>,
}

async fn list_crop_batches(
    State(state): State<AppState>,
    Query(query): Query<CropBatchQuery>,
) -> impl IntoResponse {
    let mut sql = String::from("SELECT cb.id, cb.area_id, cb.crop_id, cb.plant_date, cb.expected_harvest_date, cb.status, cb.created_at, c.name as crop_name, a.name as area_name FROM crop_batches cb LEFT JOIN crops c ON cb.crop_id = c.id LEFT JOIN areas a ON cb.area_id = a.id WHERE 1=1");
    let mut bindings = Vec::new();

    if let Some(ref area_id) = query.area_id {
        sql.push_str(" AND cb.area_id = ?");
        bindings.push(area_id.clone());
    }
    if let Some(ref status) = query.status {
        sql.push_str(" AND cb.status = ?");
        bindings.push(status.clone());
    }

    let mut query_builder = sqlx::query_as::<_, (String, String, String, i64, Option<i64>, String, i64, String, String)>(&sql);
    for binding in bindings {
        query_builder = query_builder.bind(binding);
    }

    let rows = query_builder.fetch_all(&state.pool).await;

    match rows {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.0,
                        "area_id": r.1,
                        "crop_id": r.2,
                        "plant_date": r.3,
                        "expected_harvest_date": r.4,
                        "status": r.5,
                        "created_at": r.6,
                        "crop_name": r.7,
                        "area_name": r.8
                    })
                })
                .collect();
            Json(result).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn get_crop_batch(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let row = sqlx::query_as::<_, (String, String, String, i64, Option<i64>, String, i64, String, String)>(
        "SELECT cb.id, cb.area_id, cb.crop_id, cb.plant_date, cb.expected_harvest_date, cb.status, cb.created_at, c.name, a.name FROM crop_batches cb LEFT JOIN crops c ON cb.crop_id = c.id LEFT JOIN areas a ON cb.area_id = a.id WHERE cb.id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await;

    match row {
        Ok(Some(r)) => Json(serde_json::json!({
            "id": r.0,
            "area_id": r.1,
            "crop_id": r.2,
            "plant_date": r.3,
            "expected_harvest_date": r.4,
            "status": r.5,
            "created_at": r.6,
            "crop_name": r.7,
            "area_name": r.8
        }))
        .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Crop batch not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateCropBatchRequest {
    pub status: Option<String>,
    pub expected_harvest_date: Option<i64>,
}

async fn update_crop_batch(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateCropBatchRequest>,
) -> impl IntoResponse {
    let mut sql = String::from("UPDATE crop_batches SET ");
    let mut updates = Vec::new();

    if req.status.is_some() {
        updates.push("status = ?");
    }
    if req.expected_harvest_date.is_some() {
        updates.push("expected_harvest_date = ?");
    }

    if updates.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "No fields to update"}))).into_response();
    }

    sql.push_str(&updates.join(", "));
    sql.push_str(" WHERE id = ?");

    let mut query = sqlx::query(&sql);
    if let Some(ref status) = req.status {
        query = query.bind(status);
    }
    if let Some(date) = req.expected_harvest_date {
        query = query.bind(date);
    }
    query = query.bind(&id);

    let result = query.execute(&state.pool).await;

    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({"message": "Crop batch updated"})).into_response(),
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Crop batch not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn delete_crop_batch(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM crop_batches WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({"message": "Crop batch deleted"})).into_response(),
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Crop batch not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

// ========== 茬口数据查询 API ==========

#[derive(Debug, Deserialize)]
pub struct CropBatchReadingsQuery {
    pub metric: Option<String>,
    pub start: Option<i64>,
    pub end: Option<i64>,
}

async fn get_crop_batch_readings(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<CropBatchReadingsQuery>,
) -> impl IntoResponse {
    // 先获取茬口信息（区域ID）
    let batch = sqlx::query_as::<_, (String,)>(
        "SELECT area_id FROM crop_batches WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await;

    let area_id = match batch {
        Ok(Some(row)) => row.0,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Crop batch not found"}))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    let has_metric = query.metric.is_some();
    let has_start = query.start.is_some();
    let has_end = query.end.is_some();
    let mut sql = String::from("SELECT sr.id, sr.device_id, sr.metric, sr.value, sr.unit, sr.timestamp FROM sensor_readings sr INNER JOIN devices d ON sr.device_id = d.id WHERE d.area_id = ?");
    if has_metric { sql.push_str(" AND sr.metric = ?"); }
    if has_start { sql.push_str(" AND sr.timestamp >= ?"); }
    if has_end { sql.push_str(" AND sr.timestamp <= ?"); }
    sql.push_str(" ORDER BY sr.timestamp DESC");

    let mut q = sqlx::query_as::<_, (i64, String, String, f64, String, i64)>(&sql)
        .bind(&area_id);
    if let Some(ref metric) = query.metric { q = q.bind(metric); }
    if let Some(start) = query.start { q = q.bind(start); }
    if let Some(end) = query.end { q = q.bind(end); }
    let rows = q.fetch_all(&state.pool).await;

    match rows {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.0,
                        "device_id": r.1,
                        "metric": r.2,
                        "value": r.3,
                        "unit": r.4,
                        "timestamp": r.5
                    })
                })
                .collect();
            Json(result).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}
