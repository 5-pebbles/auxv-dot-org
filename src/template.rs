use std::path::{Path, PathBuf};

use axum::response::Html;

use crate::{error::ServerError, markdown::markdown_template_cache};

pub async fn render_template(template_path: &str) -> Result<Html<String>, ServerError> {
    let pages = markdown_template_cache();

    let rendered_html = pages
        .get(Path::new(template_path))
        .ok_or_else(|| ServerError::NotFound(PathBuf::new().join(template_path)))?;
    Ok(rendered_html.to_owned())
}
