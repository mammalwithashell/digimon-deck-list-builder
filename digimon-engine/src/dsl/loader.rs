//! YAML loader — file and directory entrypoints.
//!
//! Parses `*.yaml` files into `CardSpec` without semantic validation or
//! cards.json cross-check (those land in Tasks 11 and 12 respectively).

use std::fs;
use std::path::{Path, PathBuf};

use crate::dsl::errors::DslError;
use crate::dsl::spec::CardSpec;

/// Load and parse a single YAML file into a `CardSpec`. Does NOT cross-check
/// against `cards.json` (see Task 11) and does NOT validate semantically
/// (see Task 12).
pub fn load_file(path: &Path) -> Result<CardSpec, DslError> {
    let raw = fs::read_to_string(path).map_err(|e| DslError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    let spec: CardSpec = serde_yml::from_str(&raw).map_err(|e| DslError::Yaml {
        path: path.display().to_string(),
        source: e,
    })?;
    Ok(spec)
}

/// Load every `*.yaml` under `dir` (recursive). Fails fast on the first error.
pub fn load_dir(dir: &Path) -> Result<Vec<CardSpec>, DslError> {
    let mut out = Vec::new();
    for entry_path in walk_yaml_files(dir) {
        out.push(load_file(&entry_path)?);
    }
    Ok(out)
}

/// Load every `*.yaml` under `dir` (recursive). Collects errors separately
/// instead of failing fast — used by tooling that wants a full report.
pub fn load_dir_ok(dir: &Path) -> (Vec<CardSpec>, Vec<DslError>) {
    let mut ok = Vec::new();
    let mut errs = Vec::new();
    for entry_path in walk_yaml_files(dir) {
        match load_file(&entry_path) {
            Ok(spec) => ok.push(spec),
            Err(e) => errs.push(e),
        }
    }
    (ok, errs)
}

fn walk_yaml_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !dir.exists() {
        return out;
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(iter) = fs::read_dir(&d) else { continue };
        for entry in iter.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().map_or(false, |e| e == "yaml" || e == "yml") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}
