use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    error::StartError,
    html::render_html,
    markdown::{load_markdown_recursive, MarkdownPage},
};

#[derive(Clone)]
pub struct ServerState {
    pub searchable: Vec<MarkdownPage>,
    pub rendered: HashMap<&'static str, &'static str>,
}

impl ServerState {
    pub fn new() -> Result<Self, StartError> {
        let raw_markdown = load_markdown_recursive(Vec::new(), Path::new("pages"))?;
        let rendered_html = render_html(&raw_markdown)?;

        Ok(Self {
            searchable: raw_markdown,
            rendered: rendered_html,
        })
    }
}
