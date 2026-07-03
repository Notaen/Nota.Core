use axum::{
    Json, Router, extract::Request, http::StatusCode, middleware, response::Response, routing::get,
};

use serde::Serialize;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::debug;

async fn log_request(req: Request, next: middleware::Next) -> Response {
    debug!("Received: {} {}", req.method(), req.uri());
    next.run(req).await
}

mod admin;
mod session;

async fn root() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not Found")
}

#[derive(Serialize)]
struct Health<'a> {
    version: &'a str,
}

async fn health() -> (StatusCode, Json<Health<'static>>) {
    (
        StatusCode::OK,
        Json(Health {
            version: env!("CARGO_PKG_VERSION"),
        }),
    )
}

pub async fn serve(cancel_token: CancellationToken) {
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .nest("/admin", admin::router(cancel_token.clone()))
        .nest("/session", session::router())
        .layer(middleware::from_fn(log_request));

    let listener = TcpListener::bind("127.0.0.1:2349").await.unwrap();
    debug!("Server start at 127.0.0.1:2349");

    let shutdown_future = async move {
        cancel_token.cancelled().await;
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_future)
        .await
        .unwrap();
}
