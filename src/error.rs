use std::path::PathBuf;

use axum::{http::StatusCode, response::IntoResponse};

#[derive(Debug)]
pub enum ServerError {
    IoError(tokio::io::Error),
    LiquidError(liquid::Error),
    NotFound(PathBuf),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::IoError(error) => {
                (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
            }
            Self::LiquidError(error) => {
                (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
            }
            Self::NotFound(path) => (StatusCode::NOT_FOUND, "404".to_string()).into_response(),
        }
    }
}

impl From<tokio::io::Error> for ServerError {
    fn from(error: tokio::io::Error) -> Self {
        Self::IoError(error)
    }
}

impl From<liquid::Error> for ServerError {
    fn from(error: liquid::Error) -> Self {
        Self::LiquidError(error)
    }
}
