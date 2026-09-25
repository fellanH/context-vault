//! Tech-stack detection for project ingest: backend languages + JS/TS frameworks.

use serde_json::Value;
use std::path::Path;

pub(super) fn detect_tech_stack(project_path: &str, pkg_json: Option<&Value>) -> Vec<String> {
    let mut stack = vec![];

    check_backend_languages(&mut stack, project_path);

    if let Some(pkg) = pkg_json {
        check_javascript_stack(&mut stack, project_path, pkg);
    }

    stack.sort();
    stack.dedup();
    stack
}

#[allow(dead_code)]
fn check_backend_languages(stack: &mut Vec<String>, project_path: &str) {
    let checks = [
        ("python", &["pyproject.toml", "setup.py"][..]),
        ("rust", &["Cargo.toml"][..]),
        ("go", &["go.mod"][..]),
    ];

    for (lang, files) in &checks {
        if files.iter().any(|f| Path::new(project_path).join(f).exists()) {
            stack.push(lang.to_string());
        }
    }
}

#[allow(dead_code)]
fn check_javascript_stack(stack: &mut Vec<String>, project_path: &str, pkg: &Value) {
    stack.push("javascript".to_string());

    let all_deps = collect_dependencies(pkg);

    if all_deps.contains_key("typescript") || Path::new(project_path).join("tsconfig.json").exists() {
        stack.push("typescript".to_string());
    }

    check_framework_deps(stack, &all_deps);
}

#[allow(dead_code)]
fn collect_dependencies(pkg: &Value) -> serde_json::Map<String, Value> {
    let mut deps = serde_json::Map::new();

    if let Some(obj) = pkg.get("dependencies").and_then(|v| v.as_object()) {
        for (k, v) in obj {
            deps.insert(k.clone(), v.clone());
        }
    }

    if let Some(obj) = pkg.get("devDependencies").and_then(|v| v.as_object()) {
        for (k, v) in obj {
            deps.insert(k.clone(), v.clone());
        }
    }

    deps
}

#[allow(dead_code)]
fn check_framework_deps(stack: &mut Vec<String>, all_deps: &serde_json::Map<String, Value>) {
    let checks = [
        (&["react", "react-dom"][..], "react"),
        (&["next"][..], "nextjs"),
        (&["vue"][..], "vue"),
        (&["svelte"][..], "svelte"),
        (&["express"][..], "express"),
        (&["fastify"][..], "fastify"),
        (&["hono"][..], "hono"),
        (&["vite"][..], "vite"),
        (&["electron"][..], "electron"),
        (&["tauri", "@tauri-apps/api"][..], "tauri"),
    ];

    for (deps, name) in &checks {
        if deps.iter().any(|d| all_deps.contains_key(*d)) {
            stack.push(name.to_string());
        }
    }
}
