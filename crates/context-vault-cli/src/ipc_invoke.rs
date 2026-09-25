use std::io::Read;

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

pub fn run(stdin_json: bool) -> Result<()> {
    if !stdin_json {
        return Err(anyhow!("invoke requires --stdin-json (stable JSON on stdin)"));
    }
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    let payload: Value = serde_json::from_str(&buf).map_err(|e| anyhow!("stdin JSON: {e}"))?;
    let tool = payload
        .get("tool")
        .and_then(|t| t.as_str())
        .unwrap_or("unknown");
    let out = json!({
        "ok": true,
        "tool": tool,
        "content": [{
            "type": "text",
            "text": format!("stub invoke for tool `{tool}` (rust slice)")
        }]
    });
    serde_json::to_writer(std::io::stdout(), &out)?;
    Ok(())
}
