use agri_core::db;
use anyhow::Result;
use axum::Router;
use tower_http::services::ServeDir;
use tracing::info;

mod routes;
mod state;
mod middleware;
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

    let app_state = state::AppState::new(pool);

    // 启动规则引擎（异步后台任务）
    let rule_state = app_state.clone();
    tokio::spawn(async move {
        // 等待 2 秒让服务器先启动
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        if let Err(e) = rule_engine::start(rule_state).await {
            tracing::error!("Rule engine error: {}", e);
        }
    });

    let api_router = routes::create_router(app_state);

    let app = Router::new()
        .merge(api_router)
        .fallback_service(ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("Server listening on 0.0.0.0:3000");
    info!("Dashboard: http://localhost:3000");

    axum::serve(listener, app).await?;
    Ok(())
}
