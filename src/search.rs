use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::state::ServerState;

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
}

#[derive(Serialize)]
pub struct SearchResult {
    title: &'static str,
    url: &'static str,
    short: &'static str,
}

pub async fn search(
    Query(SearchQuery { q: query }): Query<SearchQuery>,
    State(ServerState { searchable, .. }): State<ServerState>,
) -> Json<Vec<SearchResult>> {
    Json(
        searchable
            .iter()
            .filter(|p| p.title.contains(&query) || p.raw_text.contains(&query))
            .map(|p| SearchResult {
                title: p.title,
                url: p.url,
                short: p.short,
            })
            .collect(),
    )
}
