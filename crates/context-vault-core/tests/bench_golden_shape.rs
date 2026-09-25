//! Regression: stable JSON shape for `bench` fixture output (keys only).

use context_vault_core::bench_harness::{BenchArgs, DiskMode, WorkloadMix};
use serde_json::Value;

fn assert_keys(obj: &Value, path: &str, expected: &[&str]) {
    let m = obj
        .as_object()
        .unwrap_or_else(|| panic!("{path}: not an object"));
    for k in expected {
        assert!(
            m.contains_key(*k),
            "missing key `{k}` under {path} (have {:?})",
            m.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn bench_fixture_json_shape_matches_golden_keys() {
    let out = context_vault_core::bench_harness::run_bench(BenchArgs {
        disk_mode: DiskMode::A,
        workload: WorkloadMix::ReadHeavy,
        seed: 99,
        corpus_root: None,
        corpus_entries: 8,
        schema_fixture_rel: "fixtures/bench/reference_schema.sql",
        vault_db: None,
        archive_json: None,
    })
    .expect("run_bench");

    let v = serde_json::to_value(&out).expect("serialize");
    let golden: Value =
        serde_json::from_str(include_str!("fixtures/bench_golden_keys.json")).expect("golden");

    assert_keys(
        &v["bench-meta"],
        "bench-meta",
        golden["bench-meta"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect::<Vec<_>>()
            .as_slice(),
    );
    assert_keys(
        &v["timings"],
        "timings",
        golden["timings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect::<Vec<_>>()
            .as_slice(),
    );
    assert_keys(
        &v["timings"]["sqlite"],
        "timings.sqlite",
        golden["sqlite_fixture"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect::<Vec<_>>()
            .as_slice(),
    );
    assert_keys(
        &v["corpus"],
        "corpus",
        golden["corpus"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect::<Vec<_>>()
            .as_slice(),
    );
    assert_keys(
        &v["disk_modes"],
        "disk_modes",
        golden["disk_modes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect::<Vec<_>>()
            .as_slice(),
    );
    assert_keys(
        &v["workload_mixes"],
        "workload_mixes",
        golden["workload_mixes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect::<Vec<_>>()
            .as_slice(),
    );

    assert!(
        v["timings"]["sqlite"]["fts_user_probe"].is_null(),
        "fixture run must have fts_user_probe: null"
    );
}
