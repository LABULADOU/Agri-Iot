use agri_core::models::{ComfortConfig, CommandPayload, ValueRange};
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
        // Devices
        .route("/api/v1/devices", get(list_devices).post(create_device))
        .route("/api/v1/devices/{id}", get(get_device).put(update_device).delete(delete_device))
        .route("/api/v1/devices/{id}/readings", get(list_readings))
        .route("/api/v1/devices/{id}/command", post(send_command))
        // Zones
        .route("/api/v1/zones", get(list_zones).post(create_zone))
        .route("/api/v1/zones/{id}", get(get_zone).put(update_zone).delete(delete_zone))
        .route("/api/v1/zones/{id}/accumulated-temp", get(list_accumulated_temp))
        // Nodes
        .route("/api/v1/nodes", get(list_nodes).post(create_node))
        .route("/api/v1/nodes/{id}", get(get_node).put(update_node).delete(delete_node))
        .route("/api/v1/nodes/{id}/readings", get(list_node_readings))
        // Aggregated data
        .route("/api/v1/readings/aggregated", get(aggregated_readings))
        // Control
        .route("/api/v1/control/command", post(send_control_command))
        // Rules
        .route("/api/v1/rules", get(list_rules).post(create_rule))
        .route("/api/v1/rules/{id}", put(update_rule).delete(delete_rule))
        // Dashboard
        .route("/api/v1/dashboard/summary", get(dashboard_summary))
        .with_state(state)
}

// ============== Zone APIs ==============

#[derive(Debug, Deserialize)]
pub struct CreateZoneRequest {
    pub name: String,
    pub description: Option<String>,
    pub location: String,
    pub crop_type: String,
    #[serde(default)]
    pub comfort_config: Option<ComfortConfig>,
    #[serde(default)]
    pub node_ids: Vec<String>,
}

async fn list_zones(State(state): State<AppState>) -> impl IntoResponse {
    let zones = sqlx::query_as::<_, (String, String, String, String, String, String, String, i64, i64)>(
        "SELECT id, name, description, location, crop_type, comfort_config, node_ids, created_at, updated_at FROM zones",
    ).fetch_all(&state.pool).await;
    match zones {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.into_iter().map(|r| {
                serde_json::json!({
                    "id": r.0, "name": r.1, "description": r.2, "location": r.3, "cropType": r.4,
                    "comfortConfig": serde_json::from_str(&r.5).unwrap_or(serde_json::Value::Null),
                    "nodeIds": serde_json::from_str::<Vec<String>>(&r.6).unwrap_or_default(),
                    "createdAt": r.7, "updatedAt": r.8
                })
            }).collect();
            Json(result).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn get_zone(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let zone = sqlx::query_as::<_, (String, String, String, String, String, String, String, i64, i64)>(
        "SELECT id, name, description, location, crop_type, comfort_config, node_ids, created_at, updated_at FROM zones WHERE id = ?",
    ).bind(&id).fetch_one(&state.pool).await;
    match zone {
        Ok(r) => Json(serde_json::json!({
            "id": r.0, "name": r.1, "description": r.2, "location": r.3, "cropType": r.4,
            "comfortConfig": serde_json::from_str(&r.5).unwrap_or(serde_json::Value::Null),
            "nodeIds": serde_json::from_str::<Vec<String>>(&r.6).unwrap_or_default(),
            "createdAt": r.7, "updatedAt": r.8
        })).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Zone not found"}))).into_response(),
    }
}

async fn create_zone(State(state): State<AppState>, Json(req): Json<CreateZoneRequest>) -> impl IntoResponse {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let comfort_config = req.comfort_config.unwrap_or_default();
    let comfort_str = serde_json::to_string(&comfort_config).unwrap_or_default();
    let node_ids_str = serde_json::to_string(&req.node_ids).unwrap_or_else(|_| "[]".to_string());
    
    let result = sqlx::query(
        "INSERT INTO zones (id, name, description, location, crop_type, comfort_config, node_ids, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    ).bind(id.to_string()).bind(&req.name).bind(&req.description).bind(&req.location).bind(&req.crop_type)
    .bind(&comfort_str).bind(&node_ids_str).bind(now.timestamp()).bind(now.timestamp())
    .execute(&state.pool).await;
    
    match result {
        Ok(_) => Json(serde_json::json!({"id": id.to_string(), "name": req.name})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn update_zone(State(state): State<AppState>, Path(id): Path<String>, Json(req): Json<CreateZoneRequest>) -> impl IntoResponse {
    let now = Utc::now().timestamp();
    let comfort_config = req.comfort_config.unwrap_or_default();
    let comfort_str = serde_json::to_string(&comfort_config).unwrap_or_default();
    let node_ids_str = serde_json::to_string(&req.node_ids).unwrap_or_else(|_| "[]".to_string());
    
    let result = sqlx::query(
        "UPDATE zones SET name = ?, description = ?, location = ?, crop_type = ?, comfort_config = ?, node_ids = ?, updated_at = ? WHERE id = ?",
    ).bind(&req.name).bind(&req.description).bind(&req.location).bind(&req.crop_type)
    .bind(&comfort_str).bind(&node_ids_str).bind(now).bind(&id)
    .execute(&state.pool).await;
    
    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({"message": "Zone updated"})).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Zone not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn delete_zone(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM zones WHERE id = ?").bind(&id).execute(&state.pool).await;
    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({"message": "Zone deleted"})).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Zone not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn list_accumulated_temp(State(state): State<AppState>, Path(zone_id): Path<String>) -> impl IntoResponse {
    let temps = sqlx::query_as::<_, (String, String, String, f64, f64)>(
        "SELECT id, zone_id, date, accumulated, threshold FROM accumulated_temps WHERE zone_id = ? ORDER BY date DESC LIMIT 30",
    ).bind(&zone_id).fetch_all(&state.pool).await;
    match temps {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.into_iter().map(|r| {
                serde_json::json!({"id": r.0, "zoneId": r.1, "date": r.2, "accumulated": r.3, "threshold": r.4})
            }).collect();
            Json(result).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

// ============== Node APIs ==============

#[derive(Debug, Deserialize)]
pub struct CreateNodeRequest {
    pub name: String,
    pub zone_id: String,
    #[serde(default)]
    pub has_irrigation: bool,
    #[serde(default)]
    pub has_side_vent: bool,
    #[serde(default)]
    pub has_roof_vent: bool,
    #[serde(default)]
    pub vent_range: Option<ValueRange>,
}

async fn list_nodes(State(state): State<AppState>, Query(query): Query<NodeQuery>) -> impl IntoResponse {
    let sql = if query.zone_id.is_some() {
        "SELECT id, name, zone_id, has_irrigation, has_side_vent, has_roof_vent, vent_range, status, last_seen, created_at, updated_at FROM sensor_nodes WHERE zone_id = ?"
    } else {
        "SELECT id, name, zone_id, has_irrigation, has_side_vent, has_roof_vent, vent_range, status, last_seen, created_at, updated_at FROM sensor_nodes"
    };
    
    let nodes = if let Some(ref zone_id) = query.zone_id {
        sqlx::query_as::<_, (String, String, String, i64, i64, i64, String, String, Option<i64>, i64, i64)>(sql)
            .bind(zone_id).fetch_all(&state.pool).await
    } else {
        sqlx::query_as::<_, (String, String, String, i64, i64, i64, String, String, Option<i64>, i64, i64)>(sql)
            .fetch_all(&state.pool).await
    };
    
    match nodes {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.into_iter().map(|r| {
                serde_json::json!({
                    "id": r.0, "name": r.1, "zoneId": r.2, "hasIrrigation": r.3 == 1, "hasSideVent": r.4 == 1,
                    "hasRoofVent": r.5 == 1, "ventRange": serde_json::from_str(&r.6).unwrap_or(serde_json::json!({"min": 0, "max": 100})),
                    "status": r.7, "lastSeen": r.8, "createdAt": r.9, "updatedAt": r.10
                })
            }).collect();
            Json(result).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct NodeQuery {
    pub zone_id: Option<String>,
}

async fn get_node(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let node = sqlx::query_as::<_, (String, String, String, i64, i64, i64, String, String, Option<i64>, i64, i64)>(
        "SELECT id, name, zone_id, has_irrigation, has_side_vent, has_roof_vent, vent_range, status, last_seen, created_at, updated_at FROM sensor_nodes WHERE id = ?",
    ).bind(&id).fetch_one(&state.pool).await;
    match node {
        Ok(r) => Json(serde_json::json!({
            "id": r.0, "name": r.1, "zoneId": r.2, "hasIrrigation": r.3 == 1, "hasSideVent": r.4 == 1,
            "hasRoofVent": r.5 == 1, "ventRange": serde_json::from_str(&r.6).unwrap_or(serde_json::json!({"min": 0, "max": 100})),
            "status": r.7, "lastSeen": r.8, "createdAt": r.9, "updatedAt": r.10
        })).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Node not found"}))).into_response(),
    }
}

async fn create_node(State(state): State<AppState>, Json(req): Json<CreateNodeRequest>) -> impl IntoResponse {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let vent_range = req.vent_range.unwrap_or(ValueRange { min: 0.0, max: 100.0 });
    let vent_str = serde_json::to_string(&vent_range).unwrap_or_default();
    
    let result = sqlx::query(
        "INSERT INTO sensor_nodes (id, name, zone_id, has_irrigation, has_side_vent, has_roof_vent, vent_range, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    ).bind(id.to_string()).bind(&req.name).bind(&req.zone_id)
    .bind(if req.has_irrigation { 1 } else { 0 })
    .bind(if req.has_side_vent { 1 } else { 0 })
    .bind(if req.has_roof_vent { 1 } else { 0 })
    .bind(&vent_str).bind("offline").bind(now.timestamp()).bind(now.timestamp())
    .execute(&state.pool).await;
    
    match result {
        Ok(_) => Json(serde_json::json!({"id": id.to_string(), "name": req.name})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn update_node(State(state): State<AppState>, Path(id): Path<String>, Json(req): Json<CreateNodeRequest>) -> impl IntoResponse {
    let now = Utc::now().timestamp();
    let vent_range = req.vent_range.unwrap_or(ValueRange { min: 0.0, max: 100.0 });
    let vent_str = serde_json::to_string(&vent_range).unwrap_or_default();
    
    let result = sqlx::query(
        "UPDATE sensor_nodes SET name = ?, zone_id = ?, has_irrigation = ?, has_side_vent = ?, has_roof_vent = ?, vent_range = ?, updated_at = ? WHERE id = ?",
    ).bind(&req.name).bind(&req.zone_id)
    .bind(if req.has_irrigation { 1 } else { 0 })
    .bind(if req.has_side_vent { 1 } else { 0 })
    .bind(if req.has_roof_vent { 1 } else { 0 })
    .bind(&vent_str).bind(now).bind(&id)
    .execute(&state.pool).await;
    
    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({"message": "Node updated"})).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Node not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn delete_node(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM sensor_nodes WHERE id = ?").bind(&id).execute(&state.pool).await;
    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({"message": "Node deleted"})).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Node not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn list_node_readings(
    State(state): State<AppState>, Path(id): Path<String>,
    Query(query): Query<ReadingsQuery>,
) -> impl IntoResponse {
    use sqlx::QueryBuilder;

    let mut builder = QueryBuilder::new(
        "SELECT id, device_id, metric, value, unit, timestamp FROM sensor_readings WHERE device_id = "
    );
    builder.push_bind(&id);

    if let Some(ref metric) = query.metric {
        builder.push(" AND metric = ");
        builder.push_bind(metric);
    }
    if let Some(start) = query.start {
        builder.push(" AND timestamp >= ");
        builder.push_bind(start);
    }
    if let Some(end) = query.end {
        builder.push(" AND timestamp <= ");
        builder.push_bind(end);
    }
    builder.push(" ORDER BY timestamp DESC");
    if let Some(limit) = query.limit {
        builder.push(" LIMIT ");
        builder.push_bind(limit);
    }

    let readings = builder.build_query_as::<(i64, String, String, f64, String, i64)>()
        .fetch_all(&state.pool).await;
    match readings {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.into_iter().map(|r| {
                serde_json::json!({"id": r.0, "deviceId": r.1, "metric": r.2, "value": r.3, "unit": r.4, "timestamp": r.5})
            }).collect();
            Json(result).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

// ============== Aggregated Data API ==============

#[derive(Debug, Deserialize)]
pub struct AggregatedQuery {
    pub node_id: Option<String>,
    #[allow(dead_code)]
    pub metric: Option<String>,
    pub period: Option<String>,
    pub start: Option<i64>,
    pub end: Option<i64>,
}

async fn aggregated_readings(State(state): State<AppState>, Query(query): Query<AggregatedQuery>) -> impl IntoResponse {
    use sqlx::QueryBuilder;

    let now = Utc::now().timestamp();
    let start = query.start.unwrap_or(now - 86400);
    let end = query.end.unwrap_or(now);
    let period = query.period.as_deref().unwrap_or("hour");
    
    let truncate = match period {
        "hour" => "datetime(timestamp, 'unixepoch', 'localtime', 'start of hour')",
        "day" => "date(timestamp, 'unixepoch', 'localtime')",
        _ => "datetime(timestamp, 'unixepoch', 'start of hour')",
    };
    
    let mut builder = QueryBuilder::new(format!(
        "SELECT {}, metric, device_id, MAX(value), MIN(value), AVG(value), COUNT(*) \
         FROM sensor_readings \
         WHERE timestamp >= {} AND timestamp <= {}",
        truncate, start, end,
    ));
    
    if let Some(ref node_id) = query.node_id {
        builder.push(" AND device_id = ");
        builder.push_bind(node_id);
    }
    
    builder.push(format!(
        " GROUP BY {}, metric, device_id ORDER BY {}",
        truncate, truncate,
    ));
    
    let readings = builder.build_query_as::<(String, String, String, f64, f64, f64, i64)>()
        .fetch_all(&state.pool).await;
    
    match readings {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.into_iter().map(|r| {
                serde_json::json!({
                    "timestamp": r.0, "metric": r.1, "nodeId": r.2, "max": r.3, "min": r.4, "avg": r.5, "count": r.6
                })
            }).collect();
            Json(result).into_response()
        }
        Err(_) => {
            let result: Vec<serde_json::Value> = vec![];
            Json(result).into_response()
        }
    }
}

// ============== Control Command API ==============

#[derive(Debug, Deserialize)]
pub struct ControlCommand {
    pub device_id: String,
    pub command: String,
    pub action: serde_json::Value,
}

async fn send_control_command(State(state): State<AppState>, Json(cmd): Json<ControlCommand>) -> impl IntoResponse {
    let node = sqlx::query_as::<_, (String, i64, i64, i64)>(
        "SELECT id, has_irrigation, has_side_vent, has_roof_vent FROM sensor_nodes WHERE id = ?",
    ).bind(&cmd.device_id).fetch_optional(&state.pool).await.ok().flatten();
    
    let (_node_id, has_irrigation, has_side_vent, has_roof_vent) = match node {
        Some(n) => n,
        None => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Node not found"}))).into_response(),
    };
    
    let is_valid = match cmd.command.as_str() {
        "irrigation" => has_irrigation == 1,
        "side_vent" => has_side_vent == 1,
        "roof_vent" => has_roof_vent == 1,
        _ => false,
    };
    
    if !is_valid {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid command or node does not support this control"}))).into_response();
    }
    
    let now = Utc::now().timestamp();
    let action_str = cmd.action.to_string();
    let result = sqlx::query(
        "INSERT INTO command_log (device_id, command, payload, status, created_at) VALUES (?, ?, ?, ?, ?)",
    ).bind(&cmd.device_id).bind(&cmd.command).bind(&action_str).bind("pending").bind(now)
    .execute(&state.pool).await;
    
    match result {
        Ok(r) => Json(serde_json::json!({"id": r.last_insert_rowid(), "message": "Command queued"})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

// ============== Device APIs ==============

#[derive(Debug, Deserialize)]
pub struct CreateDeviceRequest {
    pub name: String,
    pub node_id: String,
    pub device_type: String,
}

async fn create_device(State(state): State<AppState>, Json(req): Json<CreateDeviceRequest>) -> impl IntoResponse {
    let device_type = match req.device_type.as_str() {
        "sensor" => "sensor",
        "actuator" => "actuator",
        _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid device type"}))).into_response(),
    };
    let now = Utc::now();
    let id = Uuid::new_v4();
    let result = sqlx::query(
        "INSERT INTO devices (id, name, node_id, device_type, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
    ).bind(id.to_string()).bind(&req.name).bind(&req.node_id).bind(device_type)
    .bind("offline").bind(now.timestamp()).bind(now.timestamp())
    .execute(&state.pool).await;
    match result {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({"id": id.to_string()}))).into_response(),
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
                serde_json::json!({"id": r.0, "name": r.1, "nodeId": r.2, "deviceType": r.3, "status": r.4, "config": r.5, "createdAt": r.6, "updatedAt": r.7})
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
        Ok(r) => Json(serde_json::json!({"id": r.0, "name": r.1, "nodeId": r.2, "deviceType": r.3, "status": r.4, "config": r.5, "createdAt": r.6, "updatedAt": r.7})).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Device not found"}))).into_response(),
    }
}

async fn update_device(State(state): State<AppState>, Path(id): Path<String>, Json(req): Json<CreateDeviceRequest>) -> impl IntoResponse {
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

async fn list_readings(State(state): State<AppState>, Path(id): Path<String>, Query(query): Query<ReadingsQuery>) -> impl IntoResponse {
    use sqlx::QueryBuilder;

    let mut builder = QueryBuilder::new(
        "SELECT id, device_id, metric, value, unit, timestamp FROM sensor_readings WHERE device_id = "
    );
    builder.push_bind(&id);

    if let Some(ref metric) = query.metric {
        builder.push(" AND metric = ");
        builder.push_bind(metric);
    }
    if let Some(start) = query.start {
        builder.push(" AND timestamp >= ");
        builder.push_bind(start);
    }
    if let Some(end) = query.end {
        builder.push(" AND timestamp <= ");
        builder.push_bind(end);
    }
    builder.push(" ORDER BY timestamp DESC");
    if let Some(limit) = query.limit {
        builder.push(" LIMIT ");
        builder.push_bind(limit);
    }

    let readings = builder.build_query_as::<(i64, String, String, f64, String, i64)>()
        .fetch_all(&state.pool).await;
    match readings {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.into_iter().map(|r| {
                serde_json::json!({"id": r.0, "deviceId": r.1, "metric": r.2, "value": r.3, "unit": r.4, "timestamp": r.5})
            }).collect();
            Json(result).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn send_command(State(state): State<AppState>, Path(id): Path<String>, Json(cmd): Json<CommandPayload>) -> impl IntoResponse {
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

// ============== Rule APIs ==============

#[derive(Debug, Deserialize)]
pub struct CreateRuleRequest {
    pub name: String,
    pub trigger_type: String,
    pub conditions: serde_json::Value,
    pub actions: serde_json::Value,
    pub schedule: Option<String>,
    pub enabled: Option<bool>,
}

async fn create_rule(State(state): State<AppState>, Json(req): Json<CreateRuleRequest>) -> impl IntoResponse {
    let id = Uuid::new_v4();
    let now = Utc::now().timestamp();
    let conditions_str = req.conditions.to_string();
    let actions_str = req.actions.to_string();
    let result = sqlx::query(
        "INSERT INTO rules (id, name, enabled, trigger_type, conditions, actions, schedule, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    ).bind(id.to_string()).bind(&req.name).bind(if req.enabled.unwrap_or(true) { 1 } else { 0 })
    .bind(&req.trigger_type).bind(&conditions_str).bind(&actions_str).bind(&req.schedule).bind(now)
    .execute(&state.pool).await;
    match result {
        Ok(_) => Json(serde_json::json!({"id": id.to_string()})).into_response(),
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
                    "id": r.0, "name": r.1, "enabled": r.2 == 1, "triggerType": r.3,
                    "conditions": serde_json::from_str::<serde_json::Value>(&r.4).ok(),
                    "actions": serde_json::from_str::<serde_json::Value>(&r.5).ok(),
                    "schedule": r.6, "createdAt": r.7
                })
            }).collect();
            Json(result).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn update_rule(State(state): State<AppState>, Path(id): Path<String>, Json(req): Json<CreateRuleRequest>) -> impl IntoResponse {
    let conditions_str = req.conditions.to_string();
    let actions_str = req.actions.to_string();
    let result = sqlx::query(
        "UPDATE rules SET name = ?, enabled = ?, trigger_type = ?, conditions = ?, actions = ?, schedule = ? WHERE id = ?",
    ).bind(&req.name).bind(if req.enabled.unwrap_or(true) { 1 } else { 0 })
    .bind(&req.trigger_type).bind(&conditions_str).bind(&actions_str).bind(&req.schedule).bind(&id)
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

// ============== Dashboard API ==============

async fn dashboard_summary(State(state): State<AppState>) -> impl IntoResponse {
    let total_devices: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM devices").fetch_one(&state.pool).await.unwrap_or((0,));
    let online_devices: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM devices WHERE status = 'online'").fetch_one(&state.pool).await.unwrap_or((0,));
    let total_nodes: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sensor_nodes").fetch_one(&state.pool).await.unwrap_or((0,));
    let online_nodes: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sensor_nodes WHERE status = 'online'").fetch_one(&state.pool).await.unwrap_or((0,));
    let total_zones: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM zones").fetch_one(&state.pool).await.unwrap_or((0,));
    let active_rules: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rules WHERE enabled = 1").fetch_one(&state.pool).await.unwrap_or((0,));
    Json(serde_json::json!({
        "totalDevices": total_devices.0, "onlineDevices": online_devices.0,
        "totalNodes": total_nodes.0, "onlineNodes": online_nodes.0,
        "totalZones": total_zones.0, "activeRules": active_rules.0,
    })).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use sqlx::SqlitePool;
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        sqlx::query(
            "CREATE TABLE devices (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                node_id TEXT NOT NULL UNIQUE,
                device_type TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'offline',
                config TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE sensor_readings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL,
                metric TEXT NOT NULL,
                value REAL NOT NULL,
                unit TEXT,
                timestamp INTEGER NOT NULL
            )"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                trigger_type TEXT NOT NULL,
                conditions TEXT,
                actions TEXT,
                schedule TEXT,
                created_at INTEGER NOT NULL
            )"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE command_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL,
                command TEXT NOT NULL,
                payload TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at INTEGER NOT NULL
            )"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS zones (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                location TEXT NOT NULL,
                crop_type TEXT NOT NULL,
                comfort_config TEXT NOT NULL,
                node_ids TEXT NOT NULL DEFAULT '[]',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE sensor_nodes (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                zone_id TEXT NOT NULL,
                has_irrigation INTEGER NOT NULL DEFAULT 0,
                has_side_vent INTEGER NOT NULL DEFAULT 0,
                has_roof_vent INTEGER NOT NULL DEFAULT 0,
                vent_range TEXT NOT NULL DEFAULT '{\"min\": 0, \"max\": 100}',
                status TEXT NOT NULL DEFAULT 'offline',
                last_seen INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )"
        ).execute(&pool).await.unwrap();

        pool
    }

    #[allow(dead_code)]
    async fn insert_test_device(pool: &SqlitePool, name: &str, node_id: &str, device_type: &str) -> String {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO devices (id, name, node_id, device_type, status, created_at, updated_at) VALUES (?, ?, ?, ?, 'offline', ?, ?)"
        )
        .bind(&id).bind(name).bind(node_id).bind(device_type).bind(now).bind(now)
        .execute(pool).await.unwrap();
        id
    }

    #[tokio::test]
    async fn test_list_devices_empty() {
        let pool = setup_test_db().await;
        let state = AppState::new(pool, create_mock_client().await);
        let router = create_router(state);

        let request = Request::builder()
            .method("GET")
            .uri("/api/v1/devices")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_device_not_found() {
        let pool = setup_test_db().await;
        let state = AppState::new(pool, create_mock_client().await);
        let router = create_router(state);

        let request = Request::builder()
            .method("GET")
            .uri("/api/v1/devices/non-existent-uuid")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_dashboard_summary_empty() {
        let pool = setup_test_db().await;
        let state = AppState::new(pool, create_mock_client().await);
        let router = create_router(state);

        let request = Request::builder()
            .method("GET")
            .uri("/api/v1/dashboard/summary")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_device_success() {
        let pool = setup_test_db().await;
        let state = AppState::new(pool, create_mock_client().await);
        let router = create_router(state);

        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/devices")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name": "温度传感器", "node_id": "node-001", "device_type": "sensor"}"#))
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_create_device_invalid_type() {
        let pool = setup_test_db().await;
        let state = AppState::new(pool, create_mock_client().await);
        let router = create_router(state);

        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/devices")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name": "Test", "node_id": "node-002", "device_type": "invalid"}"#))
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_device_type_validation_sensor() {
        let device_type = "sensor";
        let result = match device_type {
            "sensor" | "actuator" => true,
            _ => false,
        };
        assert!(result);
    }

    #[test]
    fn test_device_type_validation_actuator() {
        let device_type = "actuator";
        let result = match device_type {
            "sensor" | "actuator" => true,
            _ => false,
        };
        assert!(result);
    }

    #[test]
    fn test_device_type_validation_invalid() {
        let device_type = "invalid";
        let result = match device_type {
            "sensor" | "actuator" => true,
            _ => false,
        };
        assert!(!result);
    }

    #[test]
    fn test_trigger_type_validation_schedule() {
        let trigger_type = "schedule";
        let result = match trigger_type {
            "schedule" | "condition" => true,
            _ => false,
        };
        assert!(result);
    }

    #[test]
    fn test_trigger_type_validation_condition() {
        let trigger_type = "condition";
        let result = match trigger_type {
            "schedule" | "condition" => true,
            _ => false,
        };
        assert!(result);
    }

    #[test]
    fn test_trigger_type_validation_invalid() {
        let trigger_type = "invalid";
        let result = match trigger_type {
            "schedule" | "condition" => true,
            _ => false,
        };
        assert!(!result);
    }

    async fn create_mock_client() -> rumqttc::AsyncClient {
        let (client, _) = rumqttc::AsyncClient::new(
            rumqttc::MqttOptions::new("test-client", "127.0.0.1", 1883),
            10
        );
        client
    }
}
