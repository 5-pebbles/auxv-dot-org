#![feature(async_closure)]

use axum::{extract::Request, routing::get, Router};
use std::env::args;
use tower_http::services::ServeDir;

mod error;
mod markdown;
mod template;
use template::render_template;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Router::new()
        .route("/", get(|| async { render_template("index.md").await }))
        .route(
            "/*path",
            get(async |request: Request| {
                render_template(request.uri().path().trim_start_matches('/')).await
            }),
        )
        .nest_service("/assets", ServeDir::new("./assets"));

    #[cfg(not(feature = "https"))]
    let acceptor = {
        use axum_server::accept::DefaultAcceptor;
        DefaultAcceptor::new()
    };

    #[cfg(feature = "https")]
    let acceptor = {
        use rustls_acme::{caches::DirCache, AcmeConfig};
        use tokio_stream::StreamExt;
        // Enable TLS via Let's Encrypt:
        let mut state = AcmeConfig::new(vec!["auxv.org"])
            .contact(vec!["mailto:5-pebble@protonmail.com"])
            .cache_option(Some(DirCache::new("lets_encrypt_cache")))
            .directory_lets_encrypt(true)
            .state();

        let tmp = state.axum_acceptor(state.default_rustls_config());
        tokio::spawn(async move {
            loop {
                match state.next().await.unwrap() {
                    Ok(ok) => println!("Acme Event: {:?}", ok),
                    Err(err) => println!("Acme Error: {:?}", err),
                }
            }
        });

        tmp
    };

    let address = args()
        .nth(1)
        .unwrap_or_else(|| {
            if cfg!(feature = "https") {
                "0.0.0.0:443"
            } else {
                "0.0.0.0:80"
            }
            .to_string()
        })
        .parse()
        .expect("You sure are a dumb ass... I couldn't parse that address.");

    axum_server::bind(address)
        .acceptor(acceptor)
        .serve(app.into_make_service())
        .await
}
