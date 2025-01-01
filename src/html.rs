use std::{
    collections::HashMap,
    fs::read_to_string,
    path::{Path, PathBuf},
};

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use twemoji::TwemojiParser;

use crate::{error::StartError, markdown::MarkdownPage};

pub fn render_html(
    markdown_pages: &Vec<MarkdownPage>,
) -> Result<HashMap<&'static str, &'static str>, StartError> {
    let template = read_to_string(Path::new("assets/templates/template.html"))?;
    let mut html: HashMap<&'static str, &'static str> = HashMap::new();

    for MarkdownPage {
        title,
        markdown: text,
        url,
        ..
    } in markdown_pages
    {
        let markdown_options = Options::empty()
            | Options::ENABLE_TABLES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_HEADING_ATTRIBUTES;

        let markdown_parser = generate_heading_slugs(Parser::new_ext(text, markdown_options));
        let mut markdown_content_as_html = String::new();
        pulldown_cmark::html::push_html(&mut markdown_content_as_html, markdown_parser.into_iter());

        let title = format!("{} | Auxv.org", title);
        let html_content = template
            .clone()
            .replace("{{html}}", &markdown_content_as_html)
            .replace("{{title}}", &title);

        let mut emoji_parser = TwemojiParser::inline_from_local_file(PathBuf::from("emojis"));
        let emoji_substitute_content = emoji_parser.parse(&html_content);

        let static_html = Box::leak(emoji_substitute_content.into_boxed_str());

        html.insert(url, static_html);
    }

    Ok(html)
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
