use chrono::{DateTime, Utc};
use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type, Decode};
use sqlx::sqlite::{SqliteTypeInfo, SqliteValueRef};
use std::fmt;
use std::ops::Deref;
use uuid::Uuid;

/// UUID 包装，用于 sqlx TEXT → Uuid 转换（SQLite 存 TEXT 而非 BLOB）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UuidText(pub Uuid);

impl Type<sqlx::Sqlite> for UuidText {
    fn type_info() -> SqliteTypeInfo { <&str as Type<sqlx::Sqlite>>::type_info() }
}
impl<'r> Decode<'r, sqlx::Sqlite> for UuidText {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let s = <&str as Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(UuidText(Uuid::parse_str(s).map_err(|e| format!("UUID parse: {}", e))?))
    }
}
impl Deref for UuidText {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl From<Uuid> for UuidText {
    fn from(u: Uuid) -> Self { UuidText(u) }
}
impl From<UuidText> for Uuid {
    fn from(u: UuidText) -> Self { u.0 }
}
impl fmt::Display for UuidText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Device {
    pub id: UuidText,
    pub name: String,
    pub node_id: String,
    pub device_type: DeviceType,
    pub status: DeviceStatus,
    pub config: Option<JsonValue>,
    pub area_id: Option<UuidText>,
    pub comfort_config: Option<JsonValue>,
    pub capabilities: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Device {
    /// 检查设备是否支持给定 capability
    pub fn has_capability(&self, cap: &str) -> bool {
        self.capabilities.as_ref().and_then(|c| c.0.as_array()).map_or(false, |arr| {
            arr.iter().any(|v| v.as_str() == Some(cap))
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    Sensor,
    Actuator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceStatus {
    Online,
    Offline,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SensorReading {
    pub id: i64,
    pub device_id: UuidText,
    pub metric: String,
    pub value: f64,
    pub unit: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Area {
    pub id: UuidText,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Crop {
    pub id: UuidText,
    pub name: String,
    pub comfort_config: JsonValue,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CropBatch {
    pub id: UuidText,
    pub area_id: UuidText,
    pub crop_id: UuidText,
    pub plant_date: DateTime<Utc>,
    pub expected_harvest_date: Option<DateTime<Utc>>,
    pub status: CropBatchStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CropBatchStatus {
    Active,
    Harvested,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Rule {
    pub id: UuidText,
    pub name: String,
    pub enabled: bool,
    pub trigger_type: TriggerType,
    pub conditions: JsonValue,
    pub actions: JsonValue,
    pub schedule: Option<String>,
    pub priority: i32,
    pub auto_execute: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    Schedule,
    Condition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandLog {
    pub id: i64,
    pub device_id: UuidText,
    pub command: String,
    pub payload: Option<JsonValue>,
    pub status: CommandStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Pending,
    Sent,
    Completed,
    Failed,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryPayload {
    pub metrics: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPayload {
    pub command: String,
    pub params: serde_json::Value,
}

// ========== sqlx 类型实现 ==========

fn decode_str(value: SqliteValueRef<'_>) -> Result<String, Box<dyn std::error::Error + Send + Sync + 'static>> {
    Ok(<&str as Decode<sqlx::Sqlite>>::decode(value)
        .map_err(|_| "invalid string column")?.to_string())
}

macro_rules! impl_sqlx_enum {
    ($ty:ty, $($variant:ident => $str:expr),+ $(,)?) => {
        impl Type<sqlx::Sqlite> for $ty {
            fn type_info() -> SqliteTypeInfo { <&str as Type<sqlx::Sqlite>>::type_info() }
        }
        impl<'r> Decode<'r, sqlx::Sqlite> for $ty {
            fn decode(value: SqliteValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
                let s = decode_str(value)?;
                match s.as_str() { $($str => Ok(Self::$variant),)+ _ => Err(format!("unknown {}", s).into()), }
            }
        }
    };
}

impl_sqlx_enum!(DeviceType, Sensor => "sensor", Actuator => "actuator");
impl_sqlx_enum!(DeviceStatus, Online => "online", Offline => "offline", Error => "error");
impl_sqlx_enum!(TriggerType, Schedule => "schedule", Condition => "condition");
impl_sqlx_enum!(CropBatchStatus, Active => "active", Harvested => "harvested", Failed => "failed");
impl_sqlx_enum!(CommandStatus, Pending => "pending", Sent => "sent", Completed => "completed", Failed => "failed", Timeout => "timeout");

/// JSON 值包装，用于 sqlx 的 TEXT ↔ serde_json::Value 转换
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JsonValue(pub serde_json::Value);

impl Type<sqlx::Sqlite> for JsonValue {
    fn type_info() -> SqliteTypeInfo { <&str as Type<sqlx::Sqlite>>::type_info() }
}
impl<'r> Decode<'r, sqlx::Sqlite> for JsonValue {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let s = decode_str(value)?;
        Ok(JsonValue(serde_json::from_str(&s).map_err(|e| format!("JSON decode: {}", e))?))
    }
}
impl Deref for JsonValue {
    type Target = serde_json::Value;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl From<serde_json::Value> for JsonValue {
    fn from(v: serde_json::Value) -> Self { JsonValue(v) }
}
impl From<JsonValue> for serde_json::Value {
    fn from(v: JsonValue) -> Self { v.0 }
}

/// 传感器数据计算工具函数
pub struct SensorUtils;

impl SensorUtils {
    /// 计算温度平均值
    pub fn average_temperature(readings: &[f64]) -> Option<f64> {
        if readings.is_empty() {
            return None;
        }
        let sum: f64 = readings.iter().sum();
        Some(sum / readings.len() as f64)
    }

    /// 计算湿度百分比是否在正常范围内(0-100)
    pub fn is_valid_humidity(humidity: f64) -> bool {
        humidity >= 0.0 && humidity <= 100.0
    }

    /// 检查温度是否超过阈值
    pub fn is_temperature_alert(temperature: f64, threshold: f64) -> bool {
        temperature > threshold
    }

    /// 找出最大传感器读数
    pub fn max_reading(readings: &[f64]) -> Option<f64> {
        readings.iter().copied().fold(None, |max, x| match max {
            None => Some(x),
            Some(m) if x > m => Some(x),
            _ => max,
        })
    }
}

// ========== AI 决策系统模型 ==========

/// 作物知识库
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CropProfile {
    pub id: String,
    pub name: String,
    pub variety: Option<String>,
    pub growth_stages: Option<String>,
    pub soil_temp_min: Option<f64>,
    pub soil_temp_max: Option<f64>,
    pub soil_temp_optimal: Option<f64>,
    pub soil_moisture_min: Option<f64>,
    pub soil_moisture_max: Option<f64>,
    pub soil_moisture_optimal: Option<f64>,
    pub air_temp_min: Option<f64>,
    pub air_temp_max: Option<f64>,
    pub air_temp_optimal: Option<f64>,
    pub air_humidity_min: Option<f64>,
    pub air_humidity_max: Option<f64>,
    pub air_humidity_optimal: Option<f64>,
    pub ec_min: Option<f64>,
    pub ec_max: Option<f64>,
    pub ec_optimal: Option<f64>,
    pub ventilation_preference: Option<String>,
    pub wind_sensitivity: Option<f64>,
    pub embedding_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 病虫害知识库
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PestKnowledge {
    pub id: String,
    pub name: String,
    pub crop_types: Option<String>,
    pub trigger_conditions: Option<String>,
    pub symptoms: Option<String>,
    pub severity: Option<String>,
    pub prevention: Option<String>,
    pub treatment: Option<String>,
    pub medication: Option<String>,
    pub is_emergency: i64,
    pub emergency_action: Option<String>,
    pub source: Option<String>,
    pub confidence: f64,
    pub embedding_id: Option<String>,
    pub created_at: i64,
}

/// 调控案例库
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ControlCase {
    pub id: String,
    pub area_id: Option<String>,
    pub crop_profile_id: Option<String>,
    pub situation: Option<String>,
    pub weather_forecast: Option<String>,
    pub action_taken: Option<String>,
    pub manual_override: i64,
    pub outcome: Option<String>,
    pub effect_rating: Option<i64>,
    pub health_improvement: Option<f64>,
    pub action_duration_minutes: Option<i64>,
    pub recovery_time_minutes: Option<i64>,
    pub notes: Option<String>,
    pub timestamp: i64,
    pub embedding_id: Option<String>,
}

/// 案例有效性追踪
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CaseEffectiveness {
    pub id: i64,
    pub case_id: String,
    pub assessment_time: i64,
    pub soil_temp_score: Option<f64>,
    pub soil_moisture_score: Option<f64>,
    pub pest_occurred: i64,
    pub notes: Option<String>,
}

/// 气象知识库
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WeatherKnowledge {
    pub id: String,
    pub condition_type: String,
    pub thresholds: Option<String>,
    pub protection_rules: Option<String>,
    pub time_constraints: Option<String>,
    pub contact_required: i64,
    pub contact_urgency: Option<String>,
    pub contact_message: Option<String>,
    pub notes: Option<String>,
    pub embedding_id: Option<String>,
    pub created_at: i64,
}

/// 大棚设备配置
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GreenhouseConfig {
    pub id: String,
    pub area_id: String,
    pub top_vent_min_percent: f64,
    pub top_vent_max_percent: f64,
    pub top_vent_current_percent: f64,
    pub top_vent_device_id: Option<String>,
    pub side_vent_min_percent: f64,
    pub side_vent_max_percent: f64,
    pub side_vent_current_percent: f64,
    pub side_vent_device_id: Option<String>,
    pub irrigation_device_id: Option<String>,
    pub fertigation_device_id: Option<String>,
    pub emergency_contact_name: Option<String>,
    pub emergency_contact_phone: Option<String>,
    pub top_vent_calibrated: i64,
    pub side_vent_calibrated: i64,
    pub calibration_date: Option<i64>,
    pub updated_at: i64,
}

/// 传感器配置
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SensorConfig {
    pub id: String,
    pub area_id: String,
    pub sensor_type: String,
    pub device_id: Option<String>,
    pub calibration_offset: f64,
    pub is_active: i64,
    pub last_reading: Option<i64>,
    pub created_at: i64,
}

/// 气象数据
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WeatherData {
    pub id: i64,
    pub area_id: Option<String>,
    pub source: String,
    pub temperature: Option<f64>,
    pub humidity: Option<f64>,
    pub wind_speed: Option<f64>,
    pub wind_direction: Option<String>,
    pub precipitation: Option<f64>,
    pub snow_probability: Option<f64>,
    pub uv_index: Option<f64>,
    pub forecast_hour: Option<i64>,
    pub timestamp: i64,
}

/// 环境评估记录
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EnvAssessment {
    pub id: String,
    pub area_id: Option<String>,
    pub crop_profile_id: Option<String>,
    pub timestamp: i64,
    pub overall_score: Option<f64>,
    pub soil_temp_score: Option<f64>,
    pub soil_moisture_score: Option<f64>,
    pub ec_score: Option<f64>,
    pub air_temp_score: Option<f64>,
    pub air_humidity_score: Option<f64>,
    pub deviations: Option<String>,
    pub pest_risks: Option<String>,
    pub recommendations: Option<String>,
    pub weather_impact: Option<String>,
    pub is_emergency: i64,
    pub emergency_type: Option<String>,
}

/// 知识库更新日志
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KbUpdateLog {
    pub id: i64,
    pub update_type: String,
    pub source: String,
    pub content_summary: Option<String>,
    pub effectiveness_score: Option<f64>,
    pub timestamp: i64,
}

// ========== 通风偏好枚举 ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VentPreference {
    High,
    Medium,
    Low,
}

impl_sqlx_enum!(VentPreference, High => "high", Medium => "medium", Low => "low");

// ========== 严重程度枚举 ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl_sqlx_enum!(Severity, Low => "low", Medium => "medium", High => "high", Critical => "critical");

// ========== 评估结果枚举 ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Success,
    Partial,
    Failed,
}

impl_sqlx_enum!(Outcome, Success => "success", Partial => "partial", Failed => "failed");

// ========== 气象条件类型枚举 ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionType {
    Wind,
    Rain,
    Snow,
    Storm,
    Heat,
    Frost,
}

impl_sqlx_enum!(ConditionType, Wind => "wind", Rain => "rain", Snow => "snow", Storm => "storm", Heat => "heat", Frost => "frost");

// ========== 传感器类型枚举 ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorType {
    SoilTemp,
    SoilMoisture,
    Ec,
    AirTemp,
    AirHumidity,
}

impl_sqlx_enum!(SensorType, SoilTemp => "soil_temp", SoilMoisture => "soil_moisture", Ec => "ec", AirTemp => "air_temp", AirHumidity => "air_humidity");

// ========== 数据来源枚举 ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeatherSource {
    Api,
    Local,
    Forecast,
}

impl_sqlx_enum!(WeatherSource, Api => "api", Local => "local", Forecast => "forecast");

// ========== 更新类型枚举 ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KbUpdateType {
    CaseAdded,
    KnowledgeCurated,
    FeedbackReceived,
}

impl_sqlx_enum!(KbUpdateType, CaseAdded => "case_added", KnowledgeCurated => "knowledge_curated", FeedbackReceived => "feedback_received");

// ========== 更新来源枚举 ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KbSource {
    Manual,
    Auto,
    AiReview,
}

impl_sqlx_enum!(KbSource, Manual => "manual", Auto => "auto", AiReview => "ai_review");

// ========== AI 模块中的辅助结构体 ==========

/// 环境评估结果（用于 API 响应）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentAssessment {
    pub overall_score: f64,
    pub soil_temp_score: f64,
    pub soil_moisture_score: f64,
    pub ec_score: f64,
    pub air_temp_score: f64,
    pub air_humidity_score: f64,
    pub deviations: Vec<Deviation>,
    pub trend: String,
    pub weather_impact: WeatherImpact,
}

/// 参数偏差
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deviation {
    pub param: String,
    pub current: f64,
    pub optimal: f64,
    pub deviation_pct: f64,
}

/// 气象影响
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherImpact {
    pub has_alert: bool,
    pub alert_type: Option<String>,
    pub impact_hours: Vec<HourlyImpact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<String>,
}

/// 逐小时影响
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyImpact {
    pub hour: i64,
    pub temp_impact: String,
    pub humidity_impact: String,
}

/// 通风决策
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VentilationDecision {
    pub target_percent: f64,
    pub estimated_duration_minutes: i64,
    pub priority: ActionPriority,
}

/// 动作优先级
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionPriority {
    Normal,
    High,
    Critical,
}

/// 动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub command: String,
    pub device_type: String,
    pub target_percent: f64,
    pub requires_confirmation: bool,
    pub is_emergency: bool,
    pub notification: Option<String>,
}

/// 紧急情况类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmergencyType {
    StrongWind,
    HeavyRain,
    Snow,
    ExtremeHeat,
    ExtremeCold,
    SystemFailure,
}

/// 气象参数类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WeatherParam {
    WindSpeed,
    Precipitation,
    Temperature,
    SnowProbability,
    Humidity,
}

impl WeatherParam {
    /// 映射到 weather_data 表的列名
    pub fn as_db_field(&self) -> &'static str {
        match self {
            WeatherParam::WindSpeed => "wind_speed",
            WeatherParam::Precipitation => "precipitation",
            WeatherParam::Temperature => "temperature",
            WeatherParam::SnowProbability => "snow_probability",
            WeatherParam::Humidity => "humidity",
        }
    }
}

/// 比较运算符
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CompareOp {
    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
}

impl CompareOp {
    pub fn as_operator_str(&self) -> &'static str {
        match self {
            CompareOp::Gt => ">",
            CompareOp::Lt => "<",
            CompareOp::Gte => ">=",
            CompareOp::Lte => "<=",
            CompareOp::Eq => "==",
        }
    }

    pub fn evaluate(&self, value: f64, threshold: f64) -> bool {
        match self {
            CompareOp::Gt => value > threshold,
            CompareOp::Lt => value < threshold,
            CompareOp::Gte => value >= threshold,
            CompareOp::Lte => value <= threshold,
            CompareOp::Eq => (value - threshold).abs() < 0.001,
        }
    }
}

/// 紧急规则触发条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCondition {
    pub weather_param: WeatherParam,
    pub operator: CompareOp,
    pub threshold: f64,
    pub duration_minutes: Option<u32>,
}

/// 紧急规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyRule {
    pub emergency_type: EmergencyType,
    pub condition: TriggerCondition,
    pub immediate_action: Action,
    pub requires_confirmation: bool,
    pub contact_required: bool,
    pub contact_urgency: String,
    pub notification_template: String,
    pub night_alert: bool,
}

/// 紧急情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emergency {
    pub emergency_type: EmergencyType,
    pub confidence: f64,
    pub message: String,
    pub triggered_at: DateTime<Utc>,
    pub pauses_auto_mode: bool,
    pub night_additional_contact: bool,
}

/// 通风类型
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VentType {
    Top,
    Side,
}

/// EC 趋势
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ECTrend {
    InsufficientData,
    Rising,
    Falling,
    Stable,
}

/// EC 趋势数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ECTrends {
    pub readings: Vec<(DateTime<Utc>, f64)>,
    pub period_hours: u32,
}

impl ECTrends {
    pub fn analyze(&self) -> ECTrend {
        if self.readings.len() < 3 {
            return ECTrend::InsufficientData;
        }
        let slope = self.calculate_slope();
        match slope {
            s if s > 0.1 => ECTrend::Rising,
            s if s < -0.1 => ECTrend::Falling,
            _ => ECTrend::Stable,
        }
    }

    fn calculate_slope(&self) -> f64 {
        let n = self.readings.len() as f64;
        let sum_x: f64 = self.readings.iter().enumerate().map(|(i, _)| i as f64).sum();
        let sum_y: f64 = self.readings.iter().map(|(_, v)| v).sum();
        let sum_xy: f64 = self.readings.iter().enumerate().map(|(i, (_, v))| i as f64 * v).sum();
        let sum_xx: f64 = self.readings.iter().enumerate().map(|(i, _)| (i as f64).powi(2)).sum();
        (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x.powi(2))
    }
}

/// EC 推荐
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ECRecommendation {
    NoAction,
    IncreaseEC { suggested_delta: f64, reason: String },
    DecreaseEC { suggested_delta: f64, reason: String },
    ManualIntervention { reason: String, urgency: String },
}

/// 夜间模式配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NightModeConfig {
    pub enabled: bool,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub enhanced_monitoring: bool,
    pub reduced_action_threshold: f64,
    pub night_contact_list: Vec<Contact>,
}

impl NightModeConfig {
    pub fn is_night_time_naive(&self, current_time: NaiveTime) -> bool {
        if self.start_time > self.end_time {
            current_time >= self.start_time || current_time <= self.end_time
        } else {
            current_time >= self.start_time && current_time <= self.end_time
        }
    }

    pub fn is_night_time(&self, now: DateTime<Utc>) -> bool {
        let local = now.with_timezone(&chrono::Local);
        self.is_night_time_naive(local.time())
    }
}

/// 联系人
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub name: String,
    pub phone: String,
    pub priority: u32,
}

/// 量程校准结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationResult {
    pub device_id: String,
    pub range: (f64, f64),
    pub calibration_date: DateTime<Utc>,
    pub verified: bool,
}

/// EC 管理器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ECManager {
    pub optimal_ec_min: f64,
    pub optimal_ec_max: f64,
    pub warning_threshold_low: f64,
    pub warning_threshold_high: f64,
}

impl ECManager {
    pub fn analyze_ec(&self, current_ec: f64, _trend: &ECTrends, _area_id: &str) -> ECRecommendation {
        match current_ec {
            x if x < self.warning_threshold_low => ECRecommendation::ManualIntervention {
                reason: format!("EC值({:.2})严重偏低，可能需要补充肥料", current_ec),
                urgency: "high".to_string(),
            },
            x if x < self.optimal_ec_min => ECRecommendation::IncreaseEC {
                suggested_delta: self.optimal_ec_min - current_ec,
                reason: "EC 略低，建议增加施肥浓度".to_string(),
            },
            x if x > self.optimal_ec_max && x < self.warning_threshold_high => ECRecommendation::DecreaseEC {
                suggested_delta: current_ec - self.optimal_ec_max,
                reason: "EC 偏高，建议降低施肥浓度或清水冲洗".to_string(),
            },
            x if x > self.warning_threshold_high => ECRecommendation::ManualIntervention {
                reason: format!("EC值({:.2})过高，可能造成盐害，请立即处理", current_ec),
                urgency: "critical".to_string(),
            },
            _ => ECRecommendation::NoAction,
        }
    }
}

/// 通风控制器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VentilationController {
    pub top_vent_range: (f64, f64),
    pub side_vent_range: (f64, f64),
    pub top_vent_current: f64,
    pub side_vent_current: f64,
}

impl VentilationController {
    pub fn calculate_target_position(
        &self,
        target_temp: f64,
        current_temp: f64,
        target_humidity: f64,
        current_humidity: f64,
        ventilation_type: VentType,
    ) -> VentilationDecision {
        let range = match ventilation_type {
            VentType::Top => self.top_vent_range,
            VentType::Side => self.side_vent_range,
        };
        let temp_score = ((current_temp - target_temp) / 10.0).clamp(-1.0, 1.0);
        let hum_score = ((current_humidity - target_humidity) / 20.0).clamp(0.0, 1.0);
        let open_percent = ((temp_score + hum_score) / 2.0 * 100.0).clamp(range.0, range.1);
        VentilationDecision {
            target_percent: open_percent,
            estimated_duration_minutes: (open_percent / 10.0 * 5.0) as i64,
            priority: if temp_score > 0.5 || hum_score > 0.7 { ActionPriority::High } else { ActionPriority::Normal },
        }
    }

    pub fn emergency_close(&self, _target: VentType) -> Action {
        Action {
            command: "CLOSE".to_string(),
            device_type: "vent".to_string(),
            target_percent: 0.0,
            requires_confirmation: false,
            is_emergency: true,
            notification: Some("紧急关闭通风口".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;
    use chrono::Utc;

    // ========== SensorUtils 测试 ==========

    /// 测试温度平均值计算 - 正常情况
    #[test]
    fn test_average_temperature_normal() {
        let readings = vec![20.0, 25.0, 30.0];
        let avg = SensorUtils::average_temperature(&readings);
        assert_eq!(avg, Some(25.0));
    }

    /// 测试温度平均值计算 - 空数组边界情况
    #[test]
    fn test_average_temperature_empty() {
        let readings: Vec<f64> = vec![];
        let avg = SensorUtils::average_temperature(&readings);
        assert_eq!(avg, None);
    }

    /// 测试温度平均值计算 - 单个值
    #[test]
    fn test_average_temperature_single() {
        let readings = vec![42.0];
        let avg = SensorUtils::average_temperature(&readings);
        assert_eq!(avg, Some(42.0));
    }

    /// 测试温度平均值计算 - 负值
    #[test]
    fn test_average_temperature_negative() {
        let readings = vec![-10.0, 0.0, 10.0];
        let avg = SensorUtils::average_temperature(&readings);
        assert_eq!(avg, Some(0.0));
    }

    /// 测试湿度有效性检查 - 正常范围内
    #[test]
    fn test_is_valid_humidity_normal() {
        assert!(SensorUtils::is_valid_humidity(50.0));
        assert!(SensorUtils::is_valid_humidity(0.0));
        assert!(SensorUtils::is_valid_humidity(100.0));
    }

    /// 测试湿度有效性检查 - 超出范围
    #[test]
    fn test_is_valid_humidity_out_of_range() {
        assert!(!SensorUtils::is_valid_humidity(-1.0));
        assert!(!SensorUtils::is_valid_humidity(101.0));
        assert!(!SensorUtils::is_valid_humidity(150.0));
    }

    /// 测试温度告警判断 - 超过阈值
    #[test]
    fn test_is_temperature_alert_exceeded() {
        assert!(SensorUtils::is_temperature_alert(35.0, 30.0));
        assert!(SensorUtils::is_temperature_alert(30.1, 30.0));
    }

    /// 测试温度告警判断 - 未超过阈值
    #[test]
    fn test_is_temperature_alert_not_exceeded() {
        assert!(!SensorUtils::is_temperature_alert(25.0, 30.0));
        assert!(!SensorUtils::is_temperature_alert(30.0, 30.0));
    }

    /// 测试最大读数查找 - 正常情况
    #[test]
    fn test_max_reading_normal() {
        let readings = vec![10.0, 50.0, 30.0, 40.0];
        let max = SensorUtils::max_reading(&readings);
        assert_eq!(max, Some(50.0));
    }

    /// 测试最大读数查找 - 空数组边界情况
    #[test]
    fn test_max_reading_empty() {
        let readings: Vec<f64> = vec![];
        let max = SensorUtils::max_reading(&readings);
        assert_eq!(max, None);
    }

    /// 测试最大读数查找 - 单个值
    #[test]
    fn test_max_reading_single() {
        let readings = vec![99.9];
        let max = SensorUtils::max_reading(&readings);
        assert_eq!(max, Some(99.9));
    }

    /// 测试最大读数查找 - 负值
    #[test]
    fn test_max_reading_negative() {
        let readings = vec![-50.0, -10.0, -30.0];
        let max = SensorUtils::max_reading(&readings);
        assert_eq!(max, Some(-10.0));
    }

    // ========== 模型序列化/反序列化测试 ==========

    /// 测试 DeviceType 枚举序列化
    #[test]
    fn test_device_type_serialization() {
        let sensor = DeviceType::Sensor;
        let json = serde_json::to_string(&sensor).unwrap();
        assert_eq!(json, "\"sensor\"");

        let actuator = DeviceType::Actuator;
        let json = serde_json::to_string(&actuator).unwrap();
        assert_eq!(json, "\"actuator\"");
    }

    /// 测试 DeviceType 枚举反序列化
    #[test]
    fn test_device_type_deserialization() {
        let sensor: DeviceType = serde_json::from_str("\"sensor\"").unwrap();
        match sensor {
            DeviceType::Sensor => (),
            _ => panic!("Expected Sensor"),
        }

        let actuator: DeviceType = serde_json::from_str("\"actuator\"").unwrap();
        match actuator {
            DeviceType::Actuator => (),
            _ => panic!("Expected Actuator"),
        }
    }

    /// 测试 DeviceStatus 枚举序列化
    #[test]
    fn test_device_status_serialization() {
        let online = DeviceStatus::Online;
        let json = serde_json::to_string(&online).unwrap();
        assert_eq!(json, "\"online\"");

        let offline = DeviceStatus::Offline;
        let json = serde_json::to_string(&offline).unwrap();
        assert_eq!(json, "\"offline\"");

        let error = DeviceStatus::Error;
        let json = serde_json::to_string(&error).unwrap();
        assert_eq!(json, "\"error\"");
    }

    /// 测试 TriggerType 枚举序列化
    #[test]
    fn test_trigger_type_serialization() {
        let schedule = TriggerType::Schedule;
        let json = serde_json::to_string(&schedule).unwrap();
        assert_eq!(json, "\"schedule\"");

        let condition = TriggerType::Condition;
        let json = serde_json::to_string(&condition).unwrap();
        assert_eq!(json, "\"condition\"");
    }

    /// 测试 TelemetryPayload 反序列化 - 合法 JSON
    #[test]
    fn test_telemetry_payload_valid() {
        let json = json!({
            "metrics": {
                "temperature": 25.5,
                "humidity": 60.0
            }
        });
        let payload: TelemetryPayload = serde_json::from_value(json).unwrap();
        assert!(payload.metrics.get("temperature").is_some());
        assert_eq!(payload.metrics["temperature"], 25.5);
    }

    /// 测试 TelemetryPayload 反序列化 - 缺少 metrics 字段（异常）
    #[test]
    fn test_telemetry_payload_missing_metrics() {
        let json = json!({
            "data": {
                "temperature": 25.5
            }
        });
        let result: Result<TelemetryPayload, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    /// 测试 CommandPayload 序列化和反序列化
    #[test]
    fn test_command_payload_serde() {
        let payload = CommandPayload {
            command: "irrigation_on".to_string(),
            params: json!({"duration": 30}),
        };
        let serialized = serde_json::to_string(&payload).unwrap();
        let deserialized: CommandPayload = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.command, "irrigation_on");
        assert_eq!(deserialized.params["duration"], 30);
    }

    /// 测试 Device 模型创建
    #[test]
    fn test_device_model_creation() {
        let device = Device {
            id: Uuid::new_v4().into(),
            name: "温度传感器".to_string(),
            node_id: "node-001".to_string(),
            device_type: DeviceType::Sensor,
            status: DeviceStatus::Online,
            config: Some(JsonValue(json!({"interval": 60}))),
            area_id: None,
            comfort_config: None,
            capabilities: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert_eq!(device.name, "温度传感器");
        assert_eq!(device.node_id, "node-001");
        match device.device_type {
            DeviceType::Sensor => (),
            _ => panic!("Expected Sensor"),
        }
    }

    /// 测试 SensorReading 模型创建
    #[test]
    fn test_sensor_reading_model_creation() {
        let reading = SensorReading {
            id: 1,
            device_id: Uuid::new_v4().into(),
            metric: "temperature".to_string(),
            value: 25.5,
            unit: "℃".to_string(),
            timestamp: Utc::now(),
        };
        assert_eq!(reading.metric, "temperature");
        assert_eq!(reading.value, 25.5);
        assert_eq!(reading.unit, "℃");
    }

    /// 测试 Rule 模型创建
    #[test]
    fn test_rule_model_creation() {
        let rule = Rule {
            id: Uuid::new_v4().into(),
            name: "温度告警".to_string(),
            enabled: true,
            trigger_type: TriggerType::Condition,
            conditions: JsonValue(json!({"conditions": [{"metric": "temperature", "operator": ">", "value": 30.0}]})),
            actions: JsonValue(json!({"actions": [{"device_id": "dev-001", "command": "alarm_on"}]})),
            schedule: None,
            priority: 1,
            auto_execute: true,
            created_at: Utc::now(),
        };
        assert_eq!(rule.name, "温度告警");
        assert!(rule.enabled);
        assert!(rule.auto_execute);
        match rule.trigger_type {
            TriggerType::Condition => (),
            _ => panic!("Expected Condition"),
        }
    }

    /// 测试 Rule 禁用状态
    #[test]
    fn test_rule_disabled() {
        let rule = Rule {
            id: Uuid::new_v4().into(),
            name: "定时浇水".to_string(),
            enabled: false,
            trigger_type: TriggerType::Schedule,
            conditions: JsonValue(json!({})),
            actions: JsonValue(json!({})),
            schedule: Some("at 08:00".to_string()),
            priority: 0,
            auto_execute: false,
            created_at: Utc::now(),
        };
        assert!(!rule.enabled);
        assert_eq!(rule.schedule, Some("at 08:00".to_string()));
    }

    /// 测试 CropBatchStatus 枚举序列化
    #[test]
    fn test_crop_batch_status_serialization() {
        let active = CropBatchStatus::Active;
        assert_eq!(serde_json::to_string(&active).unwrap(), "\"active\"");
        let harvested = CropBatchStatus::Harvested;
        assert_eq!(serde_json::to_string(&harvested).unwrap(), "\"harvested\"");
        let failed = CropBatchStatus::Failed;
        assert_eq!(serde_json::to_string(&failed).unwrap(), "\"failed\"");
    }

    /// 测试 CropBatchStatus 枚举反序列化
    #[test]
    fn test_crop_batch_status_deserialization() {
        let active: CropBatchStatus = serde_json::from_str("\"active\"").unwrap();
        match active {
            CropBatchStatus::Active => (),
            _ => panic!("Expected Active"),
        }
    }

    /// 测试 CommandStatus 枚举序列化
    #[test]
    fn test_command_status_serialization() {
        assert_eq!(serde_json::to_string(&CommandStatus::Pending).unwrap(), "\"pending\"");
        assert_eq!(serde_json::to_string(&CommandStatus::Sent).unwrap(), "\"sent\"");
        assert_eq!(serde_json::to_string(&CommandStatus::Completed).unwrap(), "\"completed\"");
        assert_eq!(serde_json::to_string(&CommandStatus::Failed).unwrap(), "\"failed\"");
        assert_eq!(serde_json::to_string(&CommandStatus::Timeout).unwrap(), "\"timeout\"");
    }

    /// 测试 CommandStatus 枚举反序列化
    #[test]
    fn test_command_status_deserialization() {
        let pending: CommandStatus = serde_json::from_str("\"pending\"").unwrap();
        match pending {
            CommandStatus::Pending => (),
            _ => panic!("Expected Pending"),
        }
    }

    /// 测试 Area 模型创建
    #[test]
    fn test_area_model_creation() {
        let area = Area {
            id: Uuid::new_v4().into(),
            name: "A区".to_string(),
            description: Some("温室A区".to_string()),
            created_at: Utc::now(),
        };
        assert_eq!(area.name, "A区");
        assert_eq!(area.description, Some("温室A区".to_string()));
    }

    /// 测试 Crop 模型创建
    #[test]
    fn test_crop_model_creation() {
        let crop = Crop {
            id: Uuid::new_v4().into(),
            name: "番茄".to_string(),
            comfort_config: JsonValue(json!({"temperature": {"min": 15, "max": 30}})),
            created_at: Utc::now(),
        };
        assert_eq!(crop.name, "番茄");
        assert_eq!(crop.comfort_config["temperature"]["min"], 15);
    }

    /// 测试 CropBatch 模型创建
    #[test]
    fn test_crop_batch_model_creation() {
        let batch = CropBatch {
            id: Uuid::new_v4().into(),
            area_id: Uuid::new_v4().into(),
            crop_id: Uuid::new_v4().into(),
            plant_date: Utc::now(),
            expected_harvest_date: None,
            status: CropBatchStatus::Active,
            created_at: Utc::now(),
        };
        match batch.status {
            CropBatchStatus::Active => (),
            _ => panic!("Expected Active"),
        }
    }

    /// 测试 CommandLog 反序列化
    #[test]
    fn test_command_log_serde() {
        let json = json!({
            "id": 1,
            "device_id": "550e8400-e29b-41d4-a716-446655440000",
            "command": "irrigation_on",
            "payload": {"duration": 30},
            "status": "pending",
            "created_at": "2026-05-09T00:00:00Z"
        });
        let log: CommandLog = serde_json::from_value(json).unwrap();
        assert_eq!(log.command, "irrigation_on");
        match log.status {
            CommandStatus::Pending => (),
            _ => panic!("Expected Pending"),
        }
    }
}
