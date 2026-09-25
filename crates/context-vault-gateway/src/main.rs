//! `context-vault-gateway` — self-hosted HTTP wrapper over the Rust engine.
//!
//! Port of `packages/hosted-gateway/src/server.ts`: a thin, bearer-authenticated
//! REST API (`/api/vault/status|entries|search|health`) backed by the same
//! `SqliteBackend` the local MCP server uses. Config via env:
//!   GATEWAY_API_KEY (required), GATEWAY_HOST (127.0.0.1), GATEWAY_PORT (8787).
//!   db/vault paths resolve via `context_vault_core::paths` (env var / config.json /
//!   default — see that module for precedence).

mod handlers;

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
use context_vault_core::backends::{FastembedEmbedder, NoOpEmbedder, SqliteBackend};
use context_vault_core::engine::{Embedder, VaultEngine};

pub(crate) struct AppState {
    pub(crate) engine: Arc<dyn VaultEngine>,
    pub(crate) api_key: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("GATEWAY_API_KEY")
        .context("GATEWAY_API_KEY is required (bearer token for the gateway)")?;
    let host = std::env::var("GATEWAY_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("GATEWAY_PORT").unwrap_or_else(|_| "8787".to_string());

    let engine = build_engine().await?;
    let state = Arc::new(AppState { engine, api_key });

    let app = Router::new()
        .route("/api/vault/status", get(handlers::status_handler))
        .route("/api/vault/entries", post(handlers::save_handler))
        .route("/api/vault/entries/{id}", get(handlers::get_handler).delete(handlers::delete_handler))
        .route("/api/vault/search", get(handlers::search_handler))
        .route_layer(from_fn_with_state(state.clone(), handlers::auth))
        // health is unauthenticated (liveness probe)
        .route("/api/vault/health", get(handlers::health_handler))
        .with_state(state);

    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("[gateway] listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn build_engine() -> Result<Arc<dyn VaultEngine>> {
    let db = context_vault_core::paths::resolve_db_path()?;
    let vault = context_vault_core::paths::resolve_vault_dir()?;
    let embedder = init_embedder().await;
    let backend = SqliteBackend::new(&db, &vault, embedder)?;
    backend.init().await?;
    Ok(Arc::new(backend))
}

async fn init_embedder() -> Arc<dyn Embedder> {
    let cache_dir = match context_vault_core::paths::resolve_model_cache_dir() {
        Ok(p) => std::path::PathBuf::from(p),
        Err(e) => {
            eprintln!("[gateway] failed to resolve model cache dir: {e} — embedding model unavailable");
            return Arc::new(NoOpEmbedder);
        }
    };
    match tokio::task::spawn_blocking(move || FastembedEmbedder::new(cache_dir)).await {
        Ok(Ok(e)) => Arc::new(e),
        _ => {
            eprintln!("[gateway] embedding model unavailable — FTS-only search");
            Arc::new(NoOpEmbedder)
        }
    }
}
