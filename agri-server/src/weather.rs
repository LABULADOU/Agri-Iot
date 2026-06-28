use axum::{extract::Query, Json};
use serde::Deserialize;
use std::sync::OnceLock;

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("reqwest::Client::new")
    })
}

#[derive(Deserialize)]
pub struct WeatherParams {
    location: String,
}

#[derive(Deserialize)]
pub struct GeoParams {
    location: String,
    number: Option<u32>,
}

fn parse_location(loc: &str) -> (f64, f64) {
    if let Some((lat_str, lon_str)) = loc.split_once(',') {
        if let (Ok(lat), Ok(lon)) = (lat_str.trim().parse::<f64>(), lon_str.trim().parse::<f64>()) {
            return (lat, lon);
        }
    }
    (39.92, 116.41) // default: Beijing
}

fn wmo_text(code: i64) -> &'static str {
    match code {
        0 => "晴",
        1 => "少云",
        2 => "多云",
        3 => "阴",
        45 | 48 => "雾",
        51 | 53 | 55 => "毛毛雨",
        56 | 57 => "冻雨",
        61 => "小雨",
        63 => "中雨",
        65 => "大雨",
        66 | 67 => "冻雨",
        71 => "小雪",
        73 => "中雪",
        75 => "大雪",
        77 => "雪粒",
        80 => "阵雨",
        81 => "阵雨",
        82 => "大阵雨",
        85 => "阵雪",
        86 => "大阵雪",
        95 => "雷暴",
        96 | 99 => "雷暴冰雹",
        _ => "未知",
    }
}

fn wmo_icon(code: i64) -> &'static str {
    match code {
        0 => "100",
        1 => "101",
        2 => "102",
        3 => "104",
        45 | 48 => "500",
        51 | 53 | 55 => "300",
        56 | 57 => "306",
        61 => "305",
        63 => "306",
        65 => "307",
        66 | 67 => "306",
        71 => "400",
        73 => "401",
        75 => "402",
        77 => "404",
        80 => "300",
        81 => "301",
        82 => "302",
        85 => "400",
        86 => "402",
        95 => "310",
        96 | 99 => "312",
        _ => "999",
    }
}

fn wind_dir(deg: f64) -> String {
    match deg as i32 {
        0..=22 => "北风",
        23..=67 => "东北风",
        68..=112 => "东风",
        113..=157 => "东南风",
        158..=202 => "南风",
        203..=247 => "西南风",
        248..=292 => "西风",
        293..=337 => "西北风",
        338..=360 => "北风",
        _ => "未知",
    }
    .to_string()
}

fn beaufort(kmh: f64) -> String {
    match kmh as i32 {
        0..=1 => "0",
        2..=5 => "1",
        6..=11 => "2",
        12..=19 => "3",
        20..=28 => "4",
        29..=38 => "5",
        39..=49 => "6",
        50..=61 => "7",
        62..=74 => "8",
        75..=88 => "9",
        89..=102 => "10",
        103..=117 => "11",
        _ => "12",
    }
    .to_string()
}

async fn openmeteo_get(url: &str) -> Result<serde_json::Value, String> {
    let resp = http_client().get(url).send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let body = resp.bytes().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("upstream {}: {:?}", status, &body[..body.len().min(200)]));
    }
    serde_json::from_slice(&body).map_err(|e| format!("parse: {}", e))
}

pub async fn get_weather_now(Query(params): Query<WeatherParams>) -> Json<serde_json::Value> {
    let (lat, lon) = parse_location(&params.location);
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,apparent_temperature,precipitation,weather_code,wind_speed_10m,wind_direction_10m,surface_pressure,visibility,uv_index",
        lat, lon
    );
    match openmeteo_get(&url).await {
        Ok(om) => {
            let c = &om["current"];
            let code = c["weather_code"].as_i64().unwrap_or(0);
            let t = c["temperature_2m"].as_f64().unwrap_or(0.0);
            let f = c["apparent_temperature"].as_f64().unwrap_or(0.0);
            let h = c["relative_humidity_2m"].as_f64().unwrap_or(0.0);
            let p = c["precipitation"].as_f64().unwrap_or(0.0);
            let ws = c["wind_speed_10m"].as_f64().unwrap_or(0.0);
            let wd = c["wind_direction_10m"].as_f64().unwrap_or(0.0);
            let pr = c["surface_pressure"].as_f64().unwrap_or(0.0);
            let v = c["visibility"].as_f64().unwrap_or(0.0);
            let time = c["time"].as_str().unwrap_or("");
            Json(serde_json::json!({"code": "200", "now": {
                "temp": format!("{:.1}", t),
                "feelsLike": format!("{:.1}", f),
                "text": wmo_text(code),
                "icon": wmo_icon(code),
                "humidity": format!("{:.0}", h),
                "windDir": wind_dir(wd),
                "windScale": beaufort(ws),
                "windSpeed": format!("{:.1}", ws),
                "precip": format!("{:.1}", p),
                "pressure": format!("{:.0}", pr),
                "vis": format!("{:.0}", v / 1000.0),
                "obsTime": time,
            }}))
        }
        Err(e) => Json(serde_json::json!({"code": "500", "error": e})),
    }
}

pub async fn get_forecast_3d(Query(params): Query<WeatherParams>) -> Json<serde_json::Value> {
    let (lat, lon) = parse_location(&params.location);
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum,precipitation_probability_max,wind_speed_10m_max,wind_direction_10m_dominant&forecast_days=3&timezone=auto",
        lat, lon
    );
    match openmeteo_get(&url).await {
        Ok(om) => {
            let d = &om["daily"];
            let times = d["time"].as_array().map(|a| a.as_slice()).unwrap_or(&[]);
            let mut days = Vec::new();
            for i in 0..times.len() {
                let code = d["weather_code"][i].as_i64().unwrap_or(0);
                let ws = d["wind_speed_10m_max"][i].as_f64().unwrap_or(0.0);
                let wd = d["wind_direction_10m_dominant"][i].as_f64().unwrap_or(0.0);
                days.push(serde_json::json!({
                    "fxDate": d["time"][i].as_str().unwrap_or(""),
                    "tempMax": format!("{:.0}", d["temperature_2m_max"][i].as_f64().unwrap_or(0.0)),
                    "tempMin": format!("{:.0}", d["temperature_2m_min"][i].as_f64().unwrap_or(0.0)),
                    "textDay": wmo_text(code),
                    "iconDay": wmo_icon(code),
                    "windDirDay": wind_dir(wd),
                    "windScaleDay": beaufort(ws),
                    "precip": format!("{:.1}", d["precipitation_sum"][i].as_f64().unwrap_or(0.0)),
                    "pop": format!("{:.0}", d["precipitation_probability_max"][i].as_f64().unwrap_or(0.0)),
                }));
            }
            Json(serde_json::json!({"code": "200", "daily": days}))
        }
        Err(e) => Json(serde_json::json!({"code": "500", "error": e})),
    }
}

pub async fn get_forecast_24h(Query(params): Query<WeatherParams>) -> Json<serde_json::Value> {
    let (lat, lon) = parse_location(&params.location);
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&hourly=temperature_2m,precipitation_probability,precipitation,weather_code,wind_speed_10m&forecast_hours=24&timezone=auto",
        lat, lon
    );
    match openmeteo_get(&url).await {
        Ok(om) => {
            let h = &om["hourly"];
            let times = h["time"].as_array().map(|a| a.as_slice()).unwrap_or(&[]);
            let mut hours = Vec::new();
            for i in 0..times.len() {
                let code = h["weather_code"][i].as_i64().unwrap_or(0);
                hours.push(serde_json::json!({
                    "fxTime": h["time"][i].as_str().unwrap_or(""),
                    "temp": format!("{:.1}", h["temperature_2m"][i].as_f64().unwrap_or(0.0)),
                    "text": wmo_text(code),
                    "icon": wmo_icon(code),
                    "precip": format!("{:.1}", h["precipitation"][i].as_f64().unwrap_or(0.0)),
                    "pop": format!("{:.0}", h["precipitation_probability"][i].as_f64().unwrap_or(0.0)),
                }));
            }
            Json(serde_json::json!({"code": "200", "hourly": hours}))
        }
        Err(e) => Json(serde_json::json!({"code": "500", "error": e})),
    }
}

pub async fn get_minutely(Query(params): Query<WeatherParams>) -> Json<serde_json::Value> {
    let (lat, lon) = parse_location(&params.location);
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&hourly=temperature_2m,precipitation_probability,precipitation,weather_code&forecast_hours=24&timezone=auto",
        lat, lon
    );
    let empty = serde_json::json!({"summary": "无降水数据", "hourly": []});
    match openmeteo_get(&url).await {
        Ok(om) => {
            let h = &om["hourly"];
            let times = h["time"].as_array().map(|a| a.as_slice()).unwrap_or(&[]);
            let next: Vec<serde_json::Value> = times.iter().enumerate().take(6).map(|(i, t)| {
                let code = h["weather_code"][i].as_i64().unwrap_or(0);
                serde_json::json!({
                    "time": t.as_str().unwrap_or(""),
                    "text": wmo_text(code),
                    "temp": format!("{:.1}", h["temperature_2m"][i].as_f64().unwrap_or(0.0)),
                    "precip": format!("{:.1}", h["precipitation"][i].as_f64().unwrap_or(0.0)),
                    "pop": format!("{:.0}", h["precipitation_probability"][i].as_f64().unwrap_or(0.0)),
                })
            }).collect();
            let has_rain = next.iter().any(|h| {
                h["pop"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0) > 30.0
            });
            let summary = if has_rain { "未来数小时有降水" } else { "未来数小时无降水" };
            Json(serde_json::json!({"summary": summary, "hourly": next}))
        }
        Err(_) => Json(empty),
    }
}

pub async fn get_air_now(Query(_params): Query<WeatherParams>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"code": "200", "now": {
        "aqi": "0",
        "level": "--",
        "category": "--",
        "pm2p5": "0",
        "pm10": "0",
        "no2": "0",
        "so2": "0",
        "co": "0",
        "o3": "0",
    }}))
}

pub async fn get_indices() -> Json<serde_json::Value> {
    Json(serde_json::json!({"code": "200", "daily": []}))
}

pub async fn get_warning(Query(_params): Query<WeatherParams>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"warning": []}))
}

pub async fn geo_lookup(Query(params): Query<GeoParams>) -> Json<serde_json::Value> {
    let num = params.number.unwrap_or(10).min(10);
    let query = params.location.trim().to_string();
    let url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count={}&language=zh&format=json",
        query, num
    );
    match openmeteo_get(&url).await {
        Ok(geo) => {
            let results = geo["results"].as_array().cloned().unwrap_or_default();
            let mut seen = std::collections::HashSet::new();
            let mut cities: Vec<serde_json::Value> = results.into_iter()
                .filter(|r| {
                    let name = r["name"].as_str().unwrap_or("");
                    let adm1 = r["admin1"].as_str().unwrap_or("");
                    let country = r["country"].as_str().unwrap_or("");
                    seen.insert(format!("{}|{}|{}", name, adm1, country))
                })
                .map(|r| {
                    let lat = r["latitude"].as_f64().unwrap_or(39.92);
                    let lon = r["longitude"].as_f64().unwrap_or(116.41);
                    serde_json::json!({
                        "name": r["name"].as_str().unwrap_or(""),
                        "id": format!("{:.2},{:.2}", lat, lon),
                        "adm1": r["admin1"].as_str().unwrap_or(""),
                        "adm2": r["admin2"].as_str().or_else(|| r["country"].as_str()).unwrap_or(""),
                    })
                })
                .collect();
            // 精确匹配优先: 搜索词等于完整名称或名称前缀
            cities.sort_by(|a, b| {
                let a_name = a["name"].as_str().unwrap_or("");
                let b_name = b["name"].as_str().unwrap_or("");
                let a_exact = a_name == query;
                let b_exact = b_name == query;
                let a_prefix = a_name.starts_with(&query);
                let b_prefix = b_name.starts_with(&query);
                (b_exact as i8, b_prefix as i8).cmp(&(a_exact as i8, a_prefix as i8))
            });
            cities.truncate(5);
            Json(serde_json::json!({"code": "200", "location": cities}))
        }
        Err(e) => Json(serde_json::json!({"code": "500", "error": e})),
    }
}
