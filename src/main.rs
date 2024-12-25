#![feature(async_closure)]

use axum::{extract::Request, routing::get, Router};
use std::{env::args, path::PathBuf};
use tower_http::services::ServeDir;

mod error;
mod markdown;
use axum_server::accept::DefaultAcceptor;
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

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Router::new()
        .route(
            "/",
            get(|| async {
                render_markdown(PathBuf::from("pages/index.md"), liquid_object()).await
            }),
        )
        .route(
            "/*path",
            get(async |request: Request| {
                let file_path =
                    PathBuf::from("pages").join(request.uri().path().trim_start_matches('/'));
                render_markdown(file_path.with_extension("md"), liquid_object()).await
            }),
        )
        .nest_service("/assets", ServeDir::new("./assets"));

    #[cfg(not(feature = "https"))]
    let acceptor = { DefaultAcceptor::new() };

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
