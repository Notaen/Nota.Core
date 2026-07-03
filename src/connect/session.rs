use axum::{
    Router,
    extract::{Json, Path},
    http::StatusCode,
    routing::{delete, get, post},
};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::session::manager;

#[derive(Deserialize)]
struct CreateSession {
    creator: String,
}

#[derive(Deserialize)]
struct SetArchiveAt {
    archive_at: DateTime<Utc>,
}

pub(super) fn router() -> Router {
    Router::new()
        .route(
            "/",
            get(async move || {
                let metadata = manager::SessionManager::list_metadata();
                (StatusCode::OK, Json(metadata))
            }),
        )
        .route(
            "/",
            post(|Json(payload): Json<CreateSession>| async move {
                let creator = payload.creator.trim().to_string();
                if creator.is_empty() {
                    return (
                        StatusCode::BAD_REQUEST,
                        axum::Json(serde_json::json!({"error": "creator is required"})),
                    );
                }

                match manager::SessionManager::new_session(creator).await {
                    Ok(sid) => (
                        StatusCode::CREATED,
                        axum::Json(serde_json::json!({"sid": sid})),
                    ),
                    Err(e) => {
                        tracing::error!("Failed to create session: {e:?}");
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            axum::Json(serde_json::json!({"error": "Failed to create session"})),
                        )
                    }
                }
            }),
        )
        .route(
            "/{sid}/archive_at",
            get(|Path(sid): Path<String>| async move {
                match manager::SessionManager::get_archive_at(&sid) {
                    Some(archive_at) => (
                        StatusCode::OK,
                        Json(serde_json::json!({"archive_at": archive_at})),
                    ),
                    None => (
                        StatusCode::NOT_FOUND,
                        Json(serde_json::json!({"error": "Session not found"})),
                    ),
                }
            }),
        )
        .route(
            "/{sid}/archive_at",
            post(
                |Path(sid): Path<String>, Json(payload): Json<SetArchiveAt>| async move {
                    match manager::SessionManager::set_archive_at(&sid, Some(payload.archive_at))
                        .await
                    {
                        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))),
                        Err(e) => {
                            tracing::error!("Failed to set archive_at: {e:?}");
                            (
                                StatusCode::NOT_FOUND,
                                Json(serde_json::json!({"error": e.to_string()})),
                            )
                        }
                    }
                },
            ),
        )
        .route(
            "/{sid}/archive_at",
            delete(|Path(sid): Path<String>| async move {
                match manager::SessionManager::set_archive_at(&sid, None).await {
                    Ok(()) => (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))),
                    Err(e) => {
                        tracing::error!("Failed to delete archive_at: {e:?}");
                        (
                            StatusCode::NOT_FOUND,
                            Json(serde_json::json!({"error": e.to_string()})),
                        )
                    }
                }
            }),
        )
}
