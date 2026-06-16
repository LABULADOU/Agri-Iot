use agri_core::ai::assess::{add_weather_impact, assess_environment};
use agri_core::ai::calibration::calibrate_ventilator;
use agri_core::ai::emergency::{check_emergency_basic, WeatherAlertInput};
use agri_core::ai::fertigation::analyze_ec;
use agri_core::ai::knowledge::ObsidianKnowledge;
use agri_core::ai::llm::{HistoryMessage, LlmProvider, AgentResponse, SYSTEM_PROMPT_AGENT};
use agri_core::ai::retrieval::RetrievalEngine;
use agri_core::models::{
    ControlCase, CropProfile, ECTrends, ECRecommendation,
    GreenhouseConfig, NightModeConfig, PestKnowledge, WeatherData,
    WeatherKnowledge, ECManager,
};
use crate::response::{internal_err, not_found};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

// ========== Request/Response 类型 ==========

#[derive(Debug, Deserialize)]
pub struct AssessRequest {
    pub area_id: String,
    pub include_weather: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct KnowledgeSearchQuery {
    pub query: Option<String>,
    pub area_id: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct KnowledgeCaseRequest {
    pub case_data: serde_json::Value,
    pub outcome: Option<String>,
    pub effectiveness: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct VentilationControlRequest {
    pub area_id: String,
    pub vent_type: String,
    pub target_percent: f64,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct EmergencyResponse {
    #[serde(rename = "type")]
    emergency_type: String,
    confidence: f64,
    message: String,
    triggered_at: i64,
    pauses_auto_mode: bool,
    night_additional_contact: bool,
}

// ========== 路由 ==========

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/ai/assess", post(assess))
        .route("/api/v1/ai/emergency/status", get(emergency_status))
        .route("/api/v1/ai/knowledge/search", get(knowledge_search))
        .route("/api/v1/ai/knowledge/cases", get(knowledge_cases).post(knowledge_add_case))
        .route("/api/v1/ai/knowledge/obsidian/note", get(obsidian_read_note))
        .route("/api/v1/ai/knowledge/obsidian/search", get(obsidian_search))
        .route("/api/v1/ai/knowledge/obsidian/case", post(obsidian_add_case))
        .route("/api/v1/ai/ventilation/config/:area_id", get(ventilation_config))
        .route("/api/v1/ai/ventilation/calibrate/:device_id", post(calibrate))
        .route("/api/v1/ai/ec/analyze/:area_id", get(ec_analyze))
        .route("/api/v1/ai/control/ventilation", post(control_ventilation))
        .route("/api/v1/ai/agent/query", post(agent_query))
        .with_state(state)
}

// ========== 处理函数 ==========

/// POST /api/v1/ai/assess — 环境评估
async fn assess(
    State(state): State<AppState>,
    Json(req): Json<AssessRequest>,
) -> impl IntoResponse {
    // 获取最近传感器读数
    let readings = sqlx::query_as::<_, (String, f64)>(
        "SELECT metric, value FROM sensor_readings
         WHERE device_id IN (SELECT id FROM devices WHERE area_id = ?)
         AND timestamp > datetime('now', '-1 hour')
         ORDER BY timestamp DESC LIMIT 100"
    )
    .bind(&req.area_id)
    .fetch_all(&state.pool)
    .await;

    let readings = match readings {
        Ok(r) => r,
        Err(e) => return internal_err(e),
    };

    let mut soil_temp = 25.0;
    let mut soil_moisture = 70.0;
    let mut ec = 2.0;
    let mut air_temp = 28.0;
    let mut air_humidity = 65.0;

    for (metric, value) in &readings {
        match metric.as_str() {
            "soil_temperature" => soil_temp = *value,
            "soil_moisture" => soil_moisture = *value,
            "ec" => ec = *value,
            "temperature" => air_temp = *value,
            "humidity" => air_humidity = *value,
            _ => {}
        }
    }

    // 获取当前茬口对应的作物配置
    let crop = sqlx::query_as::<_, CropProfile>(
        "SELECT cp.* FROM crop_profiles cp
         JOIN crop_batches cb ON cb.crop_id = cp.id
         WHERE cb.area_id = ? AND cb.status = 'active'
         LIMIT 1"
    )
    .bind(&req.area_id)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let assessment = assess_environment(soil_temp, soil_moisture, ec, air_temp, air_humidity, crop.as_ref());

    // 获取气象数据
    let (assessment, weather_json) = if req.include_weather.unwrap_or(false) {
        let weather = sqlx::query_as::<_, WeatherData>(
            "SELECT * FROM weather_data WHERE area_id = ? ORDER BY timestamp DESC LIMIT 1"
        )
        .bind(&req.area_id)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None);
        let assessed = add_weather_impact(assessment, weather.as_ref());
        let wj = weather.map(|w| serde_json::json!({
            "temperature": w.temperature,
            "humidity": w.humidity,
            "wind_speed": w.wind_speed,
            "precipitation": w.precipitation,
            "snow_probability": w.snow_probability,
        }));
        (assessed, wj)
    } else {
        (assessment, None)
    };

    let assessment_id = Uuid::new_v4().to_string();

    // 存入 env_assessments 表
    let _ = sqlx::query(
        "INSERT INTO env_assessments (id, area_id, overall_score, soil_temp_score,
         soil_moisture_score, ec_score, air_temp_score, air_humidity_score, timestamp)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&assessment_id)
    .bind(&req.area_id)
    .bind(assessment.overall_score)
    .bind(assessment.soil_temp_score)
    .bind(assessment.soil_moisture_score)
    .bind(assessment.ec_score)
    .bind(assessment.air_temp_score)
    .bind(assessment.air_humidity_score)
    .bind(Utc::now().timestamp())
    .execute(&state.pool)
    .await;

    (StatusCode::OK, Json(serde_json::json!({
        "assessment_id": assessment_id,
        "scores": {
            "overall": assessment.overall_score,
            "soil_temp": assessment.soil_temp_score,
            "soil_moisture": assessment.soil_moisture_score,
            "ec": assessment.ec_score,
            "air_temp": assessment.air_temp_score,
            "air_humidity": assessment.air_humidity_score,
        },
        "deviations": assessment.deviations,
        "pest_risks": [],
        "recommendations": [],
        "weather_impact": assessment.weather_impact,
        "weather": weather_json,
    }))).into_response()
}

/// GET /api/v1/ai/emergency/status — 紧急情况状态
async fn emergency_status(State(state): State<AppState>) -> impl IntoResponse {
    // 获取最新气象数据
    let weather = sqlx::query_as::<_, WeatherData>(
        "SELECT * FROM weather_data ORDER BY timestamp DESC LIMIT 1"
    )
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let mut active_emergencies: Vec<EmergencyResponse> = Vec::new();

    if let Some(w) = weather {
        let input = WeatherAlertInput {
            wind_speed_kmh: w.wind_speed,
            precipitation_mm_per_hour: w.precipitation,
            temperature_celsius: w.temperature,
            snow_probability: w.snow_probability,
            humidity: w.humidity,
        };
        let emergencies = check_emergency_basic(&input);
        active_emergencies = emergencies.into_iter().map(|e| EmergencyResponse {
            emergency_type: format!("{:?}", e.emergency_type),
            confidence: e.confidence,
            message: e.message,
            triggered_at: e.triggered_at.timestamp(),
            pauses_auto_mode: e.pauses_auto_mode,
            night_additional_contact: e.night_additional_contact,
        }).collect();
    }

    let night_config = NightModeConfig {
        enabled: true,
        start_time: chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
        end_time: chrono::NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
        enhanced_monitoring: true,
        reduced_action_threshold: 0.7,
        night_contact_list: vec![],
    };
    let night_mode_active = night_config.is_night_time(Utc::now());

    let pauses_auto_mode = active_emergencies.iter().any(|e| e.pauses_auto_mode);

    (StatusCode::OK, Json(serde_json::json!({
        "active_emergencies": active_emergencies,
        "night_mode_active": night_mode_active,
        "pauses_auto_mode": pauses_auto_mode,
    }))).into_response()
}

/// GET /api/v1/ai/knowledge/search — 知识库搜索
async fn knowledge_search(
    State(state): State<AppState>,
    Query(q): Query<KnowledgeSearchQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(10);

    let mut results: Vec<serde_json::Value> = Vec::new();

    // 搜索作物知识
    if let Ok(crops) = sqlx::query_as::<_, CropProfile>(
        "SELECT * FROM crop_profiles WHERE name LIKE ? OR id LIKE ? LIMIT ?"
    )
    .bind(format!("%{}%", q.query.as_deref().unwrap_or("")))
    .bind(format!("%{}%", q.query.as_deref().unwrap_or("")))
    .bind(limit)
    .fetch_all(&state.pool)
    .await {
        for c in crops {
            results.push(serde_json::json!({
                "type": "crop_profile",
                "id": c.id,
                "name": c.name,
                "data": c,
            }));
        }
    }

    // 搜索病虫害知识
    if let Ok(pests) = sqlx::query_as::<_, PestKnowledge>(
        "SELECT * FROM pest_knowledge WHERE name LIKE ? OR id LIKE ? LIMIT ?"
    )
    .bind(format!("%{}%", q.query.as_deref().unwrap_or("")))
    .bind(format!("%{}%", q.query.as_deref().unwrap_or("")))
    .bind(limit)
    .fetch_all(&state.pool)
    .await {
        for p in pests {
            results.push(serde_json::json!({
                "type": "pest_knowledge",
                "id": p.id,
                "name": p.name,
                "data": p,
            }));
        }
    }

    // 搜索气象知识
    if let Ok(weather) = sqlx::query_as::<_, WeatherKnowledge>(
        "SELECT * FROM weather_knowledge WHERE condition_type LIKE ? OR id LIKE ? LIMIT ?"
    )
    .bind(format!("%{}%", q.query.as_deref().unwrap_or("")))
    .bind(format!("%{}%", q.query.as_deref().unwrap_or("")))
    .bind(limit)
    .fetch_all(&state.pool)
    .await {
        for w in weather {
            results.push(serde_json::json!({
                "type": "weather_knowledge",
                "id": w.id,
                "condition_type": w.condition_type,
                "data": w,
            }));
        }
    }

    (StatusCode::OK, Json(results)).into_response()
}

/// GET /api/v1/ai/knowledge/cases — 获取调控案例
async fn knowledge_cases(
    State(state): State<AppState>,
    Query(q): Query<KnowledgeSearchQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(20);

    let cases = if let Some(area_id) = &q.area_id {
        sqlx::query_as::<_, ControlCase>(
            "SELECT * FROM control_cases WHERE area_id = ? ORDER BY timestamp DESC LIMIT ?"
        )
        .bind(area_id)
        .bind(limit)
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query_as::<_, ControlCase>(
            "SELECT * FROM control_cases ORDER BY timestamp DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&state.pool)
        .await
    };

    match cases {
        Ok(cases) => (StatusCode::OK, Json(serde_json::json!(cases))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

/// POST /api/v1/ai/knowledge/cases — 添加调控案例
async fn knowledge_add_case(
    State(state): State<AppState>,
    Json(req): Json<KnowledgeCaseRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().timestamp();

    let result = sqlx::query(
        "INSERT INTO control_cases (id, situation, action_taken, outcome, effect_rating, timestamp)
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(req.case_data.to_string())
    .bind(serde_json::json!({}).to_string())
    .bind(&req.outcome)
    .bind(req.effectiveness)
    .bind(now)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({"id": id, "message": "Case created"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

/// GET /api/v1/ai/ventilation/config/:area_id — 获取通风配置
async fn ventilation_config(
    State(state): State<AppState>,
    Path(area_id): Path<String>,
) -> impl IntoResponse {
    let config = sqlx::query_as::<_, GreenhouseConfig>(
        "SELECT * FROM greenhouse_config WHERE area_id = ?"
    )
    .bind(&area_id)
    .fetch_optional(&state.pool)
    .await;

    match config {
        Ok(Some(c)) => (StatusCode::OK, Json(serde_json::json!(c))).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "No config found for this area"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

/// POST /api/v1/ai/ventilation/calibrate/:device_id — 校准卷膜器
async fn calibrate(
    Path(device_id): Path<String>,
) -> impl IntoResponse {
    let result = calibrate_ventilator(&device_id, "");
    (StatusCode::OK, Json(serde_json::json!(result))).into_response()
}

/// GET /api/v1/ai/ec/analyze/:area_id — EC 值分析
async fn ec_analyze(
    State(state): State<AppState>,
    Path(area_id): Path<String>,
) -> impl IntoResponse {
    // 获取最近 EC 读数
    let readings = sqlx::query_as::<_, (i64, f64)>(
        "SELECT strftime('%s', timestamp), value FROM sensor_readings
         WHERE metric = 'ec' AND device_id IN (SELECT id FROM devices WHERE area_id = ?)
         ORDER BY timestamp DESC LIMIT 10"
    )
    .bind(&area_id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let current_ec = readings.first().map(|(_, v)| *v).unwrap_or(2.0);

    let ec_readings: Vec<(chrono::DateTime<Utc>, f64)> = readings.into_iter()
        .map(|(ts, v)| {
            let dt = chrono::DateTime::from_timestamp(ts, 0).unwrap_or(Utc::now());
            (dt, v)
        })
        .collect();

    let trend = ECTrends {
        readings: ec_readings,
        period_hours: 24,
    };

    let manager = ECManager {
        optimal_ec_min: 1.5,
        optimal_ec_max: 4.0,
        warning_threshold_low: 0.5,
        warning_threshold_high: 6.0,
    };

    let recommendation = analyze_ec(&manager, current_ec, &trend, &area_id);

    let rec_str = match &recommendation {
        ECRecommendation::NoAction => "NoAction",
        ECRecommendation::IncreaseEC { .. } => "IncreaseEC",
        ECRecommendation::DecreaseEC { .. } => "DecreaseEC",
        ECRecommendation::ManualIntervention { .. } => "ManualIntervention",
    };

    (StatusCode::OK, Json(serde_json::json!({
        "current_ec": current_ec,
        "trend": trend.analyze(),
        "recommendation": rec_str,
        "details": recommendation,
    }))).into_response()
}

/// POST /api/v1/ai/control/ventilation — 控制通风
async fn control_ventilation(
    State(state): State<AppState>,
    Json(req): Json<VentilationControlRequest>,
) -> impl IntoResponse {
    // 检查是否有紧急情况
    let weather = sqlx::query_as::<_, WeatherData>(
        "SELECT * FROM weather_data ORDER BY timestamp DESC LIMIT 1"
    )
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    if let Some(w) = weather {
        let input = WeatherAlertInput {
            wind_speed_kmh: w.wind_speed,
            precipitation_mm_per_hour: w.precipitation,
            temperature_celsius: w.temperature,
            snow_probability: w.snow_probability,
            humidity: w.humidity,
        };
        let emergencies = check_emergency_basic(&input);
        if !emergencies.is_empty() {
            // 发送紧急关闭命令
            let cmd_id = Uuid::new_v4();
            if let Some(device_id) = get_vent_device_id(&state, &req.area_id, &req.vent_type).await {
                let _ = sqlx::query(
                    "INSERT INTO command_log (device_id, command, payload, status, created_at)
                     VALUES (?, 'CLOSE', '{\"emergency\": true}', 'pending', datetime('now'))"
                )
                .bind(&device_id)
                .execute(&state.pool)
                .await;
            }

            return (StatusCode::OK, Json(serde_json::json!({
                "command_id": cmd_id.to_string(),
                "status": "emergency_overridden",
                "emergency_overridden": true,
                "message": emergencies.first().map(|e| e.message.as_str()).unwrap_or("紧急情况"),
            }))).into_response();
        }
    }

    // 正常发送通风命令
    let cmd_id = Uuid::new_v4();
    let device_id = get_vent_device_id(&state, &req.area_id, &req.vent_type).await;

    if let Some(did) = device_id {
        let payload = serde_json::json!({
            "command": if req.target_percent > 0.0 { "OPEN" } else { "CLOSE" },
            "target_percent": req.target_percent,
            "reason": req.reason,
        });

        let _ = sqlx::query(
            "INSERT INTO command_log (device_id, command, payload, status, created_at)
             VALUES (?, ?, ?, 'pending', datetime('now'))"
        )
        .bind(&did)
        .bind(payload["command"].as_str().unwrap_or("OPEN"))
        .bind(payload.to_string())
        .execute(&state.pool)
        .await;
    }

    (StatusCode::OK, Json(serde_json::json!({
        "command_id": cmd_id.to_string(),
        "status": "executed",
        "emergency_overridden": false,
        "message": "通风控制命令已发送",
    }))).into_response()
}

// ========== Obsidian 知识库端点 ==========

#[derive(Debug, Deserialize)]
struct ObsidianNoteQuery {
    path: String,
}

#[derive(Debug, Deserialize)]
struct ObsidianSearchQuery {
    query: String,
}

#[derive(Debug, Deserialize)]
struct ObsidianCaseRequest {
    area_id: String,
    situation: String,
    outcome: String,
}

/// GET /api/v1/ai/knowledge/obsidian/note?path=...
async fn obsidian_read_note(
    State(state): State<AppState>,
    Query(q): Query<ObsidianNoteQuery>,
) -> impl IntoResponse {
    let vault_path = match &state.obsidian_vault_path {
        Some(p) => p.clone(),
        None => return not_found(Some("OBSIDIAN_VAULT_PATH not set")),
    };
    let vault = ObsidianKnowledge::new(&vault_path);
    match vault.read_note(&q.path) {
        Ok(content) => (StatusCode::OK, Json(serde_json::json!({"path": q.path, "content": content}))).into_response(),
        Err(e) => not_found(Some(&e.to_string())),
    }
}

/// GET /api/v1/ai/knowledge/obsidian/search?query=...
async fn obsidian_search(
    State(state): State<AppState>,
    Query(q): Query<ObsidianSearchQuery>,
) -> impl IntoResponse {
    let vault_path = match &state.obsidian_vault_path {
        Some(p) => p.clone(),
        None => return not_found(Some("OBSIDIAN_VAULT_PATH not set")),
    };
    let vault = ObsidianKnowledge::new(&vault_path);
    match vault.search(&q.query) {
        Ok(results) => (StatusCode::OK, Json(serde_json::json!(results))).into_response(),
        Err(e) => internal_err(e),
    }
}

/// POST /api/v1/ai/knowledge/obsidian/case
async fn obsidian_add_case(
    State(state): State<AppState>,
    Json(req): Json<ObsidianCaseRequest>,
) -> impl IntoResponse {
    let vault_path = match &state.obsidian_vault_path {
        Some(p) => p.clone(),
        None => return not_found(Some("OBSIDIAN_VAULT_PATH not set")),
    };
    let vault = ObsidianKnowledge::new(&vault_path);
    let case_id = Uuid::new_v4().to_string();
    match vault.append_case(&req.area_id, &case_id, &req.situation, &req.outcome) {
        Ok(file_path) => (StatusCode::CREATED, Json(serde_json::json!({"id": case_id, "file_path": file_path}))).into_response(),
        Err(e) => internal_err(e),
    }
}

// ========== Agent 查询端点 ==========

#[derive(Debug, Deserialize)]
struct AgentQueryRequest {
    query: String,
    node_id: Option<String>,
    history: Option<Vec<agri_core::ai::llm::HistoryMessage>>,
}

/// POST /api/v1/ai/agent/query — 自然语言查询
async fn agent_query(
    State(state): State<AppState>,
    Json(req): Json<AgentQueryRequest>,
) -> impl IntoResponse {
    // 1. 构建 LLM Provider
    let provider = match LlmProvider::from_env() {
        Ok(p) => p,
        Err(e) => {
            return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({
                "error": format!("LLM not configured: {}", e),
                "answer": "AI 助手未配置，请设置 LLM_API_KEY 等环境变量",
                "data_sources": [],
                "follow_up_questions": [],
            }))).into_response();
        }
    };

    // 2. 构建 RAG 上下文
    let node_id = req.node_id.as_deref().unwrap_or("");
    let vault_path = state.obsidian_vault_path.clone().unwrap_or_default();
    let mut retrieval = RetrievalEngine::new(state.pool.clone());
    if !vault_path.is_empty() {
        retrieval = retrieval.with_vault(ObsidianKnowledge::new(&vault_path));
    }

    let context_json = match retrieval.build(node_id, 3).await {
        Ok(r) => serde_json::to_string_pretty(&r).unwrap_or_default(),
        Err(e) => {
            tracing::warn!("[agent] retrieval failed: {}", e);
            format!("检索失败: {}", e)
        }
    };

    // 3. 调用 LLM（携带对话历史）
    let user_prompt = format!(
        "用户问题：{}\n\n当前系统状态：\n{}",
        req.query, context_json
    );

    let history = req.history.as_deref().unwrap_or(&[]);
    match provider.chat_with_history(SYSTEM_PROMPT_AGENT, history, &user_prompt).await {
        Ok(answer) => {
            (StatusCode::OK, Json(serde_json::json!({
                "answer": answer,
                "data_sources": ["sensor_readings", "weather_data", "crop_profiles", "control_cases"],
                "follow_up_questions": [
                    "需要我查询更多历史数据吗？",
                    "想了解某个指标的详细趋势吗？",
                    "需要我给出调控建议吗？",
                ],
            }))).into_response()
        }
        Err(e) => {
            tracing::warn!("[agent] LLM call failed: {}", e);
            (StatusCode::OK, Json(serde_json::json!({
                "answer": "AI 暂时无法回答，请稍后重试",
                "data_sources": [],
                "follow_up_questions": [],
            }))).into_response()
        }
    }
}
/// 辅助：获取通风设备 ID
async fn get_vent_device_id(state: &AppState, area_id: &str, vent_type: &str) -> Option<String> {
    let col = match vent_type {
        "top" => "top_vent_device_id",
        "side" => "side_vent_device_id",
        _ => return None,
    };
    let query = format!("SELECT {} FROM greenhouse_config WHERE area_id = ?", col);
    let result: Result<Option<(Option<String>,)>, _> = sqlx::query_as(&query)
        .bind(area_id)
        .fetch_optional(&state.pool)
        .await;
    result.ok().flatten().and_then(|r| r.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, http::StatusCode};
    use sqlx::SqlitePool;
    use tower::ServiceExt;
    use tokio::sync::broadcast;

    /// 创建内存 SQLite + 基础表 + AI 表
    async fn setup_ai_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        // 基础表（sensor_readings 在 001_init.sql 中，devices 表被 assess handler 查询）
        let base_tables = [
            "CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY, name TEXT NOT NULL, node_id TEXT NOT NULL,
                device_type TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'offline',
                config TEXT, area_id TEXT, comfort_config TEXT,
                capabilities TEXT NOT NULL DEFAULT '[\"sensor\"]',
                created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
            )",
            "CREATE TABLE IF NOT EXISTS sensor_readings (
                id INTEGER PRIMARY KEY AUTOINCREMENT, device_id TEXT NOT NULL,
                metric TEXT NOT NULL, value REAL NOT NULL,
                unit TEXT NOT NULL DEFAULT '', timestamp INTEGER NOT NULL
            )",
        ];
        for sql in base_tables {
            sqlx::query(sql).execute(&pool).await.unwrap();
        }

        let ai_tables = [
            "CREATE TABLE IF NOT EXISTS crop_profiles (id TEXT PRIMARY KEY, name TEXT NOT NULL, variety TEXT, growth_stages TEXT, soil_temp_min REAL, soil_temp_max REAL, soil_temp_optimal REAL, soil_moisture_min REAL, soil_moisture_max REAL, soil_moisture_optimal REAL, air_temp_min REAL, air_temp_max REAL, air_temp_optimal REAL, air_humidity_min REAL, air_humidity_max REAL, air_humidity_optimal REAL, ec_min REAL, ec_max REAL, ec_optimal REAL, ventilation_preference TEXT, wind_sensitivity REAL, embedding_id TEXT, created_at INTEGER NOT NULL DEFAULT 0, updated_at INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE IF NOT EXISTS pest_knowledge (id TEXT PRIMARY KEY, name TEXT NOT NULL, crop_types TEXT, trigger_conditions TEXT, symptoms TEXT, severity TEXT, prevention TEXT, treatment TEXT, medication TEXT, is_emergency INTEGER NOT NULL DEFAULT 0, emergency_action TEXT, source TEXT, confidence REAL NOT NULL DEFAULT 0.8, embedding_id TEXT, created_at INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE IF NOT EXISTS control_cases (id TEXT PRIMARY KEY, area_id TEXT, crop_profile_id TEXT, situation TEXT, weather_forecast TEXT, action_taken TEXT, manual_override INTEGER NOT NULL DEFAULT 0, outcome TEXT, effect_rating INTEGER, health_improvement REAL, action_duration_minutes INTEGER, recovery_time_minutes INTEGER, notes TEXT, timestamp INTEGER NOT NULL DEFAULT 0, embedding_id TEXT)",
            "CREATE TABLE IF NOT EXISTS weather_knowledge (id TEXT PRIMARY KEY, condition_type TEXT NOT NULL, thresholds TEXT, protection_rules TEXT, time_constraints TEXT, contact_required INTEGER NOT NULL DEFAULT 0, contact_urgency TEXT, contact_message TEXT, notes TEXT, embedding_id TEXT, created_at INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE IF NOT EXISTS greenhouse_config (id TEXT PRIMARY KEY, area_id TEXT NOT NULL, top_vent_min_percent REAL NOT NULL DEFAULT 0, top_vent_max_percent REAL NOT NULL DEFAULT 100, top_vent_current_percent REAL NOT NULL DEFAULT 0, top_vent_device_id TEXT, side_vent_min_percent REAL NOT NULL DEFAULT 0, side_vent_max_percent REAL NOT NULL DEFAULT 100, side_vent_current_percent REAL NOT NULL DEFAULT 0, side_vent_device_id TEXT, irrigation_device_id TEXT, fertigation_device_id TEXT, emergency_contact_name TEXT, emergency_contact_phone TEXT, top_vent_calibrated INTEGER NOT NULL DEFAULT 0, side_vent_calibrated INTEGER NOT NULL DEFAULT 0, calibration_date INTEGER, updated_at INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE IF NOT EXISTS sensor_config (id TEXT PRIMARY KEY, area_id TEXT NOT NULL, sensor_type TEXT NOT NULL, device_id TEXT, calibration_offset REAL NOT NULL DEFAULT 0, is_active INTEGER NOT NULL DEFAULT 1, last_reading INTEGER, created_at INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE IF NOT EXISTS weather_data (id INTEGER PRIMARY KEY AUTOINCREMENT, area_id TEXT, source TEXT NOT NULL, temperature REAL, humidity REAL, wind_speed REAL, wind_direction TEXT, precipitation REAL, snow_probability REAL, uv_index REAL, forecast_hour INTEGER, timestamp INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE IF NOT EXISTS env_assessments (id TEXT PRIMARY KEY, area_id TEXT, crop_profile_id TEXT, timestamp INTEGER NOT NULL DEFAULT 0, overall_score REAL, soil_temp_score REAL, soil_moisture_score REAL, ec_score REAL, air_temp_score REAL, air_humidity_score REAL, deviations TEXT, pest_risks TEXT, recommendations TEXT, weather_impact TEXT, is_emergency INTEGER NOT NULL DEFAULT 0, emergency_type TEXT)",
            "CREATE TABLE IF NOT EXISTS kb_update_log (id INTEGER PRIMARY KEY AUTOINCREMENT, update_type TEXT NOT NULL, source TEXT NOT NULL, content_summary TEXT, effectiveness_score REAL, timestamp INTEGER NOT NULL DEFAULT 0)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_greenhouse_config_area ON greenhouse_config(area_id)",
            "CREATE INDEX IF NOT EXISTS idx_weather_data_area ON weather_data(area_id)",
            "CREATE INDEX IF NOT EXISTS idx_weather_data_timestamp ON weather_data(timestamp)",
            "CREATE INDEX IF NOT EXISTS idx_env_assessments_area ON env_assessments(area_id)",
        ];

        for sql in ai_tables {
            sqlx::query(sql).execute(&pool).await.unwrap();
        }
        pool
    }

    fn make_state(pool: SqlitePool) -> AppState {
        let (tx, _) = broadcast::channel(256);
        AppState {
            pool,
            mqtt_client: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
            rules_cache: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
            event_tx: tx,
            obsidian_vault_path: None,
            emergency_ctx: std::sync::Arc::new(tokio::sync::Mutex::new(
                agri_core::ai::emergency::EmergencyContext::new()
            )),
            telemetry_limiter: std::sync::Arc::new(crate::rate_limiter::RateLimiter::new(1000, 1)),
        }
    }

    /// POST /api/v1/ai/assess — 空数据库默认评估
    #[tokio::test]
    async fn test_assess_default() {
        let pool = setup_ai_db().await;
        let state = make_state(pool);
        let router = create_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/ai/assess")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"area_id": "zone-1", "include_weather": false}"#))
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// GET /api/v1/ai/emergency/status — 无气象数据
    #[tokio::test]
    async fn test_emergency_status_empty() {
        let pool = setup_ai_db().await;
        let state = make_state(pool);
        let router = create_router(state);

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/ai/emergency/status")
            .body(Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// GET /api/v1/ai/ventilation/config/:area_id — 无配置返回 404
    #[tokio::test]
    async fn test_ventilation_config_not_found() {
        let pool = setup_ai_db().await;
        let state = make_state(pool);
        let router = create_router(state);

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/ai/ventilation/config/nonexistent")
            .body(Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    /// GET /api/v1/ai/knowledge/cases — 空列表
    #[tokio::test]
    async fn test_knowledge_cases_empty() {
        let pool = setup_ai_db().await;
        let state = make_state(pool);
        let router = create_router(state);

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/ai/knowledge/cases")
            .body(Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// POST /api/v1/ai/knowledge/cases — 添加案例
    #[tokio::test]
    async fn test_knowledge_add_case() {
        let pool = setup_ai_db().await;
        let state = make_state(pool);
        let router = create_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/ai/knowledge/cases")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"case_data": {"soil_temp": 25}, "outcome": "success", "effectiveness": 5}"#))
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    /// GET /api/v1/ai/ec/analyze/:area_id — 无数据默认值
    #[tokio::test]
    async fn test_ec_analyze_empty() {
        let pool = setup_ai_db().await;
        let state = make_state(pool);
        let router = create_router(state);

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/ai/ec/analyze/zone-1")
            .body(Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// POST /api/v1/ai/ventilation/calibrate/:device_id
    #[tokio::test]
    async fn test_calibrate() {
        let pool = setup_ai_db().await;
        let state = make_state(pool);
        let router = create_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/ai/ventilation/calibrate/vent-001")
            .body(Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// POST /api/v1/ai/control/ventilation — 无紧急情况可正常执行
    #[tokio::test]
    async fn test_control_ventilation() {
        let pool = setup_ai_db().await;
        let state = make_state(pool);
        let router = create_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/ai/control/ventilation")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"area_id": "zone-1", "vent_type": "top", "target_percent": 80, "reason": "降温"}"#))
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// GET /api/v1/ai/knowledge/search — 空搜索
    #[tokio::test]
    async fn test_knowledge_search_empty() {
        let pool = setup_ai_db().await;
        let state = make_state(pool);
        let router = create_router(state);

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/ai/knowledge/search?query=番茄")
            .body(Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
