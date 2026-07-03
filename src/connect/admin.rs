use axum::{Router, http::StatusCode, routing::post};
use tokio_util::sync::CancellationToken;

pub(super) fn router(stop_token: CancellationToken) -> Router {
    Router::new().route("/stop", post(move || stop(stop_token.clone())))
}

async fn stop(stop_token: CancellationToken) -> (StatusCode, &'static str) {
    stop_token.cancel();
    (StatusCode::OK, "Shutdown Nota.Core")
}
