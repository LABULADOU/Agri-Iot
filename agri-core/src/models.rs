use chrono::{DateTime, Utc};
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
