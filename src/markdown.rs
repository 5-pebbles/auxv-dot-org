use std::{
    collections::HashMap,
    fs::{read_dir, read_to_string},
    path::{Path, PathBuf},
    sync::OnceLock,
};

use liquid::{ParserBuilder, Template};
use pulldown_cmark::{html, Event, Options, Parser, Tag, TagEnd};

use crate::error::ServerError;

type TemplateCache = HashMap<Box<Path>, Template>;

static TEMPLATE_CACHE: OnceLock<TemplateCache> = OnceLock::new();

pub fn markdown_template_cache() -> &'static TemplateCache {
    TEMPLATE_CACHE.get_or_init(|| {
        load_pages_recursive(HashMap::new(), Path::new("pages"))
            .expect("Failed to initialize template cache")
    })
}

fn load_pages_recursive(
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

fn process_markdown_file(path: &Path) -> Result<Option<Template>, ServerError> {
    if !path.extension().map_or(false, |ext| ext == "md") {
        return Ok(None);
    }

    let markdown = read_to_string(path).map_err(|_| ServerError::NotFound(path.to_path_buf()))?;

    let markdown_options = Options::empty()
        | Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_HEADING_ATTRIBUTES;

    let markdown_parser = generate_heading_slugs(Parser::new_ext(&markdown, markdown_options));
    let mut html_output = String::new();
    html::push_html(&mut html_output, markdown_parser.into_iter());

    let title = generate_page_title(path)?;
    let template_content = generate_template_content(&html_output, &title);

    let liquid_parser = ParserBuilder::with_stdlib()
        .build()
        .expect("Failed to build liquid parser");

    let template = liquid_parser.parse(&template_content)?;

    Ok(Some(template))
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

fn generate_template_content(html_content: &str, title: &str) -> String {
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
