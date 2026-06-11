use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
};
use crate::state::AppState;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct WsRequest {
    id: Option<String>,
    cmd: String,
    #[serde(flatten)]
    params: HashMap<String, serde_json::Value>,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut event_rx = state.event_tx.subscribe();
    let mut subs: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();
    tracing::info!("WS handle_socket started");

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let req: WsRequest = match serde_json::from_str(&text) {
                            Ok(r) => r,
                            Err(e) => {
                                let _ = socket.send(Message::Text(
                                    serde_json::json!({"cmd": "error", "error": format!("parse error: {}", e)}).to_string()
                                )).await;
                                continue;
                            }
                        };

                        match req.cmd.as_str() {
                            "subscribe" => {
                                if let Some(id) = &req.id {
                                    subs.insert(id.clone(), req.params.clone());
                                    let _ = socket.send(Message::Text(
                                        serde_json::json!({"id": id, "cmd": "subscribed"}).to_string()
                                    )).await;
                                }
                            }
                            "query" => {
                                let query_type = req.params.get("type")
                                    .and_then(|v| v.as_str()).unwrap_or("");
                                let result = match query_type {
                                    "readings" => handle_readings_query(&state, &req.params).await,
                                    "aggregate" => handle_aggregate_query(&state, &req.params).await,
                                    "devices" => handle_devices_query(&state, &req.params).await,
                                    _ => Err("unknown query type".to_string()),
                                };
                                let resp = match result {
                                    Ok(data) => serde_json::json!({"id": req.id, "cmd": "data", "data": data}),
                                    Err(e) => serde_json::json!({"id": req.id, "cmd": "error", "error": e}),
                                };
                                let _ = socket.send(Message::Text(resp.to_string())).await;
                            }
                            "unsubscribe" => {
                                if let Some(id) = &req.id {
                                    subs.remove(id);
                                }
                            }
                            _ => {
                                let _ = socket.send(Message::Text(
                                    serde_json::json!({"id": req.id, "cmd": "error", "error": "unknown command"}).to_string()
                                )).await;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            event = event_rx.recv() => {
                match event {
                    Ok(data) => {
                        tracing::trace!("WS received broadcast event, data len: {}", data.len());
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&data) {
                            let event_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            let node_id = parsed.get("node_id").and_then(|v| v.as_str()).unwrap_or("");
                            for (id, params) in &subs {
                                let sub_type = params.get("type").and_then(|v| v.as_str()).unwrap_or("telemetry");
                                if sub_type != event_type { continue; }
                                if let Some(nodes) = params.get("nodes").and_then(|v| v.as_array()) {
                                    if !nodes.is_empty() && !nodes.iter().any(|n| n.as_str() == Some(node_id)) { continue; }
                                }
                                let _ = socket.send(Message::Text(
                                    serde_json::json!({"id": id, "cmd": "push", "data": parsed}).to_string()
                                )).await;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("WS broadcast lagged by {} messages", n);
                    }
                    Err(_) => break,
                }
            }
        }
    }
}

async fn handle_readings_query(
    state: &AppState,
    params: &HashMap<String, serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let device_id = params.get("device_id").and_then(|v| v.as_str()).ok_or("missing device_id")?;
    let limit = params.get("limit").and_then(|v| v.as_i64()).unwrap_or(100).max(1).min(5000);

    let rows = sqlx::query_as::<_, (i64, String, String, f64, String, i64)>(
        "SELECT id, device_id, metric, value, unit, timestamp FROM sensor_readings \
         WHERE device_id = ? ORDER BY timestamp DESC LIMIT ?"
    )
    .bind(device_id)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| format!("db error: {}", e))?;

    let result: Vec<serde_json::Value> = rows.into_iter().map(|(id, did, metric, value, unit, ts)| {
        serde_json::json!({"id": id, "device_id": did, "metric": metric, "value": value, "unit": unit, "timestamp": ts})
    }).collect();

    Ok(serde_json::json!(result))
}

async fn handle_aggregate_query(
    state: &AppState,
    params: &HashMap<String, serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let device_id = params.get("device_id").and_then(|v| v.as_str()).ok_or("missing device_id")?;
    let metric = params.get("metric").and_then(|v| v.as_str()).ok_or("missing metric")?;
    let period = params.get("period").and_then(|v| v.as_str()).unwrap_or("hour");
    let now = chrono::Utc::now().timestamp();
    let start = params.get("start").and_then(|v| v.as_i64()).unwrap_or(now - 86400);
    let end = params.get("end").and_then(|v| v.as_i64()).unwrap_or(now);

    let fmt = match period {
        "month" => "%Y-%m",
        "week" => "%Y-W%W",
        "day" => "%Y-%m-%d",
        "10min" => "",
        _ => "%Y-%m-%d %H:00:00",
    };

    let rows = if period == "10min" {
        sqlx::query_as::<_, (String, String, f64, f64, f64, i64)>(
            "SELECT strftime('%Y-%m-%d %H:%M', datetime(((timestamp / 600) * 600), 'unixepoch')) as bucket, \
             metric, MAX(value), MIN(value), AVG(value), COUNT(*) \
             FROM sensor_readings \
             WHERE device_id = ? AND metric = ? AND timestamp >= ? AND timestamp <= ? \
             GROUP BY bucket, metric ORDER BY bucket ASC"
        )
        .bind(device_id)
        .bind(metric)
        .bind(start)
        .bind(end)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| format!("db error: {}", e))?
    } else {
        sqlx::query_as::<_, (String, String, f64, f64, f64, i64)>(
            "SELECT strftime(?, datetime(timestamp, 'unixepoch')) as bucket, \
             metric, MAX(value), MIN(value), AVG(value), COUNT(*) \
             FROM sensor_readings \
             WHERE device_id = ? AND metric = ? AND timestamp >= ? AND timestamp <= ? \
             GROUP BY bucket, metric ORDER BY bucket ASC"
        )
        .bind(fmt)
        .bind(device_id)
        .bind(metric)
        .bind(start)
        .bind(end)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| format!("db error: {}", e))?
    };

    let result: Vec<serde_json::Value> = rows.into_iter().map(|(bucket, m, max_val, min_val, avg_val, cnt)| {
        serde_json::json!({"timestamp": bucket, "metric": m, "max": max_val, "min": min_val, "avg": avg_val, "count": cnt})
    }).collect();

    Ok(serde_json::json!(result))
}

async fn handle_devices_query(
    state: &AppState,
    _params: &HashMap<String, serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let rows = sqlx::query_as::<_, (String, String, String, String, Option<String>, String)>(
        "SELECT id, node_id, name, device_type, area_id, status FROM devices ORDER BY name ASC"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| format!("db error: {}", e))?;

    let result: Vec<serde_json::Value> = rows.into_iter().map(|(id, node_id, name, dt, area_id, status)| {
        serde_json::json!({"id": id, "node_id": node_id, "name": name, "device_type": dt, "area_id": area_id, "status": status})
    }).collect();

    Ok(serde_json::json!(result))
}
