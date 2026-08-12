use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::response;
use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/mixing/fertilizer/recommend", post(recommend_fertilizer))
        .route("/api/v1/mixing/pesticide/recommend", post(recommend_pesticide))
        .route("/api/v1/mixing/recipes", get(list_recipes))
        .route("/api/v1/mixing/recipes/:id/apply", post(apply_recipe))
        .route("/api/v1/mixing/presets", get(list_presets).post(create_preset))
        .route("/api/v1/mixing/presets/:id", put(update_preset).delete(delete_preset))
        .with_state(state)
}

// ==================== 生长阶段推理（纯函数） ====================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrowthStage {
    Seedling,   // 苗期 0-15天
    Vegetative, // 营养生长期 15-40天
    Flowering,  // 开花期 40-60天
    Fruiting,   // 结果期 60天+
}

impl GrowthStage {
    pub fn key(&self) -> &'static str {
        match self {
            GrowthStage::Seedling => "seedling",
            GrowthStage::Vegetative => "vegetative",
            GrowthStage::Flowering => "flowering",
            GrowthStage::Fruiting => "fruiting",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            GrowthStage::Seedling => "苗期",
            GrowthStage::Vegetative => "营养生长期",
            GrowthStage::Flowering => "开花期",
            GrowthStage::Fruiting => "结果期",
        }
    }
}

/// 按生长天数推断阶段（带边界修正）
pub fn infer_growth_stage(days: i64) -> GrowthStage {
    match days {
        d if d < 0 => GrowthStage::Seedling,
        d if d < 15 => GrowthStage::Seedling,
        d if d < 40 => GrowthStage::Vegetative,
        d if d < 60 => GrowthStage::Flowering,
        _ => GrowthStage::Fruiting,
    }
}

/// 阶段基准配肥方案（N-P-K 百分比 + 亩用量）
pub fn stage_fertilizer_base(stage: GrowthStage) -> serde_json::Value {
    match stage {
        GrowthStage::Seedling => json!({
            "npk": [20, 20, 20], "amount_per_mu": 5.0, "unit": "kg/亩",
            "ec_target": 1.2, "dilution": "2500倍", "water_volume": 60,
            "name": "平衡型水溶肥(20-20-20)",
        }),
        GrowthStage::Vegetative => json!({
            "npk": [30, 10, 20], "amount_per_mu": 8.0, "unit": "kg/亩",
            "ec_target": 1.8, "dilution": "2000倍", "water_volume": 80,
            "name": "高氮型水溶肥(30-10-20)",
        }),
        GrowthStage::Flowering => json!({
            "npk": [10, 30, 20], "amount_per_mu": 6.0, "unit": "kg/亩",
            "ec_target": 2.0, "dilution": "1800倍", "water_volume": 80,
            "name": "高磷型水溶肥(10-30-20)",
        }),
        GrowthStage::Fruiting => json!({
            "npk": [15, 10, 35], "amount_per_mu": 10.0, "unit": "kg/亩",
            "ec_target": 2.2, "dilution": "1500倍", "water_volume": 100,
            "name": "高钾型水溶肥(15-10-35)",
        }),
    }
}

/// 根据作物 EC 目标与当前土壤 EC 输出用量修正系数和说明
pub fn ec_adjustment(current_ec: Option<f64>, crop_ec_target: Option<f64>) -> (f64, String) {
    let (Some(cur), Some(target)) = (current_ec, crop_ec_target) else {
        return (1.0, String::new());
    };
    if cur <= 0.0 {
        // EC=0 是传感器无效读数（短路/未接入），不触发施肥调整
        (1.0, String::new())
    } else if cur > target + 0.5 {
        (0.8, format!("土壤 EC {:.1} 高于目标 {:.1}，建议减量 20%，必要时清水淋洗", cur, target))
    } else if cur < target - 0.5 {
        (1.2, format!("土壤 EC {:.1} 低于目标 {:.1}，建议增量 20%", cur, target))
    } else {
        (1.0, format!("土壤 EC {:.1} 处于目标附近（{:.1}），按基准量施用", cur, target))
    }
}

/// 生成完整配肥方案
pub fn build_fertilizer_plan(
    stage: GrowthStage,
    growth_days: i64,
    current_ec: Option<f64>,
    ec_target: Option<f64>,
    soil_temp: Option<f64>,
) -> serde_json::Value {
    let base = stage_fertilizer_base(stage);
    let (factor, ec_note) = ec_adjustment(current_ec, ec_target);
    let amount = base["amount_per_mu"].as_f64().unwrap_or(5.0) * factor;
    let npk = base["npk"].as_array().unwrap();

    let mut adjustments = Vec::new();
    if !ec_note.is_empty() {
        adjustments.push(ec_note);
    }
    if let Some(temp) = soil_temp {
        if temp < 12.0 {
            adjustments.push(format!("土壤温度 {:.1}°C 过低，根系吸收弱，建议先提温再施肥或叶面补充", temp));
        } else if temp > 32.0 {
            adjustments.push(format!("土壤温度 {:.1}°C 过高，建议降低浓度并分次施用", temp));
        }
    }

    json!({
        "type": "fertilizer",
        "stage": stage.label(),
        "stage_key": stage.key(),
        "growth_days": growth_days,
        "plan": {
            "items": [{
                "name": base["name"].as_str().unwrap_or(""),
                "n": npk[0], "p": npk[1], "k": npk[2],
                "amount": (amount * 10.0).round() / 10.0,
                "unit": base["unit"].as_str().unwrap_or("kg/亩"),
            }],
            "dilution": base["dilution"].as_str().unwrap_or(""),
            "water_volume": base["water_volume"].as_f64().unwrap_or(80.0),
            "ec_target": base["ec_target"].as_f64().unwrap_or(1.8),
        },
        "adjustments": adjustments,
        "reasoning": format!(
            "当前为{}（种植后第 {} 天），按阶段基准配方 {}（N-P-K {}-{}-{}），亩用量 {:.1} kg{}",
            stage.label(), growth_days,
            base["name"].as_str().unwrap_or(""),
            npk[0], npk[1], npk[2], amount,
            if adjustments.is_empty() { "，无额外修正" } else { "" },
        ),
    })
}

/// 生成配药方案（无知识库条目时给出降级提示）
pub fn build_pesticide_plan(
    pest: &str,
    stage: Option<&GrowthStage>,
    knowledge: Option<&serde_json::Value>,
    preset: Option<&serde_json::Value>,
) -> serde_json::Value {
    if let Some(preset) = preset {
        return json!({
            "type": "pesticide",
            "target_pest": pest,
            "stage": stage.map(|s| s.label()).unwrap_or("—"),
            "plan": preset,
            "adjustments": [],
            "reasoning": "命中预设方案（人工确认过的配方）".to_string(),
            "source": "preset",
        });
    }

    match knowledge {
        Some(k) => {
            let medication = k.get("medication").and_then(|m| m.as_str()).unwrap_or("");
            let treatment = k.get("treatment").and_then(|m| m.as_str()).unwrap_or("");
            let severity = k.get("severity").and_then(|m| m.as_str()).unwrap_or("medium");
            let mut plan = json!({
                "items": [],
                "dilution": "",
                "safety_interval_days": 7,
            });
            let mut items = Vec::new();
            if !medication.is_empty() {
                items.push(json!({"name": medication, "source": "知识库"}));
            }
            plan["items"] = json!(items);
            json!({
                "type": "pesticide",
                "target_pest": pest,
                "stage": stage.map(|s| s.label()).unwrap_or("—"),
                "plan": plan,
                "severity": severity,
                "treatment": treatment,
                "adjustments": [],
                "reasoning": format!("知识库命中「{}」，请核对登记作物与稀释浓度后使用", k.get("name").and_then(|n| n.as_str()).unwrap_or(pest)),
                "source": "knowledge",
            })
        }
        None => json!({
            "type": "pesticide",
            "target_pest": pest,
            "plan": json!({"items": [], "dilution": "", "safety_interval_days": 7}),
            "adjustments": ["知识库中未找到该病虫害条目，请在 AI 知识库补充 pest_knowledge 记录后再生成方案"],
            "reasoning": "无法自动推荐，需人工介入".to_string(),
            "source": "fallback",
        }),
    }
}

// ==================== 推荐端点 ====================

#[derive(Debug, Deserialize)]
pub struct FertilizerRecommendRequest {
    pub area_id: String,
    pub crop_batch_id: Option<String>,
    pub growth_days: Option<i64>,
}

async fn recommend_fertilizer(
    State(state): State<AppState>,
    Json(req): Json<FertilizerRecommendRequest>,
) -> impl IntoResponse {
    let now = Utc::now().timestamp();

    // 1. 生长天数：优先客户端指定，否则由 crop_batch.plant_date 推算
    let (growth_days, crop_name, stage_override) = match &req.crop_batch_id {
        Some(batch_id) => {
            let row = sqlx::query(
                "SELECT cb.plant_date, c.name AS crop_name FROM crop_batches cb LEFT JOIN crops c ON c.id = cb.crop_id WHERE cb.id = ?",
            )
            .bind(batch_id)
            .fetch_optional(&state.pool)
            .await;
            match row {
                Ok(Some(r)) => {
                    let plant_date: i64 = r.try_get("plant_date").unwrap_or(now);
                    let days = req.growth_days.unwrap_or_else(|| ((now - plant_date) / 86400).max(0));
                    let name: Option<String> = r.try_get("crop_name").unwrap_or(None);
                    (days, name.unwrap_or_default(), days)
                }
                _ => {
                    let days = req.growth_days.unwrap_or(0).max(0);
                    (days, String::new(), days)
                }
            }
        }
        None => {
            let days = req.growth_days.unwrap_or(0).max(0);
            (days, String::new(), days)
        }
    };

    let stage = infer_growth_stage(stage_override);

    // 2. 查询 crop_profile 的 EC 目标（同一 crop 名称匹配）
    let (ec_target, current_ec, soil_temp) = fetch_soil_context(&state, &req.area_id, &crop_name).await;

    // 3. 查询预设（crop 无关的全局 fertilizer 阶段预设）
    let preset = sqlx::query(
        "SELECT items, dilution, dosage_per_unit, water_volume, ec_target, note FROM mixing_presets WHERE mix_type = 'fertilizer' AND stage_key = ? AND (crop_id IS NULL OR crop_id = '') ORDER BY created_at DESC LIMIT 1",
    )
    .bind(stage.key())
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten()
    .map(|r| serde_json::json!({
        "items": r.try_get::<Option<String>, _>("items").unwrap_or(None).and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()).unwrap_or(json!([])),
        "dilution": r.try_get::<Option<String>, _>("dilution").unwrap_or(None).unwrap_or_default(),
        "water_volume": r.try_get::<Option<String>, _>("water_volume").unwrap_or(None).unwrap_or_default(),
        "note": r.try_get::<Option<String>, _>("note").unwrap_or(None).unwrap_or_default(),
    }));

    let plan = if let Some(p) = preset {
        json!({
            "type": "fertilizer",
            "stage": stage.label(),
            "stage_key": stage.key(),
            "growth_days": growth_days,
            "plan": p,
            "adjustments": [],
            "reasoning": "命中预设方案".to_string(),
            "source": "preset",
        })
    } else {
        build_fertilizer_plan(stage, growth_days, current_ec, ec_target, soil_temp)
    };

    // 4. 保存配方记录
    let recipe_id = Uuid::new_v4().to_string();
    let input = json!({"area_id": req.area_id, "crop_batch_id": req.crop_batch_id, "growth_days": growth_days});
    let _ = sqlx::query(
        "INSERT INTO mixing_recipes (id, area_id, crop_batch_id, mix_type, stage_key, growth_days, input, result, status, created_at) VALUES (?, ?, ?, 'fertilizer', ?, ?, ?, ?, 'generated', ?)",
    )
    .bind(&recipe_id)
    .bind(&req.area_id)
    .bind(&req.crop_batch_id)
    .bind(stage.key())
    .bind(growth_days)
    .bind(input.to_string())
    .bind(plan.to_string())
    .bind(now)
    .execute(&state.pool)
    .await;

    (StatusCode::OK, Json(plan)).into_response()
}

async fn fetch_soil_context(
    state: &AppState,
    area_id: &str,
    crop_name: &str,
) -> (Option<f64>, Option<f64>, Option<f64>) {
    // 1. crop_profile EC 目标
    let ec_target = if crop_name.is_empty() {
        None
    } else {
        sqlx::query("SELECT ec_optimal FROM crop_profiles WHERE name = ? ORDER BY created_at DESC LIMIT 1")
            .bind(crop_name)
            .fetch_optional(&state.pool)
            .await
            .ok()
            .flatten()
            .and_then(|r| r.try_get::<Option<f64>, _>("ec_optimal").ok().flatten())
    };

    // 2. 该区域设备的最新土壤 EC / 温度
    let row = sqlx::query(
        "SELECT
            (SELECT value FROM sensor_readings sr JOIN devices d ON d.id = sr.device_id WHERE d.area_id = ? AND sr.metric = 'ec' ORDER BY sr.timestamp DESC LIMIT 1) AS ec,
            (SELECT value FROM sensor_readings sr JOIN devices d ON d.id = sr.device_id WHERE d.area_id = ? AND sr.metric = 'soil_temperature' ORDER BY sr.timestamp DESC LIMIT 1) AS temp",
    )
    .bind(area_id)
    .bind(area_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let (current_ec, soil_temp) = match row {
        Some(r) => (
            r.try_get::<Option<f64>, _>("ec").ok().flatten(),
            r.try_get::<Option<f64>, _>("temp").ok().flatten(),
        ),
        None => (None, None),
    };

    (ec_target, current_ec, soil_temp)
}

#[derive(Debug, Deserialize)]
pub struct PesticideRecommendRequest {
    pub area_id: String,
    pub crop_batch_id: Option<String>,
    pub target_pest: String,
}

async fn recommend_pesticide(
    State(state): State<AppState>,
    Json(req): Json<PesticideRecommendRequest>,
) -> impl IntoResponse {
    if req.target_pest.trim().is_empty() {
        return response::bad_request("target_pest is required");
    }
    let now = Utc::now().timestamp();

    // 1. 生长阶段
    let growth_days = match &req.crop_batch_id {
        Some(batch_id) => {
            let row = sqlx::query("SELECT plant_date FROM crop_batches WHERE id = ?")
                .bind(batch_id)
                .fetch_optional(&state.pool)
                .await;
            match row {
                Ok(Some(r)) => {
                    let plant_date: i64 = r.try_get("plant_date").unwrap_or(now);
                    ((now - plant_date) / 86400).max(0)
                }
                _ => 0,
            }
        }
        None => 0,
    };
    let stage = infer_growth_stage(growth_days);

    // 2. 知识库检索
    let knowledge = sqlx::query(
        "SELECT name, medication, treatment, severity FROM pest_knowledge WHERE name LIKE ? ORDER BY confidence DESC LIMIT 1",
    )
    .bind(format!("%{}%", req.target_pest))
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten()
    .map(|r| serde_json::json!({
        "name": r.try_get::<String, _>("name").unwrap_or_default(),
        "medication": r.try_get::<Option<String>, _>("medication").unwrap_or(None).unwrap_or_default(),
        "treatment": r.try_get::<Option<String>, _>("treatment").unwrap_or(None).unwrap_or_default(),
        "severity": r.try_get::<Option<String>, _>("severity").unwrap_or(None).unwrap_or_default(),
    }));

    // 3. 预设检索
    let preset = sqlx::query(
        "SELECT name, items, dilution, dosage_per_unit, water_volume, safety_interval_days, note FROM mixing_presets WHERE mix_type = 'pesticide' AND name LIKE ? ORDER BY created_at DESC LIMIT 1",
    )
    .bind(format!("%{}%", req.target_pest))
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten()
    .map(|r| serde_json::json!({
        "name": r.try_get::<String, _>("name").unwrap_or_default(),
        "items": r.try_get::<Option<String>, _>("items").unwrap_or(None).and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()).unwrap_or(json!([])),
        "dilution": r.try_get::<Option<String>, _>("dilution").unwrap_or(None).unwrap_or_default(),
        "dosage_per_unit": r.try_get::<Option<String>, _>("dosage_per_unit").unwrap_or(None).unwrap_or_default(),
        "water_volume": r.try_get::<Option<String>, _>("water_volume").unwrap_or(None).unwrap_or_default(),
        "safety_interval_days": r.try_get::<i64, _>("safety_interval_days").unwrap_or(0),
        "note": r.try_get::<Option<String>, _>("note").unwrap_or(None).unwrap_or_default(),
    }));

    let plan = build_pesticide_plan(&req.target_pest, Some(&stage), knowledge.as_ref(), preset.as_ref());

    // 4. 保存
    let recipe_id = Uuid::new_v4().to_string();
    let input = json!({"area_id": req.area_id, "crop_batch_id": req.crop_batch_id, "target_pest": req.target_pest});
    let _ = sqlx::query(
        "INSERT INTO mixing_recipes (id, area_id, crop_batch_id, mix_type, stage_key, growth_days, input, result, status, created_at) VALUES (?, ?, ?, 'pesticide', ?, ?, ?, ?, 'generated', ?)",
    )
    .bind(&recipe_id)
    .bind(&req.area_id)
    .bind(&req.crop_batch_id)
    .bind(stage.key())
    .bind(growth_days)
    .bind(input.to_string())
    .bind(plan.to_string())
    .bind(now)
    .execute(&state.pool)
    .await;

    (StatusCode::OK, Json(plan)).into_response()
}

// ==================== 配方历史与应用 ====================

fn recipe_to_json(row: &sqlx::sqlite::SqliteRow) -> serde_json::Value {
    serde_json::json!({
        "id": row.try_get::<String, _>("id").unwrap_or_default(),
        "area_id": row.try_get::<String, _>("area_id").unwrap_or_default(),
        "area_name": row.try_get::<Option<String>, _>("area_name").unwrap_or(None).unwrap_or_default(),
        "crop_batch_id": row.try_get::<Option<String>, _>("crop_batch_id").unwrap_or(None).unwrap_or_default(),
        "mix_type": row.try_get::<String, _>("mix_type").unwrap_or_default(),
        "stage_key": row.try_get::<String, _>("stage_key").unwrap_or_default(),
        "growth_days": row.try_get::<i64, _>("growth_days").unwrap_or(0),
        "result": row.try_get::<Option<String>, _>("result").unwrap_or(None).and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()).unwrap_or(json!({})),
        "status": row.try_get::<String, _>("status").unwrap_or_default(),
        "farm_op_id": row.try_get::<String, _>("farm_op_id").unwrap_or_default(),
        "created_at": row.try_get::<i64, _>("created_at").unwrap_or(0),
    })
}

#[derive(Debug, Deserialize)]
pub struct ListRecipesQuery {
    pub area_id: Option<String>,
    pub mix_type: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

async fn list_recipes(
    State(state): State<AppState>,
    Query(q): Query<ListRecipesQuery>,
) -> impl IntoResponse {
    let page = q.page.unwrap_or(1).max(1);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * limit;

    let mut sql = String::from(
        "SELECT r.*, a.name AS area_name FROM mixing_recipes r LEFT JOIN areas a ON a.id = r.area_id WHERE 1=1"
    );
    let mut binds: Vec<String> = Vec::new();
    if let Some(ref area_id) = q.area_id {
        sql.push_str(" AND r.area_id = ?");
        binds.push(area_id.clone());
    }
    if let Some(ref mix_type) = q.mix_type {
        sql.push_str(" AND r.mix_type = ?");
        binds.push(mix_type.clone());
    }
    sql.push_str(" ORDER BY r.created_at DESC LIMIT ? OFFSET ?");
    binds.push(limit.to_string());
    binds.push(offset.to_string());

    let mut query = sqlx::query(&sql);
    for b in &binds {
        query = query.bind(b);
    }

    match query.fetch_all(&state.pool).await {
        Ok(rows) => {
            let recipes: Vec<serde_json::Value> = rows.iter().map(recipe_to_json).collect();
            Json(serde_json::json!({"recipes": recipes, "page": page, "limit": limit})).into_response()
        }
        Err(e) => response::internal_err(e),
    }
}

async fn apply_recipe(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return response::internal_err(e),
    };

    let recipe = match sqlx::query("SELECT * FROM mixing_recipes WHERE id = ?")
        .bind(&id)
        .fetch_optional(&mut *tx)
        .await
    {
        Ok(Some(r)) => r,
        Ok(None) => {
            let _ = tx.rollback().await;
            return response::not_found(Some("Recipe not found"));
        }
        Err(e) => {
            let _ = tx.rollback().await;
            return response::internal_err(e);
        }
    };

    let status: String = recipe.try_get("status").unwrap_or_default();
    if status == "applied" {
        let _ = tx.rollback().await;
        return response::bad_request("Recipe already applied");
    }

    let area_id: String = recipe.try_get("area_id").unwrap_or_default();
    let mix_type: String = recipe.try_get("mix_type").unwrap_or_default();
    let result_str: Option<String> = recipe.try_get("result").unwrap_or(None);
    let result = result_str
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .unwrap_or(json!({}));
    let now = Utc::now().timestamp();
    let mut warnings: Vec<String> = Vec::new();

    // 1. 创建农事日志（planned 状态）
    let op_id = Uuid::new_v4().to_string();
    let category = if mix_type == "fertilizer" { "施肥" } else { "打药" };
    let npk = result.pointer("/plan/items/0");
    let content = if mix_type == "fertilizer" {
        let item = npk.and_then(|i| i.get("name")).and_then(|n| n.as_str()).unwrap_or("");
        format!("【系统生成】{} 配方：{}，亩用量约 {:.1}{}，稀释比例 {}", 
            result.get("stage").and_then(|s| s.as_str()).unwrap_or(""),
            item,
            npk.and_then(|i| i.get("amount")).and_then(|a| a.as_f64()).unwrap_or(0.0),
            npk.and_then(|i| i.get("unit")).and_then(|u| u.as_str()).unwrap_or("kg/亩"),
            result.pointer("/plan/dilution").and_then(|d| d.as_str()).unwrap_or(""),
        )
    } else {
        format!("【系统生成】防治{}：{}", 
            result.get("target_pest").and_then(|s| s.as_str()).unwrap_or(""),
            result.pointer("/plan/items/0/name").and_then(|n| n.as_str()).unwrap_or(""),
        )
    };
    let details = json!({"recipe_id": id, "result": result});

    let op_sql = sqlx::query(
        "INSERT INTO farm_operations (id, area_id, log_date, log_time, category, content, operator, status, weather, crop_status, notes, details, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, '系统', 'planned', '', '', '', ?, ?, ?)",
    )
    .bind(&op_id)
    .bind(&area_id)
    .bind(Utc::now().format("%Y-%m-%d").to_string())
    .bind(Utc::now().format("%H:%M").to_string())
    .bind(category)
    .bind(&content)
    .bind(details.to_string())
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await;

    if let Err(e) = op_sql {
        let _ = tx.rollback().await;
        return response::internal_err(e);
    }

    // 2. 按名称匹配库存并出库
    if let Some(items) = result.pointer("/plan/items").and_then(|i| i.as_array()) {
        for item in items {
            let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("");
            if name.is_empty() {
                continue;
            }
            let amount = item.get("amount").and_then(|a| a.as_f64()).unwrap_or(0.0);
            if amount <= 0.0 {
                continue;
            }
            let inv = sqlx::query("SELECT id, stock FROM inventory_items WHERE name = ? ORDER BY created_at DESC LIMIT 1")
                .bind(name)
                .fetch_optional(&mut *tx)
                .await
                .ok()
                .flatten();
            if let Some(inv) = inv {
                let inv_id: String = inv.try_get("id").unwrap_or_default();
                let stock: f64 = inv.try_get("stock").unwrap_or(0.0);
                let out_qty = if stock >= amount { amount } else {
                    warnings.push(format!("「{}」库存不足（剩 {:.1}），已按实际出库", name, stock));
                    stock
                };
                if out_qty > 0.0 {
                    let _ = sqlx::query(
                        "INSERT INTO inventory_transactions (id, item_id, txn_type, quantity, operator, related_type, related_id, note, created_at) VALUES (?, ?, 'out', ?, '系统', 'mixing', ?, ?, ?)",
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind(&inv_id)
                    .bind(out_qty)
                    .bind(&id)
                    .bind("配肥配药自动扣减")
                    .bind(now)
                    .execute(&mut *tx)
                    .await;
                    let _ = sqlx::query("UPDATE inventory_items SET stock = stock - ?, updated_at = ? WHERE id = ?")
                        .bind(out_qty)
                        .bind(now)
                        .bind(&inv_id)
                        .execute(&mut *tx)
                        .await;
                }
            } else {
                warnings.push(format!("库存中未找到「{}」，未扣减", name));
            }
        }
    }

    // 3. 更新配方状态
    let update = sqlx::query("UPDATE mixing_recipes SET status = 'applied', farm_op_id = ?, warnings = ? WHERE id = ?")
        .bind(&op_id)
        .bind(serde_json::to_string(&warnings).unwrap_or_default())
        .bind(&id)
        .execute(&mut *tx)
        .await;
    if let Err(e) = update {
        let _ = tx.rollback().await;
        return response::internal_err(e);
    }

    match tx.commit().await {
        Ok(_) => Json(json!({"message": "Recipe applied", "farm_op_id": op_id, "warnings": warnings})).into_response(),
        Err(e) => response::internal_err(e),
    }
}

// ==================== Presets ====================

fn preset_to_json(row: &sqlx::sqlite::SqliteRow) -> serde_json::Value {
    serde_json::json!({
        "id": row.try_get::<String, _>("id").unwrap_or_default(),
        "crop_id": row.try_get::<Option<String>, _>("crop_id").unwrap_or(None).unwrap_or_default(),
        "mix_type": row.try_get::<String, _>("mix_type").unwrap_or_default(),
        "stage_key": row.try_get::<String, _>("stage_key").unwrap_or_default(),
        "name": row.try_get::<String, _>("name").unwrap_or_default(),
        "items": row.try_get::<Option<String>, _>("items").unwrap_or(None).and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()).unwrap_or(json!([])),
        "dilution": row.try_get::<String, _>("dilution").unwrap_or_default(),
        "dosage_per_unit": row.try_get::<String, _>("dosage_per_unit").unwrap_or_default(),
        "water_volume": row.try_get::<String, _>("water_volume").unwrap_or_default(),
        "ec_target": row.try_get::<Option<f64>, _>("ec_target").unwrap_or(None).or_else(|| row.try_get::<f64, _>("ec_target").ok()),
        "safety_interval_days": row.try_get::<i64, _>("safety_interval_days").unwrap_or(0),
        "note": row.try_get::<String, _>("note").unwrap_or_default(),
        "created_at": row.try_get::<i64, _>("created_at").unwrap_or(0),
    })
}

async fn list_presets(
    State(state): State<AppState>,
    Query(q): Query<serde_json::Value>,
) -> impl IntoResponse {
    let mix_type = q.get("mix_type").and_then(|v| v.as_str()).unwrap_or("");

    let mut sql = String::from("SELECT * FROM mixing_presets WHERE 1=1");
    let mut binds: Vec<String> = Vec::new();
    if !mix_type.is_empty() {
        sql.push_str(" AND mix_type = ?");
        binds.push(mix_type.to_string());
    }
    sql.push_str(" ORDER BY created_at DESC");

    let mut query = sqlx::query(&sql);
    for b in &binds {
        query = query.bind(b);
    }

    match query.fetch_all(&state.pool).await {
        Ok(rows) => {
            let presets: Vec<serde_json::Value> = rows.iter().map(preset_to_json).collect();
            Json(json!({"presets": presets})).into_response()
        }
        Err(e) => response::internal_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreatePresetRequest {
    pub crop_id: Option<String>,
    pub mix_type: String,
    pub stage_key: Option<String>,
    pub name: String,
    pub items: Option<serde_json::Value>,
    pub dilution: Option<String>,
    pub dosage_per_unit: Option<String>,
    pub water_volume: Option<String>,
    pub ec_target: Option<f64>,
    pub safety_interval_days: Option<i64>,
    pub note: Option<String>,
}

async fn create_preset(
    State(state): State<AppState>,
    Json(req): Json<CreatePresetRequest>,
) -> impl IntoResponse {
    if !["fertilizer", "pesticide"].contains(&req.mix_type.as_str()) {
        return response::bad_request("Invalid mix_type, must be: fertilizer, pesticide");
    }
    if req.name.trim().is_empty() {
        return response::bad_request("Name is required");
    }
    let id = Uuid::new_v4();
    let now = Utc::now().timestamp();
    let items = serde_json::to_string(&req.items.unwrap_or(json!([]))).unwrap_or_default();

    let result = sqlx::query(
        "INSERT INTO mixing_presets (id, crop_id, mix_type, stage_key, name, items, dilution, dosage_per_unit, water_volume, ec_target, safety_interval_days, note, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(&req.crop_id)
    .bind(&req.mix_type)
    .bind(req.stage_key.as_deref().unwrap_or(""))
    .bind(&req.name)
    .bind(&items)
    .bind(req.dilution.as_deref().unwrap_or(""))
    .bind(req.dosage_per_unit.as_deref().unwrap_or(""))
    .bind(req.water_volume.as_deref().unwrap_or(""))
    .bind(req.ec_target)
    .bind(req.safety_interval_days.unwrap_or(7))
    .bind(req.note.as_deref().unwrap_or(""))
    .bind(now)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => (StatusCode::CREATED, Json(json!({"id": id.to_string(), "message": "Preset created"}))).into_response(),
        Err(e) => response::internal_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdatePresetRequest {
    pub crop_id: Option<String>,
    pub mix_type: Option<String>,
    pub stage_key: Option<String>,
    pub name: Option<String>,
    pub items: Option<serde_json::Value>,
    pub dilution: Option<String>,
    pub dosage_per_unit: Option<String>,
    pub water_volume: Option<String>,
    pub ec_target: Option<f64>,
    pub safety_interval_days: Option<i64>,
    pub note: Option<String>,
}

async fn update_preset(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdatePresetRequest>,
) -> impl IntoResponse {
    let now = Utc::now().timestamp();
    let items = req.items.map(|v| serde_json::to_string(&v).unwrap_or_default());

    let result = sqlx::query(
        "UPDATE mixing_presets SET crop_id = COALESCE(?, crop_id), mix_type = COALESCE(?, mix_type), stage_key = COALESCE(?, stage_key), name = COALESCE(?, name), items = COALESCE(?, items), dilution = COALESCE(?, dilution), dosage_per_unit = COALESCE(?, dosage_per_unit), water_volume = COALESCE(?, water_volume), ec_target = COALESCE(?, ec_target), safety_interval_days = COALESCE(?, safety_interval_days), note = COALESCE(?, note) WHERE id = ?",
    )
    .bind(&req.crop_id)
    .bind(&req.mix_type)
    .bind(&req.stage_key)
    .bind(&req.name)
    .bind(&items)
    .bind(&req.dilution)
    .bind(&req.dosage_per_unit)
    .bind(&req.water_volume)
    .bind(req.ec_target)
    .bind(req.safety_interval_days)
    .bind(&req.note)
    .bind(&id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Json(json!({"message": "Preset updated"})).into_response(),
        Err(e) => response::internal_err(e),
    }
}

async fn delete_preset(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM mixing_presets WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(_) => Json(json!({"message": "Preset deleted"})).into_response(),
        Err(e) => response::internal_err(e),
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
            concat!(env!("CARGO_MANIFEST_DIR"), "/../agri-core/migrations/002_ai_knowledge.sql"),
            concat!(env!("CARGO_MANIFEST_DIR"), "/../agri-core/migrations/009_farm_operations.sql"),
            concat!(env!("CARGO_MANIFEST_DIR"), "/../agri-core/migrations/012_inventory.sql"),
            concat!(env!("CARGO_MANIFEST_DIR"), "/../agri-core/migrations/014_mixing.sql"),
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

    #[test]
    fn growth_stage_inference() {
        assert_eq!(infer_growth_stage(3), GrowthStage::Seedling);
        assert_eq!(infer_growth_stage(15), GrowthStage::Vegetative);
        assert_eq!(infer_growth_stage(39), GrowthStage::Vegetative);
        assert_eq!(infer_growth_stage(40), GrowthStage::Flowering);
        assert_eq!(infer_growth_stage(60), GrowthStage::Fruiting);
        assert_eq!(infer_growth_stage(120), GrowthStage::Fruiting);
    }

    #[test]
    fn ec_adjustment_rules() {
        let (f, note) = ec_adjustment(Some(3.0), Some(2.0));
        assert_eq!(f, 0.8);
        assert!(!note.is_empty());
        let (f, _) = ec_adjustment(Some(1.0), Some(2.0));
        assert_eq!(f, 1.2);
        let (f, _) = ec_adjustment(Some(1.9), Some(2.0));
        assert_eq!(f, 1.0);
        let (f, _) = ec_adjustment(None, Some(2.0));
        assert_eq!(f, 1.0);
        let (f, _) = ec_adjustment(None, None);
        assert_eq!(f, 1.0);
        let (f, _) = ec_adjustment(Some(0.0), None);
        assert_eq!(f, 1.0);
        let (f, _) = ec_adjustment(Some(0.0), Some(2.0));
        assert_eq!(f, 1.0);
    }

    #[tokio::test]
    async fn fetch_soil_context_no_sensors_returns_none() {
        // 无传感器/无作物配置的区域：EC 目标与采集值都必须是 None，绝不能是 Some(0.0)
        let pool = setup_db().await;
        let state = test_state(pool);
        let (ec_target, current_ec, soil_temp) = fetch_soil_context(&state, "none", "").await;
        assert_eq!(ec_target, None);
        assert_eq!(current_ec, None);
        assert_eq!(soil_temp, None);
    }

    #[test]
    fn fertilizer_plan_has_stage_formula() {
        let plan = build_fertilizer_plan(GrowthStage::Fruiting, 65, None, None, None);
        assert_eq!(plan["stage"], "结果期");
        let item = &plan["plan"]["items"][0];
        assert_eq!(item["k"], 35);
        assert!(item["amount"].as_f64().unwrap() > 0.0);
    }

    #[tokio::test]
    async fn fertilizer_recommend_endpoint() {
        let pool = setup_db().await;
        let app = create_router(test_state(pool)).into_service();

        let res = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/mixing/fertilizer/recommend")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"area_id":"a1","growth_days":20}"#))
                .unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let plan: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(plan["stage"], "营养生长期");
        assert_eq!(plan["plan"]["items"][0]["n"], 30);

        // recipe saved
        let res = app.clone().oneshot(
            Request::builder().method("GET").uri("/api/v1/mixing/recipes").body(Body::empty()).unwrap(),
        ).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(list["recipes"].as_array().unwrap().len(), 1);
        assert_eq!(list["recipes"][0]["mix_type"], "fertilizer");
    }

    #[tokio::test]
    async fn pesticide_recommend_with_knowledge_and_fallback() {
        let pool = setup_db().await;
        let app = create_router(test_state(pool.clone())).into_service();

        // 知识库命中 → source=knowledge
        pool.execute("INSERT INTO pest_knowledge (id, name, crop_types, medication, treatment, severity, confidence, created_at) VALUES ('p1', '白粉虱', '番茄', '25%噻嗪酮乳油 2000倍', '黄板诱杀+药剂防治', 'medium', 0.9, 0)").await.unwrap();
        let res = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/mixing/pesticide/recommend")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"area_id":"a1","target_pest":"白粉虱"}"#))
                .unwrap(),
        ).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let plan: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(plan["source"], "knowledge");
        assert_eq!(plan["plan"]["items"][0]["name"], "25%噻嗪酮乳油 2000倍");

        // 未命中 → source=fallback + warning
        let res = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/mixing/pesticide/recommend")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"area_id":"a1","target_pest":"未知病害X"}"#))
                .unwrap(),
        ).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let plan: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(plan["source"], "fallback");
        assert!(!plan["adjustments"].as_array().unwrap().is_empty());

        // 空 pest 拒绝
        let res = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/mixing/pesticide/recommend")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"area_id":"a1","target_pest":""}"#))
                .unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn apply_recipe_creates_op_and_deducts_inventory() {
        let pool = setup_db().await;
        let app = create_router(test_state(pool.clone())).into_service();

        pool.execute("INSERT INTO areas (id, name, created_at) VALUES ('a1', '主大棚', 0)").await.unwrap();
        pool.execute("INSERT INTO inventory_items (id, name, category, unit, price, stock, created_at, updated_at) VALUES ('i1', '高氮型水溶肥(30-10-20)', 'fertilizer', 'kg', 3.0, 50, 0, 0)").await.unwrap();

        // 生成配方（营养生长期，命中库存同名肥料）
        let res = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/mixing/fertilizer/recommend")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"area_id":"a1","growth_days":20}"#))
                .unwrap(),
        ).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let plan: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(plan["plan"]["items"][0]["name"], "高氮型水溶肥(30-10-20)");

        let recipe_id = {
            let res = app.clone().oneshot(
                Request::builder().method("GET").uri("/api/v1/mixing/recipes").body(Body::empty()).unwrap(),
            ).await.unwrap();
            let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
            let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
            list["recipes"][0]["id"].as_str().unwrap().to_string()
        };

        // 应用配方
        let res = app.clone().oneshot(
            Request::builder().method("POST").uri(format!("/api/v1/mixing/recipes/{}/apply", recipe_id)).body(Body::empty()).unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let applied: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let op_id = applied["farm_op_id"].as_str().unwrap().to_string();

        // farm_operation created (planned)
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM farm_operations WHERE id = ?")
            .bind(&op_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.0, 1);

        // stock deducted: 50 - 8 = 42
        let row: (f64,) = sqlx::query_as("SELECT stock FROM inventory_items WHERE id = 'i1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.0, 42.0);

        // double apply rejected
        let res = app.clone().oneshot(
            Request::builder().method("POST").uri(format!("/api/v1/mixing/recipes/{}/apply", recipe_id)).body(Body::empty()).unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn presets_crud() {
        let pool = setup_db().await;
        let app = create_router(test_state(pool)).into_service();

        let res = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/mixing/presets")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"mix_type":"pesticide","name":"白粉虱预案","items":[{"name":"25%噻嗪酮乳油","dilution":"2000倍"}],"safety_interval_days":7}"#))
                .unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let res = app.clone().oneshot(
            Request::builder().method("GET").uri("/api/v1/mixing/presets?mix_type=pesticide").body(Body::empty()).unwrap(),
        ).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(list["presets"].as_array().unwrap().len(), 1);
        assert_eq!(list["presets"][0]["safety_interval_days"], 7);

        // invalid mix_type rejected
        let res = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/mixing/presets")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"mix_type":"bad","name":"x"}"#))
                .unwrap(),
        ).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}