//! The situation and vault-map reads of `SqliteBackend` (see `situations` and `vault_map`).

use anyhow::Result;
use async_trait::async_trait;
use cv_engine::situations::{SituationEngine, PAGE_CHARS};
use serde_json::Value;

use super::sqlite::SqliteBackend;

#[async_trait]
impl SituationEngine for SqliteBackend {
    async fn list_situations(&self) -> Result<Value> {
        let db = self.raw_db();
        tokio::task::spawn_blocking(move || super::situations::list_situations(&db.lock().unwrap())).await?
    }

    async fn read_situation(&self, situation: &str, cursor: Option<&str>) -> Result<Value> {
        let (db, situation, cursor) = (self.raw_db(), situation.to_string(), cursor.map(String::from));
        tokio::task::spawn_blocking(move || {
            super::situations::read_situation(&db.lock().unwrap(), &situation, cursor.as_deref(), PAGE_CHARS)
        })
        .await?
    }

    async fn vault_map(&self, force: bool) -> Result<Value> {
        let db = self.raw_db();
        tokio::task::spawn_blocking(move || super::vault_map::get(&db.lock().unwrap(), force, chrono::Utc::now())).await?
    }
}
