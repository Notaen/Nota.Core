use std::sync::Arc;

use axum::{
    Json, Router,
    extract::Request,
    http::StatusCode,
    middleware,
    response::Response,
    routing::get,
};
use nota_core::bus::EventBus;
use serde::Serialize;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

pub(crate) mod admin;
pub(crate) mod chat;

async fn log_request(req: Request, next: middleware::Next) -> Response {
    log::debug!("Received: {} {}", req.method(), req.uri());
    next.run(req).await
}

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

pub fn router(state: Arc<EventBus>, cancel_token: CancellationToken) -> Router<()> {
    Router::<Arc<EventBus>>::new()
        .route("/", get(root))
        .route("/health", get(health))
        .nest("/admin", admin::router(cancel_token.clone()))
        .nest("/chat", chat::router())
        .layer(middleware::from_fn(log_request))
        .with_state(state)
}

pub async fn serve(
    listener: TcpListener,
    bus: Arc<EventBus>,
    cancel_token: CancellationToken,
) {
    let app = router(bus, cancel_token.clone());
    log::debug!("Server listening on {}", listener.local_addr().unwrap());

    let shutdown_future = async move {
        cancel_token.cancelled().await;
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_future)
        .await
        .unwrap();
}
