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
    fn walk_dir_paths(
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

    let template = read_to_string(Path::new(PAGE_CACHE_DIR).join("templates/template.html"))?;
    let emoji_parser = EmojiParser::new(Path::new(PAGE_CACHE_DIR).join("emojis"))?;
    let markdown_options = Options::empty()
        | Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_HEADING_ATTRIBUTES;

    let pages = walk_dir_paths(PAGE_CACHE_DIR)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|path| path.extension().map_or(false, |ext| ext == "md"))
        .map(|path| {
            let title = generate_title(&path);
            let markdown = read_to_string(&path)?;
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

            let html = template
                .clone()
                .replace("{{html}}", &markdown_as_html)
                .replace("{{title}}", &title);

            let emoji_substitute_content = emoji_parser.inline_from_directory(&html);
            let leaked_emoji_substitute_content: &'static str =
                Box::leak(emoji_substitute_content.into_boxed_str());

            Ok((url, leaked_emoji_substitute_content))
        })
        .collect::<Result<HashMap<_, _>, std::io::Error>>()?;

    unsafe {
        PAGE_CACHE.write(pages);
    }

    Ok(())
}

fn generate_title(path: &Path) -> String {
    let filename = path.file_stem().unwrap().to_str().unwrap();

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

    format!("{} | Auxv.org", words.join(" "))
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
