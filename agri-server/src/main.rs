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
mod ws_handler;
mod weather;
mod ai_routes;
mod response;
mod mqtt_ws;
mod rate_limiter;
mod decision;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("agri_server=info".parse()?)
                .add_directive("agri_mqtt=info".parse()?)
                .add_directive("rumqttc=warn".parse()?),
        )
        .init();

    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "agri.db".into());
    let pool = db::create_pool(&database_url).await?;
    db::run_migrations(&pool).await?;

    let server_port: u16 = std::env::var("SERVER_PORT")
        .unwrap_or_else(|_| "3001".into())
        .parse()
        .unwrap_or(3001);

    let broker_addr: String = std::env::var("MQTT_BROKER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:1883".into());

    info!("Connecting to external MQTT broker at {}", broker_addr);

    let (mqtt_client, eventloop) = create_mqtt_client(&broker_addr);

    let app_state = state::AppState::new(pool, mqtt_client);

    // Spawn MQTT event loop listener
    let listener_pool = app_state.pool.clone();
    let listener_tx = app_state.event_tx.clone();
    tokio::spawn(async move {
        agri_mqtt::handler::start_listener(eventloop, listener_pool, Some(listener_tx)).await;
    });

    // 定期清理限流器过期桶
    let limiter = app_state.telemetry_limiter.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            limiter.cleanup();
        }
    });

    let rule_state = app_state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        if let Err(e) = rule_engine::start(rule_state).await {
            tracing::error!("Rule engine error: {}", e);
        }
    });

    let decision_state = app_state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        if let Err(e) = decision::start(decision_state).await {
            tracing::error!("Decision engine error: {}", e);
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
        .route("/mqtt", axum::routing::get(mqtt_ws::ws_handler))
        .merge(ai_routes::create_router(app_state));

    let static_dir = std::path::PathBuf::from("agri-server/static")
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from("agri-server/static"));
    let app = Router::new()
        .merge(api_router)
        .layer(middleware::from_fn(request_logger::log_requests))
        .fallback_service(service_fn(move |req: Request<Body>| {
            let dir = static_dir.clone();
            async move {
                let path = req.uri().path().trim_start_matches('/');
                let raw = if path.is_empty() { "index.html".to_string() } else { path.to_string() };
                let file_path = dir.join(&raw);
                // 路径穿越防护：规范化后验证在 static 目录内
                match file_path.canonicalize() {
                    Ok(canon) if canon.starts_with(&dir) => {
                        match tokio::fs::read(&canon).await {
                            Ok(data) => {
                                let ext = canon.extension().and_then(|e| e.to_str()).unwrap_or("");
                                Ok::<_, Infallible>(Response::builder()
                                    .header("Content-Type", content_type(ext))
                                    .body(Body::from(data))
                                    .expect("valid response builder"))
                            }
                            Err(_) => serve_index(&dir).await,
                        }
                    }
                    _ => serve_index(&dir).await,
                }
            }
        }));

    info!("Server listening on 0.0.0.0:{}", server_port);
    info!("Dashboard: http://localhost:{}", server_port);
    info!("MQTT WebSocket bridge: /mqtt (connect via wss://host/mqtt)");

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", server_port)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// 路径穿越安全 fallback：返回 index.html 或 404
async fn serve_index(dir: &std::path::Path) -> Result<Response<Body>, Infallible> {
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

fn create_mqtt_client(broker_addr: &str) -> (rumqttc::AsyncClient, rumqttc::EventLoop) {
    let client_id = "agri-server-001".to_string();
    let (host, port) = broker_addr.split_once(':')
        .map(|(h, p)| (h.to_string(), p.parse::<u16>().unwrap_or(11883)))
        .unwrap_or_else(|| (broker_addr.to_string(), 11883));
    let mut options = rumqttc::MqttOptions::new(&client_id, host, port);
    options.set_keep_alive(std::time::Duration::from_secs(30));
    options.set_clean_session(false);
    options.set_request_channel_capacity(100);
    let (client, eventloop) = rumqttc::AsyncClient::new(options, 100);
    let sub_client = client.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let topics = agri_core::topics::subscribe_topics();
        loop {
            for topic in &topics {
                if let Err(e) = sub_client.subscribe(topic, QoS::AtLeastOnce).await {
                    tracing::warn!("MQTT subscribe {} failed: {}", topic, e);
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });
    (client, eventloop)
}
