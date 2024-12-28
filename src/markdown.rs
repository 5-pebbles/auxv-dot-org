use std::{collections::HashMap, path::Path, sync::OnceLock};

use liquid::{ParserBuilder, Template};
use pulldown_cmark::{html, Event, Options, Tag, TagEnd};

pub fn markdown_template_cache() -> &'static HashMap<Box<Path>, Template> {
    fn load_pages_recursive(
        mut pages: HashMap<Box<Path>, Template>,
        directory: &Path,
    ) -> HashMap<Box<Path>, Template> {
        for entry in std::fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();

            let markdown_path = entry.path();

            if markdown_path.is_dir() {
                pages = load_pages_recursive(pages, &markdown_path);
                continue;
            }

            let markdown = std::fs::read_to_string(&markdown_path).unwrap();

            // Configure markdown parsing options:
            let markdown_options = Options::empty()
                | Options::ENABLE_TABLES
                | Options::ENABLE_STRIKETHROUGH
                | Options::ENABLE_HEADING_ATTRIBUTES;

            // Parse markdown and add IDs to headings:
            let markdown_parser = generate_heading_slugs(pulldown_cmark::Parser::new_ext(
                &markdown,
                markdown_options,
            ));

            // Convert markdown to HTML:
            let mut markdown_as_html = String::new();
            html::push_html(&mut markdown_as_html, markdown_parser.into_iter());

            let tmp = markdown_path.into_os_string().into_string().unwrap();
            let unique_path = tmp
                .strip_prefix("pages/")
                .unwrap()
                .strip_suffix(".md")
                .unwrap();
            let mut words = unique_path
                .to_string()
                .split("/")
                .last()
                .unwrap()
                .split(|c| c == '-' || c == '_')
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(first_char) => {
                            first_char.to_uppercase().collect::<String>()
                                + &chars.as_str().to_lowercase()
                        }
                        None => String::new(),
                    }
                })
                .collect::<Vec<String>>();
            words.push(" | Auxv.org".to_string());
            let title = words.join(" ");

            // Inline the template and prepare for rendering:
            let mut template_content =
                include_str!("../assets/templates/template.html").to_string();
            template_content = template_content.replace("{{html}}", &markdown_as_html);
            template_content = template_content.replace("{{title}}", &title);

            let liquid_parser = ParserBuilder::with_stdlib().build().unwrap();
            let template = liquid_parser.parse(&template_content).unwrap();

            pages.insert(Path::new(unique_path).into(), template);
        }
        pages
    }

    static HASHMAP: OnceLock<HashMap<Box<Path>, Template>> = OnceLock::new();
    HASHMAP.get_or_init(|| load_pages_recursive(HashMap::new(), Path::new("pages")))
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
