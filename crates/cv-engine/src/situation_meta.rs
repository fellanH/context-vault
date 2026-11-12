//! Folding `situations` / `situation_order` arguments into an entry's meta.
//!
//! Only what the caller supplies is validated. Stored meta that is merely carried over is left as it
//! is, so an entry hand-edited into something malformed stays editable.

use serde_json::{json, Map, Value};

use crate::situations::{normalize_order, normalize_situations};

const KEYS: [&str; 2] = ["situations", "situation_order"];

fn meta_map(meta: &Value) -> Result<Map<String, Value>, String> {
    match meta {
        Value::Object(m) => Ok(m.clone()),
        Value::Null => Ok(Map::new()),
        _ => Err("meta must be an object".into()),
    }
}

/// Situation keys inside a caller-supplied `meta` are held to the same rules as the arguments.
fn validate_supplied(map: &mut Map<String, Value>) -> Result<(), String> {
    if let Some(v) = map.get("situations") {
        let names = normalize_situations(v)?;
        map.insert("situations".into(), json!(names));
    }
    map.get("situation_order").map_or(Ok(0), normalize_order).map(|_| ())
}

/// A caller-supplied whole `meta` without these keys must not drop the stored ones (as stored).
fn carry_over(map: &mut Map<String, Value>, stored: Option<&Value>) {
    let Some(Value::Object(old)) = stored else { return };
    for key in KEYS {
        if let (false, Some(v)) = (map.contains_key(key), old.get(key)) {
            map.insert(key.to_string(), v.clone());
        }
    }
}

/// `None` keeps what is there; `null` or `[]` clears; an array replaces.
fn set_situations(map: &mut Map<String, Value>, value: Option<&Value>) -> Result<(), String> {
    let Some(value) = value else { return Ok(()) };
    let names = if value.is_null() { Vec::new() } else { normalize_situations(value)? };
    if names.is_empty() {
        map.remove("situations");
        map.remove("situation_order");
    } else {
        map.insert("situations".into(), json!(names));
    }
    Ok(())
}

/// A number sets the order; `null` resets it to the default. An entry with no situations keeps no order.
fn set_order(map: &mut Map<String, Value>, value: Option<&Value>) -> Result<(), String> {
    match value {
        None => {}
        Some(Value::Null) => {
            map.remove("situation_order");
        }
        Some(v) => {
            map.insert("situation_order".into(), json!(normalize_order(v)?));
        }
    }
    if !map.contains_key("situations") {
        map.remove("situation_order");
    }
    Ok(())
}

/// The meta to store. `meta` is the caller's whole meta (if any), `stored` the entry's current meta.
/// With no `meta` argument the stored meta is the base. `situations`: `Some(null)` or `[]` clears, an
/// array replaces, `None` keeps. `situation_order`: a number sets it, `Some(null)` resets it.
pub fn apply_to_meta(
    meta: Option<Value>,
    stored: Option<&Value>,
    situations: Option<&Value>,
    order: Option<&Value>,
) -> Result<Option<Value>, String> {
    if meta.is_none() && situations.is_none() && order.is_none() {
        return Ok(stored.cloned());
    }
    let mut map = match &meta {
        Some(m) => {
            let mut map = meta_map(m)?;
            validate_supplied(&mut map)?;
            carry_over(&mut map, stored);
            map
        }
        None => meta_map(stored.unwrap_or(&Value::Null))?,
    };
    set_situations(&mut map, situations)?;
    set_order(&mut map, order)?;
    Ok(if map.is_empty() { None } else { Some(Value::Object(map)) })
}
