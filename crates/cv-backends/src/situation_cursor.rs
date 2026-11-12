//! Opaque keyset cursor for situation pages: hex of `[ord, updated, id]`.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

pub struct Cursor {
    pub ord: i64,
    pub updated: String,
    pub id: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) || !s.is_ascii() {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

pub fn encode(ord: i64, updated: &str, id: &str) -> String {
    hex(json!([ord, updated, id]).to_string().as_bytes())
}

pub fn decode(cursor: &str) -> Result<Cursor> {
    let bad = || anyhow!("cursor is not valid");
    let value: Value = serde_json::from_slice(&unhex(cursor).ok_or_else(bad)?).map_err(|_| bad())?;
    match value.as_array().map(|a| a.as_slice()) {
        Some([Value::Number(o), Value::String(u), Value::String(i)]) => {
            Ok(Cursor { ord: o.as_i64().ok_or_else(bad)?, updated: u.clone(), id: i.clone() })
        }
        _ => Err(bad()),
    }
}
