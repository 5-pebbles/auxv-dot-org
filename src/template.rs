use std::path::{Path, PathBuf};

use axum::response::Html;

use crate::{error::ServerError, markdown::markdown_template_cache};

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

pub async fn render_template(template_path: &str) -> Result<Html<String>, ServerError> {
    let pages = markdown_template_cache();
    let rendered_html = pages
        .get(Path::new(template_path))
        .ok_or_else(|| ServerError::NotFound(PathBuf::new().join(template_path)))?
        .render(&liquid_object())?;
    Ok(Html(rendered_html))
}
