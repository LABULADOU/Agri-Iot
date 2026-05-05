use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("MQTT error: {0}")]
    Mqtt(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn as_response(&self) -> serde_json::Value {
        match self {
            AppError::DeviceNotFound(msg) => serde_json::json!({
                "code": "DEVICE_NOT_FOUND",
                "message": msg
            }),
            AppError::InvalidInput(msg) => serde_json::json!({
                "code": "INVALID_INPUT",
                "message": msg
            }),
            _ => serde_json::json!({
                "code": "INTERNAL_ERROR",
                "message": "An internal error occurred"
            }),
        }
    }
}
