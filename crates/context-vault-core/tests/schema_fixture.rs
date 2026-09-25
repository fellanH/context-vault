use std::path::PathBuf;

use rusqlite::Connection;

#[test]
fn reference_schema_sql_applies_on_stock_sqlite() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mcp_root = manifest.parent().unwrap().parent().unwrap();
    let sql = std::fs::read_to_string(mcp_root.join("fixtures/bench/reference_schema.sql")).unwrap();
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(&sql).unwrap();
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='vault'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(n, 1);
}
