use super::{run, MigrateOptions};
use std::path::Path;

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

fn md(id: Option<&str>, kind: &str, text: &str) -> String {
    let id = id.map(|id| format!("id: {id}\n")).unwrap_or_default();
    format!("---\n{id}kind: {kind}\ncreated: 2026-05-01T08:00:00Z\ntitle: T\n---\n{text}\n")
}

fn chat_line(uuid: &str, kind: &str, content: &str) -> String {
    format!(r#"{{"type":"{kind}","uuid":"{uuid}","timestamp":"2026-05-02T09:00:00Z","sessionId":"s1","message":{{"role":"{kind}","content":{content}}}}}"#)
}

fn vault(root: &Path) {
    write(root, "events/session/a.md", &md(Some("a1"), "session", "alpha session"));
    write(root, "events/tool-bypass/b.md", &md(None, "tool-bypass", "no id here"));
    write(root, "events/log/notes.md", "plain text, no frontmatter\n");
    write(root, "knowledge/rule-candidate/r.md", &md(Some("r1"), "rule-candidate", "a rule"));
    write(root, "knowledge/insight/i.md", &md(Some("i1"), "insight", "stays in main"));
    write(root, "entities/task/t.md", &md(Some("t1"), "task", "a real task stays"));
    let lines = [
        chat_line("u1", "user", r#""hello there""#),
        chat_line("u2", "assistant", r#"[{"type":"text","text":"hi"},{"type":"tool_use","name":"x"}]"#),
        chat_line("u3", "user", r#"[{"type":"tool_result","content":"ok"}]"#),
        chat_line("u1", "user", r#""hello there""#),
        "not json".into(),
    ];
    write(root, "events/chat-logs/claude-code/2026/05/s.jsonl", &lines.join("\n"));
}

#[test]
fn exhaust_and_transcripts_are_copied_verified_and_main_is_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let (main, exhaust) = (tmp.path().join("main"), tmp.path().join("exhaust"));
    vault(&main);
    let before = std::fs::read(main.join("events/tool-bypass/b.md")).unwrap();
    let options = MigrateOptions { main: main.clone(), exhaust: exhaust.clone(), chat_logs: true, legacy_ids: Default::default() };
    let report = run(&options).unwrap();
    let converted: Vec<_> = report.converted.iter().map(|(k, n)| format!("{k}={n}")).collect();
    assert_eq!(converted, ["chat-message=2", "rule-candidate=1", "session=1", "tool-bypass=1"]);
    assert_eq!((report.minted_ids, report.skipped_count, report.duplicate_ids), (1, 1, 1));
    assert_eq!((report.chat.messages, report.chat.not_messages, report.chat.unparseable), (2, 1, 1));
    assert!(report.verify.counts_match && report.verify.missing_ids == 0 && report.verify.repeated_ids == 0);
    assert_eq!(std::fs::read(main.join("events/tool-bypass/b.md")).unwrap(), before);
    assert!(main.join("events/session/a.md").exists(), "originals stay");
    assert!(run(&options).is_err(), "a second run would duplicate everything");
}

#[test]
fn minted_ids_are_stable_across_runs() {
    let tmp = tempfile::tempdir().unwrap();
    let main = tmp.path().join("main");
    vault(&main);
    let ids = |name: &str| {
        let exhaust = tmp.path().join(name);
        run(&MigrateOptions { main: main.clone(), exhaust: exhaust.clone(), chat_logs: false, legacy_ids: Default::default() }).unwrap();
        let mut ids = Vec::new();
        for segment in cv_core::segment_files(&exhaust).unwrap() {
            cv_core::read_segment(&segment, 0, |_, r| {
                ids.push(r.id);
                Ok(())
            })
            .unwrap();
        }
        ids.sort();
        ids
    };
    assert_eq!(ids("one"), ids("two"));
}

#[test]
fn a_file_without_an_id_keeps_its_v4_id() {
    let tmp = tempfile::tempdir().unwrap();
    let (main, exhaust) = (tmp.path().join("main"), tmp.path().join("exhaust"));
    vault(&main);
    let legacy = [("events/tool-bypass/b.md".to_string(), "01LEGACYID".to_string())].into();
    let report = run(&MigrateOptions { main, exhaust: exhaust.clone(), chat_logs: false, legacy_ids: legacy }).unwrap();
    assert_eq!((report.legacy_ids_used, report.minted_ids), (1, 0));
    let mut found = false;
    for segment in cv_core::segment_files(&exhaust).unwrap() {
        cv_core::read_segment(&segment, 0, |_, r| {
            found |= r.id == "01LEGACYID";
            Ok(())
        })
        .unwrap();
    }
    assert!(found);
}

#[test]
fn knowledge_files_without_an_id_get_one_written_and_become_indexable() {
    let tmp = tempfile::tempdir().unwrap();
    let (main, exhaust) = (tmp.path().join("main"), tmp.path().join("exhaust"));
    vault(&main);
    write(&main, "knowledge/insight/old.md", &md(None, "insight", "from v4"));
    write(&main, "knowledge/insight/new.md", &md(None, "insight", "v4 never saw it"));
    let before = std::fs::read(main.join("events/tool-bypass/b.md")).unwrap();
    let legacy = [("knowledge/insight/old.md".to_string(), "01V4INSIGHT".to_string())].into();
    let report = run(&MigrateOptions { main: main.clone(), exhaust, chat_logs: false, legacy_ids: legacy }).unwrap();
    assert_eq!((report.knowledge_ids_written, report.knowledge_ids_minted), (2, 1));
    let old = cv_core::markdown::read_record(&main.join("knowledge/insight/old.md")).unwrap();
    assert_eq!((old.id.as_str(), old.text.trim()), ("01V4INSIGHT", "from v4"));
    assert!(cv_core::markdown::read_record(&main.join("knowledge/insight/new.md")).is_ok());
    assert_eq!(std::fs::read(main.join("events/tool-bypass/b.md")).unwrap(), before, "exhaust files are not changed");
    let unchanged = std::fs::read(main.join("knowledge/insight/i.md")).unwrap();
    assert_eq!(unchanged, md(Some("i1"), "insight", "stays in main").into_bytes(), "files with an id are not rewritten");
}

#[test]
fn the_knowledge_index_leaves_exhaust_to_the_data_vault() {
    let tmp = tempfile::tempdir().unwrap();
    let (main, exhaust) = (tmp.path().join("main"), tmp.path().join("exhaust"));
    vault(&main);
    run(&MigrateOptions { main: main.clone(), exhaust, chat_logs: true, legacy_ids: Default::default() }).unwrap();
    let index = crate::VaultIndex::open(&main, cv_core::VaultType::Knowledge, &tmp.path().join("idx")).unwrap();
    index.update(&crate::UpdateOptions { full: true, budget: None, ..Default::default() }).unwrap();
    let hits = index.search("", &crate::SearchOptions { limit: 100, ..Default::default() }).unwrap();
    let mut kinds: Vec<_> = hits.iter().map(|h| h.record.kind.as_str()).collect();
    kinds.sort();
    assert_eq!(kinds, ["insight", "task"], "originals of session/tool-bypass/rule-candidate stay on disk but out of main search");
}

#[test]
fn id_writing_leaves_non_frontmatter_typed_ids_and_hidden_folders_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let (main, exhaust) = (tmp.path().join("main"), tmp.path().join("exhaust"));
    vault(&main);
    let untouched = [
        ("knowledge/insight/rule.md", "---\njust a horizontal rule, no frontmatter\n"),
        ("knowledge/insight/numeric.md", "---\nid: 123\nkind: insight\n---\nnumeric id\n"),
        (".obsidian/templates/t.md", "---\nkind: template\n---\nx\n"),
        ("node_modules/pkg/README.md", "---\ntitle: pkg\n---\nx\n"),
    ];
    for (rel, body) in untouched {
        write(&main, rel, body);
    }
    write(&main, "knowledge/insight/crlf.md", "---\r\nkind: insight\r\n---\r\nwindows\r\n");
    let report = run(&MigrateOptions { main: main.clone(), exhaust, chat_logs: false, legacy_ids: Default::default() }).unwrap();
    for (rel, body) in untouched {
        assert_eq!(std::fs::read_to_string(main.join(rel)).unwrap(), body, "{rel} must not change");
    }
    assert_eq!(report.knowledge_id_files, ["knowledge/insight/crlf.md"]);
    let crlf = std::fs::read_to_string(main.join("knowledge/insight/crlf.md")).unwrap();
    assert!(crlf.starts_with("---\r\nid: ") && crlf.ends_with("---\r\nwindows\r\n"), "{crlf:?}");
    assert!(cv_core::markdown::read_record(&main.join("knowledge/insight/crlf.md")).is_ok());
}
