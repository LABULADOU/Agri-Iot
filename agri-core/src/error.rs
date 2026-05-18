use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Rule not found: {0}")]
    RuleNotFound(String),

    #[error("MQTT error: {0}")]
    Mqtt(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn status_code(&self) -> u16 {
        match self {
            AppError::DeviceNotFound(_) | AppError::RuleNotFound(_) => 404,
            AppError::InvalidInput(_) => 400,
            _ => 500,
        }
    }

    pub fn as_response(&self) -> serde_json::Value {
        let code = match self {
            AppError::DeviceNotFound(_) => "DEVICE_NOT_FOUND",
            AppError::RuleNotFound(_) => "RULE_NOT_FOUND",
            AppError::InvalidInput(_) => "INVALID_INPUT",
            _ => "INTERNAL_ERROR",
        };
        serde_json::json!({
            "code": code,
            "message": self.to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 DeviceNotFound 错误状态码
    #[test]
    fn test_device_not_found_status_code() {
        let err = AppError::DeviceNotFound("device-123".to_string());
        assert_eq!(err.status_code(), 404);
    }

    /// 测试 RuleNotFound 错误状态码
    #[test]
    fn test_rule_not_found_status_code() {
        let err = AppError::RuleNotFound("rule-456".to_string());
        assert_eq!(err.status_code(), 404);
    }

    /// 测试 InvalidInput 错误状态码
    #[test]
    fn test_invalid_input_status_code() {
        let err = AppError::InvalidInput("bad input".to_string());
        assert_eq!(err.status_code(), 400);
    }

    /// 测试 AppError Display 格式化
    #[test]
    fn test_app_error_display() {
        assert_eq!(
            AppError::DeviceNotFound("dev-1".into()).to_string(),
            "Device not found: dev-1"
        );
        assert_eq!(
            AppError::InvalidInput("bad".into()).to_string(),
            "Invalid input: bad"
        );
        assert_eq!(
            AppError::RuleNotFound("rule-1".into()).to_string(),
            "Rule not found: rule-1"
        );
        assert_eq!(
            AppError::Mqtt("timeout".into()).to_string(),
            "MQTT error: timeout"
        );
        assert_eq!(
            AppError::Internal("oops".into()).to_string(),
            "Internal error: oops"
        );
    }

    /// 测试 DeviceNotFound 错误响应格式
    #[test]
    fn test_device_not_found_response() {
        let err = AppError::DeviceNotFound("device-123".to_string());
        let response = err.as_response();
        assert_eq!(response["code"], "DEVICE_NOT_FOUND");
        assert!(response["message"].as_str().unwrap().contains("device-123"));
    }

    /// 测试 RuleNotFound 错误响应格式
    #[test]
    fn test_rule_not_found_response() {
        let err = AppError::RuleNotFound("rule-456".to_string());
        let response = err.as_response();
        assert_eq!(response["code"], "RULE_NOT_FOUND");
        assert!(response["message"].as_str().unwrap().contains("rule-456"));
    }

    /// 测试 InvalidInput 错误响应格式
    #[test]
    fn test_invalid_input_response() {
        let err = AppError::InvalidInput("Invalid device type".to_string());
        let response = err.as_response();
        assert_eq!(response["code"], "INVALID_INPUT");
        assert!(response["message"].as_str().unwrap().contains("Invalid device type"));
    }

    /// 测试 Internal 错误响应格式
    #[test]
    fn test_internal_error_response() {
        let err = AppError::Internal("Something went wrong".to_string());
        let response = err.as_response();
        assert_eq!(response["code"], "INTERNAL_ERROR");
        assert!(response["message"].as_str().unwrap().contains("Something went wrong"));
    }

    /// 测试 Mqtt 错误响应格式
    #[test]
    fn test_mqtt_error_response() {
        let err = AppError::Mqtt("Connection failed".to_string());
        let response = err.as_response();
        assert_eq!(response["code"], "INTERNAL_ERROR");
        assert!(response["message"].as_str().unwrap().contains("Connection failed"));
    }
}
