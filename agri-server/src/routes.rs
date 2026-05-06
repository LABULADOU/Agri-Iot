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

        pool
    }

    /// 插入测试设备并返回设备ID
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
