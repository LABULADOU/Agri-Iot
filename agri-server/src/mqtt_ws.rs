use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tracing::info;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::sync::OnceLock;
use std::collections::HashMap;
use std::time::Duration;

/// Current firmware version string (used to prevent OTA loops).
/// Increment/deploy when the ESP32 firmware binary changes.
const FW_VERSION: &str = "20260706-210000";

/// Per-node tracker of the last injected firmware version.
/// Prevents re-injecting the same OTA command on reconnection (OTA loop).
static INJECTED_VERSIONS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

pub async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

/// Extract client_id from a raw MQTT CONNECT packet.
/// Returns None if the packet isn't a valid CONNECT.
fn extract_client_id(data: &[u8]) -> Option<String> {
    if data.len() < 10 || data[0] != 0x10 {
        return None;
    }
    let mut pos = 1usize;
    loop {
        if pos >= data.len() { return None; }
        let byte = data[pos];
        pos += 1;
        if byte & 0x80 == 0 { break; }
    }
    if pos + 2 > data.len() { return None; }
    let proto_len = u16::from_be_bytes([data[pos], data[pos+1]]) as usize;
    pos += 2;
    if pos + proto_len + 3 > data.len() { return None; }
    pos += proto_len;
    pos += 1;
    pos += 1;
    pos += 2;
    if pos + 2 > data.len() { return None; }
    let id_len = u16::from_be_bytes([data[pos], data[pos+1]]) as usize;
    pos += 2;
    if pos + id_len > data.len() { return None; }
    let id = std::str::from_utf8(&data[pos..pos+id_len]).ok()?;
    Some(id.to_string())
}

/// Build a raw MQTT PUBLISH packet for the ota_mqtt command.
/// The command tells ESP32 to enter OTA mode with the total firmware size.
/// Firmware data is sent separately as ota_chunk MQTT PUBLISH messages
/// over the same WebSocket connection (no separate HTTP/TLS needed).
fn build_ota_mqtt_command(node_id: &str, _version: &str, total_size: usize) -> Vec<u8> {
    let sig = if node_id.contains("node-001") {
        "MEYCIQDnMxSHxO7PVa6Xcc+GxspthIOx68xU0reHVjTw6cLq6wIhAO4qhvP3zSThgGKNbRns26SdpMYBzBsPU1h17VO2yGTr"
    } else {
        "MEQCICHlyZqetZuLlV2zmZ9QlanxMDdgoWDV5VglBONKHUb+AiB38WLiL8XZx9UnRJSUNMmMrRQrvJCLFckvEKwOrK/x7g=="
    };
    let payload = format!(
        r#"{{"command":"ota_mqtt","params":{{"total_size":{},"sig":"{}"}}}}"#,
        total_size, sig
    );
    let topic = format!("agri/node/{}/command/ota_mqtt", node_id);
    build_mqtt_publish(&topic, payload.as_bytes())
}

/// Build a raw MQTT PUBLISH packet (QoS 0, no retain).
fn build_mqtt_publish(topic: &str, payload: &[u8]) -> Vec<u8> {
    let topic_bytes = topic.as_bytes();
    let remaining = 2 + topic_bytes.len() + payload.len();
    let mut pkt = Vec::with_capacity(remaining + 5);
    pkt.push(0x30);
    let mut rl = remaining;
    loop {
        let mut byte = (rl & 0x7F) as u8;
        rl >>= 7;
        if rl > 0 { byte |= 0x80; }
        pkt.push(byte);
        if rl == 0 { break; }
    }
    pkt.extend_from_slice(&(topic_bytes.len() as u16).to_be_bytes());
    pkt.extend_from_slice(topic_bytes);
    pkt.extend_from_slice(payload);
    pkt
}

async fn handle_socket(mut socket: WebSocket) {
    info!("WebSocket MQTT client connected");

    let broker_addr = std::env::var("MQTT_BROKER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:1883".into());

    let tcp = match tokio::net::TcpStream::connect(&broker_addr).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Cannot connect to MQTT broker {}: {}", broker_addr, e);
            return;
        }
    };

    let (mut tcp_r, mut tcp_w) = tokio::io::split(tcp);
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(64);

    // Shared state: the client_id extracted from the CONNECT packet
    let client_id: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    // Task 1: TCP reader → mpsc channel
    let tx2 = tx.clone();
    tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut buf = [0u8; 4096];
        loop {
            match tcp_r.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if tx2.send(buf[..n].to_vec()).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Main task: WebSocket recv → TCP write  +  mpsc channel → WebSocket send
    loop {
        tokio::select! {
            // WebSocket → TCP
            ws_msg = socket.recv() => {
                match ws_msg {
                    Some(Ok(Message::Binary(data))) => {
                        if client_id.lock().await.is_none() {
                            if let Some(id) = extract_client_id(&data) {
                                *client_id.lock().await = Some(id.clone());
                                info!("ESP32 '{}' connected via WebSocket MQTT", id);
                            }
                        }
                        if tcp_w.write_all(&data).await.is_err() { break; }
                    }
                    Some(Ok(Message::Text(data))) => {
                        if tcp_w.write_all(data.as_bytes()).await.is_err() { break; }
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    _ => {}
                }
            }
            // TCP → WebSocket (via mpsc)
            data = rx.recv() => {
                match data {
                    Some(bytes) => {
                        if socket.send(Message::Binary(bytes)).await.is_err() { break; }
                        // Inject OTA after the first broker→ESP32 message (CONNACK).
                        // Only inject once per firmware version to prevent OTA loops.
                        let id = client_id.lock().await.clone();
                        if let Some(ref node_id) = id {
                            let (should, debug_map) = {
                                let map = INJECTED_VERSIONS
                                    .get_or_init(|| Mutex::new(HashMap::new()))
                                    .lock()
                                    .await;
                                let current = map.get(node_id).cloned();
                                (!matches!(&current, Some(v) if v == FW_VERSION), current)
                            };
                            let always_inject = std::env::var("OTA_FORCE_REINJECT").is_ok();
                            if should || always_inject {
                                let fw_path = format!(
                                    "agri-server/static/firmware/firmware-{}-{}.bin",
                                    node_id, FW_VERSION
                                );
                                let fw_data = if std::path::Path::new(&fw_path).exists() {
                                    std::fs::read(&fw_path).unwrap_or_else(|e| {
                                        tracing::warn!("Cannot read firmware {}: {}", fw_path, e);
                                        Vec::new()
                                    })
                                } else {
                                    tracing::warn!("Firmware file not found: {}", fw_path);
                                    Vec::new()
                                };
                                if fw_data.is_empty() {
                                    // firmware not available — skip injection
                                    // ESP32 will get it on next reconnect
                                } else {
                                    let total_size = fw_data.len();
                                    let cmd_pkt = build_ota_mqtt_command(node_id, FW_VERSION, total_size);
                                    info!("Injecting OTA MQTT command for {} (version {}, total_size={}, map={:?}, always={})",
                                        node_id, FW_VERSION, total_size, debug_map, always_inject);
                                    if socket.send(Message::Binary(cmd_pkt)).await.is_err() { break; }

                                    // Record version immediately to prevent re-injection on every
                                    // subsequent broker message.  The chunk-sending task runs
                                    // independently; if connection drops mid-OTA the version is
                                    // still recorded, requiring OTA_FORCE_REINJECT or version bump
                                    // to retry.
                                    if !always_inject {
                                        let mut map = INJECTED_VERSIONS
                                            .get_or_init(|| Mutex::new(HashMap::new()))
                                            .lock()
                                            .await;
                                        map.insert(node_id.clone(), FW_VERSION.to_string());
                                        info!("OTA injection recorded for {} -> version {}", node_id, FW_VERSION);
                                    }

                                    let tx_for_chunks = tx.clone();
                                    let node_id_for_chunks = node_id.clone();
                                    tokio::spawn(async move {
                                        let chunk_size = 4096usize;
                                        let topic = format!("agri/node/{}/command/ota_chunk", node_id_for_chunks);
                                        for chunk in fw_data.chunks(chunk_size) {
                                            let pkt = build_mqtt_publish(&topic, chunk);
                                            if tx_for_chunks.send(pkt).await.is_err() {
                                                tracing::warn!("OTA MQTT chunk send failed for {} (connection lost)", node_id_for_chunks);
                                                return;
                                            }
                                            tokio::time::sleep(Duration::from_millis(10)).await;
                                        }
                                        info!("Sent all OTA MQTT chunks for {}", node_id_for_chunks);
                                    });
                                }
                            } else {
                                info!("OTA already injected for {} (version {}), skipping — map={:?}",
                                    node_id, FW_VERSION, debug_map);
                            }
                        }
                    }
                    None => break,
                }
            }
        }
    }

    info!("WebSocket MQTT client disconnected");
}
