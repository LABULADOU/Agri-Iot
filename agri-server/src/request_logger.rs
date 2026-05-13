use axum::{
    body::Body,
    http::Request,
    middleware::Next,
    response::Response,
};
use tracing::info;

pub async fn log_requests(req: Request<Body>, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();

    let response = next.run(req).await;

    info!("{} {} -> {}", method, uri, response.status());
    response
}
