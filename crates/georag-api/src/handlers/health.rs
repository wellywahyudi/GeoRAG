use axum::{response::IntoResponse, Json};

use crate::dto::HealthResponse;

pub async fn health_check() -> impl IntoResponse {
    Json(HealthResponse::default())
}
