use axum::{Router, http::StatusCode, routing::post};
use tokio_util::sync::CancellationToken;

pub(super) fn router<S: Clone + Send + Sync + 'static>(stop_token: CancellationToken) -> Router<S> {
    Router::new().route("/stop", post(move || stop(stop_token.clone())))
}

async fn stop(stop_token: CancellationToken) -> (StatusCode, &'static str) {
    stop_token.cancel();
    (StatusCode::OK, "Shutdown Nota")
}
