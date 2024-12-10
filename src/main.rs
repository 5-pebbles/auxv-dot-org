use axum::{http::StatusCode, response::Html, routing::get, Router};
use liquid::ParserBuilder;
use pulldown_cmark::{html, Event, Options, Parser as MarkdownParser, Tag, TagEnd};
use std::{env::args, path::PathBuf};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route(
            "/",
            get(|| async { render_template(axum::extract::Path("index".to_string())).await }),
        )
        .route("/:name", get(render_template))
        .nest_service("/static", ServeDir::new("./static"));

    let address = args().nth(1).unwrap_or_else(|| "0.0.0.0:3000".to_string());
    print!("Address: {}", address);
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn render_template(
    axum::extract::Path(name): axum::extract::Path<String>,
) -> (StatusCode, Html<String>) {
    let template_path = PathBuf::from("./template.html");
    let template_content = match tokio::fs::read_to_string(template_path).await {
        Ok(content) => content,
        Err(_) => return error_page(StatusCode::NOT_FOUND).await,
    };

    let markdown_path = PathBuf::from(format!("./markdown/{}.md", name));
    let markdown_content = match tokio::fs::read_to_string(markdown_path).await {
        Ok(content) => content,
        Err(_) => return error_page(StatusCode::NOT_FOUND).await,
    };
    let mut markdown_options = Options::empty();
    markdown_options.insert(Options::ENABLE_TABLES);
    markdown_options.insert(Options::ENABLE_STRIKETHROUGH);
    markdown_options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

    // For your sanity run!!!!
    let markdown_parser = MarkdownParser::new_ext(&markdown_content, markdown_options);
    let (mut heading, mut id) = (None, String::new());
    let markdown_parser =
        markdown_parser.flat_map(move |event| match (event.to_owned(), heading.to_owned()) {
            (Event::Start(heading_tag @ Tag::Heading { .. }), None) => {
                heading = Some(heading_tag);
                id.clear();
                vec![]
            }
            (Event::Text(text), Some(_)) => {
                id.push_str(&text);
                vec![]
            }
            (Event::End(TagEnd::Heading(_)), Some(heading_tag)) => {
                heading = None;
                let new_heading = match heading_tag {
                    Tag::Heading {
                        level,
                        id: idn,
                        classes,
                        attrs,
                    } => Tag::Heading {
                        level,
                        id: Some(idn.unwrap_or_else(|| {
                            pulldown_cmark::CowStr::Boxed(
                                id.to_owned()
                                    .to_lowercase()
                                    .replace(" ", "-")
                                    .into_boxed_str(),
                            )
                        })),
                        classes,
                        attrs,
                    },
                    _ => unreachable!(),
                };
                vec![
                    Event::Start(new_heading),
                    Event::Text(pulldown_cmark::CowStr::Boxed(
                        id.to_owned().into_boxed_str(),
                    )),
                    event,
                ]
            }
            _ => vec![event],
        });
    let mut markdown_as_html = String::new();
    html::push_html(&mut markdown_as_html, markdown_parser);

    let template_parser = match ParserBuilder::with_stdlib().build() {
        Ok(parser) => parser,
        Err(_) => return error_page(StatusCode::INTERNAL_SERVER_ERROR).await,
    };
    let title = format!("{} - Miros", capitalize(&name));
    let globals = liquid::object!({
        "name": "Owen Friedman",
        "email": "5-pebble@protonmail.com",
        "phone": "(502) 230-9990",
        "github": "https://github.com/5-pebbles",
        "title": title,
        "version": env!("CARGO_PKG_VERSION"),
    });
    let html = template_parser.parse(&markdown_as_html).unwrap();
    let markdown_as_html = match html.render(&globals) {
        Ok(render) => render,
        Err(_) => return error_page(StatusCode::INTERNAL_SERVER_ERROR).await,
    };

    let globals = liquid::object!({
        "name": "Owen Friedman",
        "email": "5-pebble@protonmail.com",
        "phone": "(502) 230-9990",
        "github": "https://github.com/5-pebbles",
        "title": title,
        "version": env!("CARGO_PKG_VERSION"),
        "html": markdown_as_html,
    });
    let template = template_parser.parse(&template_content).unwrap();
    let render = match template.render(&globals) {
        Ok(render) => render,
        Err(_) => return error_page(StatusCode::INTERNAL_SERVER_ERROR).await,
    };

    (StatusCode::OK, Html(render))
}

async fn error_page(code: StatusCode) -> (StatusCode, Html<String>) {
    (
        code,
        Html(
            tokio::fs::read_to_string(format!("./errors/{}.html", code))
                .await
                .unwrap_or(code.to_string()),
        ),
    )
}

// Utils

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
