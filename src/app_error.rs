// see https://github.com/tokio-rs/axum/blob/a192480c4f8acef6a490598cf8cc3eedd1071404/examples/anyhow-error-response/src/main.rs
use axum::{http::StatusCode, response::{IntoResponse, Response}};

pub struct AppError(anyhow::Error);

impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        AppError(value)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}