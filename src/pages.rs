#![allow(static_mut_refs)]

use std::{
    collections::{HashMap, VecDeque},
    fs::{read_dir, read_to_string},
    mem::MaybeUninit,
    path::{Path, PathBuf},
};

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::emojis::EmojiParser;

pub const PAGE_CACHE_DIR: &Path = unsafe { std::mem::transmute("./pages") };
static mut PAGE_CACHE: MaybeUninit<HashMap<&'static Path, &'static str>> = MaybeUninit::uninit();

pub fn get_page_cache() -> &'static HashMap<&'static Path, &'static str> {
    unsafe { PAGE_CACHE.assume_init_ref() }
}

pub fn set_page_cache() -> Result<(), std::io::Error> {
    let template_html = read_to_string(Path::new(PAGE_CACHE_DIR).join("templates/template.html"))?;
    let emoji_parser = EmojiParser::new(Path::new(PAGE_CACHE_DIR).join("emojis"))?;
    let markdown_options = Options::empty()
        | Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_HEADING_ATTRIBUTES;

    let pages = read_dir_all(PAGE_CACHE_DIR)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|path| path.extension().map_or(false, |ext| ext == "md"))
        .map(|path| {
            let page = read_to_string(&path)?;
            let (head, markdown) = parse_head(&page).unwrap_or(("", &page));
            let url: &'static Path = Box::leak(
                path.strip_prefix(PAGE_CACHE_DIR)
                    .unwrap()
                    .with_extension("")
                    .into_boxed_path(),
            );

            let markdown_parser =
                generate_heading_slugs(Parser::new_ext(&markdown, markdown_options));
            let mut markdown_as_html = String::new();
            pulldown_cmark::html::push_html(&mut markdown_as_html, markdown_parser.into_iter());

            let emoji_substitute_markdown_as_html =
                emoji_parser.inline_from_directory(&markdown_as_html);

            let rendered_html = template_html
                .clone()
                .replace("{{html}}", &emoji_substitute_markdown_as_html)
                .replace("{{head}}", &head);

            let leaked_html: &'static str = Box::leak(rendered_html.into_boxed_str());

            Ok((url, leaked_html))
        })
        .collect::<Result<HashMap<_, _>, std::io::Error>>()?;

    unsafe {
        PAGE_CACHE.write(pages);
    }

    Ok(())
}

fn parse_head(markdown: &str) -> Option<(&str, &str)> {
    let prefix_delimiter = "<head>\n";
    let suffix_delimiter = "\n</head>\n";
    if !markdown.starts_with(prefix_delimiter) {
        return None;
    }

    let suffix_index = markdown[prefix_delimiter.len()..].find(suffix_delimiter)?;
    let head = &markdown[prefix_delimiter.len()..prefix_delimiter.len() + suffix_index];
    let remaining = &markdown[prefix_delimiter.len() + suffix_index + suffix_delimiter.len()..];

    Some((head, remaining))
}

fn read_dir_all(
    directory: impl AsRef<Path>,
) -> std::io::Result<impl Iterator<Item = std::io::Result<PathBuf>>> {
    let mut queue = VecDeque::new();
    queue.extend(read_dir(directory)?);

    Ok(std::iter::from_fn(move || {
        while let Some(entry) = queue.pop_front() {
            let path = match entry {
                Ok(v) => v.path(),
                Err(e) => return Some(Err(e)),
            };
            if path.is_dir() {
                queue.extend(match read_dir(path) {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e)),
                });
                continue;
            }
            return Some(Ok(path));
        }
        None
    }))
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
