use std::path::PathBuf;

use axum::response::Html;
use pulldown_cmark::{html, Event, Options, Tag, TagEnd};

use crate::error::ServerError;

pub async fn render_markdown(
    markdown_path: PathBuf,
    liquid_globals: liquid::Object,
) -> Result<Html<String>, ServerError> {
    // From Markdown to HTML:
    let markdown = tokio::fs::read_to_string(markdown_path).await?;

    // Configure markdown parsing options:
    let markdown_options = Options::empty()
        | Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_HEADING_ATTRIBUTES;

    // Parse markdown and add IDs to headings:
    let markdown_parser =
        generate_heading_ids(pulldown_cmark::Parser::new_ext(&markdown, markdown_options));

    // Convert markdown to HTML:
    let mut markdown_as_html = String::new();
    html::push_html(&mut markdown_as_html, markdown_parser.into_iter());

    // Inline the template and prepare for rendering:
    let template = include_str!("../assets/templates/template.html").to_string();

    let template_parser = liquid::ParserBuilder::with_stdlib().build()?;
    let template = template_parser.parse(&template.replace("{{html}}", &markdown_as_html))?;
    let rendered_html = template.render(&liquid_globals)?;
    Ok(Html(rendered_html))
}

fn generate_heading_ids<'a>(parser: impl Iterator<Item = Event<'a>>) -> Vec<Event<'a>> {
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
