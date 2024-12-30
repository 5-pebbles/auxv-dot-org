use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use axum::response::Html;
use twemoji::TwemojiParser;

use crate::{error::ServerError, markdown::markdown_template_cache};

pub async fn render_template(template_path: &str) -> Result<Html<String>, ServerError> {
    let pages = markdown_template_cache();

    let rendered_html = pages
        .get(Path::new(template_path))
        .ok_or_else(|| ServerError::NotFound(PathBuf::new().join(template_path)))?
        .render(&liquid_object())?;

    let mut emoji_parser =
        TwemojiParser::inline_from_local_file(PathBuf::from_str("assets/emoji/svg").unwrap());
    // let mut emoji_parser = TwemojiParser::link_from_url(
    //     PathBuf::from_str("/assets/emoji/svg").unwrap(),
    //     "svg".to_string(),
    // );
    let html_with_inlined_emojis = emoji_parser.parse(&rendered_html);

    Ok(Html(html_with_inlined_emojis))
}

fn liquid_object() -> liquid::Object {
    // NOTE: {{html}} and {{title}} are set in the cache.
    liquid::object!({
        "name": "Owen Friedman",
        "email": "5-pebble@protonmail.com",
        "phone": "(502) 230-9990",
        "github": "https://github.com/5-pebbles",
        "version": env!("CARGO_PKG_VERSION"),
    })
}
