use axum::{
    extract::Request,
    http::Response,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use std::{env, path::PathBuf};
use tower::ServiceExt;
use tower_http::services::{fs::ServeFileSystemResponseBody, ServeDir};

mod error;
mod markdown;
use error::ServerError;
use markdown::render_markdown;

fn liquid_object() -> liquid::Object {
    liquid::object!({
        "name": "Owen Friedman",
        "email": "5-pebble@protonmail.com",
        "phone": "(502) 230-9990",
        "github": "https://github.com/5-pebbles",
        "title": "Auxv",
        "version": env!("CARGO_PKG_VERSION"),
    })
}

enum ServerResponse {
    File(Response<ServeFileSystemResponseBody>),
    Html(Html<String>),
}

impl IntoResponse for ServerResponse {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::File(file) => file.into_response(),
            Self::Html(html) => html.into_response(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure routes:
    let app = Router::new()
        .route(
            "/",
            get(|| async {
                render_markdown(PathBuf::from("frontend/index.md"), liquid_object()).await
            }),
        )
        .route("/*path", get(handle_route));

    // Determine server address (default to 0.0.0.0:3000)
    let address = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:3000".to_string());

    println!("Listening on: {}", address);

    // Create TCP listener and start server:
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_route(request: Request) -> Result<ServerResponse, ServerError> {
    let file_path = PathBuf::from("frontend").join(request.uri().path().trim_start_matches('/'));

    if file_path.extension().is_some() {
        return Ok(serve_static_file(request).await);
    }

    // If the file has no extension, treat it as a markdown template:
    return render_markdown(file_path.with_extension("md"), liquid_object()).await;
}

async fn serve_static_file(request: Request) -> ServerResponse {
    // ServeDir is infallible:
    // TODO: Rewrite tower_http::services::ServeFile it's just a ServeDir wrapper and a bad one at that...
    unsafe {
        ServerResponse::File(
            ServeDir::new(PathBuf::from("./frontend"))
                .oneshot(request)
                .await
                .unwrap_unchecked(),
        )
    }
}
