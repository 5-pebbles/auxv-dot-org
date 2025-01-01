use axum::{
    extract::{Path, State},
    response::Html,
};

use crate::state::ServerState;

pub async fn index(State(ServerState { rendered, .. }): State<ServerState>) -> Html<&'static str> {
    Html(
        rendered
            .get("index")
            .unwrap_or_else(|| rendered.get("404").unwrap()),
    )
}
pub async fn render(
    Path(path): Path<String>,
    State(ServerState { rendered, .. }): State<ServerState>,
) -> Html<&'static str> {
    Html(
        rendered
            .get(path.as_str())
            .unwrap_or_else(|| rendered.get("404").unwrap()),
    )
}
