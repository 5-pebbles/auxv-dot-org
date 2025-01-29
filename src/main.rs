#[macro_use]
extern crate rocket;

use std::{
    net::Ipv4Addr,
    path::{Path, PathBuf},
};

use either::Either;
use rocket::{
    fs::NamedFile,
    response::content::RawHtml,
    serde::{Serialize, json::Json},
};

use crate::pages::PAGE_CACHE_DIR;

#[cfg(feature = "https")]
mod lets_encrypt_listener;

mod pages;

#[get("/")]
async fn index() -> RawHtml<&'static str> {
    pages::get_page_cache()
        .get(Path::new("index"))
        .cloned()
        .map(RawHtml)
        .unwrap()
}

#[get("/<path..>")]
async fn html_or_file(path: PathBuf) -> Option<Either<RawHtml<&'static str>, NamedFile>> {
    if path.extension().is_some() {
        NamedFile::open(Path::new(PAGE_CACHE_DIR).join(path))
            .await
            .ok()
            .map(Either::Right)
    } else {
        pages::get_page_cache()
            .get(path.as_path())
            .cloned()
            .map(RawHtml)
            .map(Either::Left)
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct QueryMatch {
    title: &'static str,
    path: String,
    matched: String,
}

fn escape_html(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

fn get_match_context(content: &str, query: &str) -> String {
    let start = content.find(query).unwrap();
    let end = start + query.len();

    let before_start = start.saturating_sub(20);
    let after_end = (end + 30).min(content.len());

    let before = &content[before_start..start];
    let matched_part = &content[start..end];
    let after = &content[end..after_end];

    format!(
        "{}<b>{}</b>{}",
        escape_html(before),
        escape_html(matched_part),
        escape_html(after)
    )
}

#[get("/search?<query>")]
async fn search(query: &str) -> Json<Vec<QueryMatch>> {
    let query_matches = pages::get_page_cache()
        .into_iter()
        .filter_map(|(path, html)| {
            let path_str = path.to_string_lossy();
            let html_contains = html.contains(query);
            let path_contains = path_str.contains(query);

            if !html_contains && !path_contains {
                return None;
            }

            let title = html
                .find("<title>")
                .and_then(|i| {
                    html[i..]
                        .find("</title>")
                        .map(|j| html[i + 7..i + j].trim())
                })
                .unwrap_or_else(|| path.to_str().unwrap_or("Untitled"));

            let matched = if html_contains {
                get_match_context(html, query)
            } else {
                get_match_context(&path_str, query)
            };

            Some(QueryMatch {
                title,
                path: path_str.to_string(),
                matched,
            })
        })
        .collect();

    Json(query_matches)
}

#[catch(404)]
fn not_found() -> RawHtml<&'static str> {
    pages::get_page_cache()
        .get(Path::new("404"))
        .cloned()
        .map(RawHtml)
        .unwrap_or_else(|| RawHtml("404 - Page not found"))
}

#[rocket::main]
async fn main() {
    pages::set_page_cache(Path::new(PAGE_CACHE_DIR)).unwrap();

    let rocket = rocket::build()
        .mount("/", routes![index, html_or_file, search])
        .register("/", catchers![not_found]);

    #[cfg(not(feature = "https"))]
    {
        use rocket::listener::tcp::TcpListener;

        let tcp_listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 80))
            .await
            .unwrap();
        rocket.launch_on(tcp_listener).await.unwrap();
    }

    #[cfg(feature = "https")]
    {
        use lets_encrypt_listener::LetsEncryptListener;
        use rustls_acme::{AcmeConfig, caches::DirCache};
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };

        // HTTP Listener for redirection:
        let http_listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 80))
            .await
            .unwrap();
        tokio::spawn(async move {
            loop {
                if let Ok((mut socket, _)) = http_listener.accept().await {
                    tokio::spawn(async move {
                        let mut buf = [0; 1024];
                        if socket.read(&mut buf).await.is_ok() {
                            // Simple HTTP 301 redirect response
                            let response = "HTTP/1.1 301 Moved Permanently\r\n\
                                          Location: https://auxv.org\r\n\
                                          Connection: close\r\n\r\n";
                            let _ = socket.write_all(response.as_bytes()).await;
                        }
                    });
                }
            }
        });

        // Enable HTTPS via Let's Encrypt:
        let tcp_listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 443))
            .await
            .unwrap();
        let acme_config = AcmeConfig::new(["auxv.org"])
            .contact(["mailto:5-pebble@protonmail.com"])
            .cache_option(Some(DirCache::new("lets_encrypt_cache")))
            .directory_lets_encrypt(true);
        let https_listener = LetsEncryptListener::new(acme_config, tcp_listener).await;
        rocket.launch_on(https_listener).await.unwrap();
    }
}
