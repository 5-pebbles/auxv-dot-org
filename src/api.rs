use std::path::{Path, PathBuf};

use either::Either;
use rocket::{
    fs::NamedFile,
    get,
    response::content::RawHtml,
    serde::{Serialize, json::Json},
};

use crate::pages::{self, PAGE_CACHE_DIR};

#[get("/")]
pub async fn index() -> RawHtml<&'static str> {
    pages::get_page_cache()
        .get(Path::new("index"))
        .cloned()
        .map(RawHtml)
        .unwrap()
}

#[get("/<path..>")]
pub async fn html_or_file(path: PathBuf) -> Option<Either<RawHtml<&'static str>, NamedFile>> {
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
    let before_start = content[..start]
        .char_indices()
        .rev()
        .nth(19)
        .map_or(0, |(i, _)| i);
    let after_end = content[end..]
        .char_indices()
        .nth(29)
        .map_or(content.len(), |(i, _)| end + i);

    format!(
        "{}<b>{}</b>{}",
        escape_html(&content[before_start..start]),
        escape_html(&content[start..end]),
        escape_html(&content[end..after_end])
    )
}

#[get("/search?<query>")]
pub async fn search(query: &str) -> Json<Vec<QueryMatch>> {
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
pub fn not_found() -> RawHtml<&'static str> {
    pages::get_page_cache()
        .get(Path::new("404"))
        .cloned()
        .map(RawHtml)
        .unwrap_or_else(|| RawHtml("404 - Page not found"))
}
