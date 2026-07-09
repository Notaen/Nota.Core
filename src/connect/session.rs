use axum::{
    Router,
    extract::{Json, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;

use crate::session::SessionManager;

#[derive(Deserialize)]
struct CreateSession {
    creator: String,
}

#[derive(Deserialize)]
struct SetArchiveAt {
    archive_at: Option<i64>,
}

async fn list_metadata() -> impl IntoResponse {
    let metadata = SessionManager::get().list_metadata().await;
    (StatusCode::OK, Json(metadata))
}

async fn create_session(Json(payload): Json<CreateSession>) -> impl IntoResponse {
    let creator = payload.creator.trim().to_string();
    if creator.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "creator is required"})),
        );
    }

    match SessionManager::get().new_session(creator).await {
        Ok(sid) => (StatusCode::CREATED, Json(serde_json::json!({"sid": sid}))),
        Err(e) => {
            tracing::error!("Failed to create session: {e:?}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to create session"})),
            )
        }
    }
}

async fn get_archive_at(Path(sid): Path<String>) -> impl IntoResponse {
    let session_map = SessionManager::get().session_map.read().await;
    let session = session_map.get(&sid);
    match session {
        // TODO: Can be better. Maybe a wrapper to convert `Result` into `Response`
        Some(s) => (
            StatusCode::OK,
            Json(serde_json::json!({"archive_at": s.metadata.archive_at})),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session not found"})),
        ),
    }
}

async fn set_archive_at(
    Path(sid): Path<String>,
    Json(payload): Json<SetArchiveAt>,
) -> impl IntoResponse {
    let mut session_map = SessionManager::get().session_map.write().await;
    let session = session_map.get_mut(&sid);
    match session {
        // TODO: Can be better. Maybe a wrapper to convert `Result` into `Response`
        Some(s) => {
            s.set_archive_at(payload.archive_at).await;
            (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session not found"})),
        ),
    }
}

pub(super) fn router() -> Router {
    Router::new()
        .route("/", get(list_metadata))
        .route("/", post(create_session))
        .route("/{sid}/archive_at", get(get_archive_at))
        .route("/{sid}/archive_at", post(set_archive_at))
}
