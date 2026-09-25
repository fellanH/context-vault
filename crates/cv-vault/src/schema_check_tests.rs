use crate::capture::write_entry;
use crate::schema_test_support::{err, input, vault, DATA_VAULT};
use serde_json::{json, Value};

fn ok_price() -> Value {
    json!({"amount": 950, "currency": "SEK", "observed_on": "2026-09-25", "sensitivity": "public"})
}

#[test]
fn valid_record_saves() {
    let dir = vault(DATA_VAULT);
    let r = write_entry(dir.path().to_str().unwrap(), &input("price-point", ok_price(), &[])).unwrap();
    assert!(std::path::Path::new(&r.file_path).exists());
}

#[test]
fn missing_required_and_wrong_types_all_reported_and_nothing_written() {
    let dir = vault(DATA_VAULT);
    let e = err(
        &dir,
        &input(
            "price-point",
            json!({"amount": "950", "currency": "GBP", "observed_on": "yesterday", "hours": 1.5, "sensitivity": "public"}),
            &[],
        ),
    );
    assert!(e.contains("meta.amount must be number"), "{e}");
    assert!(e.contains("meta.currency = \"GBP\" is not one of"), "{e}");
    assert!(e.contains("meta.observed_on must be date"), "{e}");
    assert!(e.contains("meta.hours must be integer"), "{e}");
    assert!(e.contains("4 problems"), "{e}");
    // Refused before any write: no folder for the kind was created.
    assert!(!dir.path().join("structured").exists());

    let e = err(&dir, &input("price-point", json!({"sensitivity": "public"}), &[]));
    assert!(e.contains("meta.amount is required"), "{e}");
    assert!(e.contains("meta.observed_on is required"), "{e}");
}

#[test]
fn rfc3339_counts_as_date() {
    let dir = vault(DATA_VAULT);
    let mut m = ok_price();
    m["observed_on"] = json!("2026-09-25T10:00:00+02:00");
    write_entry(dir.path().to_str().unwrap(), &input("price-point", m, &[])).unwrap();
}

#[test]
fn closed_registry_refuses_unknown_kind_with_suggestion() {
    let dir = vault(DATA_VAULT);
    let e = err(&dir, &input("price-piont", ok_price(), &[]));
    assert!(e.contains("not in this vault's closed kind registry"), "{e}");
    assert!(e.contains("Did you mean `price-point`?"), "{e}");

    let e = err(&dir, &input("acme-corp", json!({"sensitivity": "public"}), &[]));
    assert!(!e.contains("Did you mean"), "{e}");
}

#[test]
fn personal_free_refuses_personal_and_unclassified() {
    let dir = vault(DATA_VAULT);
    let classified = |s: &str| json!({"sensitivity": s});

    assert!(err(&dir, &input("gotcha", json!({}), &[])).contains("meta.sensitivity is required"));
    assert!(err(&dir, &input("gotcha", classified("personal"), &[])).contains("personal-free"));
    assert!(err(&dir, &input("gotcha", classified("Personal"), &[])).contains("personal-free"));
    assert!(err(&dir, &input("gotcha", classified("secret"), &[])).contains("is not one of"));
    assert!(err(
        &dir,
        &input("gotcha", json!({"sensitivity": "public", "subject": "Person"}), &[])
    )
    .contains("meta.subject = \"Person\""));
    assert!(err(&dir, &input("gotcha", classified("internal"), &["person:jane-doe"]))
        .contains("names a person"));

    // No meta at all is still unclassified.
    let mut bare = input("gotcha", json!({}), &[]);
    bare.meta = None;
    assert!(err(&dir, &bare).contains("meta.sensitivity is required"));

    write_entry(
        dir.path().to_str().unwrap(),
        &input("gotcha", json!({"sensitivity": "internal", "subject": "Organization"}), &["client:acme"]),
    )
    .unwrap();
}

#[test]
fn non_object_meta_is_refused_when_schema_applies() {
    let dir = vault(DATA_VAULT);
    let e = err(&dir, &input("price-point", json!("{\"amount\": 1}"), &[]));
    assert!(e.contains("meta must be a JSON object, got string"), "{e}");
}

