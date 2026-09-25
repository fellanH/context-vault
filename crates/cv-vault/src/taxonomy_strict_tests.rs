use crate::capture::write_entry;
use crate::schema_test_support::{err, input, vault, DATA_VAULT};
use crate::taxonomy::VaultTaxonomy;
use serde_json::json;

#[test]
fn vault_without_refusers_accepts_anything_as_before() {
    let dir = vault("schema_version = 1\n[default]\ncategory = \"knowledge\"\n");
    write_entry(dir.path().to_str().unwrap(), &input("anything-goes", json!("not an object"), &["person:x"])).unwrap();

    let none = tempfile::TempDir::new().unwrap();
    write_entry(none.path().to_str().unwrap(), &input("reguest", json!({}), &[])).unwrap();
}

#[test]
fn broken_vault_toml_refuses_writes_instead_of_dropping_refusers() {
    let dir = vault("schema_version = 1\n[vault]\npersonal_free = tru\n");
    let e = err(&dir, &input("gotcha", json!({"sensitivity": "personal"}), &[]));
    assert!(e.contains("save refused"), "{e}");
}

#[test]
fn misspelt_policy_key_refuses_writes() {
    let dir = vault("schema_version = 1\n[vault]\npersonal_fre = true\n");
    assert!(err(&dir, &input("gotcha", json!({}), &[])).contains("unknown field `personal_fre`"));
}

#[test]
fn misspelt_kind_key_refuses_writes_only_when_refusers_declared() {
    let with_refusers = vault(
        "schema_version = 1\n[kinds.gotcha]\nfeilds = { x = 1 }\n[kinds.price-point.fields]\namount = { type = \"number\", required = true }\n",
    );
    let e = err(&with_refusers, &input("gotcha", json!({}), &[]));
    assert!(e.contains("kinds.gotcha.feilds"), "{e}");

    // A plain taxonomy file with a stray key keeps saving (warning only), as before.
    let plain = vault("schema_version = 1\n[kinds.gotcha]\ncolour = \"red\"\n");
    write_entry(plain.path().to_str().unwrap(), &input("gotcha", json!({}), &[])).unwrap();
}

#[test]
fn schema_round_trips_through_to_toml_string() {
    let dir = vault(DATA_VAULT);
    let t = VaultTaxonomy::load_strict(dir.path().to_str().unwrap()).unwrap();
    let again = vault(&t.to_toml_string());
    let t2 = VaultTaxonomy::load_strict(again.path().to_str().unwrap()).unwrap();
    assert_eq!(t2.policy(), t.policy());
    assert_eq!(t2.rule_for("price-point"), t.rule_for("price-point"));
    assert!(t2.kinds().contains_key("gotcha"));
}

