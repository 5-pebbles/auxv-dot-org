use std::{
    fs::{read_dir, read_to_string},
    path::{Path, PathBuf},
};

use pulldown_cmark::{html, Event, Options, Parser, Tag, TagEnd};
use twemoji::TwemojiParser;

use crate::{error::ServerError, pages::TemplateCache};

pub fn load_pages_recursive(
    mut pages: TemplateCache,
    directory: &Path,
) -> Result<TemplateCache, ServerError> {
    for entry in read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            pages = load_pages_recursive(pages, &path)?;
            continue;
        }

        if let Some(template) = process_markdown_file(&path)? {
            let key_path = generate_key_path(&path)?;
            pages.insert(key_path.into_boxed_path(), template);
        }
    }
    Ok(pages)
}

fn process_markdown_file(path: &Path) -> Result<Option<String>, ServerError> {
    if !path.extension().map_or(false, |ext| ext == "md") {
        return Ok(None);
    }

    let markdown = read_to_string(path).map_err(|_| ServerError::NotFound(path.to_path_buf()))?;

    let markdown_options = Options::empty()
        | Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_HEADING_ATTRIBUTES;

    let markdown_parser = generate_heading_slugs(Parser::new_ext(&markdown, markdown_options));
    let mut html_content = String::new();
    html::push_html(&mut html_content, markdown_parser.into_iter());

    let title = generate_page_title(path)?;
    let html_content = generate_html_content(&html_content, &title);

    let mut emoji_parser = TwemojiParser::inline_from_local_file(PathBuf::from("emojis"));
    let emoji_substitute_content = emoji_parser.parse(&html_content);

    Ok(Some(emoji_substitute_content))
}

fn generate_key_path(path: &Path) -> Result<PathBuf, ServerError> {
    let path_str = path
        .to_str()
        .ok_or_else(|| ServerError::BadRequest("Invalid path encoding".to_string()))?
        .strip_prefix("pages/")
        .expect("Path must be under 'pages' directory")
        .strip_suffix(".md")
        .expect("File must have .md extension");

    Ok(PathBuf::from(path_str))
}

fn generate_page_title(path: &Path) -> Result<String, ServerError> {
    let filename = path
        .file_stem()
        .ok_or_else(|| ServerError::BadRequest("Failed to get file name".to_string()))?
        .to_str()
        .ok_or_else(|| ServerError::BadRequest("Invalid file name encoding".to_string()))?;

    let words: Vec<String> = filename
        .split(|c| c == '-' || c == '_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first_char) => {
                    first_char.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
                None => String::new(),
            }
        })
        .collect();

    Ok(format!("{} | Auxv.org", words.join(" ")))
}

fn generate_html_content(html_content: &str, title: &str) -> String {
    let template = include_str!("../assets/templates/template.html");
    template
        .replace("{{html}}", html_content)
        .replace("{{title}}", title)
}

fn generate_heading_slugs<'a>(parser: impl Iterator<Item = Event<'a>>) -> Vec<Event<'a>> {
    fn generate_slug(text: &str) -> String {
        text.to_lowercase()
            .chars()
            // Keep only alphanumeric characters and spaces
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .trim()
            .replace(' ', "-")
    }

    #[derive(PartialEq)]
    enum ParserState<'a> {
        Normal,
        InHeading {
            original_tag: Tag<'a>,
            text_content: String,
            nested_events: Vec<Event<'a>>,
        },
    }

    let mut state = ParserState::Normal;

    let mut all_events = Vec::new();
    for event in parser {
        state = match (event, state) {
            (Event::Start(original_tag @ Tag::Heading { .. }), ParserState::Normal) => {
                ParserState::InHeading {
                    original_tag,
                    text_content: String::new(),
                    nested_events: Vec::new(),
                }
            }
            (
                Event::Text(text),
                ParserState::InHeading {
                    mut text_content,
                    mut nested_events,
                    original_tag,
                },
            ) => {
                text_content.push_str(&text);
                nested_events.push(Event::Text(text));
                ParserState::InHeading {
                    text_content,
                    nested_events,
                    original_tag,
                }
            }
            (
                event @ Event::End(TagEnd::Heading(_)),
                ParserState::InHeading {
                    original_tag:
                        Tag::Heading {
                            level,
                            id: _,
                            classes,
                            attrs,
                        },

                    text_content,
                    nested_events,
                },
            ) => {
                all_events.push(Event::Start(Tag::Heading {
                    level,
                    id: Some(pulldown_cmark::CowStr::Boxed(
                        generate_slug(&text_content).into_boxed_str(),
                    )),
                    classes,
                    attrs,
                }));
                all_events.extend(nested_events);
                all_events.push(event);
                ParserState::Normal
            }
            (
                event,
                ParserState::InHeading {
                    mut nested_events,
                    original_tag,
                    text_content,
                },
            ) => {
                nested_events.push(event);
                ParserState::InHeading {
                    original_tag,
                    text_content,
                    nested_events,
                }
            }
            (event, ParserState::Normal) => {
                all_events.push(event);
                ParserState::Normal
            }
        }
    }

    all_events
}
