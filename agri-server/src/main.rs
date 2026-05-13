use agri_core::db;
use anyhow::Result;
use axum::{Router, middleware};
use tower_http::services::ServeDir;
use tracing::info;

mod routes;
mod state;
mod request_logger;
mod rule_engine;

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

    let mqtt_host = std::env::var("MQTT_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let mqtt_port = std::env::var("MQTT_PORT")
        .unwrap_or_else(|_| "1883".into())
        .parse::<u16>()
        .unwrap_or(1883);

    // 仅在本地模式下启动内置 Broker（127.0.0.1/localhost 时启动）
    if mqtt_host == "127.0.0.1" || mqtt_host == "localhost" {
        let broker_port = mqtt_port;
        std::thread::spawn(move || {
            if let Err(e) = agri_mqtt::broker::start_broker(broker_port) {
                tracing::error!("MQTT Broker failed: {}", e);
            }
        });
        info!("MQTT Broker started on port {}", mqtt_port);
    } else {
        info!("Using external MQTT broker at {}:{}", mqtt_host, mqtt_port);
    }

    // 创建MQTT客户端和事件循环
    let (mqtt_client, eventloop) = agri_mqtt::client::create_client(&mqtt_host, mqtt_port, "agri-server")?;
    info!("MQTT client created");

    // 启动MQTT消息处理（传入eventloop）
    let handler_pool = pool.clone();
    tokio::spawn(async move {
        agri_mqtt::handler::start_listener(eventloop, handler_pool).await;
    });

    let app_state = state::AppState::new(pool, mqtt_client);

    // 启动规则引擎（异步后台任务）
    let rule_state = app_state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        if let Err(e) = rule_engine::start(rule_state).await {
            tracing::error!("Rule engine error: {}", e);
        }
    });

    let api_router = routes::create_router(app_state);

    let app = Router::new()
        .merge(api_router)
        .layer(middleware::from_fn(request_logger::log_requests))
        .fallback_service(ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("Server listening on 0.0.0.0:3000");
    info!("Dashboard: http://localhost:3000");

    axum::serve(listener, app).await?;
    Ok(())
}
