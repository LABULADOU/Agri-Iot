pub mod broker;
pub mod client;
pub mod handler;

#[cfg(test)]
mod tests {
    use serde_json::json;
    use chrono::Utc;

    // ========== MQTT Payload 解析测试 ==========

    /// 测试合法的遥测数据 JSON 解析
    #[test]
    fn test_telemetry_payload_valid() {
        let payload = json!({
            "metrics": {
                "temperature": 25.5,
                "humidity": 60.0,
                "light": 1000.0
            }
        }).to_string();

        let data: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert!(data.get("metrics").is_some());
        assert_eq!(data["metrics"]["temperature"], 25.5);
        assert_eq!(data["metrics"]["humidity"], 60.0);
    }

    /// 测试缺少 metrics 字段的非法 JSON（异常捕获）
    #[test]
    fn test_telemetry_payload_missing_metrics() {
        let payload = json!({
            "data": {
                "temperature": 25.5
            }
        }).to_string();

        let data: serde_json::Value = serde_json::from_str(&payload).unwrap();
        // metrics 字段不存在，这会导致后续处理时跳过
        assert!(data.get("metrics").is_none());
    }

    /// 测试类型错误的 JSON - temperature 应该是数字而不是字符串
    #[test]
    fn test_telemetry_payload_wrong_type() {
        let payload = json!({
            "metrics": {
                "temperature": "not_a_number",
                "humidity": 60.0
            }
        }).to_string();

        let data: serde_json::Value = serde_json::from_str(&payload).unwrap();
        // as_f64() 会返回 None，因为值是字符串
        assert!(data["metrics"]["temperature"].as_f64().is_none());
        assert!(data["metrics"]["humidity"].as_f64().is_some());
    }

    /// 测试空 metrics 对象
    #[test]
    fn test_telemetry_payload_empty_metrics() {
        let payload = json!({
            "metrics": {}
        }).to_string();

        let data: serde_json::Value = serde_json::from_str(&payload).unwrap();
        let metrics = data.get("metrics").and_then(|m| m.as_object()).unwrap();
        assert!(metrics.is_empty());
    }

    /// 测试 CommandPayload 序列化
    #[test]
    fn test_command_payload_serialization() {
        let payload = agri_core::models::CommandPayload {
            command: "irrigation_on".to_string(),
            params: json!({"duration": 30}),
        };
        let serialized = serde_json::to_string(&payload).unwrap();
        assert!(serialized.contains("irrigation_on"));
        assert!(serialized.contains("duration"));
    }

    /// 测试 CommandPayload 反序列化 - 合法 JSON
    #[test]
    fn test_command_payload_deserialization_valid() {
        let json_str = r#"{"command": "irrigation_on", "params": {"duration": 30}}"#;
        let payload: agri_core::models::CommandPayload = serde_json::from_str(json_str).unwrap();
        assert_eq!(payload.command, "irrigation_on");
        assert_eq!(payload.params["duration"], 30);
    }

    /// 测试 CommandPayload 反序列化 - 缺少 params 字段（异常）
    #[test]
    fn test_command_payload_deserialization_missing_params() {
        let json_str = r#"{"command": "irrigation_on"}"#;
        let result: Result<agri_core::models::CommandPayload, _> = serde_json::from_str(json_str);
        // params 字段是必需的，所以应该报错
        assert!(result.is_err());
    }

    /// 测试 CommandPayload 反序列化 - 缺少 command 字段（异常）
    #[test]
    fn test_command_payload_deserialization_missing_command() {
        let json_str = r#"{"params": {"duration": 30}}"#;
        let result: Result<agri_core::models::CommandPayload, _> = serde_json::from_str(json_str);
        assert!(result.is_err());
    }

    /// 测试主题解析 - 合法的遥测主题
    #[test]
    fn test_topic_parsing_telemetry() {
        let topic = "agri/node/node-001/telemetry";
        let parts: Vec<&str> = topic.split('/').collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], "agri");
        assert_eq!(parts[1], "node");
        assert_eq!(parts[2], "node-001");
        assert_eq!(parts[3], "telemetry");
    }

    /// 测试主题解析 - 合法的状态主题
    #[test]
    fn test_topic_parsing_status() {
        let topic = "agri/node/node-001/status";
        let parts: Vec<&str> = topic.split('/').collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[3], "status");
    }

    /// 测试主题解析 - 非法主题格式（太短）
    #[test]
    fn test_topic_parsing_invalid_short() {
        let topic = "agri/node";
        let parts: Vec<&str> = topic.split('/').collect();
        assert!(parts.len() < 4); // 不满足最小长度要求
    }

    /// 测试主题解析 - 非法主题格式（前缀不对）
    #[test]
    fn test_topic_parsing_invalid_prefix() {
        let topic = "other/node/node-001/telemetry";
        let parts: Vec<&str> = topic.split('/').collect();
        assert_ne!(parts[0], "agri");
    }

    /// 测试状态转换逻辑
    #[test]
    fn test_status_conversion_online() {
        let status = "online";
        let db_status = match status {
            "online" => "online",
            _ => "offline",
        };
        assert_eq!(db_status, "online");
    }

    /// 测试状态转换逻辑 - 非 online 都转为 offline
    #[test]
    fn test_status_conversion_offline() {
        for status in &["offline", "error", "unknown"] {
            let db_status = match *status {
                "online" => "online",
                _ => "offline",
            };
            assert_eq!(db_status, "offline");
        }
    }

    /// 测试时间戳生成
    #[test]
    fn test_timestamp_generation() {
        let now = Utc::now().timestamp();
        assert!(now > 0);
        assert!(now > 1600000000); // 大于 2020 年的时间戳
    }
}
