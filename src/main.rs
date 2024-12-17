use axum::{
    extract::Request,
    http::Response,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use std::{
    env,
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
};
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

    #[cfg(feature = "https")]
    {
        use rustls_acme::{caches::DirCache, AcmeConfig};
        use tokio_stream::StreamExt;

        // Enable TLS via Let's Encrypt:
        let mut state = AcmeConfig::new(vec!["auxv.org"])
            .contact(vec!["mailto:5-pebble@protonmail.com"])
            .cache_option(Some(DirCache::new("lets_encrypt_cache")))
            .directory_lets_encrypt(true)
            .state();
        let acceptor = state.axum_acceptor(state.default_rustls_config());
        tokio::spawn(async move {
            loop {
                match state.next().await.unwrap() {
                    Ok(ok) => println!("event: {:?}", ok),
                    Err(err) => println!("error: {:?}", err),
                }
            }
        });

        // Run the server with HTTPS:
        let address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 443));
        axum_server::bind(address)
            .acceptor(acceptor)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
    #[cfg(not(feature = "https"))]
    {
        // Run the server with HTTP:
        let address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 80));
        axum_server::bind(address)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }

    Ok(())
}

async fn handle_route(request: Request) -> Result<ServerResponse, ServerError> {
    println!("New Request: {}", request.uri().path());
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
