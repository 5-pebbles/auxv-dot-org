use std::path::PathBuf;

use axum::{http::StatusCode, response::IntoResponse};

#[derive(Debug)]
pub enum StartError {
    IoError(tokio::io::Error),
    EncodingError,
}

impl From<tokio::io::Error> for StartError {
    fn from(error: tokio::io::Error) -> Self {
        Self::IoError(error)
    }
}
