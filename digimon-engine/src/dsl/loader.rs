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

// ---------------------------------------------------------------------------
// Cross-check API (Task 11)
// ---------------------------------------------------------------------------

use crate::dsl::errors::ValidationError;
use crate::dsl::spec::{CardKind, ColorSpec};
use std::collections::HashMap;

/// Minimal card-data row used by `cross_check`. The real runtime uses
/// `crate::card_data::CardData`; Phase 0 tests use [`CardDataDbStub`] so
/// the DSL module can test without pulling in the full engine.
pub trait CardDataDb {
    fn lookup(&self, card_id: &str) -> Option<CardDataRow<'_>>;
}

pub struct CardDataRow<'a> {
    pub name: &'a str,
    pub kind: CardKind,
    pub level: Option<u8>,
    pub dp: Option<i32>,
    pub cost: Option<i32>,
    pub colors: &'a [ColorSpec],
}

pub struct CardDataDbStub {
    inner: HashMap<String, StubRow>,
}

struct StubRow {
    name: String,
    kind: CardKind,
    level: Option<u8>,
    dp: Option<i32>,
    cost: Option<i32>,
    colors: Vec<ColorSpec>,
}

impl CardDataDbStub {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn with_card(
        mut self,
        id: &str,
        name: &str,
        kind: CardKind,
        level: Option<u8>,
        dp: Option<i32>,
        cost: Option<i32>,
        colors: Vec<ColorSpec>,
    ) -> Self {
        self.inner.insert(
            id.into(),
            StubRow {
                name: name.into(),
                kind,
                level,
                dp,
                cost,
                colors,
            },
        );
        self
    }
}

impl Default for CardDataDbStub {
    fn default() -> Self {
        Self::new()
    }
}

impl CardDataDb for CardDataDbStub {
    fn lookup(&self, card_id: &str) -> Option<CardDataRow<'_>> {
        self.inner.get(card_id).map(|r| CardDataRow {
            name: &r.name,
            kind: r.kind,
            level: r.level,
            dp: r.dp,
            cost: r.cost,
            colors: &r.colors,
        })
    }
}

/// Cross-check a `CardSpec` against a `CardDataDb`. Verifies name, kind,
/// level, DP, cost, and colors match the structured data. Returns a
/// `ValidationError` on the first discrepancy (fail-fast, not aggregate).
pub fn cross_check(
    spec: &CardSpec,
    db: &dyn CardDataDb,
) -> Result<(), ValidationError> {
    let row = db.lookup(&spec.card).ok_or_else(|| ValidationError {
        card_id: spec.card.clone(),
        path: "card".into(),
        message: format!("unknown card_id {}", spec.card),
    })?;

    if row.name != spec.name {
        return Err(ValidationError {
            card_id: spec.card.clone(),
            path: "name".into(),
            message: format!(
                "name mismatch: yaml={} cards.json={}",
                spec.name, row.name
            ),
        });
    }
    if row.kind != spec.kind {
        return Err(ValidationError {
            card_id: spec.card.clone(),
            path: "kind".into(),
            message: format!(
                "kind mismatch: yaml={:?} cards.json={:?}",
                spec.kind, row.kind
            ),
        });
    }
    if row.level != spec.level {
        return Err(ValidationError {
            card_id: spec.card.clone(),
            path: "level".into(),
            message: format!(
                "level mismatch: yaml={:?} cards.json={:?}",
                spec.level, row.level
            ),
        });
    }
    if row.dp != spec.dp {
        return Err(ValidationError {
            card_id: spec.card.clone(),
            path: "dp".into(),
            message: format!(
                "dp mismatch: yaml={:?} cards.json={:?}",
                spec.dp, row.dp
            ),
        });
    }
    if row.cost != spec.cost {
        return Err(ValidationError {
            card_id: spec.card.clone(),
            path: "cost".into(),
            message: format!(
                "cost mismatch: yaml={:?} cards.json={:?}",
                spec.cost, row.cost
            ),
        });
    }
    let spec_set: std::collections::BTreeSet<_> = spec.color.iter().copied().collect();
    let row_set: std::collections::BTreeSet<_> = row.colors.iter().copied().collect();
    if spec_set != row_set {
        return Err(ValidationError {
            card_id: spec.card.clone(),
            path: "color".into(),
            message: format!(
                "color mismatch: yaml={:?} cards.json={:?}",
                spec.color, row.colors
            ),
        });
    }

    Ok(())
}
