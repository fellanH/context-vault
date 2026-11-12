use crate::registry::{Registry, VaultEntry, VaultType};

fn two_vaults(root: &std::path::Path) -> Registry {
    let mut r = Registry { default: "main".into(), ..Default::default() };
    let entry = |sub: &str, kind, by_default| VaultEntry {
        kind,
        dir: root.join(sub).to_string_lossy().into(),
        search_by_default: by_default,
    };
    r.vaults.insert("main".into(), entry("main", VaultType::Knowledge, true));
    r.vaults.insert("exhaust".into(), entry("exhaust", VaultType::Data, false));
    r
}

#[test]
fn saves_loads_and_resolves() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("vaults.toml");
    let reg = two_vaults(tmp.path());
    reg.save(&path).unwrap();
    let back = Registry::load(&path).unwrap();
    assert_eq!(back, reg);
    assert_eq!(back.resolve(Some("exhaust")).unwrap().1.kind, VaultType::Data);
    let raw = std::fs::read_to_string(&path).unwrap();
    assert!(raw.contains("type = \"data\""), "{raw}");
}

#[test]
fn missing_file_is_empty_registry() {
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(Registry::load(&tmp.path().join("nope.toml")).unwrap(), Registry::default());
}

#[test]
fn refuses_bad_registries() {
    let tmp = tempfile::tempdir().unwrap();
    let mut r = two_vaults(tmp.path());
    r.default = "gone".into();
    assert!(r.validate().unwrap_err().to_string().contains("not in the registry"));

    let mut r = two_vaults(tmp.path());
    r.vaults.get_mut("exhaust").unwrap().dir = tmp.path().join("main/inner").to_string_lossy().into();
    assert!(r.validate().unwrap_err().to_string().contains("inside vault `main`"));

    let mut r = two_vaults(tmp.path());
    let e = r.vaults.remove("exhaust").unwrap();
    r.vaults.insert("has space".into(), e);
    assert!(r.validate().unwrap_err().to_string().contains("vault name"));

    let path = tmp.path().join("typo.toml");
    std::fs::write(&path, "default = \"main\"\n[vaults.main]\ntype = \"knowledge\"\ndir = \"/x\"\nsearch_by_defualt = false\n").unwrap();
    assert!(format!("{:#}", Registry::load(&path).unwrap_err()).contains("search_by_defualt"));
}

#[test]
fn unknown_name_lists_known_vaults() {
    let tmp = tempfile::tempdir().unwrap();
    let e = two_vaults(tmp.path()).resolve(Some("nope")).unwrap_err().to_string();
    assert!(e.contains("exhaust") && e.contains("main"), "{e}");
}

#[test]
fn tilde_dir_expands_to_home() {
    let e = VaultEntry { kind: VaultType::Knowledge, dir: "~/some/vault".into(), search_by_default: true };
    assert_eq!(e.dir_path(), dirs::home_dir().unwrap().join("some/vault"));
}
