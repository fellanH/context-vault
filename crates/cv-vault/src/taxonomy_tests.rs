use super::*;

#[test]
fn minimal_imposes_no_ontology() {
    let t = VaultTaxonomy::minimal();

    // No grouping assumed for any kind, known-sounding or not: flat, named after itself.
    assert_eq!(t.folder_for("insight"), "insight");
    assert_eq!(t.folder_for("contact"), "contact");
    assert_eq!(t.folder_for("totally-unknown-kind"), "totally-unknown-kind");

    assert_eq!(t.tier_for("insight"), "working");
    assert!(!t.is_identity("contact"));
    assert_eq!(t.category_for("insight"), "");
}

#[test]
fn missing_vault_toml_falls_back_to_minimal() {
    let temp = tempfile::TempDir::new().unwrap();
    let t = VaultTaxonomy::load(temp.path().to_str().unwrap());
    assert_eq!(t.folder_for("insight"), "insight");
}

#[test]
fn invalid_vault_toml_falls_back_to_minimal() {
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::write(temp.path().join(VAULT_TOML_FILENAME), "not valid toml {{{").unwrap();
    let t = VaultTaxonomy::load(temp.path().to_str().unwrap());
    assert_eq!(t.folder_for("insight"), "insight");
}

#[test]
fn custom_kind_and_folder_override() {
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::write(
        temp.path().join(VAULT_TOML_FILENAME),
        r#"
schema_version = 1

[default]
category = "misc"
tier = "working"

[kinds.recipe]
category = "cooking"
tier = "durable"
folder = "cooking/recipes"

[kinds.contact]
category = "people"
identity = true
"#,
    )
    .unwrap();

    let t = VaultTaxonomy::load(temp.path().to_str().unwrap());

    assert_eq!(t.category_for("recipe"), "cooking");
    assert_eq!(t.folder_for("recipe"), "cooking/recipes");
    assert_eq!(t.tier_for("recipe"), "durable");

    assert_eq!(t.category_for("contact"), "people");
    assert!(t.is_identity("contact"));
    assert_eq!(t.folder_for("contact"), "people/contact");
    // contact didn't specify tier -> falls back to this file's [default].
    assert_eq!(t.tier_for("contact"), "working");

    // unlisted kind -> this file's [default], not any implicit ontology.
    assert_eq!(t.category_for("something-else"), "misc");
}

#[test]
fn round_trips_through_schema_init_output() {
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::write(
        temp.path().join(VAULT_TOML_FILENAME),
        VaultTaxonomy::minimal().to_toml_string(),
    )
    .unwrap();

    let reloaded = VaultTaxonomy::load(temp.path().to_str().unwrap());
    assert_eq!(reloaded.folder_for("insight"), "insight");
    assert_eq!(reloaded.tier_for("insight"), "working");
    assert!(!reloaded.is_identity("insight"));
}
