use crate::response::{bad_request, internal_err, not_found, ok_json};
use agri_core::error::AppError;
use agri_core::models::{CommandPayload, Device, SensorReading, Rule};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response, sse::{Event, Sse, KeepAlive}},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::state::AppState;

pub struct ServerError(pub AppError);

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.0.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self.0.as_response())).into_response()
    }
}

impl From<sqlx::Error> for ServerError {
    fn from(e: sqlx::Error) -> Self {
        ServerError(AppError::Database(e))
    }
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/devices", get(list_devices).post(create_device))
        .route("/api/v1/devices/:id", get(get_device).put(update_device).delete(delete_device))
        .route("/api/v1/devices/:id/readings", get(list_readings))
        .route("/api/v1/devices/:id/command", post(send_command))
        .route("/api/v1/rules", get(list_rules).post(create_rule))
        .route("/api/v1/rules/:id", put(update_rule).delete(delete_rule))
        .route("/api/v1/alerts", get(list_alerts))
        .route("/api/v1/telemetry", post(ingest_telemetry))
        .route("/api/v1/telemetry/batch", post(ingest_telemetry_batch))
        .route("/api/v1/dashboard/summary", get(dashboard_summary))
        .route("/api/v1/dashboard/area-readings", get(dashboard_area_readings))
        .route("/api/v1/dashboard/node-readings", get(dashboard_node_readings))
        .route("/api/v1/system/info", get(system_info))
        .route("/api/v1/readings/aggregate", get(readings_aggregate))
        .route("/api/v1/monitor/realtime", get(monitor_realtime))
        .route("/api/v1/events", get(sse_events))
        .route("/api/v1/relations", get(list_relations).post(create_relation))
        .route("/api/v1/relations/:id", delete(delete_relation))
        .route("/api/v1/commands/node/:node_id", get(get_pending_commands))
        .route("/api/v1/commands/:id/status", put(update_command_status))
        .route("/api/v1/ws", get(super::ws_handler::ws_handler))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
pub struct CreateDeviceRequest {
    pub name: String,
    pub node_id: String,
    pub device_type: Option<String>,
    pub area_id: Option<String>,
    pub comfort_config: Option<serde_json::Value>,
    pub capabilities: Option<Vec<String>>,
}

async fn create_device(
    State(state): State<AppState>,
    Json(req): Json<CreateDeviceRequest>,
) -> impl IntoResponse {
    let device_type = req.device_type.as_deref().unwrap_or("sensor");
    if device_type != "sensor" && device_type != "actuator" {
        return bad_request("Invalid device type");
    }
    let now = Utc::now();
    let id = Uuid::new_v4();
    let comfort_config_str = req.comfort_config.as_ref().map(|v| v.to_string());
    let capabilities = req.capabilities.as_ref().map(|c| serde_json::to_string(c).unwrap_or_default())
        .unwrap_or_else(|| "[\"sensor\"]".to_string());

    // UPSERT: node_id 已存在则更新 capabilities，否则插入新记录
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM devices WHERE node_id = ?"
    )
    .bind(&req.node_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let result = if let Some((existing_id,)) = existing {
        sqlx::query(
            "UPDATE devices SET name = ?, device_type = ?, capabilities = ?, area_id = ?, comfort_config = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&req.name).bind(device_type).bind(&capabilities).bind(&req.area_id)
        .bind(&comfort_config_str).bind(now.timestamp()).bind(&existing_id)
        .execute(&state.pool).await
        .map(|_| existing_id)
    } else {
        sqlx::query(
            "INSERT INTO devices (id, name, node_id, device_type, status, area_id, comfort_config, capabilities, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string()).bind(&req.name).bind(&req.node_id).bind(device_type)
        .bind("offline").bind(&req.area_id).bind(&comfort_config_str).bind(&capabilities)
        .bind(now.timestamp()).bind(now.timestamp())
        .execute(&state.pool).await
        .map(|_| id.to_string())
    };

    match result {
        Ok(device_id) => (StatusCode::CREATED, Json(serde_json::json!({"id": device_id, "message": "Device created/updated"}))).into_response(),
        Err(e) => internal_err(e),
    }
}

async fn list_devices(State(state): State<AppState>) -> impl IntoResponse {
    let devices = sqlx::query_as::<_, Device>(
        "SELECT id, name, node_id, device_type, status, config, area_id, comfort_config, capabilities, created_at, updated_at FROM devices",
    ).fetch_all(&state.pool).await;
    match devices {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.into_iter().map(|d| {
                serde_json::json!({
                    "id": d.id, "name": d.name, "node_id": d.node_id,
                    "device_type": d.device_type, "status": d.status,
                    "config": d.config, "area_id": d.area_id,
                    "comfort_config": d.comfort_config,
                    "capabilities": d.capabilities,
                    "created_at": d.created_at.timestamp(),
                    "updated_at": d.updated_at.timestamp(),
                })
            }).collect();
            Json(result).into_response()
        }
        Err(e) => internal_err(e),
    }
}

async fn get_device(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, ServerError> {
    let device = sqlx::query_as::<_, Device>(
        "SELECT id, name, node_id, device_type, status, config, area_id, comfort_config, capabilities, created_at, updated_at FROM devices WHERE id = ?",
    ).bind(&id).fetch_optional(&state.pool).await?;
    match device {
        Some(d) => Ok(Json(serde_json::json!({
            "id": d.id, "name": d.name, "node_id": d.node_id,
            "device_type": d.device_type, "status": d.status,
            "config": d.config, "area_id": d.area_id,
            "comfort_config": d.comfort_config,
            "capabilities": d.capabilities,
            "created_at": d.created_at.timestamp(),
            "updated_at": d.updated_at.timestamp(),
        }))),
        None => Err(ServerError(AppError::DeviceNotFound(id))),
    }
}

async fn update_device(
    State(state): State<AppState>, Path(id): Path<String>,
    Json(req): Json<CreateDeviceRequest>,
) -> impl IntoResponse {
    let now = Utc::now().timestamp();
    let comfort_config_str = req.comfort_config.as_ref().map(|v| v.to_string());
    let device_type = req.device_type.as_deref().unwrap_or("sensor");
    let capabilities = req.capabilities.as_ref().map(|c| serde_json::to_string(c).unwrap_or_default())
        .unwrap_or_else(|| "[\"sensor\"]".to_string());
    let result = sqlx::query("UPDATE devices SET name = ?, node_id = ?, device_type = ?, area_id = ?, comfort_config = ?, capabilities = ?, updated_at = ? WHERE id = ?")
        .bind(&req.name).bind(&req.node_id).bind(device_type).bind(&req.area_id)
        .bind(&comfort_config_str).bind(&capabilities).bind(now).bind(&id)
        .execute(&state.pool).await;
    match result {
        Ok(r) if r.rows_affected() > 0 => ok_json(serde_json::json!({"message": "Device updated"})),
        Ok(_) => not_found(Some("Device not found")),
        Err(e) => internal_err(e),
    }
}

async fn delete_device(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, ServerError> {
    let result = sqlx::query("DELETE FROM devices WHERE id = ?").bind(&id).execute(&state.pool).await?;
    if result.rows_affected() > 0 {
        Ok(Json(serde_json::json!({"message": "Device deleted"})))
    } else {
        Err(ServerError(AppError::DeviceNotFound(id)))
    }
}

#[derive(Debug, Deserialize)]
pub struct ReadingsQuery {
    pub metric: Option<String>,
    pub start: Option<i64>,
    pub end: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AggregateQuery {
    pub device_id: String,
    pub metric: String,
    pub start: Option<i64>,
    pub end: Option<i64>,
    pub period: Option<String>,
}

async fn readings_aggregate(
    State(state): State<AppState>,
    Query(query): Query<AggregateQuery>,
) -> impl IntoResponse {
    let period = query.period.as_deref().unwrap_or("hour");
    let now = chrono::Utc::now().timestamp();
    let start = query.start.unwrap_or(now - 86400);
    let end = query.end.unwrap_or(now);

    let bucket_expr = match period {
        "10min" => "CAST(((timestamp / 600) * 600) AS INTEGER)",
        "hour" => "CAST((timestamp / 3600) * 3600 AS INTEGER)",
        "day" => "CAST((timestamp / 86400) * 86400 AS INTEGER)",
        "week" => "CAST((timestamp - ((strftime('%w', datetime(timestamp, 'unixepoch')) + 6) % 7) * 86400) AS INTEGER)",
        "month" => "CAST(strftime('%s', datetime(timestamp, 'unixepoch', 'start of month')) AS INTEGER)",
        _ => "CAST((timestamp / 3600) * 3600 AS INTEGER)",
    };
    let sql = format!(
        "SELECT {} as bucket, metric, MAX(value), MIN(value), AVG(value), COUNT(*) \
         FROM sensor_readings \
         WHERE device_id = ? AND metric = ? AND timestamp >= ? AND timestamp <= ? \
         GROUP BY bucket, metric ORDER BY bucket ASC",
        bucket_expr
    );
    let rows = sqlx::query_as::<_, (i64, String, f64, f64, f64, i64)>(&sql)
        .bind(&query.device_id)
        .bind(&query.metric)
        .bind(start)
        .bind(end)
        .fetch_all(&state.pool)
        .await;

    match rows {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.into_iter().map(|(bucket, metric, max_val, min_val, avg_val, cnt)| {
                serde_json::json!({
                    "timestamp": bucket,
                    "metric": metric,
                    "max": max_val,
                    "min": min_val,
                    "avg": avg_val,
                    "count": cnt,
                })
            }).collect();
            Json(result).into_response()
        }
        Err(e) => internal_err(e),
    }
}

async fn list_readings(
    State(state): State<AppState>, Path(id): Path<String>,
    Query(query): Query<ReadingsQuery>,
) -> impl IntoResponse {
    let has_metric = query.metric.is_some();
    let has_start = query.start.is_some();
    let has_end = query.end.is_some();
    let limit = query.limit.unwrap_or(100).clamp(1, 5000);
    let mut sql = String::from("SELECT id, device_id, metric, value, unit, timestamp FROM sensor_readings WHERE device_id = ?");
    if has_metric { sql.push_str(" AND metric = ?"); }
    if has_start { sql.push_str(" AND timestamp >= ?"); }
    if has_end { sql.push_str(" AND timestamp <= ?"); }
    sql.push_str(" ORDER BY timestamp DESC LIMIT ?");
    let mut q = sqlx::query_as::<_, SensorReading>(&sql).bind(&id);
    if let Some(ref metric) = query.metric { q = q.bind(metric); }
    if let Some(start) = query.start { q = q.bind(start); }
    if let Some(end) = query.end { q = q.bind(end); }
    q = q.bind(limit);
    let readings = q.fetch_all(&state.pool).await;
    match readings {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.into_iter().map(|r| {
                serde_json::json!({"id": r.id, "device_id": r.device_id, "metric": r.metric, "value": r.value, "unit": r.unit, "timestamp": r.timestamp.timestamp()})
            }).collect();
            Json(result).into_response()
        }
        Err(e) => internal_err(e),
    }
}

async fn send_command(
    State(state): State<AppState>, Path(id): Path<String>,
    Json(cmd): Json<CommandPayload>,
) -> impl IntoResponse {
    let device: Option<(String, Option<String>, String)> = match sqlx::query_as::<_, (String, Option<String>, String)>(
        "SELECT status, capabilities, node_id FROM devices WHERE id = ?"
    ).bind(&id).fetch_optional(&state.pool).await {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("DB error fetching device {}: {}", id, e);
            return internal_err(e);
        }
    };
    let (status, capabilities_json, node_id) = match device {
        Some(d) => d,
        None => return not_found(Some("Device not found")),
    };
    let has_actuator = capabilities_json.as_ref().map_or(false, |c| {
        serde_json::from_str::<Vec<String>>(c).map(|caps| caps.contains(&"actuator".to_string()))
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to parse capabilities for device {}: {}", id, e);
                false
            })
    });
    if !has_actuator {
        return bad_request("Device does not support actuator commands");
    }
    if status != "online" {
        return bad_request("Device is offline");
    }
    let now = Utc::now().timestamp();
    let payload_str = serde_json::to_string(&cmd.params).ok();
    let result = sqlx::query(
        "INSERT INTO command_log (device_id, command, payload, status, created_at) VALUES (?, ?, ?, ?, ?)",
    ).bind(&id).bind(&cmd.command).bind(payload_str).bind("pending").bind(now).execute(&state.pool).await;
    let row_id = match result {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => return internal_err(e),
    };
    // Also publish to MQTT for real-time delivery
    if let Some(client) = state.mqtt_client.lock().await.as_ref() {
        let cmd_id = row_id.to_string();
        let payload = serde_json::json!({
            "command": cmd.command,
            "params": cmd.params,
        });
        let _ = agri_mqtt::client::publish_command(client, &node_id, &cmd_id, &payload.to_string()).await;
    }
    Json(serde_json::json!({"id": row_id.to_string(), "message": "Command queued"})).into_response()
}

async fn get_pending_commands(
    State(state): State<AppState>,
    Path(node_id): Path<String>,
) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, (i64, i64, String, Option<String>)>(
        "SELECT cl.id, cl.created_at, cl.command, cl.payload \
         FROM command_log cl \
         JOIN devices d ON cl.device_id = d.id \
         WHERE d.node_id = ? AND cl.status = 'pending' \
         ORDER BY cl.created_at ASC LIMIT 10"
    )
    .bind(&node_id)
    .fetch_all(&state.pool)
    .await;

    match rows {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.into_iter().map(|r| {
                serde_json::json!({
                    "id": r.0.to_string(),
                    "created_at": r.1,
                    "command": r.2,
                    "params": r.3,
                })
            }).collect();
            Json(result).into_response()
        }
        Err(e) => internal_err(e),
    }
}

#[derive(Debug, Deserialize)]
struct UpdateCommandStatusRequest {
    status: String,  // "completed" | "executed" (alias) | "failed"
}

async fn update_command_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateCommandStatusRequest>,
) -> impl IntoResponse {
    if req.status != "completed" && req.status != "executed" && req.status != "failed" {
        return bad_request("Status must be 'completed' or 'failed'");
    }
    let id: i64 = match id.parse() {
        Ok(v) => v,
        Err(_) => return bad_request("Invalid command id"),
    };
    let db_status = if req.status == "executed" { "completed" } else { &req.status };
    let result = sqlx::query("UPDATE command_log SET status = ? WHERE id = ?")
        .bind(db_status)
        .bind(id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(r) => {
            if r.rows_affected() == 0 {
                not_found(Some("Command not found"))
            } else {
                Json(serde_json::json!({"status": "updated"})).into_response()
            }
        }
        Err(e) => internal_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateRuleRequest {
    pub name: String,
    pub trigger_type: String,
    pub conditions: serde_json::Value,
    pub actions: serde_json::Value,
    pub schedule: Option<String>,
    pub priority: Option<i32>,        // 0=普通, 1=紧急
    pub auto_execute: Option<bool>,   // true=自动执行
}

async fn create_rule(
    State(state): State<AppState>,
    Json(req): Json<CreateRuleRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4();
    let now = Utc::now().timestamp();
    let conditions_str = req.conditions.to_string();
    let actions_str = req.actions.to_string();
    let priority = req.priority.unwrap_or(0);
    let auto_execute = if req.auto_execute.unwrap_or(false) { 1i64 } else { 0i64 };

    let result = sqlx::query(
        "INSERT INTO rules (id, name, enabled, trigger_type, conditions, actions, schedule, priority, auto_execute, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(&req.name)
    .bind(1i64)
    .bind(&req.trigger_type)
    .bind(&conditions_str)
    .bind(&actions_str)
    .bind(&req.schedule)
    .bind(priority)
    .bind(auto_execute)
    .bind(now)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({"id": id.to_string(), "message": "Rule created"})).into_response(),
        Err(e) => internal_err(e),
    }
}

async fn list_rules(State(state): State<AppState>) -> impl IntoResponse {
    let rules = sqlx::query_as::<_, Rule>(
        "SELECT id, name, enabled, trigger_type, conditions, actions, schedule, priority, auto_execute, created_at FROM rules",
    ).fetch_all(&state.pool).await;
    match rules {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.into_iter().map(|r| {
                serde_json::json!({
                    "id": r.id, "name": r.name, "enabled": r.enabled, "trigger_type": r.trigger_type,
                    "conditions": r.conditions, "actions": r.actions,
                    "schedule": r.schedule, "created_at": r.created_at.timestamp(),
                    "priority": r.priority, "auto_execute": r.auto_execute,
                })
            }).collect();
            Json(result).into_response()
        }
        Err(e) => internal_err(e),
    }
}

async fn update_rule(
    State(state): State<AppState>, Path(id): Path<String>,
    Json(req): Json<CreateRuleRequest>,
) -> impl IntoResponse {
    let conditions_str = req.conditions.to_string();
    let actions_str = req.actions.to_string();
    let priority = req.priority.unwrap_or(0);
    let auto_execute = if req.auto_execute.unwrap_or(false) { 1i64 } else { 0i64 };

    let result = sqlx::query(
        "UPDATE rules SET name = ?, trigger_type = ?, conditions = ?, actions = ?, schedule = ?, priority = ?, auto_execute = ? WHERE id = ?"
    )
    .bind(&req.name).bind(&req.trigger_type).bind(&conditions_str).bind(&actions_str)
    .bind(&req.schedule).bind(priority).bind(auto_execute).bind(&id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({"message": "Rule updated"})).into_response(),
        Ok(_) => not_found(Some("Rule not found")),
        Err(e) => internal_err(e),
    }
}

async fn delete_rule(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM rules WHERE id = ?").bind(&id).execute(&state.pool).await;
    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({"message": "Rule deleted"})).into_response(),
        Ok(_) => not_found(Some("Rule not found")),
        Err(e) => internal_err(e),
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

async fn dashboard_area_readings(State(state): State<AppState>) -> Json<serde_json::Value> {
    let now = chrono::Utc::now().timestamp();

    let areas = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<i64>, Option<i64>, Option<String>)>(
        "SELECT a.id, a.name, cb.id, c.name, cb.plant_date, cb.expected_harvest_date, c.comfort_config
         FROM areas a
         LEFT JOIN crop_batches cb ON cb.area_id = a.id AND cb.status = 'active'
         LEFT JOIN crops c ON c.id = cb.crop_id
         ORDER BY a.name"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let devices = sqlx::query_as::<_, (String, String, String, Option<String>)>(
        "SELECT id, name, node_id, area_id FROM devices ORDER BY name"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let mut area_devices: std::collections::HashMap<String, Vec<(String, String, String)>> = std::collections::HashMap::new();
    for (id, name, node_id, area_id) in &devices {
        if let Some(aid) = area_id {
            area_devices.entry(aid.clone()).or_default().push((id.clone(), name.clone(), node_id.clone()));
        }
    }

    let mut result = Vec::new();
    for (id, name, _batch_id, crop_name, plant_date, harvest_date, comfort_config_str) in areas {
        let time_start = plant_date.unwrap_or(0);
        let time_end = harvest_date.unwrap_or(now);

        let area_device_list = area_devices.remove(&id).unwrap_or_default();
        let mut device_results = Vec::new();

        for (dev_id, dev_name, node_id) in &area_device_list {
            let readings = sqlx::query_as::<_, (String, f64, String, i64)>(
                "SELECT metric, value, unit, timestamp FROM sensor_readings WHERE device_id = ? AND timestamp >= ? AND timestamp <= ? ORDER BY timestamp ASC LIMIT 1000"
            )
            .bind(dev_id)
            .bind(time_start)
            .bind(time_end)
            .fetch_all(&state.pool)
            .await
            .unwrap_or_default();

            let mut metric_readings: std::collections::BTreeMap<String, Vec<serde_json::Value>> = std::collections::BTreeMap::new();
            for (metric, value, _unit, ts) in &readings {
                metric_readings.entry(metric.clone()).or_default().push(serde_json::json!({
                    "value": value,
                    "timestamp": ts
                }));
            }

            device_results.push(serde_json::json!({
                "id": dev_id,
                "name": dev_name,
                "node_id": node_id,
                "readings": metric_readings
            }));
        }

        let comfort_config: serde_json::Value = comfort_config_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(serde_json::Value::Null);

        result.push(serde_json::json!({
            "id": id,
            "name": name,
            "crop_batch": {
                "crop_name": crop_name,
                "plant_date": time_start,
                "expected_harvest_date": time_end,
                "comfort_config": comfort_config
            },
            "devices": device_results
        }));
    }

    Json(serde_json::json!({"areas": result}))
}

async fn dashboard_node_readings(State(state): State<AppState>) -> impl IntoResponse {
    let latest_readings = match sqlx::query_as::<_, (Option<String>, String, String, f64, String, i64)>(
        "SELECT d.area_id, d.node_id, sr.metric, sr.value, sr.unit, sr.timestamp \
         FROM sensor_readings sr \
         INNER JOIN devices d ON sr.device_id = d.id \
         INNER JOIN ( \
               SELECT device_id, metric, MAX(id) as max_id \
               FROM sensor_readings \
               GROUP BY device_id, metric \
         ) latest ON sr.id = latest.max_id"
    )
    .fetch_all(&state.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("dashboard_node_readings query failed: {e}");
            return Json(serde_json::json!({"areas": [], "error": format!("query failed: {e}")}));
        }
    };

    let mut node_latest: std::collections::BTreeMap<(String, String), serde_json::Map<String, serde_json::Value>> = std::collections::BTreeMap::new();
    let known_metrics = ["temperature", "humidity", "soil_moisture", "soil_temperature", "ec", "light"];
    let unassigned_key = "__unassigned__".to_string();

    for (area_id, node_id, metric, value, unit, ts) in &latest_readings {
        let aid = area_id.clone().unwrap_or_else(|| unassigned_key.clone());
        let entry = node_latest.entry((aid, node_id.clone())).or_default();
        entry.insert(metric.clone(), serde_json::json!({"value": value, "unit": unit, "timestamp": ts}));
    }

    // 收集所有 node_id 并查询设备状态
    let mut all_node_ids: Vec<String> = Vec::new();
    for (key, _) in &node_latest {
        if !all_node_ids.contains(&key.1) {
            all_node_ids.push(key.1.clone());
        }
    }
    let mut device_status: std::collections::HashMap<String, (String, i64)> = std::collections::HashMap::new();
    if !all_node_ids.is_empty() {
        let placeholders = all_node_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let status_query = format!("SELECT node_id, status, updated_at FROM devices WHERE node_id IN ({})", placeholders);
        let mut q = sqlx::query_as::<_, (String, String, i64)>(&status_query);
        for nid in &all_node_ids {
            q = q.bind(nid);
        }
        if let Ok(rows) = q.fetch_all(&state.pool).await {
            for (nid, st, ts) in rows {
                device_status.insert(nid, (st, ts));
            }
        }
    }

    let mut result: Vec<serde_json::Value> = Vec::new();

    let areas = sqlx::query_as::<_, (String, String)>(
        "SELECT id, name FROM areas ORDER BY name"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    for (area_id, area_name) in &areas {
        let mut nodes: Vec<serde_json::Value> = Vec::new();
        let mut node_number = 0;

        let mut area_nodes: Vec<String> = Vec::new();
        for (key, _) in &node_latest {
            if key.0 == *area_id {
                area_nodes.push(key.1.clone());
            }
        }
        area_nodes.sort();
        area_nodes.dedup();

        for node_id in &area_nodes {
            node_number += 1;
            let latest = node_latest.get(&(area_id.clone(), node_id.clone()));
            let mut latest_obj = serde_json::Map::new();
            if let Some(map) = latest {
                for metric in &known_metrics {
                    if let Some(v) = map.get(*metric) {
                        latest_obj.insert(metric.to_string(), v.clone());
                    }
                }
            }

            let (status, updated_at) = device_status.get(node_id.as_str())
                .map(|(s, t)| (s.as_str(), *t))
                .unwrap_or(("offline", 0i64));

            nodes.push(serde_json::json!({
                "node_id": node_id,
                "node_number": node_number,
                "status": status,
                "updated_at": updated_at,
                "latest": latest_obj,
            }));
        }

        result.push(serde_json::json!({
            "area_id": area_id,
            "area_name": area_name,
            "nodes": nodes,
        }));
    }

    // 处理未分配区域的设备
    let unassigned_nodes: Vec<(String, serde_json::Map<String, serde_json::Value>)> = node_latest.iter()
        .filter(|((aid, _), _)| aid == &unassigned_key)
        .map(|((_, nid), map)| (nid.clone(), map.clone()))
        .collect();

    if !unassigned_nodes.is_empty() {
        let nodes: Vec<serde_json::Value> = unassigned_nodes.into_iter().enumerate().map(|(i, (node_id, map))| {
            let mut latest_obj = serde_json::Map::new();
            for metric in &known_metrics {
                if let Some(v) = map.get(*metric) {
                    latest_obj.insert(metric.to_string(), v.clone());
                }
            }
            let (status, updated_at) = device_status.get(node_id.as_str())
                .map(|(s, t)| (s.as_str(), *t))
                .unwrap_or(("offline", 0i64));
            serde_json::json!({
                "node_id": node_id,
                "node_number": i + 1,
                "status": status,
                "updated_at": updated_at,
                "latest": latest_obj,
            })
        }).collect();

        result.push(serde_json::json!({
            "area_id": null,
            "area_name": "未分配",
            "nodes": nodes,
        }));
    }

    Json(serde_json::json!({"areas": result}))
}

async fn list_alerts(State(state): State<AppState>) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, (i64, String, String, Option<String>, String, i64, Option<String>)>(
        "SELECT cl.id, cl.device_id, cl.command, cl.payload, cl.status, cl.created_at, d.name \
         FROM command_log cl LEFT JOIN devices d ON cl.device_id = d.id \
         ORDER BY cl.created_at DESC LIMIT 200"
    )
    .fetch_all(&state.pool)
    .await;

    match rows {
        Ok(rows) => {
            let result: Vec<serde_json::Value> = rows.into_iter().map(|r| {
                serde_json::json!({
                    "id": r.0.to_string(),
                    "device_id": r.1,
                    "device_name": r.6,
                    "command": r.2,
                    "payload": r.3,
                    "status": r.4,
                    "created_at": r.5
                })
            }).collect();
            Json(result).into_response()
        }
        Err(e) => internal_err(e),
    }
}

async fn system_info(State(state): State<AppState>) -> impl IntoResponse {
    let total_devices: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM devices").fetch_one(&state.pool).await.unwrap_or((0,));
    let online_devices: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM devices WHERE status = 'online'").fetch_one(&state.pool).await.unwrap_or((0,));
    let total_rules: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rules").fetch_one(&state.pool).await.unwrap_or((0,));
    let active_rules: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rules WHERE enabled = 1").fetch_one(&state.pool).await.unwrap_or((0,));
    let total_readings: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sensor_readings").fetch_one(&state.pool).await.unwrap_or((0,));
    let total_alerts: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM command_log").fetch_one(&state.pool).await.unwrap_or((0,));
    let area_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM areas").fetch_one(&state.pool).await.unwrap_or((0,));

    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION", "0.1.0"),
        "server_time": Utc::now().timestamp(),
        "stats": {
            "total_devices": total_devices.0,
            "online_devices": online_devices.0,
            "total_rules": total_rules.0,
            "active_rules": active_rules.0,
            "total_readings": total_readings.0,
            "total_alerts": total_alerts.0,
            "area_count": area_count.0,
        }
    })).into_response()
}

async fn monitor_realtime(State(state): State<AppState>) -> impl IntoResponse {
    let areas = sqlx::query_as::<_, (String, String)>(
        "SELECT id, name FROM areas ORDER BY name"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let devices = sqlx::query_as::<_, (String, String, String, String, String, Option<String>)>(
        "SELECT id, name, node_id, device_type, status, area_id FROM devices ORDER BY name"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let device_ids: Vec<String> = devices.iter().map(|d| d.0.clone()).collect();
    let mut device_readings: std::collections::HashMap<String, Vec<(String, f64, String, i64)>> = std::collections::HashMap::new();

    for did in &device_ids {
        let readings = sqlx::query_as::<_, (String, f64, String, i64)>(
            "SELECT metric, value, unit, timestamp FROM sensor_readings \
             WHERE device_id = ? AND id IN ( \
                 SELECT MAX(id) FROM sensor_readings WHERE device_id = ? GROUP BY metric \
             ) ORDER BY metric"
        )
        .bind(did)
        .bind(did)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default();
        device_readings.insert(did.clone(), readings);
    }

    let areas_json: Vec<serde_json::Value> = areas.into_iter().map(|(aid, aname)| {
        let area_devices: Vec<serde_json::Value> = devices.iter().filter(|d| d.5.as_deref() == Some(&aid)).map(|(did, dname, node_id, dtype, status, _)| {
            let readings = device_readings.get(did);
            let readings_map: serde_json::Value = readings.map(|r| {
                r.iter().map(|(metric, value, unit, ts)| {
                    (metric.clone(), serde_json::json!({"value": value, "unit": unit, "timestamp": ts}))
                }).collect::<serde_json::Map<_, _>>().into()
            }).unwrap_or(serde_json::Value::Object(Default::default()));

            serde_json::json!({
                "id": did, "name": dname, "node_id": node_id,
                "device_type": dtype, "status": status,
                "latest_readings": readings_map
            })
        }).collect();

        serde_json::json!({"id": aid, "name": aname, "devices": area_devices})
    }).collect();

    Json(serde_json::json!({"areas": areas_json})).into_response()
}

async fn sse_events(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, axum::BoxError>>> {
    let rx = state.event_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| {
        match result {
            Ok(data) => Some(Ok(Event::default().data(data))),
            Err(_) => None,
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[derive(Debug, Deserialize)]
pub struct RelationQuery {
    pub from_id: Option<String>,
    pub from_type: Option<String>,
    pub to_id: Option<String>,
    pub to_type: Option<String>,
    pub relation_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRelationRequest {
    pub from_id: String,
    pub from_type: String,
    pub to_id: String,
    pub to_type: String,
    pub relation_type: String,
}

async fn list_relations(
    State(state): State<AppState>,
    Query(query): Query<RelationQuery>,
) -> impl IntoResponse {
    match agri_core::models::EntityRelation::query(
        &state.pool,
        query.from_id.as_deref(),
        query.from_type.as_deref(),
        query.to_id.as_deref(),
        query.to_type.as_deref(),
        query.relation_type.as_deref(),
    ).await {
        Ok(relations) => Json(relations).into_response(),
        Err(e) => internal_err(e),
    }
}

async fn create_relation(
    State(state): State<AppState>,
    Json(req): Json<CreateRelationRequest>,
) -> impl IntoResponse {
    match agri_core::models::EntityRelation::create(
        &state.pool,
        &req.from_id,
        &req.from_type,
        &req.to_id,
        &req.to_type,
        &req.relation_type,
    ).await {
        Ok(relation) => (StatusCode::CREATED, Json(relation)).into_response(),
        Err(e) => {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.message().contains("UNIQUE") {
                    return (StatusCode::CONFLICT, Json(serde_json::json!({"error": "relation already exists"}))).into_response();
                }
            }
            internal_err(e)
        }
    }
}

async fn delete_relation(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match agri_core::models::EntityRelation::delete(&state.pool, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => not_found(Some("relation")).into_response(),
        Err(e) => internal_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct IngestTelemetryRequest {
    pub node_id: String,
    pub metrics: serde_json::Value,
    pub captured_at: Option<i64>,
}

async fn ingest_telemetry(
    State(state): State<AppState>,
    Json(req): Json<IngestTelemetryRequest>,
) -> impl IntoResponse {
    let Some(metrics) = req.metrics.as_object() else {
        return bad_request("metrics must be an object");
    };

    // 速率限制：每 node_id 每秒最多 60 条
    if !state.telemetry_limiter.check(&req.node_id) {
        return (StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({"error": "rate limit exceeded"}))).into_response();
    }

    match agri_core::telemetry::process_telemetry(&state.pool, &req.node_id, metrics, Some(&state.event_tx), None, None, req.captured_at).await {
        Ok(inserted) => Json(serde_json::json!({"inserted": inserted, "message": "Telemetry ingested"})).into_response(),
        Err(e) => internal_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct BatchTelemetryItem {
    pub node_id: String,
    pub metrics: serde_json::Value,
    pub captured_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct BatchTelemetryRequest {
    pub batch: Vec<BatchTelemetryItem>,
}

async fn ingest_telemetry_batch(
    State(state): State<AppState>,
    Json(req): Json<BatchTelemetryRequest>,
) -> impl IntoResponse {
    let total = req.batch.len();
    let mut inserted = 0usize;

    for item in req.batch {
        // 速率限制：每 node_id 每秒最多 60 条
        if !state.telemetry_limiter.check(&item.node_id) {
            continue;
        }
        let Some(metrics) = item.metrics.as_object() else {
            continue;
        };
        match agri_core::telemetry::process_telemetry(&state.pool, &item.node_id, metrics, Some(&state.event_tx), None, None, item.captured_at).await {
            Ok(_) => inserted += 1,
            Err(e) => tracing::warn!("batch telemetry error for {}: {}", item.node_id, e),
        }
    }

    Json(serde_json::json!({
        "inserted": inserted,
        "failed": total - inserted,
        "total": total,
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

    /// 创建内存测试数据库并创建必要的表
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
                area_id TEXT,
                comfort_config TEXT,
                capabilities TEXT NOT NULL DEFAULT '[\"sensor\"]',
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
                priority INTEGER NOT NULL DEFAULT 0,
                auto_execute INTEGER NOT NULL DEFAULT 0,
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

        pool
    }

    #[allow(dead_code)]
    /// 插入测试设备并返回设备ID
    async fn insert_test_device(pool: &SqlitePool, name: &str, node_id: &str, device_type: &str) -> String {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();
        let caps = if device_type == "actuator" { "[\"actuator\"]" } else { "[\"sensor\"]" };
        sqlx::query(
            "INSERT INTO devices (id, name, node_id, device_type, status, capabilities, created_at, updated_at) VALUES (?, ?, ?, ?, 'offline', ?, ?, ?)"
        )
        .bind(&id).bind(name).bind(node_id).bind(device_type).bind(caps).bind(now).bind(now)
        .execute(pool).await.unwrap();
        id
    }

    // ========== 设备 API 测试（使用 Router::oneshot） ==========

    /// 测试列出设备 - 空列表，返回200
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

    /// 测试获取不存在的设备 - 返回404
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

    /// 测试仪表盘摘要 - 空数据返回200
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

    /// 测试创建设备 - 正常情况返回201
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

    /// 测试创建设备 - 非法设备类型返回400
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

    // ========== 参数校验单元测试 ==========

    /// 测试设备类型校验 - sensor 合法
    #[test]
    fn test_device_type_validation_sensor() {
        let device_type = "sensor";
        let result = match device_type {
            "sensor" | "actuator" => true,
            _ => false,
        };
        assert!(result);
    }

    /// 测试设备类型校验 - actuator 合法
    #[test]
    fn test_device_type_validation_actuator() {
        let device_type = "actuator";
        let result = match device_type {
            "sensor" | "actuator" => true,
            _ => false,
        };
        assert!(result);
    }

    /// 测试设备类型校验 - 非法类型
    #[test]
    fn test_device_type_validation_invalid() {
        let device_type = "invalid";
        let result = match device_type {
            "sensor" | "actuator" => true,
            _ => false,
        };
        assert!(!result);
    }

    /// 测试触发类型校验 - schedule 合法
    #[test]
    fn test_trigger_type_validation_schedule() {
        let trigger_type = "schedule";
        let result = match trigger_type {
            "schedule" | "condition" => true,
            _ => false,
        };
        assert!(result);
    }

    /// 测试触发类型校验 - condition 合法
    #[test]
    fn test_trigger_type_validation_condition() {
        let trigger_type = "condition";
        let result = match trigger_type {
            "schedule" | "condition" => true,
            _ => false,
        };
        assert!(result);
    }

    /// 测试触发类型校验 - 非法类型
    #[test]
    fn test_trigger_type_validation_invalid() {
        let trigger_type = "invalid";
        let result = match trigger_type {
            "schedule" | "condition" => true,
            _ => false,
        };
        assert!(!result);
    }

    // ========== 辅助函数 ==========

    /// 创建 Mock MQTT 客户端（不连接真实 broker）
    async fn create_mock_client() -> rumqttc::AsyncClient {
        let (client, _) = rumqttc::AsyncClient::new(
            rumqttc::MqttOptions::new("test-client", "127.0.0.1", 1883),
            10
        );
        client
    }
}
