use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub location: String,
    pub crop_type: String,
    pub comfort_config: ComfortConfig,
    pub node_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComfortConfig {
    pub air_temp: ValueRange,
    pub air_humidity: ValueRange,
    pub soil_temp: ValueRange,
    pub soil_moisture: ValueRange,
    pub ec_value: ValueRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueRange {
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorNode {
    pub id: Uuid,
    pub name: String,
    pub zone_id: Uuid,
    pub has_irrigation: bool,
    pub has_side_vent: bool,
    pub has_roof_vent: bool,
    pub vent_range: ValueRange,
    pub status: DeviceStatus,
    pub last_seen: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedReading {
    pub timestamp: DateTime<Utc>,
    pub metric: String,
    pub node_id: String,
    pub max: f64,
    pub min: f64,
    pub avg: f64,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccumulatedTemp {
    pub id: Uuid,
    pub zone_id: Uuid,
    pub date: String,
    pub accumulated: f64,
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: Uuid,
    pub name: String,
    pub node_id: String,
    pub device_type: DeviceType,
    pub status: DeviceStatus,
    pub config: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    pub device_id: Uuid,
    pub metric: String,
    pub value: f64,
    pub unit: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Rule {
    pub id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub trigger_type: TriggerType,
    pub conditions: serde_json::Value,
    pub actions: serde_json::Value,
    pub schedule: Option<String>,
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
    pub device_id: Uuid,
    pub command: String,
    pub payload: Option<serde_json::Value>,
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
            id: Uuid::new_v4(),
            name: "温度传感器".to_string(),
            node_id: "node-001".to_string(),
            device_type: DeviceType::Sensor,
            status: DeviceStatus::Online,
            config: Some(json!({"interval": 60})),
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
            device_id: Uuid::new_v4(),
            metric: "temperature".to_string(),
            value: 25.5,
            unit: "℃".to_string(),
            timestamp: Utc::now(),
        };
        assert_eq!(reading.metric, "temperature");
        assert_eq!(reading.value, 25.5);
        assert_eq!(reading.unit, "℃");
    }
}
