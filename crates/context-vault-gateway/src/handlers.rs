//! HTTP handlers and the bearer-auth guard for the gateway routes.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use context_vault_core::engine::{SaveInput, SearchOptions};
use serde::Deserialize;
use serde_json::json;

use crate::AppState;

/// Bearer-auth guard: rejects any request whose `Authorization: Bearer <token>`
/// doesn't match GATEWAY_API_KEY. Applied to all routes except /health.
pub(crate) async fn auth(State(state): State<Arc<AppState>>, req: axum::extract::Request, next: Next) -> Response {
    let presented = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));
    if presented == Some(state.api_key.as_str()) {
        return next.run(req).await;
    }
    (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Invalid or missing API key" }))).into_response()
}

pub(crate) async fn health_handler() -> Response {
    (StatusCode::OK, Json(json!({ "ok": true }))).into_response()
}

pub(crate) async fn status_handler(State(state): State<Arc<AppState>>) -> Response {
    match state.engine.stats().await {
        Ok(s) => (
            StatusCode::OK,
            Json(json!({
                "total_entries": s.total_entries,
                "indexed_entries": s.indexed_entries,
                "by_kind": s.by_kind,
                "by_tier": s.by_tier,
            })),
        )
            .into_response(),
        Err(e) => server_error(e),
    }
}

pub(crate) async fn save_handler(State(state): State<Arc<AppState>>, Json(input): Json<SaveInput>) -> Response {
    match state.engine.save(input).await {
        Ok(out) => (StatusCode::OK, Json(json!({ "id": out.id }))).into_response(),
        Err(e) => server_error(e),
    }
}

#[derive(Deserialize)]
pub(crate) struct SearchParams {
    query: String,
    limit: Option<usize>,
}

pub(crate) async fn search_handler(State(state): State<Arc<AppState>>, Query(p): Query<SearchParams>) -> Response {
    let opts = SearchOptions { limit: p.limit.unwrap_or(10), ..Default::default() };
    match state.engine.hybrid_search(&p.query, opts).await {
        Ok(results) => {
            let items: Vec<_> = results
                .iter()
                .map(|r| json!({ "id": r.entry.id, "title": r.entry.title, "score": r.score }))
                .collect();
            (StatusCode::OK, Json(json!({ "results": items }))).into_response()
        }
        Err(e) => server_error(e),
    }
}

pub(crate) async fn get_handler(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    match state.engine.get(&id).await {
        Ok(Some(entry)) => (StatusCode::OK, Json(json!(entry))).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => server_error(e),
    }
}

pub(crate) async fn delete_handler(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    match state.engine.delete(&id).await {
        Ok(()) => (StatusCode::OK, Json(json!({ "deleted": id }))).into_response(),
        Err(e) => server_error(e),
    }
}

fn server_error(e: anyhow::Error) -> Response {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response()
}
