use agri_core::db;
use anyhow::Result;
use axum::{Router, middleware};
use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use rumqttc::QoS;
use std::convert::Infallible;
use tower::service_fn;
use tracing::info;

fn content_type(ext: &str) -> &'static str {
    match ext {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "json" => "application/json",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        _ => "application/octet-stream",
    }
}

mod routes;
mod state;
mod request_logger;
mod rule_engine;
mod areas;
mod weather;
mod ai_routes;
mod response;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("agri_server=info".parse()?),
        )
        .init();

    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "agri.db".into());
    let pool = db::create_pool(&database_url).await?;
    db::run_migrations(&pool).await?;

    let server_port: u16 = std::env::var("SERVER_PORT")
        .unwrap_or_else(|_| "3000".into())
        .parse()
        .unwrap_or(3000);

    let broker_port: u16 = std::env::var("MQTT_BROKER_PORT")
        .unwrap_or_else(|_| "11883".into())
        .parse()
        .unwrap_or(11883);

    let local_ips = get_local_ips();

    // 启动本地 MQTT Broker (mosquitto)
    info!("Starting MQTT Broker (mosquitto) on port {}", broker_port);
    let config_path = format!("/tmp/mosquitto-agri-{}.conf", broker_port);
    let config = format!(
        "listener {} 0.0.0.0\nallow_anonymous true\n",
        broker_port
    );
    let _ = std::fs::write(&config_path, &config);
    match std::process::Command::new("mosquitto")
        .args(["-d", "-c", &config_path])
        .spawn()
    {
        Ok(mut child) => {
            std::thread::spawn(move || {
                let status = child.wait();
                if let Ok(s) = status {
                    tracing::info!("mosquitto exited with: {}", s);
                }
            });
        }
        Err(e) => {
            tracing::error!("Failed to start mosquitto: {}. Is mosquitto installed?", e);
            tracing::info!("Falling back to embedded broker (rumqttd)...");
            std::thread::spawn(move || {
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    agri_mqtt::broker::start_broker(broker_port)
                })) {
                    Ok(Ok(())) => tracing::info!("Embedded broker started successfully"),
                    Ok(Err(e)) => tracing::error!("Embedded broker failed: {}", e),
                    Err(panic) => {
                        let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                            s.to_string()
                        } else if let Some(s) = panic.downcast_ref::<String>() {
                            s.clone()
                        } else {
                            "unknown panic".to_string()
                        };
                        tracing::error!("Embedded broker thread panicked: {}", msg);
                    }
                }
            });
        }
    }

    // 等待 broker 就绪
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let (mqtt_client, eventloop) = create_mqtt_client(broker_port);

    let app_state = state::AppState::new(pool, mqtt_client);

    info!("========================================================");
    info!("  ESP32 MQTT 配置:");
    for ip in &local_ips {
        info!("    MQTT Broker -> {}:{}", ip, broker_port);
        info!("    HTTP API   -> http://{}:{}", ip, server_port);
    }
    info!("  请将 ESP32 固件中的 MQTT_SERVER 设为以上 IP 地址");
    info!("========================================================");

    // Spawn MQTT event loop listener
    let listener_pool = app_state.pool.clone();
    let listener_tx = app_state.event_tx.clone();
    tokio::spawn(async move {
        agri_mqtt::handler::start_listener(eventloop, listener_pool, Some(listener_tx)).await;
    });

    let rule_state = app_state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        if let Err(e) = rule_engine::start(rule_state).await {
            tracing::error!("Rule engine error: {}", e);
        }
    });

    let weather_router = Router::new()
        .route("/api/v1/weather/now", axum::routing::get(weather::get_weather_now))
        .route("/api/v1/weather/3d", axum::routing::get(weather::get_forecast_3d))
        .route("/api/v1/weather/24h", axum::routing::get(weather::get_forecast_24h))
        .route("/api/v1/weather/minutely", axum::routing::get(weather::get_minutely))
        .route("/api/v1/weather/air", axum::routing::get(weather::get_air_now))
        .route("/api/v1/weather/indices", axum::routing::get(weather::get_indices))
        .route("/api/v1/weather/warning", axum::routing::get(weather::get_warning))
        .route("/api/v1/weather/geo", axum::routing::get(weather::geo_lookup));

    let api_router = routes::create_router(app_state.clone())
        .merge(areas::create_router(app_state.clone()))
        .merge(weather_router)
        .merge(ai_routes::create_router(app_state));

    let static_dir = std::path::PathBuf::from("agri-server/static");
    let app = Router::new()
        .merge(api_router)
        .layer(middleware::from_fn(request_logger::log_requests))
        .fallback_service(service_fn(move |req: Request<Body>| {
            let dir = static_dir.clone();
            async move {
                let path = req.uri().path().trim_start_matches('/');
                let file_path = if path.is_empty() { dir.join("index.html") } else { dir.join(path) };
                match tokio::fs::read(&file_path).await {
                    Ok(data) => {
                        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        Ok::<_, Infallible>(Response::builder()
                            .header("Content-Type", content_type(ext))
                            .body(Body::from(data))
                            .expect("valid response builder"))
                    }
                    Err(_) => {
                        let idx = dir.join("index.html");
                        match tokio::fs::read(&idx).await {
                            Ok(data) => Ok(Response::builder()
                                .header("Content-Type", "text/html; charset=utf-8")
                                .body(Body::from(data))
                                .expect("valid response builder")),
                            Err(_) => Ok(Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .body(Body::from("Not Found"))
                                .expect("valid response builder")),
                        }
                    }
                }
            }
        }));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", server_port)).await?;
    info!("Server listening on 0.0.0.0:{}", server_port);
    info!("Dashboard: http://localhost:{}", server_port);

    axum::serve(listener, app).await?;
    Ok(())
}

fn get_local_ips() -> Vec<String> {
    let mut ips = Vec::new();
    if let Ok(output) = std::process::Command::new("hostname").arg("-I").output() {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            for ip in stdout.split_whitespace() {
                let trimmed = ip.trim();
                if !trimmed.is_empty() && trimmed != "127.0.0.1" && trimmed != "::1" {
                    ips.push(trimmed.to_string());
                }
            }
        }
    }
    ips
}

fn create_mqtt_client(broker_port: u16) -> (rumqttc::AsyncClient, rumqttc::EventLoop) {
    let client_id = format!("agri-server-{}", std::process::id());
    let mut options = rumqttc::MqttOptions::new(&client_id, "127.0.0.1", broker_port);
    options.set_keep_alive(std::time::Duration::from_secs(10));
    options.set_clean_session(false);
    let (client, eventloop) = rumqttc::AsyncClient::new(options, 10);
    let sub_client = client.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        loop {
            if let Err(e) = sub_client.subscribe("agri/node/+/telemetry", QoS::AtMostOnce).await {
                tracing::warn!("MQTT subscribe telemetry failed: {}", e);
            }
            if let Err(e) = sub_client.subscribe("agri/node/+/status", QoS::AtMostOnce).await {
                tracing::warn!("MQTT subscribe status failed: {}", e);
            }
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    });
    (client, eventloop)
}
