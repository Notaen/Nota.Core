use std::{collections::HashMap, sync::LazyLock};
use std::sync::Arc;
use std::time::Duration;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
};
use nota_core::bus::{BusEvent, EventBus};
use serde::Deserialize;
use tokio::sync::{RwLock, oneshot};

static PENDING: LazyLock<RwLock<HashMap<String, oneshot::Sender<String>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
}

async fn chat_handler(
    State(bus): State<Arc<EventBus>>,
    Json(payload): Json<ChatRequest>,
) -> impl IntoResponse {
    let request_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = oneshot::channel();

    PENDING.write().await.insert(request_id.clone(), tx);

    bus.send(BusEvent {
        sender: "user".to_string(),
        content: payload.message,
        timestamp: chrono::Utc::now().timestamp(),
        context: String::new(),
        request_id: Some(request_id.clone()),
    });

    match tokio::time::timeout(Duration::from_secs(120), rx).await {
        Ok(Ok(response)) => {
            (StatusCode::OK, Json(serde_json::json!({"response": response})))
        }
        Ok(Err(_)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "internal error"})),
        ),
        Err(_) => (
            StatusCode::GATEWAY_TIMEOUT,
            Json(serde_json::json!({"error": "timeout"})),
        ),
    }
}

pub(super) fn router() -> Router<Arc<EventBus>> {
    Router::new().route("/", post(chat_handler))
}

pub async fn run_dispatcher(bus: Arc<EventBus>) {
    let mut rx = bus.subscribe();
    while let Some(event) = rx.recv().await {
        if let Some(ref rid) = event.request_id {
            let mut pending = PENDING.write().await;
            if let Some(tx) = pending.remove(rid) {
                let _ = tx.send(event.content);
            }
        }
    }
}
