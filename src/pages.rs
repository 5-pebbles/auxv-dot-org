#![allow(static_mut_refs)]

use std::{
    collections::HashMap,
    fs::{read_dir, read_to_string},
    mem::MaybeUninit,
    path::Path,
};

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use twemoji::TwemojiParser;

pub const PAGE_CACHE_DIR: &Path = unsafe { std::mem::transmute("./pages") };
static mut PAGE_CACHE: MaybeUninit<HashMap<&'static Path, &'static str>> = MaybeUninit::uninit();

pub fn get_page_cache() -> &'static HashMap<&'static Path, &'static str> {
    unsafe { PAGE_CACHE.assume_init_ref() }
}

pub fn set_page_cache(directory: &Path) -> Result<(), std::io::Error> {
    unsafe {
        PAGE_CACHE.write(recursive_load_directory(HashMap::new(), directory)?);
    }

    Ok(())
}

fn recursive_load_directory(
    mut pages: HashMap<&'static Path, &'static str>,
    directory: &Path,
) -> Result<HashMap<&'static Path, &'static str>, std::io::Error> {
    for entry in read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            pages = recursive_load_directory(pages, &path)?;
            continue;
        }

        if !path.extension().map_or(false, |ext| ext == "md") {
            continue;
        }

        let title = generate_title(&path);
        let markdown = read_to_string(&path)?;
        let url = Box::leak(
            path.strip_prefix(PAGE_CACHE_DIR)
                .unwrap()
                .with_extension("")
                .into_boxed_path(),
        );

        let markdown_options = Options::empty()
            | Options::ENABLE_TABLES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_HEADING_ATTRIBUTES;

        let markdown_parser = generate_heading_slugs(Parser::new_ext(&markdown, markdown_options));
        let mut markdown_as_html = String::new();
        pulldown_cmark::html::push_html(&mut markdown_as_html, markdown_parser.into_iter());

        let template = read_to_string(Path::new(PAGE_CACHE_DIR).join("templates/template.html"))?;

        let html = template
            .clone()
            .replace("{{html}}", &markdown_as_html)
            .replace("{{title}}", &title);

        let mut emoji_parser =
            TwemojiParser::inline_from_local_file(Path::new(PAGE_CACHE_DIR).join("emojis"));
        let emoji_substitute_content = emoji_parser.parse(&html);
        let leaked_emoji_substitute_content = Box::leak(emoji_substitute_content.into_boxed_str());

        pages.insert(url, leaked_emoji_substitute_content);
    }

    Ok(pages)
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
