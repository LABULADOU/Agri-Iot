use axum::{extract::Query, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;

const WEATHER: &str = "https://ku36x9fh3j.re.qweatherapi.com/v7/weather";
const AIR: &str = "https://ku36x9fh3j.re.qweatherapi.com/v7/air";
const INDICES: &str = "https://ku36x9fh3j.re.qweatherapi.com/v7/indices";
const WARNING: &str = "https://ku36x9fh3j.re.qweatherapi.com/v7/warning";
const GEO: &str = "https://ku36x9fh3j.re.qweatherapi.com/geo/v2/city";

#[derive(Deserialize)]
pub struct WeatherParams {
    location: String,
}

#[derive(Deserialize)]
pub struct IndicesParams {
    location: String,
    #[serde(rename = "type")]
    type_: Option<String>,
}

#[derive(Deserialize)]
pub struct GeoParams {
    location: String,
    number: Option<u32>,
}

fn api_key() -> String {
    std::env::var("WEATHER_API_KEY").unwrap_or_default()
}

async fn proxy(url: &str) -> axum::response::Response {
    let key = api_key();
    if key.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "WEATHER_API_KEY not set"}))).into_response();
    }
    match reqwest::get(url).await {
        Ok(resp) => {
            let status = resp.status();
            match resp.bytes().await {
                Ok(body) => {
                    if !status.is_success() {
                        return (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": "upstream error", "status": status.as_u16(), "raw": format!("{:?}", &body[..body.len().min(200)])}))).into_response();
                    }
                    match serde_json::from_slice::<serde_json::Value>(&body) {
                        Ok(json) => Json(json).into_response(),
                        Err(_) => (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": "parse failed", "raw": format!("{:?}", &body[..body.len().min(200)])}))).into_response(),
                    }
                }
                Err(e) => (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
            }
        }
        Err(e) => (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

/// Like proxy(), but returns 200 with empty data instead of propagating error status.
/// Used for endpoints (minutely, warning) that may not be available on free QWeather tier.
async fn safe_proxy(url: &str, empty_body: serde_json::Value) -> axum::response::Response {
    let key = api_key();
    if key.is_empty() {
        return Json(empty_body).into_response();
    }
    match reqwest::get(url).await {
        Ok(resp) => {
            let status = resp.status();
            match resp.bytes().await {
                Ok(body) => {
                    if !status.is_success() {
                        return Json(empty_body).into_response();
                    }
                    match serde_json::from_slice::<serde_json::Value>(&body) {
                        Ok(json) => Json(json).into_response(),
                        Err(_) => Json(empty_body).into_response(),
                    }
                }
                Err(_) => Json(empty_body).into_response(),
            }
        }
        Err(_) => Json(empty_body).into_response(),
    }
}

pub async fn get_weather_now(Query(params): Query<WeatherParams>) -> axum::response::Response {
    proxy(&format!("{}/now?location={}&key={}", WEATHER, params.location, api_key())).await
}

pub async fn get_forecast_3d(Query(params): Query<WeatherParams>) -> axum::response::Response {
    proxy(&format!("{}/3d?location={}&key={}", WEATHER, params.location, api_key())).await
}

pub async fn get_forecast_24h(Query(params): Query<WeatherParams>) -> axum::response::Response {
    proxy(&format!("{}/24h?location={}&key={}", WEATHER, params.location, api_key())).await
}

pub async fn get_minutely(Query(params): Query<WeatherParams>) -> axum::response::Response {
    let url = format!("{}/24h?location={}&key={}", WEATHER, params.location, api_key());
    let empty = serde_json::json!({"summary": "无降水数据", "hourly": []});
    match safe_proxy(&url, empty.clone()).await {
        resp => {
            let (_, body) = resp.into_parts();
            let body_bytes = axum::body::to_bytes(body, 1024 * 16).await.unwrap_or_default();
            if let Ok(mut json) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
                if let Some(hourly) = json["hourly"].as_array() {
                    let next = hourly.iter().take(6).map(|h| {
                        serde_json::json!({
                            "time": h["fxTime"],
                            "text": h["text"],
                            "temp": h["temp"],
                            "precip": h["precip"],
                            "pop": h["pop"],
                        })
                    }).collect::<Vec<_>>();
                    let has_rain = next.iter().any(|h| h["pop"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) > 30.0);
                    let summary = if has_rain { "未来数小时有降水" } else { "未来数小时无降水" };
                    json["summary"] = serde_json::json!(summary);
                    json["hourly"] = serde_json::json!(next);
                    return Json(json).into_response();
                }
            }
            Json(empty).into_response()
        }
    }
}

pub async fn get_air_now(Query(params): Query<WeatherParams>) -> axum::response::Response {
    proxy(&format!("{}/now?location={}&key={}", AIR, params.location, api_key())).await
}

pub async fn get_indices(Query(params): Query<IndicesParams>) -> axum::response::Response {
    let types = params.type_.unwrap_or_else(|| "1,2,3,4,5,6,7,8,9".to_string());
    proxy(&format!("{}/1d?type={}&location={}&key={}", INDICES, types, params.location, api_key())).await
}

pub async fn get_warning(Query(params): Query<WeatherParams>) -> axum::response::Response {
    let url = format!("{}/now?location={}&key={}", WARNING, params.location, api_key());
    let empty = serde_json::json!({"warning": []});
    safe_proxy(&url, empty).await
}

pub async fn geo_lookup(Query(params): Query<GeoParams>) -> axum::response::Response {
    let num = params.number.unwrap_or(20);
    proxy(&format!("{}/lookup?location={}&number={}&key={}", GEO, params.location, num, api_key())).await
}
