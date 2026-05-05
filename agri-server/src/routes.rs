use agri_core::models::CommandPayload;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/devices", get(list_devices).post(create_device))
        .route("/api/v1/devices/{id}", get(get_device).put(update_device).delete(delete_device))
        .route("/api/v1/devices/{id}/readings", get(list_readings))
        .route("/api/v1/devices/{id}/command", post(send_command))
        .route("/api/v1/rules", get(list_rules).post(create_rule))
        .route("/api/v1/rules/{id}", put(update_rule).delete(delete_rule))
        .route("/api/v1/dashboard/summary", get(dashboard_summary))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
pub struct CreateDeviceRequest {
    pub name: String,
    pub node_id: String,
    pub device_type: String,
}

async fn create_device(
    State(state): State<AppState>,
    Json(req): Json<CreateDeviceRequest>,
) -> impl IntoResponse {
    let device_type = match req.device_type.as_str() {
        "sensor" => "sensor",
        "actuator" => "actuator",
        _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid device type"}))).into_response(),
    };
    let now = Utc::now();
    let id = Uuid::new_v4();
    let result = sqlx::query(
        "INSERT INTO devices (id, name, node_id, device_type, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string()).bind(&req.name).bind(&req.node_id).bind(device_type)
    .bind("offline").bind(now.timestamp()).bind(now.timestamp())
    .execute(&state.pool).await;
    match result {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({"id": id.to_string(), "message": "Device created"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn list_devices(State(state): State<AppState>) -> impl IntoResponse {
    let devices = sqlx::query_as::<_, (String, String, String, String, String, Option<String>, i64, i64)>(
        "SELECT id, name, node_id, device_type, status, config, created_at, updated_at FROM devices",
    ).fetch_all(&state.pool).await;
    match devices {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.into_iter().map(|r| {
                serde_json::json!({"id": r.0, "name": r.1, "node_id": r.2, "device_type": r.3, "status": r.4, "config": r.5, "created_at": r.6, "updated_at": r.7})
            }).collect();
            Json(result).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn get_device(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let device = sqlx::query_as::<_, (String, String, String, String, String, Option<String>, i64, i64)>(
        "SELECT id, name, node_id, device_type, status, config, created_at, updated_at FROM devices WHERE id = ?",
    ).bind(&id).fetch_one(&state.pool).await;
    match device {
        Ok(r) => Json(serde_json::json!({"id": r.0, "name": r.1, "node_id": r.2, "device_type": r.3, "status": r.4, "config": r.5, "created_at": r.6, "updated_at": r.7})).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Device not found"}))).into_response(),
    }
}

async fn update_device(
    State(state): State<AppState>, Path(id): Path<String>,
    Json(req): Json<CreateDeviceRequest>,
) -> impl IntoResponse {
    let now = Utc::now().timestamp();
    let result = sqlx::query("UPDATE devices SET name = ?, node_id = ?, device_type = ?, updated_at = ? WHERE id = ?")
        .bind(&req.name).bind(&req.node_id).bind(&req.device_type).bind(now).bind(&id)
        .execute(&state.pool).await;
    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({"message": "Device updated"})).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Device not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn delete_device(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM devices WHERE id = ?").bind(&id).execute(&state.pool).await;
    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({"message": "Device deleted"})).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Device not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ReadingsQuery {
    pub metric: Option<String>,
    pub start: Option<i64>,
    pub end: Option<i64>,
    pub limit: Option<i64>,
}

async fn list_readings(
    State(state): State<AppState>, Path(id): Path<String>,
    Query(query): Query<ReadingsQuery>,
) -> impl IntoResponse {
    let mut sql = String::from("SELECT id, device_id, metric, value, unit, timestamp FROM sensor_readings WHERE device_id = ?");
    if let Some(ref metric) = query.metric { sql.push_str(&format!(" AND metric = '{}'", metric)); }
    if let Some(start) = query.start { sql.push_str(&format!(" AND timestamp >= {}", start)); }
    if let Some(end) = query.end { sql.push_str(&format!(" AND timestamp <= {}", end)); }
    sql.push_str(" ORDER BY timestamp DESC");
    if let Some(limit) = query.limit { sql.push_str(&format!(" LIMIT {}", limit)); }
    let readings = sqlx::query_as::<_, (i64, String, String, f64, String, i64)>(&sql)
        .bind(&id).fetch_all(&state.pool).await;
    match readings {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.into_iter().map(|r| {
                serde_json::json!({"id": r.0, "device_id": r.1, "metric": r.2, "value": r.3, "unit": r.4, "timestamp": r.5})
            }).collect();
            Json(result).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn send_command(
    State(state): State<AppState>, Path(id): Path<String>,
    Json(cmd): Json<CommandPayload>,
) -> impl IntoResponse {
    let device: Option<(String, String)> = sqlx::query_as(
        "SELECT device_type, status FROM devices WHERE id = ?"
    ).bind(&id).fetch_optional(&state.pool).await.ok().flatten();
    let (device_type, status) = match device {
        Some(d) => d,
        None => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Device not found"}))).into_response(),
    };
    if device_type != "actuator" {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Cannot send command to sensor"}))).into_response();
    }
    if status != "online" {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Device is offline"}))).into_response();
    }
    let now = Utc::now().timestamp();
    let payload_str = serde_json::to_string(&cmd.params).ok();
    let result = sqlx::query(
        "INSERT INTO command_log (device_id, command, payload, status, created_at) VALUES (?, ?, ?, ?, ?)",
    ).bind(&id).bind(&cmd.command).bind(payload_str).bind("pending").bind(now).execute(&state.pool).await;
    match result {
        Ok(r) => Json(serde_json::json!({"id": r.last_insert_rowid(), "message": "Command queued"})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateRuleRequest {
    pub name: String,
    pub trigger_type: String,
    pub conditions: serde_json::Value,
    pub actions: serde_json::Value,
    pub schedule: Option<String>,
}

async fn create_rule(
    State(state): State<AppState>,
    Json(req): Json<CreateRuleRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4();
    let now = Utc::now().timestamp();
    let conditions_str = req.conditions.to_string();
    let actions_str = req.actions.to_string();
    let result = sqlx::query(
        "INSERT INTO rules (id, name, enabled, trigger_type, conditions, actions, schedule, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(&req.name)
    .bind(1i64)
    .bind(&req.trigger_type)
    .bind(&conditions_str)
    .bind(&actions_str)
    .bind(&req.schedule)
    .bind(now)
    .execute(&state.pool)
    .await;
    match result {
        Ok(_) => Json(serde_json::json!({"id": id.to_string(), "message": "Rule created"})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn list_rules(State(state): State<AppState>) -> impl IntoResponse {
    let rules = sqlx::query_as::<_, (String, String, i64, String, String, String, Option<String>, i64)>(
        "SELECT id, name, enabled, trigger_type, conditions, actions, schedule, created_at FROM rules",
    ).fetch_all(&state.pool).await;
    match rules {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.into_iter().map(|r| {
                serde_json::json!({
                    "id": r.0, "name": r.1, "enabled": r.2 == 1, "trigger_type": r.3,
                    "conditions": serde_json::from_str::<serde_json::Value>(&r.4).ok(),
                    "actions": serde_json::from_str::<serde_json::Value>(&r.5).ok(),
                    "schedule": r.6, "created_at": r.7
                })
            }).collect();
            Json(result).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn update_rule(
    State(state): State<AppState>, Path(id): Path<String>,
    Json(req): Json<CreateRuleRequest>,
) -> impl IntoResponse {
    let conditions_str = req.conditions.to_string();
    let actions_str = req.actions.to_string();
    let result = sqlx::query(
        "UPDATE rules SET name = ?, trigger_type = ?, conditions = ?, actions = ?, schedule = ? WHERE id = ?",
    )
    .bind(&req.name).bind(&req.trigger_type).bind(&conditions_str).bind(&actions_str).bind(&req.schedule).bind(&id)
    .execute(&state.pool).await;
    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({"message": "Rule updated"})).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Rule not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn delete_rule(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM rules WHERE id = ?").bind(&id).execute(&state.pool).await;
    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({"message": "Rule deleted"})).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Rule not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn dashboard_summary(State(state): State<AppState>) -> impl IntoResponse {
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM devices").fetch_one(&state.pool).await.unwrap_or((0,));
    let online: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM devices WHERE status = 'online'").fetch_one(&state.pool).await.unwrap_or((0,));
    let active_rules: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rules WHERE enabled = 1").fetch_one(&state.pool).await.unwrap_or((0,));
    Json(serde_json::json!({
        "total_devices": total.0, "online_devices": online.0, "active_rules": active_rules.0,
    })).into_response()
}
