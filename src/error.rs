use axum::{http::StatusCode, response::IntoResponse};

pub enum ServerError {
    IoError(tokio::io::Error),
    LiquidError(liquid::Error),
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
