# Card-scripting DSL — Phase 0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the schema + YAML loader + validator + round-trip pretty-printer + JSON Schema export for the card-scripting DSL, along with 15 golden YAMLs from the worked examples in the spec §10. No engine integration, no lowering, no `CardRegistry` touch. Produces a library module that compiles green with `cargo test --test dsl`.

**Architecture:** A new feature-flagged module at `digimon-engine/src/dsl/` (behind `dsl-yaml-loader` Cargo feature, default-on in the library crate, off in the Tauri crate per spec §7a.1). Parse-only for this phase — no lowering to `Effect` closures, no cross-card resolution. `CardSpec` structs mirror spec §3.2 exactly; validator rejects malformed content with structured errors that cite the offending file, line, and schema path. A stub `RawRustRegistry` trait allows Phase 0 to validate `raw_rust:` references without implementing the registry.

**Tech Stack:** Rust 2021 / `serde = 1` / `serde_yml = 0.0.12` (maintained fork of the archived `serde_yaml`) / `schemars = 0.8` for JSON Schema / `thiserror = 1` for error types / `indexmap = 2` for stable key ordering in pretty-printer.

**Spec reference:** `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` §§ 3, 9, 10.

---

## File structure

**Created:**

```
digimon-engine/src/dsl/
├── mod.rs                 # public surface, re-exports
├── spec.rs                # CardSpec + all nested types (§3.2–§3.13)
├── clause.rs              # ClauseSpec, TriggeredClause, DeclarativeClause (§3.5)
├── step.rs                # StepSpec (mutation verbs + control flow) (§3.7)
├── predicate.rs           # PredicateSpec tree (§3.8)
├── formula.rs             # FormulaSpec (§3.10)
├── alt_path.rs            # AltPathSpec + kinds (§3.3)
├── identity.rs            # IdentitySpec + NameAliasSpec (§3.4)
├── loader.rs              # parse YAML file / dir; cards.json cross-check
├── validator.rs           # semantic validation pass
├── raw_rust_registry.rs   # RawRustRegistry trait + StubRegistry
├── pretty.rs              # canonical YAML pretty-printer
├── schema.rs              # JSON Schema export via schemars
└── errors.rs              # DslError + ValidationError types

digimon-engine/cards/_examples/
├── ST2-13.yaml            # Hammer Spark
├── BT17-007.yaml          # Agumon
├── BT22-084.yaml          # Nokia Shiramine
├── BT5-093.yaml           # Tai Kamiya & Matt Ishida
├── BT17-015.yaml          # WarGreymon
├── AD1-025.yaml           # Omnimon
├── BT24-016.yaml          # Lamiamon
├── BT18-019.yaml          # Millenniummon
├── BT20-083.yaml          # Omekamon
├── BT18-102.yaml          # Susanoomon
├── BT13-060.yaml          # Rosemon: Burst Mode
├── BT13-007.yaml          # King Drasil_7D6
├── BT12-112.yaml          # Shoutmon X7: Superior Mode
├── BT10-111.yaml          # Shoutmon (King Version)
└── EX11-012.yaml          # Medusamon

digimon-engine/tests/dsl/
├── main.rs                # test binary entrypoint; mods the submodules
├── parse_minimal.rs       # Task 2 tests
├── parse_alt_paths.rs     # Task 3 tests
├── parse_identity.rs      # Task 4 tests
├── parse_clauses.rs       # Task 5 tests
├── parse_steps.rs         # Task 6 tests
├── parse_predicates.rs    # Task 7 tests
├── parse_formulas.rs      # Task 8 tests
├── parse_declarative.rs   # Task 9 tests
├── loader.rs              # Task 10 tests
├── cross_check.rs         # Task 11 tests
├── validator.rs           # Task 12 tests
├── raw_rust_registry.rs   # Task 13 tests
├── pretty.rs              # Task 14 tests
├── roundtrip.rs           # Task 15 tests
├── schema_export.rs       # Task 16 tests
└── phase0_exit.rs         # Task 18 integration test

tools/
└── dsl-schema-export/     # Task 16 CLI binary
    ├── Cargo.toml
    └── src/main.rs
```

**Modified:**

- `digimon-engine/Cargo.toml` — add deps + feature + `[[test]] name = "dsl"` entry
- `digimon-engine/src/lib.rs` — add `pub mod dsl;` behind `#[cfg(feature = "dsl-yaml-loader")]`
- `Cargo.toml` (workspace root, if workspace exists) — register `tools/dsl-schema-export` member

---

## Task 1: Module skeleton + dependencies + feature flag

**Files:**
- Modify: `digimon-engine/Cargo.toml`
- Modify: `digimon-engine/src/lib.rs`
- Create: `digimon-engine/src/dsl/mod.rs`
- Create: `digimon-engine/src/dsl/errors.rs`
- Create: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Add DSL dependencies and feature flag to `digimon-engine/Cargo.toml`**

Append these dependencies (after `ndarray = "0.17"`):

```toml
# DSL (Phase 0) — behind dsl-yaml-loader feature; ungated on desktop via the
# default feature set but disable-able by the Tauri crate per spec §7a.1.
serde_yml = { version = "0.0.12", optional = true }
schemars = { version = "0.8", features = ["indexmap2"], optional = true }
thiserror = { version = "1", optional = true }
indexmap = { version = "2", features = ["serde"], optional = true }

[features]
default = ["dsl-yaml-loader"]
dsl-yaml-loader = ["dep:serde_yml", "dep:schemars", "dep:thiserror", "dep:indexmap"]
```

Also append a new `[[test]]` entry:

```toml
[[test]]
name = "dsl"
path = "tests/dsl/main.rs"
required-features = ["dsl-yaml-loader"]
```

- [ ] **Step 2: Expose the DSL module from `lib.rs`**

Add after the existing `pub mod serialization;` line in `digimon-engine/src/lib.rs`:

```rust
#[cfg(feature = "dsl-yaml-loader")]
pub mod dsl;
```

- [ ] **Step 3: Write the module skeleton at `digimon-engine/src/dsl/mod.rs`**

```rust
//! Card-scripting DSL (Phase 0 — parse + validate only).
//!
//! See `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` for the
//! spec. This module provides:
//!
//! - [`spec::CardSpec`] — the authored YAML card definition.
//! - [`loader`] — YAML file/dir parsing + `cards.json` cross-check.
//! - [`validator`] — semantic validation (timings, enum names, predicate
//!   type-checks, raw_rust references).
//! - [`pretty`] — canonical YAML pretty-printer (round-trip stable).
//! - [`schema`] — JSON Schema export for IDE tooling.
//! - [`raw_rust_registry::RawRustRegistry`] — trait the validator uses to
//!   resolve `raw_rust:` fn-name references. A stub impl is provided for
//!   tests; the real registry lands in Phase 4.

pub mod alt_path;
pub mod clause;
pub mod errors;
pub mod formula;
pub mod identity;
pub mod loader;
pub mod predicate;
pub mod pretty;
pub mod raw_rust_registry;
pub mod schema;
pub mod spec;
pub mod step;
pub mod validator;

pub use errors::{DslError, ValidationError};
pub use spec::CardSpec;
```

- [ ] **Step 4: Stub all named submodules with empty `pub` interiors**

Create each of these files containing only:

```rust
//! TODO: populated by Task N of the Phase 0 plan.
```

Files to create (empty stubs for now): `alt_path.rs`, `clause.rs`, `formula.rs`, `identity.rs`, `loader.rs`, `predicate.rs`, `pretty.rs`, `raw_rust_registry.rs`, `schema.rs`, `spec.rs`, `step.rs`, `validator.rs`. The `errors.rs` file gets actual content:

`digimon-engine/src/dsl/errors.rs`:

```rust
//! Error types for the DSL.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DslError {
    #[error("IO error loading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("YAML parse error in {path}: {source}")]
    Yaml {
        path: String,
        #[source]
        source: serde_yml::Error,
    },

    #[error("validation failed with {} errors", .0.len())]
    Validation(Vec<ValidationError>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub card_id: String,
    pub path: String, // e.g. "effects[2].process[1].select_hand.filter"
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.card_id, self.path, self.message)
    }
}
```

- [ ] **Step 5: Write the test binary entrypoint at `digimon-engine/tests/dsl/main.rs`**

```rust
//! DSL test binary. Submodules (one per task group) contribute the `#[test]`
//! functions. See `digimon-engine/Cargo.toml` for the `[[test]]` entry.

mod parse_minimal;
```

Create `digimon-engine/tests/dsl/parse_minimal.rs` with just:

```rust
#[test]
fn dsl_module_loads() {
    // Sanity check: the feature-gated module is reachable from tests.
    let _ = digimon_engine::dsl::ValidationError {
        card_id: "X".into(),
        path: "y".into(),
        message: "z".into(),
    };
}
```

- [ ] **Step 6: Verify the scaffold builds and the sanity test passes**

Run: `cargo build --package digimon-engine --features dsl-yaml-loader`
Expected: compiles cleanly with no errors.

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader`
Expected: `test dsl_module_loads ... ok`. 1 passed.

- [ ] **Step 7: Commit**

```bash
git add digimon-engine/Cargo.toml digimon-engine/src/lib.rs \
        digimon-engine/src/dsl/ digimon-engine/tests/dsl/
git commit -m "dsl(phase0): scaffold DSL module + test binary"
```

---

## Task 2: Top-level CardSpec structs

**Files:**
- Modify: `digimon-engine/src/dsl/spec.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`
- Create: `digimon-engine/tests/dsl/parse_minimal.rs` (overwrite scaffold)

- [ ] **Step 1: Write the failing test for a minimal vanilla digimon parse**

Replace `digimon-engine/tests/dsl/parse_minimal.rs` with:

```rust
use digimon_engine::dsl::spec::{CardKind, CardSpec, ColorSpec};

#[test]
fn parse_vanilla_digimon() {
    let yaml = r#"
card: BT1-010
name: Agumon
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
traits: [Reptile]
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    assert_eq!(spec.card, "BT1-010");
    assert_eq!(spec.name, "Agumon");
    assert_eq!(spec.kind, CardKind::Digimon);
    assert_eq!(spec.level, Some(3));
    assert_eq!(spec.color, vec![ColorSpec::Red]);
    assert_eq!(spec.cost, Some(3));
    assert_eq!(spec.dp, Some(2000));
    assert_eq!(spec.traits, vec!["Reptile".to_string()]);
    assert!(spec.effects.is_empty());
    assert!(spec.alt_paths.is_empty());
}

#[test]
fn parse_minimal_option() {
    let yaml = r#"
card: ST2-13
name: Hammer Spark
kind: option
color: [red]
cost: 0
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    assert_eq!(spec.kind, CardKind::Option);
    assert_eq!(spec.level, None);
    assert_eq!(spec.dp, None);
}
```

- [ ] **Step 2: Run test — expect compile error ("CardSpec not defined")**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader`
Expected: compile error `cannot find type CardSpec in this scope`.

- [ ] **Step 3: Implement the top-level structs in `digimon-engine/src/dsl/spec.rs`**

```rust
//! Top-level DSL card-specification types (spec §3.2).

use serde::{Deserialize, Serialize};

/// A complete card definition as authored in YAML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardSpec {
    /// Primary key — must match `cards.json` `card_id`.
    pub card: String,
    /// Authored name — cross-checked against `cards.json` `card_name_eng`.
    pub name: String,
    /// Card kind.
    pub kind: CardKind,
    /// Level — required for digimon + digi_egg; absent for tamer / option / token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    /// Printed colors (may contain >1 for multi-color cards).
    pub color: Vec<ColorSpec>,
    /// Printed play cost. Absent for digi_egg / token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<i32>,
    /// Printed DP (digimon only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dp: Option<i32>,
    /// Traits from `type_eng`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<String>,
    /// Form from `form_eng`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    /// Attribute from `attribute_eng`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute: Option<String>,
    /// Ace `<-N>` — negative integer; lowered to on-leave-field hook.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ace_overflow: Option<i32>,
    /// Identity section (§3.4) — name aliases, mostly X-Antibody.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<crate::dsl::identity::IdentitySpec>,
    /// Alternate entry paths — digivolve / DNA / DigiXros / etc.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alt_paths: Vec<crate::dsl::alt_path::AltPathSpec>,
    /// Triggered + declarative clauses.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<crate::dsl::clause::ClauseSpec>,
    /// DSL file-format version; reserved for §9 open question #7.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_version: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardKind {
    Digimon,
    Tamer,
    Option,
    DigiEgg,
    Token,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorSpec {
    Red,
    Blue,
    Yellow,
    Green,
    Black,
    Purple,
    White,
}
```

Note: the `identity`, `alt_path`, and `clause` submodules referenced here are still empty stubs from Task 1; their types must exist (even as empty structs / enums) for this to compile. Temporarily add placeholder empty structs to those files:

`digimon-engine/src/dsl/identity.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentitySpec {}
```

`digimon-engine/src/dsl/alt_path.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AltPathSpec {}
```

`digimon-engine/src/dsl/clause.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClauseSpec {}
```

These get real content in Tasks 3, 4, 5 respectively.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader parse_minimal`
Expected: `parse_vanilla_digimon ... ok`, `parse_minimal_option ... ok`. 2 passed.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl/spec.rs \
        digimon-engine/src/dsl/identity.rs \
        digimon-engine/src/dsl/alt_path.rs \
        digimon-engine/src/dsl/clause.rs \
        digimon-engine/tests/dsl/parse_minimal.rs
git commit -m "dsl(phase0): top-level CardSpec structs + minimal parse tests"
```

---

## Task 3: AltPathSpec (evolution and assembly entry points)

**Files:**
- Modify: `digimon-engine/src/dsl/alt_path.rs`
- Create: `digimon-engine/tests/dsl/parse_alt_paths.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write failing tests covering every alt-path kind from §3.3**

Create `digimon-engine/tests/dsl/parse_alt_paths.rs`:

```rust
use digimon_engine::dsl::alt_path::{
    AltPathKind, AltPathSpec, CostSpec, MaterialSpec, RepeatSpec,
};
use digimon_engine::dsl::spec::CardSpec;

#[test]
fn parse_standard_digivolve() {
    let yaml = r#"
card: BT17-007
name: Agumon
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
alt_paths:
  - kind: digivolve
    from: { name_is: Koromon }
    cost: 0
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    assert_eq!(spec.alt_paths.len(), 1);
    let ap = &spec.alt_paths[0];
    assert!(matches!(ap.kind, AltPathKind::Digivolve));
    assert_eq!(ap.cost, Some(CostSpec::Literal(0)));
}

#[test]
fn parse_dna_digivolve() {
    let yaml = r#"
card: AD1-025
name: Omnimon
kind: digimon
level: 7
color: [red, blue]
cost: 15
dp: 13000
alt_paths:
  - kind: dna_digivolve
    materials:
      - { level_eq: 6, name_contains: Greymon }
      - { level_eq: 6, name_contains: Garurumon }
    cost: 0
    stacks_unsuspended: true
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let ap = &spec.alt_paths[0];
    assert!(matches!(ap.kind, AltPathKind::DnaDigivolve));
    assert_eq!(ap.materials.len(), 2);
    assert_eq!(ap.stacks_unsuspended, true);
}

#[test]
fn parse_digixros_unbounded() {
    let yaml = r#"
card: BT12-112
name: Shoutmon X7 Superior Mode
kind: digimon
level: 7
color: [red]
cost: 15
dp: 17000
alt_paths:
  - kind: digixros
    materials:
      - filter:
          any_of:
            - trait_has: Xros Heart
            - trait_has: Blue Flare
        repeat: unbounded
        distinct_by: card_number
    cost:
      formula:
        base: 15
        per: material_count
        delta: -1
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let ap = &spec.alt_paths[0];
    assert!(matches!(ap.kind, AltPathKind::DigiXros));
    assert_eq!(ap.materials.len(), 1);
    assert!(matches!(ap.materials[0].repeat, Some(RepeatSpec::Unbounded)));
    match &ap.cost {
        Some(CostSpec::Formula(_)) => {}
        other => panic!("expected formula cost, got {:?}", other),
    }
}

#[test]
fn parse_burst_digivolve_with_extra_cost_and_teardown() {
    let yaml = r#"
card: BT13-060
name: "Rosemon: Burst Mode"
kind: digimon
level: 7
color: [green]
cost: 15
dp: 15000
alt_paths:
  - kind: burst_digivolve
    from: { level_eq: 6, name_is: Rosemon }
    cost: 0
    extra_cost:
      - select_own_permanent:
          bind_as: yoshi
          filter: { kind: tamer, name_is: Yoshino Fujieda }
          prompt: Return Yoshino Fujieda
      - return_to_hand: { target: yoshi }
    on_burst_turn_end:
      - trash_top_source: { target: self }
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let ap = &spec.alt_paths[0];
    assert!(matches!(ap.kind, AltPathKind::BurstDigivolve));
    assert_eq!(ap.extra_cost.as_ref().map(|v| v.len()), Some(2));
    assert_eq!(ap.on_burst_turn_end.as_ref().map(|v| v.len()), Some(1));
}
```

Also modify `digimon-engine/tests/dsl/main.rs` to add:

```rust
mod parse_alt_paths;
```

- [ ] **Step 2: Run tests — expect compile errors for missing types**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader parse_alt_paths`
Expected: compile errors for `AltPathKind`, `CostSpec`, `MaterialSpec`, `RepeatSpec`.

- [ ] **Step 3: Implement `AltPathSpec` in `digimon-engine/src/dsl/alt_path.rs`**

```rust
//! Alternate entry paths — digivolve / DNA / DigiXros / Burst / Hybrid /
//! App Fusion / Activated Digivolve. Spec §3.3.

use serde::{Deserialize, Serialize};

use crate::dsl::formula::FormulaSpec;
use crate::dsl::predicate::PredicateSpec;
use crate::dsl::step::StepSpec;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AltPathSpec {
    pub kind: AltPathKind,

    /// For digivolve / activated_digivolve / burst_digivolve / assembly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<PredicateSpec>,

    /// For dna_digivolve / digixros / app_fusion / assembly.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materials: Vec<MaterialSpec>,

    /// Memory cost — literal or formula. Optional only for `cost_reduction`-driven paths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostSpec>,

    /// DNA stacks both parents under the evolved card, unsuspended.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stacks_unsuspended: bool,

    /// activated_digivolve — ignore printed digivolution requirements.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ignore_requirements: bool,

    /// Identity override (X-Antibody).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_treated_as: Option<String>,

    /// Extra cost steps paid before the path resolves (e.g. "return Yoshino to hand").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_cost: Option<Vec<StepSpec>>,

    /// Burst-digivolve: run at the end of the burst turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_burst_turn_end: Option<Vec<StepSpec>>,

    /// DigiXros `[Hand] [Counter] <Blast Digivolve>` marker form.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub marker: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AltPathKind {
    Digivolve,
    DnaDigivolve,
    DigiXros,
    BurstDigivolve,
    AppFusion,
    Assembly,
    ActivatedDigivolve,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CostSpec {
    Literal(i32),
    Formula(FormulaSpec),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialSpec {
    #[serde(flatten)]
    pub filter: PredicateSpec,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<RepeatSpec>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distinct_by: Option<DistinctBy>,

    /// Zones the material may come from (digixros cross-zone).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zones: Vec<crate::dsl::predicate::Zone>,

    /// Assembly: materials go under the evolved card.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stack_under: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RepeatSpec {
    Keyword(RepeatKeyword),
    Range { min: u8, max: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepeatKeyword {
    Unbounded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistinctBy {
    CardNumber,
    Level,
    Name,
}
```

Note: this adds `formula`, `predicate`, and `step` imports — those modules still have stub content. Ensure each has the minimum to compile:

`digimon-engine/src/dsl/formula.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormulaSpec {
    pub base: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<i32>,
}
```

`digimon-engine/src/dsl/predicate.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredicateSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_is: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_contains: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level_eq: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<crate::dsl::spec::CardKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trait_has: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any_of: Vec<PredicateSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Zone {
    Hand,
    Deck,
    Trash,
    BattleArea,
    Security,
    Breeding,
    Reveal,
    DigiEggDeck,
    Material,
}
```

`digimon-engine/src/dsl/step.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Step in a `process:` or `extra_cost:` list. Expanded in Task 6 to the
/// full mutation-verb set (§3.7); for now a free-form map so Task 3's
/// burst-digivolve YAML parses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepSpec(pub serde_yml::Value);
```

- [ ] **Step 4: Run tests — expect all 4 parse_alt_paths tests to pass**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader parse_alt_paths`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl/alt_path.rs \
        digimon-engine/src/dsl/formula.rs \
        digimon-engine/src/dsl/predicate.rs \
        digimon-engine/src/dsl/step.rs \
        digimon-engine/tests/dsl/parse_alt_paths.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl(phase0): AltPathSpec with all 7 kinds + parse tests"
```

---

## Task 4: IdentitySpec (name aliases, X-Antibody)

**Files:**
- Modify: `digimon-engine/src/dsl/identity.rs`
- Create: `digimon-engine/tests/dsl/parse_identity.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write failing test for identity parsing from spec §3.4 example**

Create `digimon-engine/tests/dsl/parse_identity.rs`:

```rust
use digimon_engine::dsl::identity::{IdentitySpec, NameAliasSpec};
use digimon_engine::dsl::predicate::Zone;
use digimon_engine::dsl::spec::CardSpec;

#[test]
fn parse_name_alias_xantibody() {
    let yaml = r#"
card: BT9-109
name: Omnimon (X Antibody)
kind: digimon
level: 7
color: [red, blue]
cost: 13
dp: 12000
identity:
  name_aliases:
    - treat_as: Omnimon
      when:
        zone: [battle_area]
        has_inherited:
          card_number_is: BT9-109
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let id = spec.identity.as_ref().unwrap();
    assert_eq!(id.name_aliases.len(), 1);
    let alias = &id.name_aliases[0];
    assert_eq!(alias.treat_as, "Omnimon");
    assert_eq!(alias.when.zone, vec![Zone::BattleArea]);
}
```

Add `mod parse_identity;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run test — expect compile error for `NameAliasSpec`**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader parse_identity`
Expected: compile error.

- [ ] **Step 3: Implement `IdentitySpec` in `digimon-engine/src/dsl/identity.rs`**

```rust
//! Identity aliases — mostly X-Antibody cards that are "treated as" their
//! un-X-Antibody name in certain zones. Spec §3.4.

use serde::{Deserialize, Serialize};

use crate::dsl::predicate::Zone;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentitySpec {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub name_aliases: Vec<NameAliasSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NameAliasSpec {
    pub treat_as: String,
    pub when: AliasCondition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AliasCondition {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zone: Vec<Zone>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_inherited: Option<InheritedFilter>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InheritedFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_number_is: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_is: Option<String>,
}
```

- [ ] **Step 4: Run test to verify pass**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader parse_identity`
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl/identity.rs \
        digimon-engine/tests/dsl/parse_identity.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl(phase0): IdentitySpec for X-Antibody name aliases"
```

---

## Task 5: Clause types (triggered vs declarative, scope, timing)

**Files:**
- Modify: `digimon-engine/src/dsl/clause.rs`
- Create: `digimon-engine/tests/dsl/parse_clauses.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write failing tests for both clause families**

Create `digimon-engine/tests/dsl/parse_clauses.rs`:

```rust
use digimon_engine::dsl::clause::{
    ClauseSpec, ClauseScope, DeclarativeKind, Timing, TimingSet,
};
use digimon_engine::dsl::spec::CardSpec;

#[test]
fn parse_triggered_clause_single_timing() {
    let yaml = r#"
card: ST2-13
name: Hammer Spark
kind: option
color: [red]
cost: 0
effects:
  - when: main_from_hand
    process:
      - gain_memory: 1
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let c = &spec.effects[0];
    let t = c.as_triggered().expect("triggered");
    assert!(matches!(t.when, TimingSet::Single(Timing::MainFromHand)));
    assert_eq!(t.scope, ClauseScope::FaceUp);
    assert!(!t.optional);
    assert!(!t.once_per_turn);
}

#[test]
fn parse_triggered_clause_multiple_timings() {
    let yaml = r#"
card: BT17-015
name: WarGreymon
kind: digimon
level: 6
color: [red]
cost: 11
dp: 12000
effects:
  - when: [on_play, when_digivolving]
    process:
      - gain_memory: 0
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let t = spec.effects[0].as_triggered().unwrap();
    match &t.when {
        TimingSet::Multi(v) => {
            assert_eq!(v, &vec![Timing::OnPlay, Timing::WhenDigivolving]);
        }
        _ => panic!("expected multi"),
    }
}

#[test]
fn parse_inherited_scope_clause() {
    let yaml = r#"
card: BT17-015
name: WarGreymon
kind: digimon
level: 6
color: [red]
cost: 11
dp: 12000
effects:
  - scope: inherited
    when: when_attacking
    once_per_turn: true
    process:
      - trash_top_security: { of: opponent }
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let t = spec.effects[0].as_triggered().unwrap();
    assert_eq!(t.scope, ClauseScope::Inherited);
    assert!(t.once_per_turn);
}

#[test]
fn parse_declarative_grant_keyword_clause() {
    let yaml = r#"
card: AD1-025
name: Omnimon
kind: digimon
level: 7
color: [red, blue]
cost: 15
dp: 13000
effects:
  - kind: grant_keyword
    keyword: Raid
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let d = spec.effects[0].as_declarative().unwrap();
    assert_eq!(d.kind, DeclarativeKind::GrantKeyword);
}
```

Add `mod parse_clauses;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run tests — expect compile errors**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader parse_clauses`
Expected: compile errors for `ClauseScope`, `Timing`, `DeclarativeKind`.

- [ ] **Step 3: Implement clause types in `digimon-engine/src/dsl/clause.rs`**

```rust
//! Clause types — triggered (with `when:` + `process:`) vs declarative
//! (with `kind:` discriminator). Spec §3.5.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::dsl::predicate::PredicateSpec;
use crate::dsl::step::StepSpec;

/// A clause is either triggered or declarative. Untagged serde enum —
/// presence of `when:` ⇒ triggered; presence of `kind:` ⇒ declarative.
///
/// The explicit `ClauseKind` key shadows `kind:` on the declarative branch
/// to avoid untagged-enum ambiguity with `kind: grant_keyword` vs
/// triggered clauses (which don't set `kind:`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClauseSpec {
    Triggered(TriggeredClause),
    Declarative(DeclarativeClause),
}

impl ClauseSpec {
    pub fn as_triggered(&self) -> Option<&TriggeredClause> {
        match self {
            ClauseSpec::Triggered(t) => Some(t),
            _ => None,
        }
    }
    pub fn as_declarative(&self) -> Option<&DeclarativeClause> {
        match self {
            ClauseSpec::Declarative(d) => Some(d),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriggeredClause {
    pub when: TimingSet,

    #[serde(default)]
    pub scope: ClauseScope,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_when: Option<PredicateSpec>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<PredicateSpec>,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub once_per_turn: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_per_turn: Option<u8>,

    #[serde(default)]
    pub process: Vec<StepSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TimingSet {
    Single(Timing),
    Multi(Vec<Timing>),
}

/// Every value allowed in `when:`. Maps 1:1 to a variant of
/// `crate::enums::EffectTiming` at lowering time (Phase 2). Spec §3.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Timing {
    OnPlay,
    WhenDigivolving,
    WhenAttacking,
    EndOfAttack,
    EndOfBattle,
    OnAttack,
    OnDeletion,
    OnAnyDeletion,
    OnEnterFieldAnyone,
    OnAllyPlayed,
    OnLeaveField,
    OnSuspend,
    OnUnsuspend,
    OnHatch,
    OnDigivolve,
    OnDnaDigivolve,
    OnDigixros,
    OnOpponentSecurityRemoved,
    OnDigivolutionCardTrashed,
    OnSecurityCheck,
    OnLoseSecurity,
    OnSecurity,
    OnOptionPlaced,
    StartOfYourTurn,
    StartOfOpponentsTurn,
    StartOfYourMainPhase,
    EndOfYourTurn,
    EndOfOpponentsTurn,
    OnAttackTargetChange,
    MainFromHand,
    MainOnField,
    MainFromTrash,
    Counter,
    BeforePayCost,
    Delayed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClauseScope {
    #[default]
    FaceUp,
    Inherited,
    Both,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclarativeClause {
    pub kind: DeclarativeKind,

    #[serde(default)]
    pub scope: ClauseScope,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_when: Option<PredicateSpec>,

    /// Free-form body keyed by clause-kind — validated in Task 9 and 12.
    /// Storing as `IndexMap<String, serde_yml::Value>` preserves key order
    /// for the pretty-printer.
    #[serde(flatten)]
    pub body: IndexMap<String, serde_yml::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclarativeKind {
    Aura,
    CostReduction,
    Replacement,
    Partition,
    AceOverflow,
    GrantKeyword,
    Delay,
    FloodGate,
    AltPathRegistration,
    RawRust,
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader parse_clauses`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl/clause.rs \
        digimon-engine/tests/dsl/parse_clauses.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl(phase0): ClauseSpec (triggered + declarative) with Timing + scope"
```

---

## Task 6: StepSpec — mutation verbs and control flow

**Files:**
- Modify: `digimon-engine/src/dsl/step.rs`
- Create: `digimon-engine/tests/dsl/parse_steps.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write failing tests covering the verb families from §3.7**

Create `digimon-engine/tests/dsl/parse_steps.rs`:

```rust
use digimon_engine::dsl::spec::CardSpec;
use digimon_engine::dsl::step::{BindingRef, RawRustStep, StepSpec};

fn parse_single_step(yaml_body: &str) -> StepSpec {
    let yaml = format!(
        r#"
card: X-1
name: Test
kind: option
color: [red]
cost: 0
effects:
  - when: main_from_hand
    process:
      - {body}
"#,
        body = yaml_body
    );
    let spec: CardSpec = serde_yml::from_str(&yaml).unwrap();
    spec.effects[0].as_triggered().unwrap().process[0].clone()
}

#[test]
fn parse_gain_memory() {
    let step = parse_single_step("gain_memory: 1");
    assert!(matches!(step, StepSpec::GainMemory(1)));
}

#[test]
fn parse_draw() {
    let step = parse_single_step("draw: { of: you, count: 2 }");
    match step {
        StepSpec::Draw(d) => assert_eq!(d.count, 2),
        _ => panic!("expected Draw"),
    }
}

#[test]
fn parse_select_trash_with_binding() {
    let yaml_body = r#"select_trash: { of: you, bind_as: pick, filter: { name_contains: Greymon }, prompt: "Return" }"#;
    let step = parse_single_step(yaml_body);
    match step {
        StepSpec::SelectTrash(s) => {
            assert_eq!(s.bind_as.as_deref(), Some("pick"));
            assert_eq!(s.prompt, "Return");
        }
        _ => panic!("expected SelectTrash"),
    }
}

#[test]
fn parse_if_then_else() {
    let yaml_body = r#"
if: { equals: [branch, 0] }
then:
  - gain_memory: 1
else:
  - gain_memory: 2"#;
    let step = parse_single_step(yaml_body);
    match step {
        StepSpec::If(i) => {
            assert_eq!(i.then.len(), 1);
            assert_eq!(i.else_.as_ref().map(|v| v.len()), Some(1));
        }
        _ => panic!("expected If"),
    }
}

#[test]
fn parse_raw_rust_step() {
    let step = parse_single_step(
        "raw_rust: { fn: my_fn, consumes: [target], binds: [output] }",
    );
    match step {
        StepSpec::RawRust(RawRustStep { fn_name, consumes, binds }) => {
            assert_eq!(fn_name, "my_fn");
            assert_eq!(consumes, vec!["target".to_string()]);
            assert_eq!(binds, vec!["output".to_string()]);
        }
        _ => panic!("expected RawRust"),
    }
}

#[test]
fn parse_delete_permanent_with_binding_ref() {
    let step = parse_single_step("delete_permanent: { target: tgt }");
    match step {
        StepSpec::DeletePermanent(d) => {
            assert_eq!(d.target, BindingRef::Named("tgt".to_string()));
        }
        _ => panic!("expected DeletePermanent"),
    }
}
```

Add `mod parse_steps;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run tests — expect compile errors**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader parse_steps`
Expected: compile errors for step variants.

- [ ] **Step 3: Implement `StepSpec` in `digimon-engine/src/dsl/step.rs`**

Replace the file contents wholesale:

```rust
//! Mutation verbs and control-flow forms for `process:` / `extra_cost:` /
//! `on_burst_turn_end:` step lists. Spec §3.7.
//!
//! The step model is a tagged enum with one variant per verb. The serde
//! representation uses a single-key map per step — e.g.
//! `gain_memory: 1`, `select_trash: { of: you, ... }` — so authors can
//! write natural YAML while the compiler sees a strict sum type.

use serde::{Deserialize, Serialize};

use crate::dsl::predicate::{PredicateSpec, Zone};

/// A single step. Parsed from a one-key YAML map via `#[serde(tag = "verb")]`
/// semantics implemented by the `untagged` + per-variant renames below.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepSpec {
    // Memory / turn
    GainMemory(i32),
    LoseMemory(i32),
    SetMemory(i32),

    // Draw / deck / hand / trash
    Draw(DrawArgs),
    TrashFromTop(DrawArgs),
    AddToHandFromDeck(HandleMoveArgs),
    AddToHandFromTrash(HandleMoveArgs),
    AddToHandFromReveal(HandleMoveArgs),
    TrashFromHandByIndex(IndexedMoveArgs),
    TrashFromReveal(HandleMoveArgs),
    ReturnToDeckFromReveal(ReturnToDeckArgs),
    ShuffleDeck(PlayerArg),
    RevealTopDeck(RevealArgs),
    PlaceRemainderOnDeck(PlaceRemainderArgs),

    // Field / permanent
    DeletePermanent(TargetArg),
    ReturnToHand(TargetArg),
    ReturnToDeck(ReturnPermanentArgs),
    Suspend(TargetArg),
    Unsuspend(TargetArg),
    DeDigivolve(DeDigivolveArgs),
    PlaceOnSecurity(PlaceOnSecurityArgs),
    PlayToken(PlayTokenArgs),
    PlaceAsBottomSource(PlaceAsBottomSourceArgs),
    TrashTopSource(TargetArg),
    Hatch(PlayerArg),

    // Play / digivolve
    PlayFromHand(PlayFromHandArgs),
    PlayFromHandFree(PlayFromHandArgs),
    PlayFromTrash(PlayFromHandArgs),
    PlayFromTrashFree(PlayFromHandArgs),
    PlayFromSecurity(serde_yml::Value), // empty map
    PlayFromMaterials(PlayFromMaterialsArgs),
    EffectInitiatedDigivolve(EffectDigivolveArgs),
    EffectInitiatedDnaDigivolve(EffectDnaDigivolveArgs),

    // Security
    TrashTopSecurity(PlayerArg),
    MarkSecurityFaceUp(MarkSecurityArgs),

    // Modifiers
    AddDpModifier(AddDpModifierArgs),
    AddModifier(AddModifierArgs),
    GrantKeyword(GrantKeywordArgs),

    // Selection
    SelectOwnPermanent(SelectFieldArgs),
    SelectOpponentPermanent(SelectFieldArgs),
    SelectHand(SelectZoneArgs),
    SelectTrash(SelectZoneArgs),
    SelectMaterial(SelectMaterialArgs),
    SelectReveal(SelectZoneArgs),
    SelectSecurity(SelectZoneArgs),
    SelectUnionZone(SelectUnionArgs),
    SelectOrderedPermutation(SelectPermutationArgs),
    SelectCountCappedMulti(SelectCountCappedArgs),
    SelectEffectChoice(SelectEffectChoiceArgs),
    AsSelectingPlayer(AsSelectingPlayerArgs),

    // Control flow
    If(IfStep),
    ForEach(ForEachStep),
    PerSelected(PerSelectedStep),
    ScheduleDelayed(ScheduleDelayedStep),
    Optional(OptionalStep),

    // Escape hatch (step-level)
    RawRust(RawRustStep),
}

// ── Binding references ──────────────────────────────────────────────

/// Used everywhere a step needs to identify a handle: literal `self`,
/// named binding (from `bind_as:`), or a structured reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BindingRef {
    Named(String),
    Structured(StructuredBindingRef),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredBindingRef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permanent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_permanent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone: Option<Zone>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of_permanent: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Player {
    You,
    Opponent,
    Any,
    Active,
}

// ── Argument structs (one per verb family) ──────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerArg {
    pub of: Player,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetArg {
    pub target: BindingRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrawArgs {
    pub of: Player,
    pub count: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandleMoveArgs {
    pub of: Player,
    pub card: BindingRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexedMoveArgs {
    pub of: Player,
    pub hand_index: BindingRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReturnToDeckArgs {
    pub of: Player,
    pub card: BindingRef,
    pub position: StackPosition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReturnPermanentArgs {
    pub target: BindingRef,
    pub position: StackPosition,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_sources: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StackPosition {
    Top,
    Bottom,
    Random,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevealArgs {
    pub of: Player,
    pub count: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone: Option<Zone>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceRemainderArgs {
    pub of: Player,
    pub position: StackPosition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeDigivolveArgs {
    pub target: BindingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_at_level: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceOnSecurityArgs {
    pub of: Player,
    pub source: BindingRef,
    pub position: StackPosition,
    #[serde(default)]
    pub face_up: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayTokenArgs {
    pub controller: Player,
    pub token_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceAsBottomSourceArgs {
    pub source: BindingRef,
    pub target: BindingRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayFromHandArgs {
    pub of: Player,
    pub hand_index: BindingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_delta: Option<CostDelta>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CostDelta {
    Keyword(CostDeltaKeyword),
    Literal(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostDeltaKeyword {
    Free,
    Printed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayFromMaterialsArgs {
    pub target: BindingRef,
    pub source_index: BindingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_delta: Option<CostDelta>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectDigivolveArgs {
    pub target: BindingRef,
    pub from_hand: BindingRef,
    pub cost: i32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ignore_requirements: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectDnaDigivolveArgs {
    pub target_a: BindingRef,
    pub target_b: BindingRef,
    pub from_hand: BindingRef,
    pub cost: i32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ignore_requirements: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkSecurityArgs {
    pub of: Player,
    pub card: BindingRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddDpModifierArgs {
    pub target: BindingRef,
    pub value: i32,
    pub expiry: String, // parsed as enum in Task 12 validation
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddModifierArgs {
    pub target: serde_yml::Value, // filter or BindingRef — type-checked in Task 12
    pub modifier: String,
    pub value: i32,
    pub expiry: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantKeywordArgs {
    pub target: BindingRef,
    pub keyword: String,
    pub expiry: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectFieldArgs {
    pub filter: PredicateSpec,
    pub bind_as: Option<String>,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectZoneArgs {
    pub of: Player,
    pub filter: PredicateSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectMaterialArgs {
    pub of_permanent: BindingRef,
    pub filter: PredicateSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectUnionArgs {
    pub of: Player,
    pub zones: Vec<Zone>,
    pub filter: PredicateSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectPermutationArgs {
    pub items: BindingRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectCountCappedArgs {
    pub of: Player,
    pub zone: Zone,
    pub max: u8,
    pub filter: PredicateSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional_zero: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distinct_by: Option<crate::dsl::alt_path::DistinctBy>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectEffectChoiceArgs {
    pub labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_as: Option<String>,
    #[serde(default)]
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AsSelectingPlayerArgs {
    pub of: Player,
    pub body: Vec<StepSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IfStep {
    #[serde(rename = "if")]
    pub if_: PredicateSpec,
    pub then: Vec<StepSpec>,
    #[serde(default, rename = "else", skip_serializing_if = "Option::is_none")]
    pub else_: Option<Vec<StepSpec>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForEachStep {
    pub over: PredicateSpec,
    pub bind_as: String,
    pub body: Vec<StepSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerSelectedStep {
    pub selection: String, // binding name of the select_count_capped_multi result
    pub bind_as: String,
    pub body: Vec<StepSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleDelayedStep {
    pub when: super::clause::Timing,
    pub body: Vec<StepSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionalStep(pub Vec<StepSpec>);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawRustStep {
    #[serde(rename = "fn")]
    pub fn_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consumes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub binds: Vec<String>,
}
```

The step enum uses `#[serde(rename_all = "snake_case")]` + the default
`external` representation, which gives exactly the `verb: args` single-key
shape card authors write. `StepSpec::GainMemory(1)` serializes to
`gain_memory: 1`, and so on — serde handles the tagging automatically.

- [ ] **Step 4: Run tests to verify all parse_steps tests pass**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader parse_steps`
Expected: 6 passed.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl/step.rs \
        digimon-engine/tests/dsl/parse_steps.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl(phase0): StepSpec enum with mutation verbs + control flow"
```

---

## Task 7: Filter predicate tree

**Files:**
- Modify: `digimon-engine/src/dsl/predicate.rs`
- Create: `digimon-engine/tests/dsl/parse_predicates.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write failing tests for leaf + compound predicates**

Create `digimon-engine/tests/dsl/parse_predicates.rs`:

```rust
use digimon_engine::dsl::predicate::{Owner, PredicateSpec, Zone};

fn parse(yaml: &str) -> PredicateSpec {
    serde_yml::from_str(yaml).unwrap()
}

#[test]
fn parse_leaf_predicates() {
    let p = parse("name_contains: Greymon");
    assert_eq!(p.name_contains.as_deref(), Some("Greymon"));

    let p = parse("level_gte: 6");
    assert_eq!(p.level_gte, Some(6));

    let p = parse("kind: digimon");
    assert_eq!(p.kind, Some(digimon_engine::dsl::spec::CardKind::Digimon));

    let p = parse("trait_has: Royal Knight");
    assert_eq!(p.trait_has.as_deref(), Some("Royal Knight"));

    let p = parse("zone: [battle_area, trash]");
    assert_eq!(p.zone, vec![Zone::BattleArea, Zone::Trash]);

    let p = parse("owner: you");
    assert_eq!(p.owner, Some(Owner::You));
}

#[test]
fn parse_compound_predicates() {
    let yaml = r#"
any_of:
  - name_contains: Garurumon
  - name_contains: Greymon
  - name_contains: Omnimon"#;
    let p = parse(yaml);
    assert_eq!(p.any_of.len(), 3);
    assert_eq!(p.any_of[0].name_contains.as_deref(), Some("Garurumon"));
}

#[test]
fn parse_nested_all_of() {
    let yaml = r#"
all_of:
  - kind: digimon
  - dp_lte: 8000
  - any_of:
      - trait_has: Reptile
      - trait_has: Dragonkin"#;
    let p = parse(yaml);
    assert_eq!(p.all_of.len(), 3);
    assert_eq!(p.all_of[2].any_of.len(), 2);
}

#[test]
fn parse_existential_any_permanent() {
    let yaml = r#"
any_permanent:
  of: you
  zone: [battle_area]
  kind: tamer
  name_contains: "Tai Kamiya""#;
    let p = parse(yaml);
    let ex = p.any_permanent.as_ref().unwrap();
    assert_eq!(ex.of, Owner::You);
    assert_eq!(ex.zone, vec![Zone::BattleArea]);
}

#[test]
fn parse_count_aggregate() {
    let yaml = r#"
count_lte:
  filter: { of: you, zone: [battle_area], kind: digimon }
  n: 1"#;
    let p = parse(yaml);
    let c = p.count_lte.as_ref().unwrap();
    assert_eq!(c.n, 1);
}
```

Add `mod parse_predicates;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run tests — expect failures for missing fields**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader parse_predicates`
Expected: compile errors.

- [ ] **Step 3: Expand `digimon-engine/src/dsl/predicate.rs` to the full §3.8 catalogue**

```rust
//! Filter / predicate tree. Spec §3.8.
//!
//! `PredicateSpec` is a flat struct where every leaf predicate is an
//! `Option<_>` field and compound forms (`all_of` / `any_of` / `none_of`
//! / `not`) are sibling fields. At evaluation time (Phase 2) every
//! present field contributes an AND-joined constraint.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::dsl::formula::FormulaSpec;
use crate::dsl::spec::{CardKind, ColorSpec};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PredicateSpec {
    // Leaf — card/permanent identity
    pub kind: Option<CardKind>,
    pub level_eq: Option<u8>,
    pub level_lte: Option<u8>,
    pub level_gte: Option<u8>,
    pub color_is: Option<ColorSpec>,
    pub color_only: Option<Vec<ColorSpec>>,
    pub trait_has: Option<String>,
    pub form_is: Option<String>,
    pub attribute_is: Option<String>,
    pub name_is: Option<String>,
    pub name_contains: Option<String>,
    pub name_in: Option<Vec<String>>,
    pub card_number_is: Option<String>,

    // Leaf — permanent-only
    pub dp_eq: Option<DpConstraint>,
    pub dp_lte: Option<DpConstraint>,
    pub dp_gte: Option<DpConstraint>,
    pub stack_size_lte: Option<u8>,
    pub stack_size_gte: Option<u8>,
    pub materials_count_lte: Option<u8>,
    pub materials_count_gte: Option<u8>,
    pub has_inherited: Option<Box<PredicateSpec>>,
    pub is_suspended: Option<bool>,
    pub is_unsuspended: Option<bool>,
    pub has_keyword: Option<String>,

    // Leaf — zone / owner
    pub zone: Vec<Zone>,
    pub owner: Option<Owner>,
    pub other: Option<bool>,
    pub of_permanent: Option<String>,

    // Leaf — source-relative
    pub source_is_tamer: Option<bool>,
    pub source_name_contains: Option<String>,
    pub source_permanent_trait_has: Option<String>,

    // Leaf — global / observer
    pub memory_lte: Option<i32>,
    pub memory_gte: Option<i32>,
    pub security_count_lte: Option<u8>,
    pub security_count_gte: Option<u8>,
    pub your_turn: Option<bool>,
    pub opponents_turn: Option<bool>,
    pub all_turns: Option<bool>,
    pub in_breeding: Option<bool>,
    pub on_field: Option<bool>,
    pub dna_origin: Option<bool>,
    pub event_target_kind: Option<CardKind>,
    pub event_target_trait_has: Option<String>,
    pub event_card_trait_has: Option<String>,

    // Binding comparisons
    pub equals: Option<Vec<serde_yml::Value>>,
    pub not_equals: Option<Vec<serde_yml::Value>>,

    // Count aggregates
    pub count_lte: Option<CountAggregate>,
    pub count_gte: Option<CountAggregate>,

    // Existential
    pub any_permanent: Option<Box<ExistentialPredicate>>,
    pub no_permanent: Option<Box<ExistentialPredicate>>,
    pub all_permanents: Option<Box<ExistentialPredicate>>,

    // Compound
    pub all_of: Vec<PredicateSpec>,
    pub any_of: Vec<PredicateSpec>,
    pub none_of: Vec<PredicateSpec>,
    pub not: Option<Box<PredicateSpec>>,

    // Filter additions that only appear in specific contexts —
    // `has_alt_path` on trash filter for BT10-111, etc. The validator
    // (Task 12) constrains these to legal positions.
    pub has_alt_path: Option<String>,

    /// Captures unrecognized fields that later tasks (declarative clauses,
    /// alt-path filters) may introduce — keeps `deny_unknown_fields`
    /// strict while allowing controlled extension.
    #[serde(flatten)]
    pub extra: IndexMap<String, serde_yml::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DpConstraint {
    Literal(i32),
    Formula(FormulaSpec),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountAggregate {
    pub filter: Box<PredicateSpec>,
    pub n: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ExistentialPredicate {
    pub of: Owner,
    #[serde(flatten)]
    pub predicate: PredicateSpec,
}

impl Default for ExistentialPredicate {
    fn default() -> Self {
        Self {
            of: Owner::You,
            predicate: PredicateSpec::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    You,
    Opponent,
    Any,
    Active,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Zone {
    Hand,
    Deck,
    Trash,
    BattleArea,
    Security,
    Breeding,
    Reveal,
    DigiEggDeck,
    Material,
}
```

Note: `#[serde(default)]` at struct-level means unset fields take their `Default` impl (all `Option::None`, empty vecs). `deny_unknown_fields` + `#[serde(flatten)] extra` is a contradiction; drop the `deny_unknown_fields` attribute on this struct to allow the extra map to absorb unknown keys. Update accordingly:

```rust
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PredicateSpec { ... }
```

(Remove `deny_unknown_fields`.)

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader parse_predicates`
Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl/predicate.rs \
        digimon-engine/tests/dsl/parse_predicates.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl(phase0): full PredicateSpec tree per spec §3.8"
```

---

## Task 8: FormulaSpec primitives

**Files:**
- Modify: `digimon-engine/src/dsl/formula.rs`
- Create: `digimon-engine/tests/dsl/parse_formulas.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write failing tests for §3.10 formula shapes**

Create `digimon-engine/tests/dsl/parse_formulas.rs`:

```rust
use digimon_engine::dsl::formula::{FormulaSpec, PerSelector};

fn parse(yaml: &str) -> FormulaSpec {
    serde_yml::from_str(yaml).unwrap()
}

#[test]
fn parse_literal() {
    let f = parse("5");
    assert!(matches!(f, FormulaSpec::Literal(5)));
}

#[test]
fn parse_base_per_delta() {
    let yaml = r#"
base: 15
per: material_count
delta: -1"#;
    let f = parse(yaml);
    match f {
        FormulaSpec::BasePerDelta { base, per, delta } => {
            assert_eq!(base, 15);
            assert!(matches!(per, PerSelector::MaterialCount));
            assert_eq!(delta, -1);
        }
        _ => panic!("expected BasePerDelta"),
    }
}

#[test]
fn parse_floor_div() {
    let f = parse("floor_div: [10, 2]");
    match f {
        FormulaSpec::FloorDiv(parts) => {
            assert_eq!(parts.len(), 2);
        }
        _ => panic!("expected FloorDiv"),
    }
}

#[test]
fn parse_aggregate() {
    let f = parse("aggregate: lowest_dp");
    assert!(matches!(f, FormulaSpec::Aggregate(_)));
}

#[test]
fn parse_raw_rust_formula() {
    let f = parse("raw_rust: my_fn");
    match f {
        FormulaSpec::RawRust(name) => assert_eq!(name, "my_fn"),
        _ => panic!("expected RawRust"),
    }
}
```

Add `mod parse_formulas;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run tests — expect compile errors**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader parse_formulas`
Expected: compile errors.

- [ ] **Step 3: Replace `digimon-engine/src/dsl/formula.rs`**

```rust
//! Formula primitives for scalar computations in predicates and clauses.
//! Spec §3.10.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FormulaSpec {
    Literal(i32),
    BasePerDelta {
        base: i32,
        per: PerSelector,
        delta: i32,
    },
    #[serde(rename_all = "snake_case")]
    FloorDiv(Vec<FormulaSpec>),
    #[serde(rename_all = "snake_case")]
    Max(Vec<FormulaSpec>),
    #[serde(rename_all = "snake_case")]
    Min(Vec<FormulaSpec>),
    #[serde(rename_all = "snake_case")]
    Aggregate(AggregateSelector),
    RawRust(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerSelector {
    MaterialCount,
    StackSize,
    AllyCount,
    DigivolutionColorCount,
    CardCountInZone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregateSelector {
    LowestDp,
    HighestDp,
    LowestLevel,
    HighestLevel,
}
```

Note: the `untagged` enum requires serde to distinguish `FloorDiv` and friends by field shape. Because multiple variants could all deserialize from the same YAML, reorder variants so `Literal` is first and structured variants follow. The test cases for `floor_div:`, `max:`, `min:`, `aggregate:`, `raw_rust:` all have single-key maps — serde_yml will try each variant in declaration order, so the variants' shapes must be distinguishable. An alternative is to use an adjacently-tagged enum with `#[serde(untagged)]` removed. For simplicity in Phase 0, switch to an externally-tagged form:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FormulaSpec {
    Literal(i32),
    Compound(CompoundFormula),
    BasePerDelta {
        base: i32,
        per: PerSelector,
        delta: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompoundFormula {
    FloorDiv(Vec<FormulaSpec>),
    Max(Vec<FormulaSpec>),
    Min(Vec<FormulaSpec>),
    Aggregate(AggregateSelector),
    RawRust(String),
}
```

Update the test to match `FormulaSpec::Compound(CompoundFormula::FloorDiv(...))` etc. Rewrite the test file accordingly:

```rust
use digimon_engine::dsl::formula::{CompoundFormula, FormulaSpec, PerSelector, AggregateSelector};

fn parse(yaml: &str) -> FormulaSpec { serde_yml::from_str(yaml).unwrap() }

#[test]
fn parse_literal() {
    assert!(matches!(parse("5"), FormulaSpec::Literal(5)));
}

#[test]
fn parse_base_per_delta() {
    let yaml = "base: 15\nper: material_count\ndelta: -1";
    match parse(yaml) {
        FormulaSpec::BasePerDelta { base, per, delta } => {
            assert_eq!((base, delta), (15, -1));
            assert!(matches!(per, PerSelector::MaterialCount));
        }
        _ => panic!("expected BasePerDelta"),
    }
}

#[test]
fn parse_floor_div() {
    match parse("floor_div: [10, 2]") {
        FormulaSpec::Compound(CompoundFormula::FloorDiv(v)) => assert_eq!(v.len(), 2),
        _ => panic!("expected FloorDiv"),
    }
}

#[test]
fn parse_aggregate() {
    match parse("aggregate: lowest_dp") {
        FormulaSpec::Compound(CompoundFormula::Aggregate(AggregateSelector::LowestDp)) => {}
        _ => panic!("expected Aggregate(LowestDp)"),
    }
}

#[test]
fn parse_raw_rust_formula() {
    match parse("raw_rust: my_fn") {
        FormulaSpec::Compound(CompoundFormula::RawRust(n)) => assert_eq!(n, "my_fn"),
        _ => panic!("expected RawRust"),
    }
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader parse_formulas`
Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl/formula.rs \
        digimon-engine/tests/dsl/parse_formulas.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl(phase0): FormulaSpec with literal / base-per-delta / compound"
```

---

## Task 9: Declarative clause body schemas

**Files:**
- Modify: `digimon-engine/src/dsl/clause.rs`
- Create: `digimon-engine/tests/dsl/parse_declarative.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

Task 5 introduced `DeclarativeClause` with a free-form `body: IndexMap<String, Value>` so the parse succeeded without per-kind schemas. Task 9 tightens each kind's body into typed args.

- [ ] **Step 1: Write failing tests per declarative-clause kind**

Create `digimon-engine/tests/dsl/parse_declarative.rs`:

```rust
use digimon_engine::dsl::clause::{DeclarativeKind, TypedDeclarativeBody};
use digimon_engine::dsl::spec::CardSpec;

fn parse(yaml: &str) -> CardSpec {
    serde_yml::from_str(yaml).unwrap()
}

fn typed_body(spec: &CardSpec, idx: usize) -> TypedDeclarativeBody {
    spec.effects[idx].as_declarative().unwrap().typed_body().unwrap()
}

#[test]
fn parse_aura_dp_grant() {
    let yaml = r#"
card: BT5-093
name: Tai & Matt
kind: tamer
color: [red, blue]
cost: 4
effects:
  - kind: aura
    active_when: your_turn
    target:
      of: you
      zone: [battle_area]
      name_contains: Omnimon
    dp_modifier: 1000
"#;
    let spec = parse(yaml);
    match typed_body(&spec, 0) {
        TypedDeclarativeBody::Aura(a) => {
            assert_eq!(a.dp_modifier, Some(1000));
        }
        _ => panic!("expected Aura"),
    }
}

#[test]
fn parse_cost_reduction_static() {
    let yaml = r#"
card: BT17-015
name: WarGreymon
kind: digimon
level: 6
color: [red]
cost: 11
dp: 12000
effects:
  - kind: cost_reduction
    scope: before_pay_cost
    when_playing_this: true
    condition:
      any_permanent:
        of: you
        kind: tamer
        name_contains: Tai Kamiya
    amount: 3
"#;
    let spec = parse(yaml);
    match typed_body(&spec, 0) {
        TypedDeclarativeBody::CostReduction(c) => {
            assert_eq!(c.amount, Some(3));
            assert!(c.when_playing_this);
        }
        _ => panic!("expected CostReduction"),
    }
}

#[test]
fn parse_flood_gate() {
    let yaml = r#"
card: BT13-007
name: King Drasil
kind: digi_egg
color: [yellow]
cost: 0
effects:
  - kind: flood_gate
    scope: face_up
    active_when: { all_of: [{ in_breeding: true }, { your_turn: true }] }
    modifier: CannotDigivolve
    target: { of: you, zone: [battle_area] }
"#;
    let spec = parse(yaml);
    match typed_body(&spec, 0) {
        TypedDeclarativeBody::FloodGate(f) => {
            assert_eq!(f.modifier, "CannotDigivolve");
        }
        _ => panic!("expected FloodGate"),
    }
}

#[test]
fn parse_grant_keyword() {
    let yaml = r#"
card: AD1-025
name: Omnimon
kind: digimon
level: 7
color: [red, blue]
cost: 15
dp: 13000
effects:
  - kind: grant_keyword
    keyword: Raid
"#;
    let spec = parse(yaml);
    match typed_body(&spec, 0) {
        TypedDeclarativeBody::GrantKeyword(k) => {
            assert_eq!(k.keyword, "Raid");
            assert_eq!(k.value, None);
        }
        _ => panic!("expected GrantKeyword"),
    }
}

#[test]
fn parse_partition_sources() {
    let yaml = r#"
card: AD1-025
name: Omnimon
kind: digimon
level: 7
color: [red, blue]
cost: 15
dp: 13000
effects:
  - kind: partition
    sources:
      - { name_contains: WarGreymon }
      - { name_contains: MetalGarurumon }
    exclude_cause: [own_effect, battle]
"#;
    let spec = parse(yaml);
    match typed_body(&spec, 0) {
        TypedDeclarativeBody::Partition(p) => {
            assert_eq!(p.sources.len(), 2);
            assert_eq!(p.exclude_cause.len(), 2);
        }
        _ => panic!("expected Partition"),
    }
}

#[test]
fn parse_raw_rust_clause() {
    let yaml = r#"
card: BT10-111
name: Shoutmon KV
kind: digimon
level: 4
color: [red]
cost: 5
dp: 4000
effects:
  - kind: raw_rust
    fn: bt10_111_replacement_wildcard
"#;
    let spec = parse(yaml);
    match typed_body(&spec, 0) {
        TypedDeclarativeBody::RawRust(r) => {
            assert_eq!(r.fn_name, "bt10_111_replacement_wildcard");
        }
        _ => panic!("expected RawRust"),
    }
}
```

Add `mod parse_declarative;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run tests — expect compile errors**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader parse_declarative`
Expected: compile errors for `TypedDeclarativeBody`.

- [ ] **Step 3: Add a typed body view and per-kind arg structs to `clause.rs`**

Append to `digimon-engine/src/dsl/clause.rs`:

```rust
use crate::dsl::formula::FormulaSpec;

#[derive(Debug, Clone, PartialEq)]
pub enum TypedDeclarativeBody {
    Aura(AuraBody),
    CostReduction(CostReductionBody),
    Replacement(ReplacementBody),
    Partition(PartitionBody),
    AceOverflow(AceOverflowBody),
    GrantKeyword(GrantKeywordBody),
    Delay(DelayBody),
    FloodGate(FloodGateBody),
    AltPathRegistration(AltPathRegistrationBody),
    RawRust(RawRustClauseBody),
}

impl DeclarativeClause {
    /// Deserialize the free-form `body:` map into the typed variant matching
    /// `self.kind`. Returns `Err` if the body does not match the kind's
    /// schema. Performs late schema binding — Task 9 introduces the typed
    /// view without changing the parse path.
    pub fn typed_body(&self) -> Result<TypedDeclarativeBody, serde_yml::Error> {
        use serde_yml::Value;

        // Construct a YAML map from the body and deserialize it as the
        // appropriate typed struct.
        let value = Value::Mapping(
            self.body.iter()
                .map(|(k, v)| (Value::String(k.clone()), v.clone()))
                .collect(),
        );

        Ok(match self.kind {
            DeclarativeKind::Aura => TypedDeclarativeBody::Aura(serde_yml::from_value(value)?),
            DeclarativeKind::CostReduction => TypedDeclarativeBody::CostReduction(serde_yml::from_value(value)?),
            DeclarativeKind::Replacement => TypedDeclarativeBody::Replacement(serde_yml::from_value(value)?),
            DeclarativeKind::Partition => TypedDeclarativeBody::Partition(serde_yml::from_value(value)?),
            DeclarativeKind::AceOverflow => TypedDeclarativeBody::AceOverflow(serde_yml::from_value(value)?),
            DeclarativeKind::GrantKeyword => TypedDeclarativeBody::GrantKeyword(serde_yml::from_value(value)?),
            DeclarativeKind::Delay => TypedDeclarativeBody::Delay(serde_yml::from_value(value)?),
            DeclarativeKind::FloodGate => TypedDeclarativeBody::FloodGate(serde_yml::from_value(value)?),
            DeclarativeKind::AltPathRegistration => TypedDeclarativeBody::AltPathRegistration(serde_yml::from_value(value)?),
            DeclarativeKind::RawRust => TypedDeclarativeBody::RawRust(serde_yml::from_value(value)?),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuraBody {
    pub target: PredicateSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dp_modifier: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grant_keyword: Option<GrantKeywordValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modifier: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantKeywordValue {
    pub keyword: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CostReductionBody {
    #[serde(default)]
    pub scope: String, // "before_pay_cost" literal
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub when_playing_this: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when_any_ally_played: Option<PredicateSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<PredicateSpec>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub once_per_turn: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_fn: Option<FormulaSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pay_cost: Option<Vec<crate::dsl::step::StepSpec>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unlocks: Vec<indexmap::IndexMap<String, serde_yml::Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplacementBody {
    pub trigger: String,
    pub process: Vec<crate::dsl::step::StepSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartitionBody {
    pub sources: Vec<PredicateSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_cause: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AceOverflowBody {
    pub value: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantKeywordBody {
    pub keyword: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DelayBody {
    pub trigger: Timing,
    pub process: Vec<crate::dsl::step::StepSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloodGateBody {
    pub modifier: String,
    pub target: PredicateSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AltPathRegistrationBody {
    pub trigger: Timing,
    pub registers: indexmap::IndexMap<String, serde_yml::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applies_to: Option<PredicateSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawRustClauseBody {
    #[serde(rename = "fn")]
    pub fn_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<Timing>,
}
```

- [ ] **Step 4: Run tests — expect all parse_declarative tests to pass**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader parse_declarative`
Expected: 6 passed.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl/clause.rs \
        digimon-engine/tests/dsl/parse_declarative.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl(phase0): typed per-kind declarative-clause bodies"
```

---

## Task 9a: i18n scaffolding (summary / summary_key / prompt_key fields)

Per spec §7b — wire locale-ready keys into every clause and prompt so Phase 1
can harvest them into `locales/en-US.json` at pack-build time. No engine
emission here; pure schema + validator work.

**Files:**
- Modify: `digimon-engine/src/dsl/clause.rs` (add 2 fields to `TriggeredClause`, 2 to `DeclarativeClause`)
- Modify: `digimon-engine/src/dsl/step.rs` (add `prompt_key:` to 8 select-args structs)
- Modify: `digimon-engine/src/dsl/validator.rs` (no-op, but assert fields round-trip)
- Create: `digimon-engine/tests/dsl/i18n_scaffolding.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write failing tests for the new fields**

Create `digimon-engine/tests/dsl/i18n_scaffolding.rs`:

```rust
use digimon_engine::dsl::clause::{ClauseSpec, DeclarativeClause};
use digimon_engine::dsl::spec::CardSpec;
use digimon_engine::dsl::step::StepSpec;

fn parse(yaml: &str) -> CardSpec { serde_yml::from_str(yaml).unwrap() }

#[test]
fn triggered_clause_parses_summary_and_summary_key() {
    let yaml = r#"
card: BT17-015
name: WarGreymon
kind: digimon
level: 6
color: [red]
cost: 11
dp: 12000
effects:
  - when: on_play
    summary: "Delete 8000 DP or digivolve Gabumon free"
    summary_key: BT17-015.onplay
    process:
      - gain_memory: 0
"#;
    let spec = parse(yaml);
    let t = spec.effects[0].as_triggered().unwrap();
    assert_eq!(t.summary.as_deref(), Some("Delete 8000 DP or digivolve Gabumon free"));
    assert_eq!(t.summary_key.as_deref(), Some("BT17-015.onplay"));
}

#[test]
fn declarative_clause_parses_summary() {
    let yaml = r#"
card: BT5-093
name: Tai & Matt
kind: tamer
color: [red, blue]
cost: 4
effects:
  - kind: aura
    summary: "+1 Security Attack on Omnimon"
    active_when: your_turn
    target:
      of: you
      zone: [battle_area]
      name_contains: Omnimon
    grant_keyword: { keyword: SecurityAttackPlus, value: 1 }
"#;
    let spec = parse(yaml);
    let d = spec.effects[0].as_declarative().unwrap();
    assert_eq!(d.summary.as_deref(), Some("+1 Security Attack on Omnimon"));
}

#[test]
fn select_hand_args_parse_prompt_key() {
    let yaml = r#"
card: X-1
name: Test
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: pick
          filter: { name_contains: Koromon }
          prompt: "Return Koromon"
          prompt_key: X-1.clause0.step0
"#;
    let spec = parse(yaml);
    let t = spec.effects[0].as_triggered().unwrap();
    match &t.process[0] {
        StepSpec::SelectHand(args) => {
            assert_eq!(args.prompt, "Return Koromon");
            assert_eq!(args.prompt_key.as_deref(), Some("X-1.clause0.step0"));
        }
        _ => panic!("expected SelectHand"),
    }
}

#[test]
fn i18n_fields_are_all_optional() {
    // Every Phase-0 YAML that parsed before Task 9a must still parse
    // without any i18n fields — they are optional-only additions.
    let yaml = r#"
card: ST2-13
name: Hammer Spark
kind: option
color: [red]
cost: 0
effects:
  - when: main_from_hand
    process:
      - gain_memory: 1
"#;
    let spec = parse(yaml);
    let t = spec.effects[0].as_triggered().unwrap();
    assert_eq!(t.summary, None);
    assert_eq!(t.summary_key, None);
}
```

Add `mod i18n_scaffolding;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run tests — expect compile errors for missing fields**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader i18n_scaffolding`
Expected: compile errors for `summary`, `summary_key`, `prompt_key`.

- [ ] **Step 3: Add `summary` + `summary_key` to `TriggeredClause` and `DeclarativeClause`**

Edit `digimon-engine/src/dsl/clause.rs`, add these two fields to `TriggeredClause`:

```rust
    /// Optional short effect summary displayed when the clause activates.
    /// Authored in `en-US`; used as the canonical localization key when
    /// translated. See spec §7b.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Explicit localization key override. If absent, derived positionally
    /// as `<card_id>.clause[<index>].summary`. Use this for cards whose
    /// translations are already in flight when YAML is reordered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_key: Option<String>,
```

Apply the identical pair of fields to `DeclarativeClause` (add above the existing `#[serde(flatten)] body` field):

```rust
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_key: Option<String>,
```

- [ ] **Step 4: Add `prompt_key` to every `select_*` args struct in `step.rs`**

Edit `digimon-engine/src/dsl/step.rs`. For each of the eight structs
`SelectFieldArgs`, `SelectZoneArgs`, `SelectMaterialArgs`,
`SelectUnionArgs`, `SelectPermutationArgs`, `SelectCountCappedArgs`,
`SelectEffectChoiceArgs`, append a field:

```rust
    /// Optional localization-key override for `prompt`. If absent, derived
    /// positionally from `(card_id, clause_index, step_path)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_key: Option<String>,
```

- [ ] **Step 5: Run tests to verify pass**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader`
Expected: all previously-passing tests remain green (no regressions) + 4
new `i18n_scaffolding` tests pass.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/dsl/clause.rs \
        digimon-engine/src/dsl/step.rs \
        digimon-engine/tests/dsl/i18n_scaffolding.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl(phase0): i18n scaffolding — summary / summary_key / prompt_key per spec §7b"
```

---

## Task 10: YAML file and directory loader

**Files:**
- Modify: `digimon-engine/src/dsl/loader.rs`
- Create: `digimon-engine/tests/dsl/loader.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`
- Create: `digimon-engine/tests/dsl/fixtures/ST2-13.yaml` (temp fixture)
- Create: `digimon-engine/tests/dsl/fixtures/bad.yaml` (temp fixture)

- [ ] **Step 1: Write failing tests covering file + directory loads**

Create fixtures:

`digimon-engine/tests/dsl/fixtures/ST2-13.yaml`:

```yaml
card: ST2-13
name: Hammer Spark
kind: option
color: [red]
cost: 0
effects:
  - when: main_from_hand
    process:
      - gain_memory: 1
  - when: on_security
    process:
      - gain_memory: 2
```

`digimon-engine/tests/dsl/fixtures/bad.yaml`:

```yaml
card: X
this_field_does_not_exist: true
```

Create `digimon-engine/tests/dsl/loader.rs`:

```rust
use digimon_engine::dsl::loader;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/dsl/fixtures")
        .join(name)
}

#[test]
fn load_file_ok() {
    let spec = loader::load_file(&fixture("ST2-13.yaml")).unwrap();
    assert_eq!(spec.card, "ST2-13");
    assert_eq!(spec.effects.len(), 2);
}

#[test]
fn load_file_missing_file() {
    let err = loader::load_file(&fixture("no-such-file.yaml")).unwrap_err();
    assert!(matches!(err, digimon_engine::dsl::DslError::Io { .. }));
}

#[test]
fn load_file_malformed_yaml() {
    let err = loader::load_file(&fixture("bad.yaml")).unwrap_err();
    assert!(matches!(err, digimon_engine::dsl::DslError::Yaml { .. }));
}

#[test]
fn load_dir_loads_all_yaml() {
    let dir = fixture("").parent().unwrap().join("fixtures");
    let specs = loader::load_dir(&dir).unwrap();
    // Includes ST2-13.yaml and bad.yaml (which errors). Use load_dir_ok which
    // skips and reports; plain load_dir returns first error.
    assert!(specs.is_err() || specs.as_ref().map(|v| v.len()).unwrap() >= 1);
}

#[test]
fn load_dir_ok_collects_errors_separately() {
    let dir = fixture("").parent().unwrap().join("fixtures");
    let (loaded, errors) = loader::load_dir_ok(&dir);
    assert!(loaded.iter().any(|s| s.card == "ST2-13"));
    assert_eq!(errors.len(), 1);
}
```

Add `mod loader;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run tests — expect compile errors / missing functions**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader loader`
Expected: compile errors for `load_file`, `load_dir`, `load_dir_ok`.

- [ ] **Step 3: Implement `digimon-engine/src/dsl/loader.rs`**

```rust
//! YAML loader — file and directory entrypoints.

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

/// Load every `*.yaml` under `dir` (recursive). Fails fast on the first
/// error.
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
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader loader`
Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl/loader.rs \
        digimon-engine/tests/dsl/loader.rs \
        digimon-engine/tests/dsl/main.rs \
        digimon-engine/tests/dsl/fixtures/
git commit -m "dsl(phase0): loader::load_file + load_dir + load_dir_ok"
```

---

## Task 11: cards.json cross-check

**Files:**
- Modify: `digimon-engine/src/dsl/loader.rs`
- Create: `digimon-engine/tests/dsl/cross_check.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write failing tests exercising cross-check vs a stub `CardDataDb`**

Create `digimon-engine/tests/dsl/cross_check.rs`:

```rust
use digimon_engine::dsl::loader::{cross_check, CardDataDbStub};
use digimon_engine::dsl::spec::{CardKind, CardSpec, ColorSpec};

fn spec_with_overrides(card: &str, name: &str, kind: CardKind, cost: Option<i32>) -> CardSpec {
    CardSpec {
        card: card.into(),
        name: name.into(),
        kind,
        level: None,
        color: vec![ColorSpec::Red],
        cost,
        dp: None,
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        alt_paths: vec![],
        effects: vec![],
        spec_version: None,
    }
}

#[test]
fn cross_check_matches() {
    let db = CardDataDbStub::new()
        .with_card("ST2-13", "Hammer Spark", CardKind::Option, None, None, Some(0), vec![ColorSpec::Red]);
    let spec = spec_with_overrides("ST2-13", "Hammer Spark", CardKind::Option, Some(0));
    assert!(cross_check(&spec, &db).is_ok());
}

#[test]
fn cross_check_mismatched_name() {
    let db = CardDataDbStub::new()
        .with_card("ST2-13", "Hammer Spark", CardKind::Option, None, None, Some(0), vec![ColorSpec::Red]);
    let spec = spec_with_overrides("ST2-13", "Wrong Name", CardKind::Option, Some(0));
    let err = cross_check(&spec, &db).unwrap_err();
    assert!(err.to_string().contains("name"));
}

#[test]
fn cross_check_mismatched_kind() {
    let db = CardDataDbStub::new()
        .with_card("ST2-13", "Hammer Spark", CardKind::Option, None, None, Some(0), vec![ColorSpec::Red]);
    let spec = spec_with_overrides("ST2-13", "Hammer Spark", CardKind::Digimon, Some(0));
    let err = cross_check(&spec, &db).unwrap_err();
    assert!(err.to_string().contains("kind"));
}

#[test]
fn cross_check_card_id_not_found() {
    let db = CardDataDbStub::new();
    let spec = spec_with_overrides("NOPE-000", "Ghost", CardKind::Option, Some(0));
    let err = cross_check(&spec, &db).unwrap_err();
    assert!(err.to_string().contains("unknown card_id"));
}
```

Add `mod cross_check;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run tests — expect compile errors**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader cross_check`
Expected: missing `cross_check`, `CardDataDbStub`.

- [ ] **Step 3: Extend `digimon-engine/src/dsl/loader.rs` with cross-check API**

Append to `loader.rs`:

```rust
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
    pub fn new() -> Self { Self { inner: HashMap::new() } }

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
/// level, DP, cost, and colors match the structured data. Returns the
/// aggregate of any discrepancies as `ValidationError`s embedded in the
/// returned error.
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
            message: format!("name mismatch: yaml={} cards.json={}", spec.name, row.name),
        });
    }
    if row.kind != spec.kind {
        return Err(ValidationError {
            card_id: spec.card.clone(),
            path: "kind".into(),
            message: format!("kind mismatch: yaml={:?} cards.json={:?}", spec.kind, row.kind),
        });
    }
    if row.level != spec.level {
        return Err(ValidationError {
            card_id: spec.card.clone(),
            path: "level".into(),
            message: format!("level mismatch: yaml={:?} cards.json={:?}", spec.level, row.level),
        });
    }
    if row.dp != spec.dp {
        return Err(ValidationError {
            card_id: spec.card.clone(),
            path: "dp".into(),
            message: format!("dp mismatch: yaml={:?} cards.json={:?}", spec.dp, row.dp),
        });
    }
    if row.cost != spec.cost {
        return Err(ValidationError {
            card_id: spec.card.clone(),
            path: "cost".into(),
            message: format!("cost mismatch: yaml={:?} cards.json={:?}", spec.cost, row.cost),
        });
    }
    // Colors: require same set (ignoring order).
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
```

Note: `ColorSpec` and `CardKind` need `Ord` derived on them for `BTreeSet` to work. Add `#[derive(Ord, PartialOrd)]` to both. Adjust `spec.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CardKind { ... }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ColorSpec { ... }
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader cross_check`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl/loader.rs \
        digimon-engine/src/dsl/spec.rs \
        digimon-engine/tests/dsl/cross_check.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl(phase0): cross_check against CardDataDb + stub for tests"
```

---

## Task 12: Semantic validator

**Files:**
- Modify: `digimon-engine/src/dsl/validator.rs`
- Create: `digimon-engine/tests/dsl/validator.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write failing tests for each validation rule**

Create `digimon-engine/tests/dsl/validator.rs`:

```rust
use digimon_engine::dsl::spec::CardSpec;
use digimon_engine::dsl::validator::{validate, ValidationContext};
use digimon_engine::dsl::raw_rust_registry::{RawRustRegistry, StubRegistry};

fn parse(yaml: &str) -> CardSpec { serde_yml::from_str(yaml).unwrap() }

fn ctx(reg: &dyn RawRustRegistry) -> ValidationContext<'_> {
    ValidationContext { raw_rust: reg }
}

#[test]
fn validate_well_formed_card_passes() {
    let spec = parse(r#"
card: ST2-13
name: Hammer Spark
kind: option
color: [red]
cost: 0
effects:
  - when: main_from_hand
    process:
      - gain_memory: 1
"#);
    let reg = StubRegistry::empty();
    assert!(validate(&spec, &ctx(&reg)).is_ok());
}

#[test]
fn validate_unknown_modifier_name_fails() {
    let spec = parse(r#"
card: X-1
name: Test
kind: tamer
color: [red]
cost: 1
effects:
  - kind: flood_gate
    active_when: { your_turn: true }
    modifier: NotAModifierEnumVariant
    target: { of: you, zone: [battle_area] }
"#);
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ctx(&reg)).unwrap_err();
    assert!(errs.iter().any(|e| e.message.contains("modifier")));
}

#[test]
fn validate_unknown_keyword_name_fails() {
    let spec = parse(r#"
card: X-1
name: Test
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - kind: grant_keyword
    keyword: Flyers
"#);
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ctx(&reg)).unwrap_err();
    assert!(errs.iter().any(|e| e.message.contains("keyword")));
}

#[test]
fn validate_invalid_expiry_fails() {
    let spec = parse(r#"
card: X-1
name: Test
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - add_dp_modifier:
          target: self
          value: 1000
          expiry: forever_and_ever
"#);
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ctx(&reg)).unwrap_err();
    assert!(errs.iter().any(|e| e.message.contains("expiry")));
}

#[test]
fn validate_declarative_body_type_mismatch() {
    let spec = parse(r#"
card: X-1
name: Test
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - kind: aura
    # missing required target: — typed_body() should error
    dp_modifier: 500
"#);
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ctx(&reg)).unwrap_err();
    assert!(errs.iter().any(|e| e.path.contains("effects[0]")));
}
```

Add `mod validator;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run tests — expect missing `validate`, `ValidationContext`**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader validator`
Expected: compile errors.

- [ ] **Step 3: Implement `digimon-engine/src/dsl/validator.rs`**

```rust
//! Semantic validator — runs after YAML parse, before lowering (Phase 2+).
//!
//! Checks:
//! - modifier / keyword / expiry strings resolve to engine enums (string table here)
//! - declarative-clause body schemas deserialize per-kind (via `typed_body`)
//! - `raw_rust:` references resolve in the registry
//! - timings in `when:` are reachable (based on a static allowlist)

use crate::dsl::clause::{ClauseSpec, DeclarativeKind, TriggeredClause};
use crate::dsl::errors::ValidationError;
use crate::dsl::raw_rust_registry::RawRustRegistry;
use crate::dsl::spec::CardSpec;
use crate::dsl::step::StepSpec;

pub struct ValidationContext<'a> {
    pub raw_rust: &'a dyn RawRustRegistry,
}

pub fn validate(spec: &CardSpec, ctx: &ValidationContext<'_>) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    for (i, clause) in spec.effects.iter().enumerate() {
        let prefix = format!("effects[{i}]");
        match clause {
            ClauseSpec::Triggered(t) => validate_triggered(t, &prefix, &spec.card, ctx, &mut errors),
            ClauseSpec::Declarative(d) => {
                // Schema-check body via typed deserialization.
                if let Err(e) = d.typed_body() {
                    errors.push(ValidationError {
                        card_id: spec.card.clone(),
                        path: prefix.clone(),
                        message: format!("declarative body schema: {e}"),
                    });
                    continue;
                }

                match d.kind {
                    DeclarativeKind::RawRust => {
                        if let Ok(crate::dsl::clause::TypedDeclarativeBody::RawRust(body)) = d.typed_body() {
                            if !ctx.raw_rust.contains_fn(&body.fn_name) {
                                errors.push(ValidationError {
                                    card_id: spec.card.clone(),
                                    path: format!("{prefix}.fn"),
                                    message: format!("unknown raw_rust fn: {}", body.fn_name),
                                });
                            }
                        }
                    }
                    DeclarativeKind::FloodGate => {
                        if let Ok(crate::dsl::clause::TypedDeclarativeBody::FloodGate(body)) = d.typed_body() {
                            if !is_known_modifier(&body.modifier) {
                                errors.push(ValidationError {
                                    card_id: spec.card.clone(),
                                    path: format!("{prefix}.modifier"),
                                    message: format!("unknown modifier: {}", body.modifier),
                                });
                            }
                        }
                    }
                    DeclarativeKind::GrantKeyword => {
                        if let Ok(crate::dsl::clause::TypedDeclarativeBody::GrantKeyword(body)) = d.typed_body() {
                            if !is_known_keyword(&body.keyword) {
                                errors.push(ValidationError {
                                    card_id: spec.card.clone(),
                                    path: format!("{prefix}.keyword"),
                                    message: format!("unknown keyword: {}", body.keyword),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn validate_triggered(
    t: &TriggeredClause,
    prefix: &str,
    card_id: &str,
    ctx: &ValidationContext<'_>,
    errors: &mut Vec<ValidationError>,
) {
    for (i, step) in t.process.iter().enumerate() {
        let sp = format!("{prefix}.process[{i}]");
        validate_step(step, &sp, card_id, ctx, errors);
    }
}

fn validate_step(
    step: &StepSpec,
    prefix: &str,
    card_id: &str,
    ctx: &ValidationContext<'_>,
    errors: &mut Vec<ValidationError>,
) {
    match step {
        StepSpec::AddDpModifier(args) => {
            if !is_known_expiry(&args.expiry) {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.expiry"),
                    message: format!("unknown expiry: {}", args.expiry),
                });
            }
        }
        StepSpec::AddModifier(args) => {
            if !is_known_modifier(&args.modifier) {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.modifier"),
                    message: format!("unknown modifier: {}", args.modifier),
                });
            }
            if !is_known_expiry(&args.expiry) {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.expiry"),
                    message: format!("unknown expiry: {}", args.expiry),
                });
            }
        }
        StepSpec::GrantKeyword(args) => {
            if !is_known_keyword(&args.keyword) {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.keyword"),
                    message: format!("unknown keyword: {}", args.keyword),
                });
            }
            if !is_known_expiry(&args.expiry) {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.expiry"),
                    message: format!("unknown expiry: {}", args.expiry),
                });
            }
        }
        StepSpec::RawRust(raw) => {
            if !ctx.raw_rust.contains_fn(&raw.fn_name) {
                errors.push(ValidationError {
                    card_id: card_id.into(),
                    path: format!("{prefix}.fn"),
                    message: format!("unknown raw_rust fn: {}", raw.fn_name),
                });
            }
        }
        StepSpec::If(i) => {
            for (k, s) in i.then.iter().enumerate() {
                validate_step(s, &format!("{prefix}.then[{k}]"), card_id, ctx, errors);
            }
            if let Some(else_) = &i.else_ {
                for (k, s) in else_.iter().enumerate() {
                    validate_step(s, &format!("{prefix}.else[{k}]"), card_id, ctx, errors);
                }
            }
        }
        StepSpec::ForEach(f) => {
            for (k, s) in f.body.iter().enumerate() {
                validate_step(s, &format!("{prefix}.body[{k}]"), card_id, ctx, errors);
            }
        }
        _ => {}
    }
}

/// Known modifier-enum names. Must match `crate::enums::ModifierType`.
fn is_known_modifier(name: &str) -> bool {
    matches!(
        name,
        "ChangeDp" | "ChangeBaseDp" | "DpFloor" | "DontHaveDp"
        | "ChangePlayCost" | "ChangeDigivolveCost" | "CannotReduceCost"
        | "CannotBeDestroyed" | "CannotBeDestroyedByBattle" | "CannotBeDestroyedByEffect" | "CannotBeRemoved"
        | "CannotAttack" | "CannotAttackPlayer" | "CanAttackUnsuspended" | "CanAttackActivePlayer" | "CannotAttackTarget"
        | "CannotSuspend" | "CannotUnsuspend"
        | "CannotBeSelectedByEffect" | "CannotBeAffected"
        | "GrantBlocker" | "GrantRush" | "GrantJamming" | "GrantPiercing" | "GrantReboot"
        | "GrantBlitz" | "GrantAlliance" | "GrantRaid" | "GrantBarrier" | "GrantArmor"
        | "GrantDecoy" | "GrantVortex" | "GrantOverclock"
        | "MayAttack" | "ForceAttack"
        | "SecurityAttackChange"
        | "CannotDigivolve" | "ChangeColor" | "AddColor" | "ChangeLevel"
        | "CannotReturnToHand" | "CannotTrash" | "CannotBlock" | "CannotCounter"
        | "DrawBlock" | "MemoryBlock" | "CannotPlayFromHand"
        | "CannotPlayDigimonByEffect" | "CannotGainMemoryByEffect" | "CannotGainMemoryExceptFromTamers"
        | "CannotReducePlayCost" | "CannotActivateMainEffects" | "CannotActivateWhenDigivolvingEffects"
        | "CannotActivateSecurityEffects" | "CannotDigivolveDigimonByEffect" | "CannotDrawByEffect"
        | "CannotAddSecurityByEffect" | "CannotTrashOpponentSecurity" | "CannotReduceOpponentSecurity"
        | "IgnoreColorRequirement"
    )
}

fn is_known_keyword(name: &str) -> bool {
    matches!(
        name,
        "Blocker" | "SecurityAttackPlus" | "SecurityAttackMinus" | "Rush" | "Jamming"
        | "Piercing" | "Reboot" | "DeDigivolve" | "DrawX" | "Blitz" | "Armor"
        | "Raid" | "Alliance" | "Blast" | "Save" | "Fortitude" | "Overclock"
        | "Barrier" | "Decoy" | "Material" | "Partition" | "Vortex" | "Collision"
        | "Progress" | "Evade" | "MaterialSave" | "Delay"
    )
}

fn is_known_expiry(name: &str) -> bool {
    matches!(
        name,
        "end_of_your_turn" | "end_of_opponents_turn" | "end_of_your_next_turn"
        | "end_of_opponents_next_turn" | "end_of_turn" | "end_of_battle"
        | "end_of_attack" | "permanent" | "until_next_unsuspend" | "while_source_exists"
    )
}
```

- [ ] **Step 4: Implement the `RawRustRegistry` trait stub (required by this task)**

Replace `digimon-engine/src/dsl/raw_rust_registry.rs`:

```rust
//! `RawRustRegistry` — resolves raw_rust fn names referenced by cards.
//!
//! Phase 0 ships the trait and a [`StubRegistry`] for tests. The real
//! registry (populated by `register_all()` on the engine crate) lands in
//! Phase 4 per spec §6.

use std::collections::HashSet;

pub trait RawRustRegistry: Send + Sync {
    fn contains_fn(&self, name: &str) -> bool;
}

#[derive(Debug, Default)]
pub struct StubRegistry {
    names: HashSet<String>,
}

impl StubRegistry {
    pub fn empty() -> Self { Self::default() }

    pub fn with<I: IntoIterator<Item = &'static str>>(names: I) -> Self {
        Self {
            names: names.into_iter().map(String::from).collect(),
        }
    }
}

impl RawRustRegistry for StubRegistry {
    fn contains_fn(&self, name: &str) -> bool {
        self.names.contains(name)
    }
}
```

- [ ] **Step 5: Run tests to verify pass**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader validator`
Expected: 5 passed.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/dsl/validator.rs \
        digimon-engine/src/dsl/raw_rust_registry.rs \
        digimon-engine/tests/dsl/validator.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl(phase0): semantic validator (modifiers, keywords, expiries, raw_rust)"
```

---

## Task 13: Raw-Rust registry integration tests

**Files:**
- Create: `digimon-engine/tests/dsl/raw_rust_registry.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write tests that exercise the registry gate from validator**

Create `digimon-engine/tests/dsl/raw_rust_registry.rs`:

```rust
use digimon_engine::dsl::raw_rust_registry::{RawRustRegistry, StubRegistry};
use digimon_engine::dsl::spec::CardSpec;
use digimon_engine::dsl::validator::{validate, ValidationContext};

fn card_with_raw_rust(fn_name: &str) -> CardSpec {
    let yaml = format!(r#"
card: BT10-111
name: Shoutmon KV
kind: digimon
level: 4
color: [red]
cost: 5
dp: 4000
effects:
  - kind: raw_rust
    fn: {fn_name}
"#);
    serde_yml::from_str(&yaml).unwrap()
}

#[test]
fn missing_fn_fails_validation() {
    let spec = card_with_raw_rust("missing_fn");
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ValidationContext { raw_rust: &reg }).unwrap_err();
    assert!(errs.iter().any(|e| e.message.contains("missing_fn")));
}

#[test]
fn registered_fn_passes_validation() {
    let spec = card_with_raw_rust("present_fn");
    let reg = StubRegistry::with(["present_fn"]);
    assert!(validate(&spec, &ValidationContext { raw_rust: &reg }).is_ok());
}

#[test]
fn step_level_raw_rust_checked_against_registry() {
    let yaml = r#"
card: X-1
name: Test
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - raw_rust:
          fn: unregistered_step_fn
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ValidationContext { raw_rust: &reg }).unwrap_err();
    assert!(errs.iter().any(|e| e.message.contains("unregistered_step_fn")));
}
```

Add `mod raw_rust_registry;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run tests to verify pass (no new impl needed — infra is in Task 12)**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader raw_rust_registry`
Expected: 3 passed.

- [ ] **Step 3: Commit**

```bash
git add digimon-engine/tests/dsl/raw_rust_registry.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl(phase0): raw_rust registry gate tests (whole-clause + step-level)"
```

---

## Task 14: Pretty-printer

**Files:**
- Modify: `digimon-engine/src/dsl/pretty.rs`
- Create: `digimon-engine/tests/dsl/pretty.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write failing tests for the pretty-printer**

Create `digimon-engine/tests/dsl/pretty.rs`:

```rust
use digimon_engine::dsl::pretty::format_spec;
use digimon_engine::dsl::spec::CardSpec;

fn parse(yaml: &str) -> CardSpec { serde_yml::from_str(yaml).unwrap() }

#[test]
fn format_roundtrip_minimal() {
    let original = r#"
card: ST2-13
name: Hammer Spark
kind: option
color:
  - red
cost: 0
"#;
    let spec = parse(original);
    let formatted = format_spec(&spec);
    let reparsed: CardSpec = serde_yml::from_str(&formatted).unwrap();
    assert_eq!(reparsed.card, spec.card);
    assert_eq!(reparsed.name, spec.name);
    assert_eq!(reparsed.kind, spec.kind);
    assert_eq!(reparsed.color, spec.color);
}

#[test]
fn format_is_idempotent() {
    let spec = parse(r#"
card: ST2-13
name: Hammer Spark
kind: option
color: [red]
cost: 0
effects:
  - when: main_from_hand
    process:
      - gain_memory: 1
"#);
    let first = format_spec(&spec);
    let reparsed: CardSpec = serde_yml::from_str(&first).unwrap();
    let second = format_spec(&reparsed);
    assert_eq!(first, second, "pretty-print should be idempotent");
}

#[test]
fn format_preserves_top_level_key_order() {
    let spec = parse(r#"
card: X-1
name: Test
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
traits: [Reptile]
"#);
    let formatted = format_spec(&spec);
    let card_idx = formatted.find("card:").unwrap();
    let name_idx = formatted.find("name:").unwrap();
    let kind_idx = formatted.find("kind:").unwrap();
    let level_idx = formatted.find("level:").unwrap();
    assert!(card_idx < name_idx && name_idx < kind_idx && kind_idx < level_idx);
}
```

Add `mod pretty;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run tests — expect missing `format_spec`**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader pretty`
Expected: compile error.

- [ ] **Step 3: Implement `digimon-engine/src/dsl/pretty.rs`**

```rust
//! Canonical YAML pretty-printer for `CardSpec`.
//!
//! Idempotent: `format_spec(parse(format_spec(spec))) == format_spec(spec)`.
//! Relies on serde_yml's default emitter with our struct field order
//! (declared in `spec.rs`). `#[serde(skip_serializing_if = "...")]` on
//! optional fields keeps output minimal.

use crate::dsl::spec::CardSpec;

pub fn format_spec(spec: &CardSpec) -> String {
    serde_yml::to_string(spec).expect("CardSpec serialization must not fail")
}
```

Phase 0 defers canonical key re-ordering beyond the struct declaration order
— `serde_yml::to_string` already emits fields in declaration order, which is
already canonicalized by the struct definitions in `spec.rs`. Tasks 15 (round-
trip) + 16 (schema) build on this.

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader pretty`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl/pretty.rs \
        digimon-engine/tests/dsl/pretty.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl(phase0): format_spec pretty-printer (idempotent)"
```

---

## Task 15: 15 worked-example YAML fixtures

**Files:**
- Create: `digimon-engine/cards/_examples/*.yaml` (15 files)

- [ ] **Step 1: Create the example directory and copy YAML from spec §10 for each card**

Create `digimon-engine/cards/_examples/ST2-13.yaml`:

```yaml
card: ST2-13
name: Hammer Spark
kind: option
color: [red]
cost: 0
effects:
  - when: main_from_hand
    process:
      - gain_memory: 1
  - when: on_security
    process:
      - gain_memory: 2
```

Create `digimon-engine/cards/_examples/BT17-007.yaml`:

```yaml
card: BT17-007
name: Agumon
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
traits: [Reptile]
alt_paths:
  - kind: digivolve
    from: { name_is: Koromon }
    cost: 0
effects:
  - when: start_of_your_main_phase
    optional: true
    condition:
      any_permanent:
        of: you
        zone: [battle_area]
        kind: tamer
        name_contains: Tai Kamiya
    process:
      - select_trash:
          of: you
          bind_as: pick
          filter:
            any_of:
              - name_contains: Garurumon
              - name_contains: Greymon
              - name_contains: Omnimon
          prompt: Return a card to hand
      - add_to_hand_from_trash: { of: you, card: pick }
  - scope: inherited
    kind: alt_path_registration
    trigger: end_of_your_turn
    registers:
      kind: dna_digivolve
      target_zone: hand
    applies_to:
      of: you
      zone: [battle_area]
```

Create `digimon-engine/cards/_examples/BT22-084.yaml`:

```yaml
card: BT22-084
name: Nokia Shiramine
kind: tamer
color: [red, blue]
cost: 5
effects:
  - when: start_of_your_turn
    condition: { memory_lte: 2 }
    process:
      - set_memory: 3
  - when: [start_of_your_main_phase, on_play]
    optional: true
    condition:
      count_lte:
        filter: { of: you, zone: [battle_area], kind: digimon }
        n: 1
    process:
      - select_hand:
          of: you
          bind_as: pick
          filter:
            any_of:
              - name_contains: Agumon
              - name_contains: Gabumon
          prompt: Play for free
      - play_from_hand_free: { of: you, hand_index: pick }
  - kind: aura
    active_when: all_turns
    target:
      of: you
      zone: [battle_area]
      any_of:
        - name_contains: Greymon
        - name_contains: Garurumon
        - name_contains: Omnimon
    dp_modifier: 1000
  - when: on_security
    process:
      - play_from_security: {}
```

Create `digimon-engine/cards/_examples/BT5-093.yaml`:

```yaml
card: BT5-093
name: Tai Kamiya & Matt Ishida
kind: tamer
color: [red, blue]
cost: 4
effects:
  - when: start_of_your_turn
    condition:
      any_permanent:
        of: opponent
        zone: [battle_area]
        kind: digimon
        level_gte: 6
    process:
      - gain_memory: 2
  - kind: aura
    active_when: your_turn
    target:
      of: you
      zone: [battle_area]
      name_contains: Omnimon
    grant_keyword: { keyword: SecurityAttackPlus, value: 1 }
  - when: on_security
    process:
      - play_from_security: {}
```

Create `digimon-engine/cards/_examples/BT17-015.yaml` (copy from spec §2.2):

```yaml
card: BT17-015
name: WarGreymon
kind: digimon
level: 6
color: [red]
cost: 11
dp: 12000
traits: [Dragonkin]
alt_paths:
  - kind: digivolve
    from: { level_eq: 5, name_contains: Greymon }
    cost: 3
effects:
  - kind: cost_reduction
    scope: before_pay_cost
    when_playing_this: true
    condition:
      any_permanent:
        of: you
        zone: [battle_area]
        kind: tamer
        name_contains: Tai Kamiya
    amount: 3
  - when: [on_play, when_digivolving]
    process:
      - select_effect_choice:
          bind_as: branch
          labels:
            - Delete opponent Digimon
            - Digivolve Gabumon free
      - if: { equals: [branch, 0] }
        then:
          - select_opponent_permanent:
              bind_as: target
              filter:
                all_of:
                  - kind: digimon
                  - dp_lte: 8000
              prompt: Delete a Digimon
          - delete_permanent: { target: target }
      - if: { equals: [branch, 1] }
        then:
          - select_own_permanent:
              bind_as: base
              filter: { name_contains: Gabumon }
              prompt: Choose Gabumon
          - select_hand:
              of: you
              bind_as: evo
              filter: { name_contains: MetalGarurumon }
              prompt: "Digivolve into..."
          - effect_initiated_digivolve:
              target: base
              from_hand: evo
              cost: 0
              ignore_requirements: true
  - scope: inherited
    when: when_attacking
    once_per_turn: true
    condition: { source_name_contains: Omnimon }
    process:
      - trash_top_security: { of: opponent }
```

Create `digimon-engine/cards/_examples/AD1-025.yaml`, `BT24-016.yaml`, `BT18-019.yaml`, `BT20-083.yaml`, `BT18-102.yaml`, `BT13-060.yaml`, `BT13-007.yaml`, `BT12-112.yaml`, `BT10-111.yaml`, `EX11-012.yaml` copying from spec §10.6 through §10.15 verbatim, adjusting only for YAML syntax correctness (quoting any string containing `:` or `<` or starting with `[`).

**i18n demonstration:** at least two fixtures must exercise the
`summary:` field introduced in Task 9a so the schema's authored
surface is tested end-to-end. Add these lines to the fixtures:

- `BT17-015.yaml`: add `summary: "Delete 8000 DP or digivolve Gabumon free"` to the `[on_play, when_digivolving]` clause; add `summary: "-3 cost with Tai Kamiya"` to the `cost_reduction` clause.
- `BT5-093.yaml`: add `summary: "+1 Security Attack on Omnimon"` to the `aura` clause.
- `AD1-025.yaml`: add `summary: "Raid / Blocker / Partition"` to one of the `grant_keyword` clauses (authored summary is a judgment call — one short label is fine).

Remaining 12 fixtures do not need summaries for Phase 0 — the schema
allows absent.

- [ ] **Step 2: Commit the fixtures without loading them yet**

```bash
git add digimon-engine/cards/_examples/
git commit -m "dsl(phase0): 15 worked-example YAMLs from spec §10"
```

---

## Task 16: Round-trip property test across all 15 examples

**Files:**
- Create: `digimon-engine/tests/dsl/roundtrip.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the round-trip property test**

Create `digimon-engine/tests/dsl/roundtrip.rs`:

```rust
use digimon_engine::dsl::loader;
use digimon_engine::dsl::pretty::format_spec;
use digimon_engine::dsl::raw_rust_registry::StubRegistry;
use digimon_engine::dsl::spec::CardSpec;
use digimon_engine::dsl::validator::{validate, ValidationContext};
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/_examples")
}

fn registry_for_examples() -> StubRegistry {
    StubRegistry::with([
        "bt13_007_royal_knight_cost_reduction",
        "bt10_111_arm_digixros_wildcard_for_turn",
        "ad1_025_on_play_process",
    ])
}

#[test]
fn every_example_parses() {
    let (loaded, errors) = loader::load_dir_ok(&examples_dir());
    assert!(errors.is_empty(), "parse errors: {:#?}", errors);
    assert_eq!(loaded.len(), 15, "expected 15 worked examples, got {}", loaded.len());
}

#[test]
fn every_example_validates() {
    let (specs, _) = loader::load_dir_ok(&examples_dir());
    let reg = registry_for_examples();
    let ctx = ValidationContext { raw_rust: &reg };
    let mut failures = Vec::new();
    for spec in &specs {
        if let Err(errs) = validate(spec, &ctx) {
            for e in errs {
                failures.push(format!("{}: {}", spec.card, e));
            }
        }
    }
    assert!(failures.is_empty(), "validation failures:\n{}", failures.join("\n"));
}

#[test]
fn every_example_round_trips() {
    let (specs, _) = loader::load_dir_ok(&examples_dir());
    for spec in specs {
        let formatted = format_spec(&spec);
        let reparsed: CardSpec = serde_yml::from_str(&formatted).unwrap_or_else(|e| {
            panic!("{} failed to reparse:\n{}\nerror: {}", spec.card, formatted, e)
        });
        assert_eq!(reparsed, spec, "round-trip mismatch for {}", spec.card);
    }
}
```

Add `mod roundtrip;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run tests — fixes the fixtures until all three pass**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader roundtrip`
Expected: 3 passed. If any fail, fix the offending YAML file — do not relax the test.

- [ ] **Step 3: Commit**

```bash
git add digimon-engine/tests/dsl/roundtrip.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl(phase0): round-trip property test across 15 examples"
```

---

## Task 17: JSON Schema export

**Files:**
- Modify: `digimon-engine/src/dsl/schema.rs`
- Modify: `digimon-engine/src/dsl/spec.rs` (add `JsonSchema` derives)
- Modify: `digimon-engine/src/dsl/clause.rs` (ditto)
- Modify: `digimon-engine/src/dsl/predicate.rs` (ditto)
- Modify: `digimon-engine/src/dsl/formula.rs` (ditto)
- Modify: `digimon-engine/src/dsl/step.rs` (ditto)
- Modify: `digimon-engine/src/dsl/alt_path.rs` (ditto)
- Modify: `digimon-engine/src/dsl/identity.rs` (ditto)
- Create: `tools/dsl-schema-export/Cargo.toml`
- Create: `tools/dsl-schema-export/src/main.rs`
- Create: `digimon-engine/tests/dsl/schema_export.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write failing test for schema export**

Create `digimon-engine/tests/dsl/schema_export.rs`:

```rust
use digimon_engine::dsl::schema::export_json_schema;

#[test]
fn schema_export_is_valid_json() {
    let s = export_json_schema();
    let v: serde_json::Value = serde_json::from_str(&s).expect("schema should be JSON");
    assert!(v.is_object());
    assert!(v.get("$schema").is_some(), "schema should declare $schema");
    assert!(v.get("title").is_some());
}

#[test]
fn schema_export_is_deterministic() {
    let a = export_json_schema();
    let b = export_json_schema();
    assert_eq!(a, b, "schema export must be deterministic");
}

#[test]
fn schema_export_mentions_top_level_card_spec_fields() {
    let s = export_json_schema();
    assert!(s.contains("\"card\""));
    assert!(s.contains("\"name\""));
    assert!(s.contains("\"kind\""));
    assert!(s.contains("\"effects\""));
}
```

Add `mod schema_export;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run tests — expect compile errors**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader schema_export`
Expected: missing `export_json_schema`.

- [ ] **Step 3: Add `JsonSchema` derives to every DSL struct**

For every struct/enum defined in `spec.rs`, `clause.rs`, `predicate.rs`, `formula.rs`, `step.rs`, `alt_path.rs`, `identity.rs`, change:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
```

to:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
```

For `PartialEq + Default` structs, add `schemars::JsonSchema` similarly.

- [ ] **Step 4: Implement `digimon-engine/src/dsl/schema.rs`**

```rust
//! JSON Schema export — consumed by the VS Code YAML extension for
//! auto-complete + inline validation.

use schemars::schema_for;

use crate::dsl::spec::CardSpec;

pub fn export_json_schema() -> String {
    let schema = schema_for!(CardSpec);
    serde_json::to_string_pretty(&schema).expect("schema serialization must not fail")
}
```

- [ ] **Step 5: Create a CLI binary for stdout export**

Create `tools/dsl-schema-export/Cargo.toml`:

```toml
[package]
name = "dsl-schema-export"
version = "0.1.0"
edition = "2021"

[dependencies]
digimon-engine = { path = "../../digimon-engine", features = ["dsl-yaml-loader"] }
```

Create `tools/dsl-schema-export/src/main.rs`:

```rust
fn main() {
    println!("{}", digimon_engine::dsl::schema::export_json_schema());
}
```

If the repo is a Cargo workspace, register `tools/dsl-schema-export` in the workspace root `Cargo.toml`'s `[workspace] members = [...]` list. Otherwise leave it as a standalone crate.

- [ ] **Step 6: Run tests to verify pass**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader schema_export`
Expected: 3 passed.

Run: `cargo run --package dsl-schema-export | head -20`
Expected: valid JSON output beginning with `{`.

- [ ] **Step 7: Commit**

```bash
git add digimon-engine/src/dsl/schema.rs \
        digimon-engine/src/dsl/spec.rs \
        digimon-engine/src/dsl/clause.rs \
        digimon-engine/src/dsl/predicate.rs \
        digimon-engine/src/dsl/formula.rs \
        digimon-engine/src/dsl/step.rs \
        digimon-engine/src/dsl/alt_path.rs \
        digimon-engine/src/dsl/identity.rs \
        tools/dsl-schema-export/ \
        digimon-engine/tests/dsl/schema_export.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl(phase0): JSON Schema export + stdout CLI binary"
```

---

## Task 18: Phase 0 exit integration test

**Files:**
- Create: `digimon-engine/tests/dsl/phase0_exit.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

Single end-to-end assertion that Phase 0 has shipped: 15 YAMLs load, validate
(with the example-card raw_rust fns registered), cross-check against real
`cards.json`, round-trip, and emit a non-empty JSON Schema.

- [ ] **Step 1: Write the integration test**

Create `digimon-engine/tests/dsl/phase0_exit.rs`:

```rust
use digimon_engine::dsl::loader::{self, cross_check, CardDataDb, CardDataDbStub};
use digimon_engine::dsl::pretty::format_spec;
use digimon_engine::dsl::raw_rust_registry::StubRegistry;
use digimon_engine::dsl::schema::export_json_schema;
use digimon_engine::dsl::spec::{CardSpec, CardKind, ColorSpec};
use digimon_engine::dsl::validator::{validate, ValidationContext};
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/_examples")
}

fn build_stub_db() -> CardDataDbStub {
    // Phase 0 ships a hand-crafted stub per card; Phase 1 will replace this
    // with a real loader over `digimon_gym/engine/data/cards.json`.
    CardDataDbStub::new()
        .with_card("ST2-13", "Hammer Spark", CardKind::Option, None, None, Some(0), vec![ColorSpec::Red])
        .with_card("BT17-007", "Agumon", CardKind::Digimon, Some(3), Some(2000), Some(3), vec![ColorSpec::Red])
        .with_card("BT22-084", "Nokia Shiramine", CardKind::Tamer, None, None, Some(5), vec![ColorSpec::Red, ColorSpec::Blue])
        .with_card("BT5-093", "Tai Kamiya & Matt Ishida", CardKind::Tamer, None, None, Some(4), vec![ColorSpec::Red, ColorSpec::Blue])
        .with_card("BT17-015", "WarGreymon", CardKind::Digimon, Some(6), Some(12000), Some(11), vec![ColorSpec::Red])
        .with_card("AD1-025", "Omnimon", CardKind::Digimon, Some(7), Some(13000), Some(15), vec![ColorSpec::Red, ColorSpec::Blue])
        .with_card("BT24-016", "Lamiamon", CardKind::Digimon, Some(5), Some(7000), Some(7), vec![ColorSpec::Red])
        .with_card("BT18-019", "Millenniummon", CardKind::Digimon, Some(7), Some(13000), Some(14), vec![ColorSpec::Black])
        .with_card("BT20-083", "Omekamon", CardKind::Digimon, Some(4), Some(4000), Some(5), vec![ColorSpec::Red, ColorSpec::Blue])
        .with_card("BT18-102", "Susanoomon", CardKind::Digimon, Some(7), Some(15000), Some(9),
            vec![ColorSpec::Red, ColorSpec::Blue, ColorSpec::Yellow, ColorSpec::Green, ColorSpec::Black, ColorSpec::Purple])
        .with_card("BT13-060", "Rosemon: Burst Mode", CardKind::Digimon, Some(7), Some(15000), Some(15), vec![ColorSpec::Green])
        .with_card("BT13-007", "King Drasil_7D6", CardKind::DigiEgg, None, None, Some(0), vec![ColorSpec::Yellow])
        .with_card("BT12-112", "Shoutmon X7: Superior Mode", CardKind::Digimon, Some(7), Some(17000), Some(15), vec![ColorSpec::Red])
        .with_card("BT10-111", "Shoutmon (King Version)", CardKind::Digimon, Some(4), Some(4000), Some(5), vec![ColorSpec::Red])
        .with_card("EX11-012", "Medusamon", CardKind::Digimon, Some(6), Some(11000), Some(11), vec![ColorSpec::Purple])
}

#[test]
fn phase_0_exit_criteria() {
    let (specs, errors) = loader::load_dir_ok(&examples_dir());
    assert!(errors.is_empty(), "parse errors: {errors:#?}");
    assert_eq!(specs.len(), 15, "expected exactly 15 examples");

    let reg = StubRegistry::with([
        "bt13_007_royal_knight_cost_reduction",
        "bt10_111_arm_digixros_wildcard_for_turn",
        "ad1_025_on_play_process",
    ]);
    let ctx = ValidationContext { raw_rust: &reg };

    let db = build_stub_db();

    for spec in &specs {
        validate(spec, &ctx).unwrap_or_else(|errs| {
            panic!("validation failed for {}: {:#?}", spec.card, errs)
        });
        cross_check(spec, &db).unwrap_or_else(|e| {
            panic!("cross-check failed for {}: {}", spec.card, e)
        });
        let printed = format_spec(spec);
        let reparsed: CardSpec = serde_yml::from_str(&printed).unwrap_or_else(|e| {
            panic!("reparse of {} failed: {e}\nprinted:\n{printed}", spec.card)
        });
        assert_eq!(&reparsed, spec, "round-trip mismatch for {}", spec.card);
    }

    let schema = export_json_schema();
    assert!(!schema.is_empty());
    assert!(schema.contains("\"CardSpec\""));
}
```

Add `mod phase0_exit;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run the full DSL test suite**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader`
Expected: every test passes; zero failures; zero warnings you can't justify.

- [ ] **Step 3: Document Phase 0 completion**

Create `digimon-engine/src/dsl/README.md`:

```markdown
# DSL — Phase 0

Parse + validate + round-trip + JSON Schema export. No engine integration.

## Entry points

- `loader::load_file(path)` / `loader::load_dir(dir)` / `loader::load_dir_ok(dir)`
- `loader::cross_check(spec, db)`
- `validator::validate(spec, ctx)`
- `pretty::format_spec(spec)`
- `schema::export_json_schema()`
- `raw_rust_registry::RawRustRegistry` trait + `StubRegistry` test impl

## Status

Phase 0 exit criteria met per `tests/dsl/phase0_exit.rs`:
- 15/15 worked examples parse, validate, cross-check, round-trip.
- JSON Schema is deterministic and non-empty.

Next: Phase 1 plan — AOT lowering to `Effect` closures + `build.rs` + rkyv
blob + `from_embedded()` (see spec §7a).
```

- [ ] **Step 4: Commit**

```bash
git add digimon-engine/tests/dsl/phase0_exit.rs \
        digimon-engine/tests/dsl/main.rs \
        digimon-engine/src/dsl/README.md
git commit -m "dsl(phase0): integration test + README; Phase 0 exit criteria green"
```

---

---

## Task 19: `dsl-lint` CLI — agent-facing and VS Code-facing linter

Ship a CLI binary that runs loader + validator on a file or directory and
emits diagnostics in one of two formats: human-readable (VS Code
problem-matcher friendly) or JSON (for agent consumption by the future
`/batch-implement-cards-rust-dsl` skill). This closes the loop between
the DSL's validation machinery and card-authoring workflows, human and
agent alike.

**Files:**
- Create: `tools/dsl-lint/Cargo.toml`
- Create: `tools/dsl-lint/src/main.rs`
- Modify: root `Cargo.toml` workspace members (add `tools/dsl-lint`)

**Scope discipline:**
- Parse + semantic validation only. **No** `cards.json` cross-check yet — needs
  the real `CardData` adapter which is Phase 1 work. `dsl-lint` gets a stub
  DB or skips cross-check entirely for Phase 0.
- No LSP, no file watching, no daemon. Invoke-and-exit.
- No new validation rules beyond what `validator::validate()` already does.

### Step 1: Scaffold the crate

Create `tools/dsl-lint/Cargo.toml`:

```toml
[package]
name = "dsl-lint"
version = "0.1.0"
edition = "2021"
description = "CLI linter for digimon-engine DSL YAML files"

[dependencies]
digimon-engine = { path = "../../digimon-engine", features = ["dsl-yaml-loader"] }
serde_json = "1"

[[bin]]
name = "dsl-lint"
path = "src/main.rs"
```

Register `tools/dsl-lint` in the root workspace `Cargo.toml`'s
`[workspace] members = [...]` list (alongside `tools/dsl-schema-export`).

### Step 2: Implement the CLI

Create `tools/dsl-lint/src/main.rs`:

```rust
//! DSL linter CLI.
//!
//! Usage:
//!   dsl-lint <path> [--format human|json] [--strict]
//!
//! Exit codes:
//!   0 — no diagnostics
//!   1 — errors found (or warnings, if --strict)
//!   2 — warnings only (non-strict)
//!   3 — usage error

use digimon_engine::dsl::loader;
use digimon_engine::dsl::raw_rust_registry::StubRegistry;
use digimon_engine::dsl::validator::{validate, ValidationContext};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Human,
    Json,
}

#[derive(Debug)]
struct Args {
    path: PathBuf,
    format: Format,
    strict: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut path: Option<PathBuf> = None;
    let mut format = Format::Human;
    let mut strict = false;
    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--format" => {
                let v = iter.next().ok_or("--format requires a value")?;
                format = match v.as_str() {
                    "human" => Format::Human,
                    "json" => Format::Json,
                    other => return Err(format!("unknown format: {other}")),
                };
            }
            "--strict" => strict = true,
            "-h" | "--help" => {
                println!("Usage: dsl-lint <path> [--format human|json] [--strict]");
                std::process::exit(0);
            }
            s if s.starts_with("--") => return Err(format!("unknown flag: {s}")),
            _ => {
                if path.is_some() {
                    return Err("multiple path arguments".into());
                }
                path = Some(PathBuf::from(arg));
            }
        }
    }
    let path = path.ok_or("missing <path> argument")?;
    Ok(Args { path, format, strict })
}

#[derive(serde::Serialize, Debug)]
struct Diagnostic {
    file: String,
    severity: Severity,
    path: String,
    message: String,
}

#[derive(serde::Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Severity {
    Error,
    Warning,
}

fn lint_file(path: &Path, diags: &mut Vec<Diagnostic>) {
    let file = path.display().to_string();
    let spec = match loader::load_file(path) {
        Ok(s) => s,
        Err(e) => {
            diags.push(Diagnostic {
                file: file.clone(),
                severity: Severity::Error,
                path: String::new(),
                message: format!("{e}"),
            });
            return;
        }
    };

    let registry = StubRegistry::with([
        // Phase 0 known fns — Phase 1 replaces with a real registry.
        "bt13_007_royal_knight_cost_reduction",
        "bt10_111_arm_digixros_wildcard_for_turn",
        "ad1_025_on_play_process",
    ]);
    let ctx = ValidationContext { raw_rust: &registry };
    if let Err(errs) = validate(&spec, &ctx) {
        for e in errs {
            diags.push(Diagnostic {
                file: file.clone(),
                severity: Severity::Error,
                path: e.path,
                message: e.message,
            });
        }
    }
}

fn walk_yaml(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    if !path.is_dir() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut stack = vec![path.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(iter) = std::fs::read_dir(&d) else { continue };
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

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("dsl-lint: {e}");
            eprintln!("try --help");
            return ExitCode::from(3);
        }
    };

    let mut diags = Vec::new();
    for file in walk_yaml(&args.path) {
        lint_file(&file, &mut diags);
    }

    match args.format {
        Format::Human => {
            for d in &diags {
                // VS Code default problem-matcher friendly shape.
                // Line/col are always 1:1 here because serde_yml doesn't
                // expose position info through our wrappers. Agents can
                // still grep by file + path.
                println!(
                    "{}:1:1: {}: [{}] {}",
                    d.file,
                    match d.severity { Severity::Error => "error", Severity::Warning => "warning" },
                    d.path, d.message
                );
            }
        }
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&diags).unwrap());
        }
    }

    let has_errors = diags.iter().any(|d| d.severity == Severity::Error);
    let has_warnings = diags.iter().any(|d| d.severity == Severity::Warning);
    if has_errors || (args.strict && has_warnings) {
        ExitCode::from(1)
    } else if has_warnings {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}
```

### Step 3: Smoke test — manual verification

Run against a known-good fixture:
```bash
cargo run -p dsl-lint -- digimon-engine/cards/_examples/ST2-13.yaml
```
Expected: empty output, exit 0.

Run against a known-bad fixture (create inline):
```bash
cat > /tmp/bad-card.yaml <<'EOF'
card: X-1
name: Test
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - kind: grant_keyword
    keyword: Flyers
EOF
cargo run -p dsl-lint -- /tmp/bad-card.yaml
```
Expected: one diagnostic mentioning `unknown keyword: Flyers`, exit 1.

Also run JSON format:
```bash
cargo run -p dsl-lint -- /tmp/bad-card.yaml --format json
```
Expected: valid JSON array with one object having `severity: "error"` and `message: "unknown keyword: Flyers"`.

Run against the whole examples directory:
```bash
cargo run -p dsl-lint -- digimon-engine/cards/_examples
```
Expected: empty output, exit 0 (all 15 fixtures are validator-clean per Task 18).

### Step 4: Commit

```bash
git add tools/dsl-lint/ Cargo.toml
git commit -m "dsl(phase0): dsl-lint CLI (human + JSON output for agents and VS Code)"
```

## Phase 0 done

All 19 tasks shipped. Phase 1 plan (AOT compiler for the declarative subset,
`build.rs` → rkyv blob, `CardRegistry::from_embedded()`, real `cards.json`
adapter for `dsl-lint` cross-check) is the next write.
