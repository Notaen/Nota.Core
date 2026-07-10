use std::sync::Arc;

use axum::{
    Router,
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use nota_core::session::SessionManager;
use serde::Deserialize;

#[derive(Deserialize)]
struct CreateSession {
    creator: String,
}

#[derive(Deserialize)]
struct SetArchiveAt {
    archive_at: Option<i64>,
}

async fn list_metadata(State(sm): State<Arc<SessionManager>>) -> impl IntoResponse {
    let metadata = sm.list_metadata().await;
    (StatusCode::OK, Json(metadata))
}

async fn create_session(
    State(sm): State<Arc<SessionManager>>,
    Json(payload): Json<CreateSession>,
) -> impl IntoResponse {
    let creator = payload.creator.trim().to_string();
    if creator.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "creator is required"})),
        );
    }

    match sm.new_session(creator).await {
        Ok(sid) => (StatusCode::CREATED, Json(serde_json::json!({"sid": sid}))),
        Err(e) => {
            log::error!("Failed to create session: {e:?}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to create session"})),
            )
        }
    }
}

async fn get_archive_at(
    State(sm): State<Arc<SessionManager>>,
    Path(sid): Path<String>,
) -> impl IntoResponse {
    match sm.get_archive_at(&sid).await {
        // TODO: Can be better. Maybe a wrapper to convert `Result` into `Response`
        Some(archive_at) => (
            StatusCode::OK,
            Json(serde_json::json!({"archive_at": archive_at})),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session not found"})),
        ),
    }
}

async fn set_archive_at(
    State(sm): State<Arc<SessionManager>>,
    Path(sid): Path<String>,
    Json(payload): Json<SetArchiveAt>,
) -> impl IntoResponse {
    match sm.set_archive_at(&sid, payload.archive_at).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))),
        Err(e) => {
            log::error!("Failed to set archive_at: {e:?}");
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Session not found"})),
            )
        }
    }
}

pub(super) fn router() -> Router<Arc<SessionManager>> {
    Router::new()
        .route("/", get(list_metadata))
        .route("/", post(create_session))
        .route("/{sid}/archive_at", get(get_archive_at))
        .route("/{sid}/archive_at", post(set_archive_at))
}
