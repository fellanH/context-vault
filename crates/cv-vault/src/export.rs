//! `context-vault export`: one kind's entries as CSV or JSON, read from the markdown
//! files (the source of truth), not the index (card 27).
//!
//! Columns: id, title, created, updated, then the kind's declared `fields` (vault.toml),
//! then `meta`: any other custom frontmatter as one JSON object.

use std::path::Path;

use anyhow::Result;
use serde_json::{Map, Value};

use crate::fm_parse::{extract_custom_meta, parse_frontmatter};
use crate::files::walk_dir;
use crate::VaultTaxonomy;

pub struct ExportRow {
    pub file_path: String,
    pub id: Option<String>,
    pub title: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub fields: Vec<Option<Value>>,
    pub meta: Map<String, Value>,
}

/// Every entry of `kind`: frontmatter `kind` wins, else the leaf folder (same rule as reindex).
pub fn collect(vault_dir: &str, taxonomy: &VaultTaxonomy, kind: &str) -> Result<(Vec<String>, Vec<ExportRow>)> {
    let declared: Vec<String> = taxonomy.rule_for(kind).fields.keys().cloned().collect();
    let mut rows: Vec<ExportRow> = walk_dir(vault_dir)?
        .into_iter()
        .filter_map(|wr| read_row(&wr.file_path, &wr.rel_dir, kind, &declared))
        .collect();
    rows.sort_by(|a, b| a.created.cmp(&b.created).then(a.file_path.cmp(&b.file_path)));
    Ok((declared, rows))
}

fn leaf(rel_dir: &str) -> &str {
    Path::new(rel_dir).file_name().and_then(|s| s.to_str()).unwrap_or("")
}

/// One file as a row, or `None` when it is unreadable or another kind.
fn read_row(file_path: &str, rel_dir: &str, kind: &str, declared: &[String]) -> Option<ExportRow> {
    let raw = std::fs::read_to_string(file_path).ok()?;
    // Cheap skip before the full parse: the file can only be this kind if its
    // folder is named after it or its frontmatter names it.
    if leaf(rel_dir) != kind && !raw.contains(&format!("kind: {kind}")) {
        return None;
    }
    let (fm, _body) = parse_frontmatter(&raw);
    let get = |k: &str| fm.get(k).and_then(|v| v.as_str()).map(|s| s.to_string());
    let file_kind = get("kind").unwrap_or_else(|| match leaf(rel_dir) {
        "" => "note".to_string(),
        l => l.to_string(),
    });
    if file_kind != kind {
        return None;
    }
    let mut meta = extract_custom_meta(&fm).unwrap_or_default();
    let fields = declared.iter().map(|f| meta.remove(f)).collect();
    meta.remove("category");
    Some(ExportRow {
        file_path: file_path.to_string(),
        id: get("id"),
        title: get("title"),
        created: get("created"),
        updated: get("updated"),
        fields,
        meta,
    })
}

fn cell(v: &Option<Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

fn csv_escape(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn to_csv(declared: &[String], rows: &[ExportRow]) -> String {
    let mut header = vec!["id", "title", "created", "updated"];
    header.extend(declared.iter().map(|s| s.as_str()));
    header.push("meta");
    let mut out = header.iter().map(|h| csv_escape(h)).collect::<Vec<_>>().join(",");
    out.push('\n');
    for r in rows {
        let mut cells = vec![
            r.id.clone().unwrap_or_default(),
            r.title.clone().unwrap_or_default(),
            r.created.clone().unwrap_or_default(),
            r.updated.clone().unwrap_or_default(),
        ];
        cells.extend(r.fields.iter().map(cell));
        cells.push(if r.meta.is_empty() { String::new() } else { Value::Object(r.meta.clone()).to_string() });
        out.push_str(&cells.iter().map(|c| csv_escape(c)).collect::<Vec<_>>().join(","));
        out.push('\n');
    }
    out
}

pub fn to_json(declared: &[String], rows: &[ExportRow]) -> Value {
    Value::Array(
        rows.iter()
            .map(|r| {
                let mut o = Map::new();
                o.insert("id".into(), r.id.clone().map(Value::String).unwrap_or(Value::Null));
                o.insert("title".into(), r.title.clone().map(Value::String).unwrap_or(Value::Null));
                o.insert("created".into(), r.created.clone().map(Value::String).unwrap_or(Value::Null));
                o.insert("updated".into(), r.updated.clone().map(Value::String).unwrap_or(Value::Null));
                for (name, v) in declared.iter().zip(&r.fields) {
                    o.insert(name.clone(), v.clone().unwrap_or(Value::Null));
                }
                o.insert("meta".into(), Value::Object(r.meta.clone()));
                Value::Object(o)
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_one_kind_with_declared_fields_and_escaping() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path();
        std::fs::write(dir.join("vault.toml"), "schema_version = 1\n[default]\ncategory = \"knowledge\"\ntier = \"working\"\nidentity = false\n[kinds.lead]\ncategory = \"data\"\ntier = \"durable\"\n[kinds.lead.fields]\nstage = { type = \"string\" }\nvalue = { type = \"number\" }\n").unwrap();
        std::fs::create_dir_all(dir.join("data/lead")).unwrap();
        std::fs::create_dir_all(dir.join("knowledge/note")).unwrap();
        std::fs::write(dir.join("data/lead/a.md"), "---\nid: A1\ntitle: Acme, Inc\ncreated: 2026-01-02\nstage: won\nvalue: 1200\nsource_url: x\n---\nbody\n").unwrap();
        std::fs::write(dir.join("data/lead/b.md"), "---\ntitle: \"Beta \"\"quoted\"\"\"\ncreated: 2026-01-01\n---\nbody\n").unwrap();
        std::fs::write(dir.join("knowledge/note/n.md"), "---\nid: N1\ntitle: not a lead\n---\nx\n").unwrap();
        let tax = VaultTaxonomy::load_strict(dir.to_str().unwrap()).unwrap();

        let (declared, rows) = collect(dir.to_str().unwrap(), &tax, "lead").unwrap();
        assert_eq!(declared, vec!["stage", "value"]);
        assert_eq!(rows.len(), 2, "the note is left out");
        let csv = to_csv(&declared, &rows);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "id,title,created,updated,stage,value,meta");
        assert!(lines[1].starts_with(",") && lines[1].contains("2026-01-01"), "older first, missing id blank: {}", lines[1]);
        assert!(lines[2].starts_with("A1,\"Acme, Inc\",2026-01-02,,won,1200,"), "{}", lines[2]);
        assert!(lines[2].contains("source_url"), "undeclared meta lands in the meta column");

        let json = to_json(&declared, &rows);
        assert_eq!(json[1]["stage"], "won");
        assert_eq!(json[1]["meta"]["source_url"], "x");
    }
}
