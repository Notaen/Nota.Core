use axum::{Router, http::StatusCode, routing::post};
use tokio_util::sync::CancellationToken;

pub(super) fn router(stop_token: CancellationToken) -> Router {
    Router::new().route("/stop", post(|| stop(stop_token)))
}

async fn stop(stop_token: CancellationToken) -> (StatusCode, &'static str) {
    stop_token.cancel();
    (StatusCode::OK, "Shutdown Nota.Core")
}
