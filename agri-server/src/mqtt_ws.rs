use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tracing::info;

pub async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    info!("WebSocket MQTT client connected");

    let broker_addr = std::env::var("MQTT_BROKER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:11883".into());

    let tcp = match tokio::net::TcpStream::connect(&broker_addr).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Cannot connect to MQTT broker {}: {}", broker_addr, e);
            return;
        }
    };

    let (mut tcp_r, mut tcp_w) = tokio::io::split(tcp);
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(64);

    // Task 1: TCP reader → mpsc channel
    tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut buf = [0u8; 4096];
        loop {
            match tcp_r.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).await.is_err() {
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
                    }
                    None => break,
                }
            }
        }
    }

    info!("WebSocket MQTT client disconnected");
}
