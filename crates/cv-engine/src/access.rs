//! What agents were shown and what they read, so the share of prompt hints an agent then
//! opens is one query on the vault's own database (OPS-473). A supertrait of `VaultEngine`;
//! a backend with no database keeps the defaults, which record nothing.

use async_trait::async_trait;

// async_trait adds must_use to its boxed Future, which is already must_use.
#[allow(clippy::double_must_use)]
#[async_trait]
pub trait AccessLog {
    /// Entries the prompt hook showed as hints in one session (`hint_log`).
    async fn log_hints(&self, _ids: &[String], _session: &str) -> anyhow::Result<()> {
        Ok(())
    }
    /// Entries a read returned, with its query (`access_log`).
    async fn log_reads(&self, _ids: &[String], _query: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
