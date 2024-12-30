use axum::response::Html;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use crate::{error::ServerError, markdown::load_pages_recursive};

pub type TemplateCache = HashMap<Box<Path>, Box<str>>;

static TEMPLATE_CACHE: OnceLock<TemplateCache> = OnceLock::new();

pub fn page_cache() -> &'static TemplateCache {
    TEMPLATE_CACHE.get_or_init(|| {
        load_pages_recursive(HashMap::new(), Path::new("pages"))
            .expect("Failed to initialize template cache")
    })
}

pub async fn fetch_page(template_path: &str) -> Result<Html<&'static str>, ServerError> {
    page_cache()
        .get(Path::new(template_path))
        .map(|p| Html(p.as_ref()))
        .ok_or_else(|| ServerError::NotFound(PathBuf::new().join(template_path)))
}
