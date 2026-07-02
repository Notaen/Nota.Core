use axum::{Router, routing::get};


pub(super) fn router() -> Router {
    Router::new()
    .route("/new", get(new))
}

async fn new() {
    todo!()
}
