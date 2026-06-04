use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

pub fn ok_json<T: Into<serde_json::Value>>(data: T) -> Response {
    Json(serde_json::json!(data.into())).into_response()
}

pub fn err_json(status: StatusCode, msg: impl ToString) -> Response {
    (status, Json(serde_json::json!({"error": msg.to_string()}))).into_response()
}

pub fn internal_err(_e: impl ToString) -> Response {
    // 不泄露内部错误信息给客户端
    err_json(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
}

pub fn not_found(msg: Option<&str>) -> Response {
    err_json(StatusCode::NOT_FOUND, msg.unwrap_or("Not found"))
}

pub fn bad_request(msg: &str) -> Response {
    err_json(StatusCode::BAD_REQUEST, msg)
}
