//! Write commands: delete.

use anyhow::Result;

use crate::engine_cmd;

pub async fn run_delete(id: &str) -> Result<()> {
    let engine = engine_cmd::build_engine().await?;
    engine.delete(id).await?;
    println!("deleted: {}", id);
    Ok(())
}
