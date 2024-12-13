use axum::{
    extract::Request,
    http::{Response, StatusCode},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use liquid::ParserBuilder as LiquidParserBuilder;
use pulldown_cmark::{html, CowStr, Event, Options, Parser as MarkdownParser, Tag, TagEnd};
use std::{env, path::PathBuf};
use tower::ServiceExt;
use tower_http::services::{fs::ServeFileSystemResponseBody, ServeDir};

enum ServerResponse {
    File(Response<ServeFileSystemResponseBody>),
    Html(Html<String>),
}

impl IntoResponse for ServerResponse {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::File(file) => file.into_response(),
            Self::Html(html) => html.into_response(),
        }
    }
}

enum ServerError {
    IoError(tokio::io::Error),
    LiquidError(liquid::Error),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::IoError(error) => {
                (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
            }
            Self::LiquidError(error) => {
                (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
            }
        }
    }
}

impl From<tokio::io::Error> for ServerError {
    fn from(error: tokio::io::Error) -> Self {
        Self::IoError(error)
    }
}

impl From<liquid::Error> for ServerError {
    fn from(error: liquid::Error) -> Self {
        Self::LiquidError(error)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure routes:
    let app = Router::new()
        .route(
            "/",
            get(|| async { render_markdown(PathBuf::from("frontend/index.md")).await }),
        )
        .route("/*path", get(handle_route));

    // Determine server address (default to 0.0.0.0:3000)
    let address = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:3000".to_string());

    println!("Listening on: {}", address);

    // Create TCP listener and start server:
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_route(request: Request) -> Result<ServerResponse, ServerError> {
    let file_path = PathBuf::from("frontend").join(request.uri().path().trim_start_matches('/'));

    if file_path.extension().is_some() {
        return Ok(serve_static_file(request).await);
    }

    // If the file has no extension, treat it as a markdown template:
    return render_markdown(file_path.with_extension("md")).await;
}

async fn serve_static_file(request: Request) -> ServerResponse {
    // ServeDir is infallible:
    // TODO: Rewrite tower_http::services::ServeFile it's just a ServeDir wrapper and a bad one at that...
    unsafe {
        ServerResponse::File(
            ServeDir::new(PathBuf::from("./frontend"))
                .oneshot(request)
                .await
                .unwrap_unchecked(),
        )
    }
}

async fn render_markdown(markdown_path: PathBuf) -> Result<ServerResponse, ServerError> {
    // From Markdown to HTML:
    let markdown = tokio::fs::read_to_string(markdown_path).await?;

    // Configure markdown parsing options:
    let markdown_options = Options::empty()
        | Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_HEADING_ATTRIBUTES;

    // Parse markdown and add IDs to headings:
    let markdown_parser =
        generate_heading_ids(MarkdownParser::new_ext(&markdown, markdown_options));

    // Convert markdown to HTML:
    let mut markdown_as_html = String::new();
    html::push_html(&mut markdown_as_html, markdown_parser.into_iter());

    // Inline the template and prepare for rendering:
    let template = include_str!("../template.html").to_string();
    let globals = liquid::object!({
        "name": "Owen Friedman",
        "email": "5-pebble@protonmail.com",
        "phone": "(502) 230-9990",
        "github": "https://github.com/5-pebbles",
        "title": "Auxv",
        "version": env!("CARGO_PKG_VERSION"),
    });

    let template_parser = LiquidParserBuilder::with_stdlib().build()?;
    let template = template_parser.parse(&template.replace("{{html}}", &markdown_as_html))?;
    let rendered_html = template.render(&globals)?;
    Ok(ServerResponse::Html(Html(rendered_html)))
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
                    id: Some(CowStr::Boxed(generate_slug(&text_content).into_boxed_str())),
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
