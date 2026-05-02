# Card-scripting DSL — Phase 1b Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the AOT lowering infrastructure: extract the DSL to a separate leaf crate so `build.rs` can depend on it, define a rkyv-friendly `CompiledCard` IR, compile authored `CardSpec`s to a `CardPack` blob at build time, and expose a `CardRegistry` that desktop binaries consume via `include_bytes!` and runtime-downloaded packs via a cache-directory loader. Still no engine integration — the compiled artifact is a standalone data structure. Phase 1c bridges it to `Effect` closures.

**Architecture:** The key constraint is that `build.rs` runs before `digimon-engine` compiles, so it can't call into `digimon-engine`'s own DSL code. The clean fix is a leaf crate `digimon-dsl/` at the workspace root holding all DSL types + loader + validator + compile + pack serialization. `digimon-engine` declares `digimon-dsl` as both a runtime dependency (for the `RealCardDataAdapter` bridge) and a build-dependency (so `build.rs` can invoke the compile pipeline). `tools/dsl-lint` and `tools/dsl-schema-export` point at `digimon-dsl` directly. `digimon-engine/src/dsl_bridge.rs` holds the one piece that needs both worlds: the `RealCardDataAdapter` that maps `crate::card_data::CardData` to `digimon_dsl::CardDataDb`.

**Tech Stack:** `bincode = "1.3"` for compact serialization of compiled card packs. (Originally specified rkyv; pivoted to bincode during execution because rkyv 0.7's `Archive` derive cannot handle self-referential recursive types like `Vec<CompiledStep>` without a cycle-breaking workaround that defeats the zero-copy benefit. Bincode handles recursive `serde::Deserialize` trivially; the ~20 ms full-pack deserialize at boot is well inside the 100 ms budget given ~4,000 cards × ~4 clauses per card.) Spec §7a.3 explicitly called out bincode as the fallback path.

**Spec reference:** `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` §§ 4 (evaluator architecture), 7a (distribution), 7a.2 (runtime updates). Phase 0 + 1a plans + their close-out messages establish the ground state.

**Phase 1a starting commit:** `32694ad3` (phase0_exit uses real adapter).

**Mid-execution revisions (applied verbally during run):**
- **Serialization:** bincode instead of rkyv (see Tech Stack above). Every `Archive`/`Serialize`/`Deserialize` rkyv derive in this plan becomes a regular `serde::{Serialize, Deserialize}` derive. `#[archive(check_bytes)]` attributes deleted. Calls like `rkyv::to_bytes` / `rkyv::from_bytes` become `bincode::serialize` / `bincode::deserialize`. No `archived_from_bytes` zero-copy view — bincode owns its deserialized output.
- **Task 9 module rename:** `digimon-engine/src/card_registry.rs` is taken by main's tensor-indexing registry (PR #341). Rename the Phase 1b T9 module to `digimon-engine/src/dsl_registry.rs`. Update `digimon_engine::card_registry::from_embedded()` → `digimon_engine::dsl_registry::from_embedded()` throughout (Tasks 9, 10, 12).

---

## File structure

**Created:**

```
digimon-dsl/                              # NEW leaf crate at workspace root
├── Cargo.toml
└── src/
    ├── lib.rs                            # Re-exports the public surface
    ├── spec.rs                           # Moved from digimon-engine/src/dsl/
    ├── clause.rs                         # Moved
    ├── step.rs                           # Moved
    ├── predicate.rs                      # Moved
    ├── formula.rs                        # Moved
    ├── alt_path.rs                       # Moved
    ├── identity.rs                       # Moved
    ├── common.rs                         # Moved
    ├── errors.rs                         # Moved
    ├── loader.rs                         # Moved (minus RealCardDataAdapter — see below)
    ├── validator.rs                      # Moved
    ├── raw_rust_registry.rs              # Moved
    ├── pretty.rs                         # Moved
    ├── schema.rs                         # Moved
    ├── compile.rs                        # NEW — CardSpec → CompiledCard lowering (Task 3)
    ├── compiled.rs                       # NEW — CompiledCard IR types (Task 2)
    ├── pack.rs                           # NEW — CardPack manifest + serialization (Task 6)
    └── registry.rs                       # NEW — CardRegistry holding CompiledCard values (Task 8)

digimon-engine/
├── build.rs                              # NEW (Task 9) — compiles cards/_examples to $OUT_DIR/cards.pack
└── src/
    ├── card_registry.rs                  # NEW (Task 10) — from_embedded() + from_pack_file() wrappers
    └── dsl_bridge.rs                     # NEW — RealCardDataAdapter moved here (depends on engine types)
```

**Modified:**

- `Cargo.toml` (workspace root) — add `digimon-dsl` to `[workspace] members`.
- `digimon-engine/Cargo.toml` — depend on `digimon-dsl` as both runtime and build dep; drop the per-module DSL deps (`serde_yml`, `schemars`, `thiserror`, `indexmap`) since they move into `digimon-dsl`.
- `digimon-engine/src/lib.rs` — replace `pub mod dsl;` with `pub use digimon_dsl as dsl;` re-export; add `pub mod card_registry;` and `pub mod dsl_bridge;` (both feature-gated).
- `tools/dsl-lint/Cargo.toml` — depend on `digimon-dsl` directly (shorter dep chain).
- `tools/dsl-schema-export/Cargo.toml` — same.
- `digimon-engine/tests/dsl/*.rs` — update imports to `digimon_dsl::...` paths where applicable.

**Deleted:**

- `digimon-engine/src/dsl/` — the entire directory tree (content moves to `digimon-dsl/src/`).

---

## Task 1: Extract `digimon-engine/src/dsl/` → new `digimon-dsl/` leaf crate

The largest task in Phase 1b, but mostly mechanical. Creates the workspace crate that `build.rs` can depend on. Must preserve existing behavior — all 76 Phase 1a tests continue to pass.

**Files:**
- Create: `digimon-dsl/Cargo.toml`
- Create: `digimon-dsl/src/lib.rs`
- Move: all files under `digimon-engine/src/dsl/` → `digimon-dsl/src/` (except `RealCardDataAdapter` — see Step 5)
- Create: `digimon-engine/src/dsl_bridge.rs`
- Modify: `Cargo.toml` (workspace root), `digimon-engine/Cargo.toml`, `digimon-engine/src/lib.rs`
- Modify: `tools/dsl-lint/Cargo.toml`, `tools/dsl-schema-export/Cargo.toml`
- Modify: `digimon-engine/tests/dsl/*.rs` (import path updates)

- [ ] **Step 1: Create `digimon-dsl/Cargo.toml`**

```toml
[package]
name = "digimon-dsl"
version = "0.1.0"
edition = "2021"
description = "Declarative YAML card-scripting DSL for the Digimon TCG engine"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yml = "0.0.12"
schemars = { version = "0.8", features = ["indexmap2"] }
thiserror = "1"
indexmap = { version = "2", features = ["serde"] }
```

No feature flags — this crate is always built (its consumers choose when to depend on it).

- [ ] **Step 2: Move every file under `digimon-engine/src/dsl/` to `digimon-dsl/src/`**

Run (from the worktree root):

```bash
mkdir -p digimon-dsl/src
git mv digimon-engine/src/dsl/mod.rs digimon-dsl/src/lib.rs
git mv digimon-engine/src/dsl/spec.rs digimon-dsl/src/
git mv digimon-engine/src/dsl/clause.rs digimon-dsl/src/
git mv digimon-engine/src/dsl/step.rs digimon-dsl/src/
git mv digimon-engine/src/dsl/predicate.rs digimon-dsl/src/
git mv digimon-engine/src/dsl/formula.rs digimon-dsl/src/
git mv digimon-engine/src/dsl/alt_path.rs digimon-dsl/src/
git mv digimon-engine/src/dsl/identity.rs digimon-dsl/src/
git mv digimon-engine/src/dsl/common.rs digimon-dsl/src/
git mv digimon-engine/src/dsl/errors.rs digimon-dsl/src/
git mv digimon-engine/src/dsl/loader.rs digimon-dsl/src/
git mv digimon-engine/src/dsl/validator.rs digimon-dsl/src/
git mv digimon-engine/src/dsl/raw_rust_registry.rs digimon-dsl/src/
git mv digimon-engine/src/dsl/pretty.rs digimon-dsl/src/
git mv digimon-engine/src/dsl/schema.rs digimon-dsl/src/
rmdir digimon-engine/src/dsl
```

The `mod.rs` becomes `lib.rs`. Update its header comment from "Card-scripting DSL (Phase 0 — parse + validate only)" to reflect that it's now the crate root. Keep all `pub mod` declarations and `pub use` re-exports.

- [ ] **Step 3: Update every `use crate::dsl::...` path inside the moved files**

Inside `digimon-dsl/src/`, any `use crate::dsl::foo::Bar;` becomes `use crate::foo::Bar;`. Any fully-qualified `crate::dsl::...` becomes just `crate::...`.

Affected files (search for `crate::dsl`):
- `spec.rs`
- `clause.rs`
- `step.rs`
- `predicate.rs`
- `formula.rs`
- `alt_path.rs`
- `identity.rs`
- `loader.rs`
- `validator.rs`
- `pretty.rs`
- `schema.rs`

- [ ] **Step 4: Remove `RealCardDataAdapter` from `loader.rs` (it depends on engine types)**

In `digimon-dsl/src/loader.rs`, delete:
- `pub struct RealCardDataAdapter` and its `impl` block
- The `from_path` method on it
- The `engine_card_kind_to_dsl` and `engine_color_to_dsl` helper fns
- The `use crate::card_data::CardData;` and `use crate::enums::{CardKind, CardColor};` engine imports at the top of `loader.rs`

**Keep** the `CardDataDb` trait, `CardDataRow`, `CardDataDbStub`, and `cross_check` function — those are DSL-pure.

- [ ] **Step 5: Create `digimon-engine/src/dsl_bridge.rs`**

This is where `RealCardDataAdapter` lives now — it needs both worlds (engine `CardData` type on input, DSL `CardDataDb` trait on output).

```rust
//! Bridge between engine-side `CardData` and DSL-side `CardDataDb`.
//! Lives in digimon-engine (not digimon-dsl) because it depends on the
//! engine's card_data + enums. digimon-dsl stays engine-agnostic so
//! build.rs can use it without a circular dependency.

use digimon_dsl::loader::{CardDataDb, CardDataRow};
use digimon_dsl::spec::{CardKind, ColorSpec};
use digimon_dsl::errors::DslError;
use std::path::Path as StdPath;

pub struct RealCardDataAdapter {
    cards: std::collections::HashMap<String, RealRow>,
}

struct RealRow {
    name: String,
    kind: CardKind,
    level: Option<u8>,
    dp: Option<i32>,
    cost: Option<i32>,
    colors: Vec<ColorSpec>,
}

impl RealCardDataAdapter {
    pub fn from_path(path: &StdPath) -> Result<Self, DslError> {
        let raw = std::fs::read_to_string(path).map_err(|e| DslError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        let parsed = crate::card_data::CardData::load_from_str(&raw).map_err(|e| DslError::Io {
            path: path.display().to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{e}")),
        })?;
        let mut cards = std::collections::HashMap::new();
        for (card_id, data) in parsed {
            cards.insert(card_id, RealRow {
                name: data.card_name,
                kind: engine_card_kind_to_dsl(data.card_kind),
                level: data.level,
                dp: data.dp,
                cost: Some(data.play_cost as i32),
                colors: data.colors.iter().map(|c| engine_color_to_dsl(*c)).collect(),
            });
        }
        Ok(Self { cards })
    }
}

fn engine_card_kind_to_dsl(k: crate::enums::CardKind) -> CardKind {
    use crate::enums::CardKind as E;
    match k {
        E::Digimon => CardKind::Digimon,
        E::Tamer => CardKind::Tamer,
        E::Option => CardKind::Option,
        E::DigiEgg => CardKind::DigiEgg,
        E::Token => CardKind::Token,
    }
}

fn engine_color_to_dsl(c: crate::enums::CardColor) -> ColorSpec {
    use crate::enums::CardColor as E;
    match c {
        E::Red => ColorSpec::Red,
        E::Blue => ColorSpec::Blue,
        E::Yellow => ColorSpec::Yellow,
        E::Green => ColorSpec::Green,
        E::Black => ColorSpec::Black,
        E::Purple => ColorSpec::Purple,
        E::White => ColorSpec::White,
    }
}

impl CardDataDb for RealCardDataAdapter {
    fn lookup(&self, card_id: &str) -> Option<CardDataRow<'_>> {
        self.cards.get(card_id).map(|r| CardDataRow {
            name: &r.name,
            kind: r.kind,
            level: r.level,
            dp: r.dp,
            cost: r.cost,
            colors: &r.colors,
        })
    }
}
```

Verify against the actual Task 6 impl (commit `55e00c33`) since field names may differ (e.g., `card_name` vs `card_name_eng`; `colors` vs `card_colors`). Task 6 established: `card_name`, `colors`, `level: Option<u8>`, `dp: Option<i32>`, `play_cost: u16`.

- [ ] **Step 6: Update workspace root `Cargo.toml`**

Add `digimon-dsl` to the `[workspace] members = [...]` list (alongside `digimon-engine`, `digimon-engine-py`, `src-tauri`, `tools/*`).

- [ ] **Step 7: Update `digimon-engine/Cargo.toml`**

Replace the DSL dep block with a single line:

```toml
# DSL lives in a separate leaf crate so build.rs can depend on it.
digimon-dsl = { path = "../digimon-dsl", optional = true }
```

Drop the `serde_yml`, `schemars`, `thiserror`, `indexmap` deps if they were only used by the DSL module (grep the engine's non-DSL source first). Keep them if other engine code uses them.

Update the feature definition:

```toml
[features]
default = ["dsl-yaml-loader"]
dsl-yaml-loader = ["dep:digimon-dsl"]
```

Keep the existing `[[test]]` entry for `dsl` with `required-features = ["dsl-yaml-loader"]`.

- [ ] **Step 8: Update `digimon-engine/src/lib.rs`**

Replace `#[cfg(feature = "dsl-yaml-loader")] pub mod dsl;` with:

```rust
#[cfg(feature = "dsl-yaml-loader")]
pub use digimon_dsl as dsl;

#[cfg(feature = "dsl-yaml-loader")]
pub mod dsl_bridge;
```

The `dsl` re-export lets existing `digimon_engine::dsl::...` call-sites continue to work. `dsl_bridge` is engine-crate-only.

- [ ] **Step 9: Update `tools/dsl-lint/Cargo.toml`**

Replace the `digimon-engine` dep with a `digimon-dsl` dep (shorter dependency chain; avoids pulling the whole engine in):

```toml
[dependencies]
digimon-dsl = { path = "../../digimon-dsl" }
serde_json = "1"
serde = { version = "1", features = ["derive"] }
```

And in `tools/dsl-lint/src/main.rs`, change every `use digimon_engine::dsl::...` to `use digimon_dsl::...`.

**Exception:** `dsl-lint --cross-check` uses `RealCardDataAdapter`. Since that adapter now lives in `digimon-engine::dsl_bridge`, the linter must still depend on `digimon-engine` (feature-flagged) to access it. Two options:

- **(a)** Revert `tools/dsl-lint` to `digimon-engine` dep with `dsl-yaml-loader` feature on. Slightly heavier but keeps --cross-check working.
- **(b)** Move `RealCardDataAdapter` to a separate tiny crate `digimon-dsl-engine-bridge` that `dsl-lint` depends on. Overkill.

Choose (a). Keep the original dep: `digimon-engine = { path = "../../digimon-engine", features = ["dsl-yaml-loader"] }`. The feature pulls `digimon-dsl` transitively. Update imports to use `digimon_engine::dsl::...` (the re-export) or `digimon_dsl::...` (direct) — both work after Step 8.

- [ ] **Step 10: Update `tools/dsl-schema-export/Cargo.toml`**

This one doesn't need engine types — switch to `digimon-dsl` directly:

```toml
[dependencies]
digimon-dsl = { path = "../../digimon-dsl" }
serde_json = "1"
```

Update `tools/dsl-schema-export/src/main.rs`: `use digimon_dsl::schema::export_json_schema;` (was `digimon_engine::dsl::schema::export_json_schema`).

- [ ] **Step 11: Update test imports**

Every test file under `digimon-engine/tests/dsl/` that imports `digimon_engine::dsl::...` works unchanged because of the re-export in Step 8. But now there's a better direct path: `digimon_dsl::...`. Prefer the direct import for clarity.

Run a find + replace:

```bash
grep -r "digimon_engine::dsl::" digimon-engine/tests/dsl/
```

Update to `digimon_dsl::...` where applicable. The only exception is `RealCardDataAdapter` — that stays `digimon_engine::dsl_bridge::RealCardDataAdapter`. Update the three test files that use it:
- `digimon-engine/tests/dsl/real_cards_json.rs`
- `digimon-engine/tests/dsl/phase0_exit.rs`
- Any others using `loader::RealCardDataAdapter`

Change `use digimon_engine::dsl::loader::RealCardDataAdapter;` → `use digimon_engine::dsl_bridge::RealCardDataAdapter;`.

- [ ] **Step 12: Build and test**

```bash
cargo build --package digimon-dsl
cargo build --package digimon-engine --features dsl-yaml-loader
cargo build -p dsl-lint
cargo build -p dsl-schema-export
cargo test --package digimon-engine --test dsl --features dsl-yaml-loader
```

Expected: all 76 tests pass. Zero warnings.

Also verify the Tauri build still succeeds:

```bash
cargo build --manifest-path src-tauri/Cargo.toml
```

The Tauri crate opted out of `dsl-yaml-loader` in Phase 0 Task 1 follow-up (`default-features = false`), so it shouldn't pull `digimon-dsl` at all.

- [ ] **Step 13: Commit**

```bash
git add -A
git commit -m "dsl(phase1b): extract digimon-dsl as leaf crate (build.rs prerequisite)"
```

---

## Task 2: Define `CompiledCard` IR types with rkyv derives

A rkyv-friendly mirror of `CardSpec` that eliminates serde-internal types (`serde_yml::Value`, `IndexMap<String, serde_yml::Value>`). The compiled IR can be zero-copy deserialized from a rkyv byte buffer.

**Files:**
- Modify: `digimon-dsl/Cargo.toml` (add rkyv dep)
- Create: `digimon-dsl/src/compiled.rs`
- Modify: `digimon-dsl/src/lib.rs` (expose the module)

- [ ] **Step 1: Add rkyv to `digimon-dsl/Cargo.toml`**

Append to `[dependencies]`:

```toml
rkyv = { version = "0.7", features = ["validation"] }
bytecheck = "0.6"  # required by rkyv's validation feature
```

- [ ] **Step 2: Create `digimon-dsl/src/compiled.rs`**

Mirror of the `spec.rs` structure, but with types rkyv can serialize. Start with the top level:

```rust
//! Compiled card IR — rkyv-friendly mirror of `CardSpec` (and its nested
//! types) used as the on-disk / in-memory format for distributed card packs.
//!
//! Phase 1b: this is pure data. Phase 1c adds the bridge from `CompiledCard`
//! to engine `Effect` closures.

use rkyv::{Archive, Serialize, Deserialize};

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledCard {
    pub card: String,
    pub name: String,
    pub kind: CompiledCardKind,
    pub level: Option<u8>,
    pub color: Vec<CompiledColor>,
    pub cost: Option<i32>,
    pub dp: Option<i32>,
    pub traits: Vec<String>,
    pub form: Option<String>,
    pub attribute: Option<String>,
    pub ace_overflow: Option<i32>,
    pub identity: Option<CompiledIdentity>,
    pub alt_paths: Vec<CompiledAltPath>,
    pub effects: Vec<CompiledClause>,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledCardKind {
    Digimon,
    Tamer,
    Option,
    DigiEgg,
    Token,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledColor {
    Red,
    Blue,
    Yellow,
    Green,
    Black,
    Purple,
    White,
}

// Further types — CompiledIdentity, CompiledAltPath, CompiledClause,
// CompiledStep, CompiledPredicate, CompiledFormula — defined in the
// subsequent steps. Stub them as empty structs for now so `CompiledCard`
// compiles:

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledIdentity;

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledAltPath;

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledClause;
```

- [ ] **Step 3: Register the module**

In `digimon-dsl/src/lib.rs`, add `pub mod compiled;` alongside the other module declarations.

- [ ] **Step 4: Smoke test — the crate still compiles**

```bash
cargo build -p digimon-dsl
cargo test -p digimon-engine --test dsl --features dsl-yaml-loader
```

Expected: all 76 tests still pass. The new module contains only stubs so no behavior changes.

- [ ] **Step 5: Commit**

```bash
git add digimon-dsl/
git commit -m "dsl(phase1b): CompiledCard top-level IR (stubs for nested types)"
```

---

## Task 3: Flesh out `CompiledClause`, `CompiledStep`, `CompiledPredicate`, etc.

Populate the stub types from Task 2 with full structural content. Keep rkyv-friendly (no serde_yml types, no IndexMap<String, Value>).

**Files:**
- Modify: `digimon-dsl/src/compiled.rs`

- [ ] **Step 1: Define `CompiledIdentity`**

```rust
#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledIdentity {
    pub name_aliases: Vec<CompiledNameAlias>,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledNameAlias {
    pub treat_as: String,
    pub zone: Vec<CompiledZone>,
    pub has_inherited_card_number: Option<String>,
    pub has_inherited_name: Option<String>,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledZone {
    Hand, Deck, Trash, BattleArea, Security, Breeding, Reveal, DigiEggDeck, Material,
}
```

- [ ] **Step 2: Define `CompiledAltPath`**

```rust
#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledAltPath {
    pub kind: CompiledAltPathKind,
    pub from: Option<Box<CompiledPredicate>>,
    pub materials: Vec<CompiledMaterial>,
    pub cost: Option<CompiledCost>,
    pub stacks_unsuspended: bool,
    pub ignore_requirements: bool,
    pub source_treated_as: Option<String>,
    pub extra_cost: Vec<CompiledStep>,
    pub on_burst_turn_end: Vec<CompiledStep>,
    pub marker: bool,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledAltPathKind {
    Digivolve, DnaDigivolve, DigiXros, BurstDigivolve, AppFusion, Assembly, ActivatedDigivolve,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledMaterial {
    pub filter: CompiledPredicate,
    pub repeat: Option<CompiledRepeat>,
    pub distinct_by: Option<CompiledDistinctBy>,
    pub zones: Vec<CompiledZone>,
    pub stack_under: bool,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub enum CompiledRepeat {
    Unbounded,
    Range { min: u8, max: u8 },
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledDistinctBy { CardNumber, Level, Name }

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub enum CompiledCost {
    Literal(i32),
    Formula(CompiledFormula),
}
```

- [ ] **Step 3: Define `CompiledPredicate` — the biggest type**

Mirror `PredicateSpec` but drop the `extra: IndexMap<String, serde_yml::Value>` field (it was a blind-spot absorber in Phase 0; the compile step either surfaces its contents into real fields or reports a validation error).

```rust
#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[archive(check_bytes)]
pub struct CompiledPredicate {
    pub kind: Option<CompiledCardKind>,
    pub level_eq: Option<u8>,
    pub level_lte: Option<u8>,
    pub level_gte: Option<u8>,
    pub color_is: Option<CompiledColor>,
    pub color_only: Option<Vec<CompiledColor>>,
    pub trait_has: Option<String>,
    pub form_is: Option<String>,
    pub attribute_is: Option<String>,
    pub name_is: Option<String>,
    pub name_contains: Option<String>,
    pub name_in: Option<Vec<String>>,
    pub card_number_is: Option<String>,
    pub dp_eq: Option<CompiledDpConstraint>,
    pub dp_lte: Option<CompiledDpConstraint>,
    pub dp_gte: Option<CompiledDpConstraint>,
    pub stack_size_lte: Option<u8>,
    pub stack_size_gte: Option<u8>,
    pub materials_count_lte: Option<u8>,
    pub materials_count_gte: Option<u8>,
    pub has_inherited: Option<Box<CompiledPredicate>>,
    pub is_suspended: Option<bool>,
    pub is_unsuspended: Option<bool>,
    pub has_keyword: Option<String>,
    pub zone: Vec<CompiledZone>,
    pub owner: Option<CompiledPlayerRef>,
    pub other: Option<bool>,
    pub of_permanent: Option<String>,
    pub source_is_tamer: Option<bool>,
    pub source_name_contains: Option<String>,
    pub source_permanent_trait_has: Option<String>,
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
    pub event_target_kind: Option<CompiledCardKind>,
    pub event_target_trait_has: Option<String>,
    pub event_card_trait_has: Option<String>,
    pub equals: Option<Vec<CompiledBindingCompare>>,
    pub not_equals: Option<Vec<CompiledBindingCompare>>,
    pub count_lte: Option<CompiledCountAggregate>,
    pub count_gte: Option<CompiledCountAggregate>,
    pub any_permanent: Option<Box<CompiledExistential>>,
    pub no_permanent: Option<Box<CompiledExistential>>,
    pub all_permanents: Option<Box<CompiledExistential>>,
    pub all_of: Vec<CompiledPredicate>,
    pub any_of: Vec<CompiledPredicate>,
    pub none_of: Vec<CompiledPredicate>,
    pub not: Option<Box<CompiledPredicate>>,
    pub has_alt_path: Option<String>,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub enum CompiledDpConstraint {
    Literal(i32),
    Formula(CompiledFormula),
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub enum CompiledBindingCompare {
    Binding(String),
    Literal(i64),
    // Phase 1b: only these two shapes. If more surface during compile,
    // extend the enum.
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledCountAggregate {
    pub filter: Box<CompiledPredicate>,
    pub n: u32,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledExistential {
    pub of: CompiledPlayerRef,
    pub predicate: CompiledPredicate,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledPlayerRef { You, Opponent, Any, Active }
```

- [ ] **Step 4: Define `CompiledFormula`**

```rust
#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub enum CompiledFormula {
    Literal(i32),
    BasePerDelta { base: i32, per: CompiledPerSelector, delta: i32 },
    FloorDiv(Vec<CompiledFormula>),
    Max(Vec<CompiledFormula>),
    Min(Vec<CompiledFormula>),
    Aggregate(CompiledAggregateSelector),
    RawRust(String),
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledPerSelector {
    MaterialCount, StackSize, AllyCount, DigivolutionColorCount, CardCountInZone,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledAggregateSelector {
    LowestDp, HighestDp, LowestLevel, HighestLevel,
}
```

- [ ] **Step 5: Define `CompiledClause` — triggered vs declarative variant**

This is the one place where compile-time resolution of the typed body pays off. The declarative clause's `IndexMap` body from `CardSpec` becomes a tagged enum here.

```rust
#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub enum CompiledClause {
    Triggered(CompiledTriggeredClause),
    Declarative(CompiledDeclarativeClause),
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledTriggeredClause {
    pub when: Vec<CompiledTiming>,  // flatten Single/Multi to always Vec
    pub scope: CompiledScope,
    pub active_when: Option<CompiledPredicate>,
    pub condition: Option<CompiledPredicate>,
    pub optional: bool,
    pub once_per_turn: bool,
    pub max_per_turn: Option<u8>,
    pub process: Vec<CompiledStep>,
    pub summary: Option<String>,
    pub summary_key: Option<String>,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub enum CompiledDeclarativeClause {
    Aura {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        target: CompiledPredicate,
        dp_modifier: Option<i32>,
        grant_keyword: Option<CompiledGrantKeywordValue>,
        modifier: Option<String>,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    CostReduction {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        reduction_timing: Option<String>,
        when_playing_this: bool,
        when_any_ally_played: Option<CompiledPredicate>,
        condition: Option<CompiledPredicate>,
        once_per_turn: bool,
        amount: Option<i32>,
        amount_fn: Option<CompiledFormula>,
        pay_cost: Vec<CompiledStep>,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    Replacement {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        trigger: String,
        process: Vec<CompiledStep>,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    Partition {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        sources: Vec<CompiledPredicate>,
        exclude_cause: Vec<String>,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    AceOverflow {
        value: i32,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    GrantKeyword {
        keyword: String,
        value: Option<i32>,
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    Delay {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        trigger: CompiledTiming,
        process: Vec<CompiledStep>,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    FloodGate {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        modifier: String,
        target: CompiledPredicate,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    AltPathRegistration {
        scope: CompiledScope,
        active_when: Option<CompiledPredicate>,
        trigger: CompiledTiming,
        applies_to: Option<CompiledPredicate>,
        // Phase 1b: `registers:` inner structure kept as a nested
        // CompiledAltPath to avoid IndexMap. Compile step lowers the
        // authored mini alt-path.
        registers: CompiledAltPath,
        summary: Option<String>,
        summary_key: Option<String>,
    },
    RawRust {
        fn_name: String,
        triggers: Vec<CompiledTiming>,
        scope: CompiledScope,
        summary: Option<String>,
        summary_key: Option<String>,
    },
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[archive(check_bytes)]
pub enum CompiledScope {
    #[default]
    FaceUp,
    Inherited,
    Both,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledTiming {
    OnPlay, WhenDigivolving, WhenAttacking, EndOfAttack, EndOfBattle, OnAttack,
    OnDeletion, OnAnyDeletion, OnEnterFieldAnyone, OnAllyPlayed, OnLeaveField,
    OnSuspend, OnUnsuspend, OnHatch, OnDigivolve, OnDnaDigivolve, OnDigixros,
    OnOpponentSecurityRemoved, OnDigivolutionCardTrashed, OnSecurityCheck,
    OnLoseSecurity, OnSecurity, OnOptionPlaced, StartOfYourTurn,
    StartOfOpponentsTurn, StartOfYourMainPhase, EndOfYourTurn, EndOfOpponentsTurn,
    OnAttackTargetChange, MainFromHand, MainOnField, MainFromTrash, Counter,
    BeforePayCost, Delayed,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledGrantKeywordValue {
    pub keyword: String,
    pub value: Option<i32>,
}
```

- [ ] **Step 6: Define `CompiledStep`**

Large enum mirroring `StepSpec`. Skip the custom Deserialize gymnastics — rkyv derives are straightforward. See the Phase 0 plan Task 6 for the full variant list; each one maps 1:1 to a `CompiledXxx` variant.

```rust
#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub enum CompiledStep {
    GainMemory(i32),
    LoseMemory(i32),
    SetMemory(i32),
    Draw { of: CompiledPlayerRef, count: u8 },
    TrashFromTop { of: CompiledPlayerRef, count: u8 },
    AddToHandFromDeck { of: CompiledPlayerRef, card: CompiledBindingRef },
    AddToHandFromTrash { of: CompiledPlayerRef, card: CompiledBindingRef },
    AddToHandFromReveal { of: CompiledPlayerRef, card: CompiledBindingRef },
    TrashFromHandByIndex { of: CompiledPlayerRef, hand_index: CompiledBindingRef },
    TrashFromReveal { of: CompiledPlayerRef, card: CompiledBindingRef },
    ReturnToDeckFromReveal { of: CompiledPlayerRef, card: CompiledBindingRef, position: CompiledStackPosition },
    ShuffleDeck { of: CompiledPlayerRef },
    RevealTopDeck { of: CompiledPlayerRef, count: u8, zone: Option<CompiledZone>, bind_as: Option<String> },
    PlaceRemainderOnDeck { of: CompiledPlayerRef, position: CompiledStackPosition },
    DeletePermanent { target: CompiledBindingRef },
    ReturnToHand { target: CompiledBindingRef },
    ReturnToDeck { target: CompiledBindingRef, position: CompiledStackPosition, include_sources: bool },
    Suspend { target: CompiledBindingRef },
    Unsuspend { target: CompiledBindingRef },
    DeDigivolve { target: CompiledBindingRef, amount: Option<u8>, stop_at_level: Option<u8> },
    PlaceOnSecurity { of: CompiledPlayerRef, source: CompiledBindingRef, position: CompiledStackPosition, face_up: bool },
    PlayToken { controller: CompiledPlayerRef, token_name: String },
    PlaceAsBottomSource { source: CompiledBindingRef, target: CompiledBindingRef },
    TrashTopSource { target: CompiledBindingRef },
    Hatch { of: CompiledPlayerRef },
    PlayFromHand { of: CompiledPlayerRef, hand_index: CompiledBindingRef, cost_delta: Option<CompiledCostDelta> },
    PlayFromHandFree { of: CompiledPlayerRef, hand_index: CompiledBindingRef },
    PlayFromTrash { of: CompiledPlayerRef, trash_index: CompiledBindingRef, cost_delta: Option<CompiledCostDelta> },
    PlayFromTrashFree { of: CompiledPlayerRef, trash_index: CompiledBindingRef },
    PlayFromSecurity,
    PlayFromMaterials { target: CompiledBindingRef, source_index: CompiledBindingRef, cost_delta: Option<CompiledCostDelta> },
    EffectInitiatedDigivolve { target: CompiledBindingRef, from_hand: CompiledBindingRef, cost: i32, ignore_requirements: bool },
    EffectInitiatedDnaDigivolve { target_a: CompiledBindingRef, target_b: CompiledBindingRef, from_hand: CompiledBindingRef, cost: i32, ignore_requirements: bool },
    TrashTopSecurity { of: CompiledPlayerRef },
    MarkSecurityFaceUp { of: CompiledPlayerRef, card: CompiledBindingRef },
    AddDpModifier { target: CompiledBindingRef, value: i32, expiry: String },
    AddModifier { target: CompiledModifierTarget, modifier: String, value: i32, expiry: String },
    GrantKeyword { target: CompiledBindingRef, keyword: String, expiry: String, value: Option<i32> },
    SelectOwnPermanent { filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool },
    SelectOpponentPermanent { filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool },
    SelectHand { of: CompiledPlayerRef, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool },
    SelectTrash { of: CompiledPlayerRef, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool },
    SelectMaterial { of_permanent: CompiledBindingRef, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool },
    SelectReveal { of: CompiledPlayerRef, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool },
    SelectSecurity { of: CompiledPlayerRef, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool },
    SelectUnionZone { of: CompiledPlayerRef, zones: Vec<CompiledZone>, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool },
    SelectOrderedPermutation { items: CompiledBindingRef, bind_as: Option<String>, prompt: String, prompt_key: Option<String> },
    SelectCountCappedMulti { of: CompiledPlayerRef, zone: CompiledZone, max: u8, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional_zero: bool, distinct_by: Option<CompiledDistinctBy> },
    SelectEffectChoice { labels: Vec<String>, bind_as: Option<String>, prompt: String, prompt_key: Option<String> },
    AsSelectingPlayer { of: CompiledPlayerRef, body: Vec<CompiledStep> },
    If { condition: CompiledPredicate, then: Vec<CompiledStep>, r#else: Vec<CompiledStep> },
    ForEach { over: CompiledPredicate, bind_as: String, body: Vec<CompiledStep> },
    PerSelected { selection: String, bind_as: String, body: Vec<CompiledStep> },
    ScheduleDelayed { when: CompiledTiming, body: Vec<CompiledStep> },
    Optional(Vec<CompiledStep>),
    RawRust { fn_name: String, consumes: Vec<String>, binds: Vec<String> },
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub enum CompiledBindingRef {
    Named(String),
    SelfRef,  // "self" literal — avoids having self be a field name
    Carrier,
    Source,
    EventTarget,
    EventCard,
    Permanent(String),
    Binding(String),
    OfPermanent(String),
    // Phase 1b covers the common cases; add variants as compile uncovers
    // shapes not representable here.
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub enum CompiledModifierTarget {
    Binding(CompiledBindingRef),
    Filter(CompiledPredicate),
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub enum CompiledCostDelta {
    Free,
    Printed,
    Literal(i32),
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledStackPosition { Top, Bottom, Random }
```

- [ ] **Step 7: Build-check that everything is rkyv-derivable**

```bash
cargo build -p digimon-dsl
```

Expected: zero errors. rkyv will complain about any type that doesn't derive `Archive` / `Serialize` / `Deserialize` — fix as you go. The `Default` derive on `CompiledPredicate` may need an explicit impl if the many-field struct doesn't auto-default; manually implement if needed.

- [ ] **Step 8: Commit**

```bash
git add digimon-dsl/src/compiled.rs
git commit -m "dsl(phase1b): full CompiledCard IR for rkyv serialization"
```

---

## Task 4: Implement `CardSpec` → `CompiledCard` lowering

The compile pass that walks the parsed YAML `CardSpec` and produces a `CompiledCard`. Every `Option<T>` in spec becomes a corresponding compiled field; `IndexMap`-backed bodies resolve via `typed_body()`; custom-deserialized `StepSpec` lowers to `CompiledStep`.

**Files:**
- Create: `digimon-dsl/src/compile.rs`
- Modify: `digimon-dsl/src/lib.rs`
- Create: `digimon-dsl/src/compile_tests.rs` (inline tests)

- [ ] **Step 1: Create the compile module with a top-level entry point**

```rust
//! CardSpec → CompiledCard lowering. Pure function over authored data;
//! no engine types touched.

use crate::compiled::*;
use crate::spec::CardSpec;
use crate::errors::ValidationError;

/// Compile a parsed CardSpec to the rkyv-friendly CompiledCard IR.
/// Errors accumulate into a Vec<ValidationError>, analogous to the
/// semantic validator — the compile step is strictly more demanding
/// than validate() since it must resolve every shape to a concrete type.
pub fn compile(spec: &CardSpec) -> Result<CompiledCard, Vec<ValidationError>> {
    let mut errors = Vec::new();

    let identity = spec.identity.as_ref().map(|id| compile_identity(id, &spec.card, &mut errors));
    let alt_paths = spec.alt_paths.iter()
        .enumerate()
        .map(|(i, ap)| compile_alt_path(ap, &format!("alt_paths[{i}]"), &spec.card, &mut errors))
        .collect();
    let effects = spec.effects.iter()
        .enumerate()
        .map(|(i, c)| compile_clause(c, &format!("effects[{i}]"), &spec.card, &mut errors))
        .collect();

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(CompiledCard {
        card: spec.card.clone(),
        name: spec.name.clone(),
        kind: compile_card_kind(spec.kind),
        level: spec.level,
        color: spec.color.iter().map(|c| compile_color(*c)).collect(),
        cost: spec.cost,
        dp: spec.dp,
        traits: spec.traits.clone(),
        form: spec.form.clone(),
        attribute: spec.attribute.clone(),
        ace_overflow: spec.ace_overflow,
        identity,
        alt_paths,
        effects,
    })
}

// ... compile_clause, compile_step, compile_predicate, etc.
// Each is ~20-80 lines of exhaustive match. Full implementations
// in Steps 2-7.
```

- [ ] **Step 2: Implement the enum mappings**

Direct mapping helpers:

```rust
fn compile_card_kind(k: crate::spec::CardKind) -> CompiledCardKind {
    use crate::spec::CardKind as S;
    match k {
        S::Digimon => CompiledCardKind::Digimon,
        S::Tamer => CompiledCardKind::Tamer,
        S::Option => CompiledCardKind::Option,
        S::DigiEgg => CompiledCardKind::DigiEgg,
        S::Token => CompiledCardKind::Token,
    }
}

fn compile_color(c: crate::spec::ColorSpec) -> CompiledColor { /* 7-variant match */ }
fn compile_player_ref(p: crate::common::PlayerRef) -> CompiledPlayerRef { /* 4-variant match */ }
fn compile_zone(z: crate::predicate::Zone) -> CompiledZone { /* 9-variant match */ }
fn compile_scope(s: crate::clause::ClauseScope) -> CompiledScope { /* 3-variant match */ }
fn compile_timing(t: crate::clause::Timing) -> CompiledTiming { /* 35-variant match */ }
fn compile_stack_position(p: crate::step::StackPosition) -> CompiledStackPosition { /* 3-variant */ }
fn compile_distinct_by(d: crate::alt_path::DistinctBy) -> CompiledDistinctBy { /* 3-variant */ }
```

- [ ] **Step 3: Implement `compile_predicate`**

Transfer every Option field with clone/recurse. The compound forms (`all_of`, `any_of`, etc.) recurse into `compile_predicate`.

```rust
fn compile_predicate(
    p: &crate::predicate::PredicateSpec,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledPredicate {
    // The spec's `extra` field is a blind spot from Phase 0. If it has
    // any entries here, surface them as validation errors:
    if !p.extra.is_empty() {
        errors.push(ValidationError {
            card_id: card_id.into(),
            path: prefix.into(),
            message: format!("unknown predicate fields: {:?}", p.extra.keys().collect::<Vec<_>>()),
        });
    }

    CompiledPredicate {
        kind: p.kind.map(compile_card_kind),
        level_eq: p.level_eq,
        level_lte: p.level_lte,
        level_gte: p.level_gte,
        color_is: p.color_is.map(compile_color),
        color_only: p.color_only.as_ref().map(|v| v.iter().map(|c| compile_color(*c)).collect()),
        trait_has: p.trait_has.clone(),
        // ... every other field
        all_of: p.all_of.iter().enumerate()
            .map(|(i, sub)| compile_predicate(sub, &format!("{prefix}.all_of[{i}]"), card_id, errors))
            .collect(),
        // ... any_of, none_of, not, any_permanent, etc.
    }
}
```

Fill in every field. This is mechanical.

- [ ] **Step 4: Implement `compile_step`**

The `StepSpec` enum → `CompiledStep` enum mapping. Match on every variant. `BindingRef::Named(s)` maps to `CompiledBindingRef::Named(s)`; the `Structured` variant maps to one of `Permanent`/`Binding`/`OfPermanent`/etc. depending on which field is set.

```rust
fn compile_binding_ref(b: &crate::step::BindingRef) -> CompiledBindingRef {
    use crate::step::BindingRef as B;
    match b {
        B::Named(n) => match n.as_str() {
            "self" => CompiledBindingRef::SelfRef,
            "carrier" => CompiledBindingRef::Carrier,
            "source" => CompiledBindingRef::Source,
            "event_target" => CompiledBindingRef::EventTarget,
            "event_card" => CompiledBindingRef::EventCard,
            _ => CompiledBindingRef::Named(n.clone()),
        },
        B::Structured(s) => {
            if let Some(p) = &s.permanent { CompiledBindingRef::Permanent(p.clone()) }
            else if let Some(b) = &s.binding { CompiledBindingRef::Binding(b.clone()) }
            else if let Some(o) = &s.of_permanent { CompiledBindingRef::OfPermanent(o.clone()) }
            // ... other fields — fall back to Named with empty string + error
            else { CompiledBindingRef::Named(String::new()) }
        }
    }
}
```

The big `compile_step` match has ~50 arms. Each is trivial after Task 3's type-to-type mapping.

- [ ] **Step 5: Implement `compile_clause`**

```rust
fn compile_clause(
    c: &crate::clause::ClauseSpec,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) -> CompiledClause {
    use crate::clause::ClauseSpec as C;
    match c {
        C::Triggered(t) => CompiledClause::Triggered(compile_triggered(t, prefix, card_id, errors)),
        C::Declarative(d) => {
            match d.typed_body() {
                Ok(body) => CompiledClause::Declarative(compile_declarative(d, body, prefix, card_id, errors)),
                Err(e) => {
                    errors.push(ValidationError {
                        card_id: card_id.into(),
                        path: prefix.into(),
                        message: format!("declarative body schema: {e}"),
                    });
                    // Placeholder that will never be used (errors non-empty)
                    CompiledClause::Declarative(CompiledDeclarativeClause::AceOverflow { value: 0, summary: None, summary_key: None })
                }
            }
        }
    }
}
```

`compile_triggered` builds `CompiledTriggeredClause` from `TriggeredClause`. `compile_declarative` matches on `TypedDeclarativeBody` and produces the corresponding `CompiledDeclarativeClause` variant.

- [ ] **Step 6: Handle `AltPathRegistrationBody.registers` carefully**

The `registers: IndexMap<String, serde_yml::Value>` is a mini-alt-path shape. Phase 1b parses it into an `AltPathSpec` via `serde_yml::from_value` and compiles that:

```rust
// Inside compile_declarative for AltPathRegistration:
let registers_value = serde_yml::Value::Mapping(body.registers.iter()
    .map(|(k, v)| (serde_yml::Value::String(k.clone()), v.clone()))
    .collect());
let registers_spec: crate::alt_path::AltPathSpec = match serde_yml::from_value(registers_value) {
    Ok(v) => v,
    Err(e) => { errors.push(ValidationError { /* ... */ }); return placeholder; }
};
let compiled_registers = compile_alt_path(&registers_spec, &format!("{prefix}.registers"), card_id, errors);
```

- [ ] **Step 7: Write a round-trip test for all 15 worked examples**

In `digimon-dsl/src/compile.rs` inline:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::load_dir_ok;
    use std::path::PathBuf;

    #[test]
    fn every_example_compiles() {
        // Note: test runs from digimon-dsl/ crate root, so the examples
        // path is relative to digimon-engine's examples directory.
        let examples = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("digimon-engine/cards/_examples");
        let (specs, errs) = load_dir_ok(&examples);
        assert!(errs.is_empty(), "parse errors: {errs:#?}");
        assert_eq!(specs.len(), 15);

        let mut failures = Vec::new();
        for spec in &specs {
            if let Err(e) = compile(spec) {
                failures.push(format!("{}: {e:#?}", spec.card));
            }
        }
        assert!(failures.is_empty(), "compile failures:\n{}", failures.join("\n"));
    }
}
```

- [ ] **Step 8: Build + test**

```bash
cargo test -p digimon-dsl compile
```

Expected: `every_example_compiles` passes. If a fixture fails to compile because a step/clause/predicate shape isn't yet handled, fix the compile code and document the new shape.

Full suite:

```bash
cargo test -p digimon-engine --test dsl --features dsl-yaml-loader
```

Expected: 76 prior tests still pass. A new `compile_tests` entry may add to the count.

- [ ] **Step 9: Commit**

```bash
git add digimon-dsl/src/compile.rs digimon-dsl/src/lib.rs
git commit -m "dsl(phase1b): CardSpec -> CompiledCard lowering for all 15 examples"
```

---

## Task 5: rkyv round-trip: `CompiledCard` → bytes → `CompiledCard`

Verify the rkyv derives actually work end-to-end before building the registry infrastructure.

**Files:**
- Modify: `digimon-dsl/src/compiled.rs` (inline test module)

- [ ] **Step 1: Add rkyv round-trip test**

At the bottom of `digimon-dsl/src/compiled.rs`:

```rust
#[cfg(test)]
mod rkyv_tests {
    use super::*;
    use rkyv::{to_bytes, from_bytes};

    fn sample_card() -> CompiledCard {
        CompiledCard {
            card: "ST2-13".into(),
            name: "Hammer Spark".into(),
            kind: CompiledCardKind::Option,
            level: None,
            color: vec![CompiledColor::Blue],
            cost: Some(0),
            dp: None,
            traits: vec![],
            form: None,
            attribute: None,
            ace_overflow: None,
            identity: None,
            alt_paths: vec![],
            effects: vec![
                CompiledClause::Triggered(CompiledTriggeredClause {
                    when: vec![CompiledTiming::MainFromHand],
                    scope: CompiledScope::FaceUp,
                    active_when: None,
                    condition: None,
                    optional: false,
                    once_per_turn: false,
                    max_per_turn: None,
                    process: vec![CompiledStep::GainMemory(1)],
                    summary: None,
                    summary_key: None,
                })
            ],
        }
    }

    #[test]
    fn compiled_card_round_trips_through_rkyv() {
        let original = sample_card();
        let bytes = to_bytes::<_, 256>(&original).expect("rkyv serialize");
        let reparsed: CompiledCard = from_bytes(&bytes).expect("rkyv deserialize");
        assert_eq!(original, reparsed);
    }

    #[test]
    fn compiled_card_zero_copy_archived_view() {
        let original = sample_card();
        let bytes = to_bytes::<_, 256>(&original).expect("rkyv serialize");
        // Access the archived (not deserialized) view.
        let archived = rkyv::check_archived_root::<CompiledCard>(&bytes)
            .expect("archived root validates");
        // Archived str derefs to str.
        assert_eq!(archived.card.as_str(), "ST2-13");
        assert_eq!(archived.name.as_str(), "Hammer Spark");
    }
}
```

- [ ] **Step 2: Build + test**

```bash
cargo test -p digimon-dsl rkyv
```

Expected: 2 passed. If rkyv complains about missing `bytecheck` on a type, add `#[archive(check_bytes)]` to it. If a variant has non-Sized internals that rkyv rejects, adjust the type.

- [ ] **Step 3: Commit**

```bash
git add digimon-dsl/src/compiled.rs
git commit -m "dsl(phase1b): rkyv round-trip test for CompiledCard"
```

---

## Task 6: Define `CardPack` manifest + serialization

The outermost container for a distributed pack: a manifest with version info + a Vec of `CompiledCard`.

**Files:**
- Create: `digimon-dsl/src/pack.rs`
- Modify: `digimon-dsl/src/lib.rs`

- [ ] **Step 1: Create `digimon-dsl/src/pack.rs`**

```rust
//! Pack container: manifest + compiled cards.
//!
//! Pack format (rkyv-serialized):
//!   pack_version: pack-level semver (authoring-tool concept; bumps when pack content changes)
//!   min_engine_version: minimum digimon-engine semver that can load this pack
//!   max_engine_version: optional upper bound for breaking schema changes
//!   required_raw_rust_fns: fn names the pack references; desktop rejects if missing
//!   pack_id: "BT17", "core", etc. for cache-dir segregation
//!   cards: every compiled card in the pack

use rkyv::{Archive, Serialize, Deserialize};
use crate::compiled::CompiledCard;

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CardPack {
    pub manifest: PackManifest,
    pub cards: Vec<CompiledCard>,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct PackManifest {
    pub pack_id: String,
    pub pack_version: String,
    pub min_engine_version: String,
    pub max_engine_version: Option<String>,
    pub required_raw_rust_fns: Vec<String>,
}

impl CardPack {
    pub fn new(pack_id: impl Into<String>, cards: Vec<CompiledCard>) -> Self {
        Self {
            manifest: PackManifest {
                pack_id: pack_id.into(),
                pack_version: env!("CARGO_PKG_VERSION").to_string(),
                min_engine_version: "0.1.0".into(),
                max_engine_version: None,
                required_raw_rust_fns: Vec::new(),
            },
            cards,
        }
    }

    /// Serialize the pack to a rkyv byte buffer. Returns an owned Vec.
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        rkyv::to_bytes::<_, 4096>(self)
            .map(|b| b.to_vec())
            .map_err(|e| format!("rkyv serialize failed: {e}"))
    }

    /// Deserialize from a rkyv byte buffer (allocates). For zero-copy
    /// access see `archived_from_bytes`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        rkyv::from_bytes(bytes).map_err(|e| format!("rkyv deserialize failed: {e}"))
    }

    /// Zero-copy archived view — prefer this at runtime.
    pub fn archived_from_bytes(bytes: &[u8]) -> Result<&ArchivedCardPack, String> {
        rkyv::check_archived_root::<CardPack>(bytes)
            .map_err(|e| format!("rkyv archive validation failed: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled::*;

    #[test]
    fn empty_pack_round_trips() {
        let pack = CardPack::new("test", vec![]);
        let bytes = pack.to_bytes().unwrap();
        let reparsed = CardPack::from_bytes(&bytes).unwrap();
        assert_eq!(reparsed.manifest.pack_id, "test");
        assert_eq!(reparsed.cards.len(), 0);
    }

    #[test]
    fn pack_with_one_card_round_trips() {
        let card = CompiledCard {
            card: "X-1".into(),
            name: "Test".into(),
            kind: CompiledCardKind::Option,
            level: None,
            color: vec![CompiledColor::Red],
            cost: Some(0),
            dp: None,
            traits: vec![],
            form: None,
            attribute: None,
            ace_overflow: None,
            identity: None,
            alt_paths: vec![],
            effects: vec![],
        };
        let pack = CardPack::new("test", vec![card.clone()]);
        let bytes = pack.to_bytes().unwrap();
        let reparsed = CardPack::from_bytes(&bytes).unwrap();
        assert_eq!(reparsed.cards, vec![card]);
    }
}
```

- [ ] **Step 2: Register the module**

In `digimon-dsl/src/lib.rs`:

```rust
pub mod pack;
pub use pack::CardPack;
```

- [ ] **Step 3: Build + test**

```bash
cargo test -p digimon-dsl pack
```

Expected: 2 passed.

- [ ] **Step 4: Commit**

```bash
git add digimon-dsl/src/pack.rs digimon-dsl/src/lib.rs
git commit -m "dsl(phase1b): CardPack manifest + rkyv round-trip"
```

---

## Task 7: `CardRegistry` struct + `from_specs(Vec<CardSpec>)` + `from_pack_bytes(&[u8])`

Holds a map of `card_id → CompiledCard` after loading. Three constructors: from an in-memory list of parsed specs (dev/test path), from a byte buffer (embedded / cached pack), and from a pack file (cache directory).

**Files:**
- Create: `digimon-dsl/src/registry.rs`
- Modify: `digimon-dsl/src/lib.rs`

- [ ] **Step 1: Create `digimon-dsl/src/registry.rs`**

```rust
//! `CardRegistry` — the runtime lookup surface. Phase 1b holds
//! `CompiledCard` values; Phase 1c wraps each with lowered `Effect`
//! closures for engine consumption.

use std::collections::HashMap;
use std::path::Path;

use crate::compile::compile;
use crate::compiled::CompiledCard;
use crate::errors::ValidationError;
use crate::pack::{CardPack, PackManifest};
use crate::spec::CardSpec;

pub struct CardRegistry {
    pub manifest: PackManifest,
    cards: HashMap<String, CompiledCard>,
}

impl CardRegistry {
    /// Dev / test path — compile in-memory without serializing.
    pub fn from_specs(pack_id: impl Into<String>, specs: &[CardSpec]) -> Result<Self, Vec<ValidationError>> {
        let mut cards = HashMap::new();
        let mut errors = Vec::new();
        for spec in specs {
            match compile(spec) {
                Ok(c) => { cards.insert(c.card.clone(), c); }
                Err(mut errs) => errors.append(&mut errs),
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let pack = CardPack::new(pack_id, cards.values().cloned().collect());
        Ok(Self { manifest: pack.manifest, cards })
    }

    /// Desktop / runtime path — zero-copy deserialize from embedded bytes.
    pub fn from_pack_bytes(bytes: &[u8]) -> Result<Self, String> {
        let pack = CardPack::from_bytes(bytes)?;
        Ok(Self::from_pack(pack))
    }

    /// Cache-directory path — read a .pack file.
    pub fn from_pack_file(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("read pack file: {e}"))?;
        Self::from_pack_bytes(&bytes)
    }

    fn from_pack(pack: CardPack) -> Self {
        let cards = pack.cards.into_iter().map(|c| (c.card.clone(), c)).collect();
        Self { manifest: pack.manifest, cards }
    }

    pub fn lookup(&self, card_id: &str) -> Option<&CompiledCard> {
        self.cards.get(card_id)
    }

    pub fn len(&self) -> usize { self.cards.len() }
    pub fn is_empty(&self) -> bool { self.cards.is_empty() }
    pub fn iter(&self) -> impl Iterator<Item = (&String, &CompiledCard)> { self.cards.iter() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::load_dir_ok;
    use std::path::PathBuf;

    #[test]
    fn registry_from_specs_compiles_all_examples() {
        let examples = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("digimon-engine/cards/_examples");
        let (specs, errs) = load_dir_ok(&examples);
        assert!(errs.is_empty());
        let registry = CardRegistry::from_specs("phase1b-test", &specs).expect("compile");
        assert_eq!(registry.len(), 15);
        assert!(registry.lookup("ST2-13").is_some());
    }

    #[test]
    fn registry_round_trips_through_bytes() {
        let examples = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("digimon-engine/cards/_examples");
        let (specs, _) = load_dir_ok(&examples);
        let registry = CardRegistry::from_specs("phase1b-test", &specs).unwrap();

        // Rebuild the pack and serialize.
        let pack = CardPack {
            manifest: registry.manifest.clone(),
            cards: registry.iter().map(|(_, c)| c.clone()).collect(),
        };
        let bytes = pack.to_bytes().unwrap();

        // Deserialize fresh registry and compare.
        let reparsed = CardRegistry::from_pack_bytes(&bytes).unwrap();
        assert_eq!(reparsed.len(), registry.len());
        for (card_id, card) in registry.iter() {
            assert_eq!(reparsed.lookup(card_id), Some(card));
        }
    }
}
```

- [ ] **Step 2: Register**

In `digimon-dsl/src/lib.rs`:

```rust
pub mod registry;
pub use registry::CardRegistry;
```

- [ ] **Step 3: Build + test**

```bash
cargo test -p digimon-dsl registry
```

Expected: 2 passed.

- [ ] **Step 4: Commit**

```bash
git add digimon-dsl/src/registry.rs digimon-dsl/src/lib.rs
git commit -m "dsl(phase1b): CardRegistry with from_specs / from_pack_bytes / from_pack_file"
```

---

## Task 8: `digimon-engine/build.rs` — compile `cards/_examples/` to `$OUT_DIR/cards.pack`

Uses `digimon-dsl` as a `[build-dependencies]` entry to avoid circular deps.

**Files:**
- Create: `digimon-engine/build.rs`
- Modify: `digimon-engine/Cargo.toml`

- [ ] **Step 1: Add build-dependency on `digimon-dsl`**

In `digimon-engine/Cargo.toml`, add:

```toml
[build-dependencies]
digimon-dsl = { path = "../digimon-dsl" }
```

- [ ] **Step 2: Create `digimon-engine/build.rs`**

```rust
//! Compile digimon-engine/cards/_examples/*.yaml into $OUT_DIR/cards.pack
//! at build time. The resulting blob is `include_bytes!`-ed by
//! `src/card_registry.rs` to give the desktop binary zero-copy access
//! to compiled cards.
//!
//! Phase 1b: operates only on the _examples directory. Phase 1c will
//! point this at digimon-engine/cards/ (the real pack root).

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=cards/_examples");

    let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/_examples");
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR set by cargo"));
    let pack_path = out_dir.join("cards.pack");

    let (specs, parse_errors) = digimon_dsl::loader::load_dir_ok(&examples_dir);
    if !parse_errors.is_empty() {
        for e in &parse_errors {
            println!("cargo:warning=dsl parse error: {e}");
        }
        panic!("dsl parse errors in cards/_examples/ — see warnings above");
    }

    let registry = match digimon_dsl::CardRegistry::from_specs("core", &specs) {
        Ok(r) => r,
        Err(errs) => {
            for e in &errs {
                println!("cargo:warning=dsl compile error: {e}");
            }
            panic!("dsl compile errors in cards/_examples/ — see warnings above");
        }
    };

    let pack = digimon_dsl::CardPack {
        manifest: registry.manifest.clone(),
        cards: registry.iter().map(|(_, c)| c.clone()).collect(),
    };

    let bytes = pack.to_bytes().expect("rkyv serialize cards.pack");
    std::fs::write(&pack_path, &bytes).expect("write cards.pack");

    println!("cargo:rustc-env=CARDS_PACK_PATH={}", pack_path.display());
}
```

- [ ] **Step 3: Verify cargo picks it up**

```bash
cargo build --package digimon-engine --features dsl-yaml-loader 2>&1 | head -30
```

Expected: the build prints "Compiling digimon-engine ..." and finishes clean. `$OUT_DIR/cards.pack` exists after the build (find it with `find target -name cards.pack`).

- [ ] **Step 4: Commit**

```bash
git add digimon-engine/Cargo.toml digimon-engine/build.rs
git commit -m "dsl(phase1b): digimon-engine/build.rs compiles cards.pack at build time"
```

---

## Task 9: `digimon-engine/src/card_registry.rs` — `from_embedded()` + `from_pack_file()`

Wraps `digimon_dsl::CardRegistry` with engine-crate-side constructors: `from_embedded()` uses `include_bytes!("cards.pack")` via the OUT_DIR mechanism.

**Files:**
- Create: `digimon-engine/src/card_registry.rs`
- Modify: `digimon-engine/src/lib.rs`

- [ ] **Step 1: Create the module**

```rust
//! Engine-side `CardRegistry` adapters — the desktop / runtime
//! entry points that wrap digimon_dsl::CardRegistry.

use digimon_dsl::CardRegistry;
use std::path::Path;

// Embedded pack blob, produced by build.rs.
static CARDS_PACK: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cards.pack"));

/// Load the card registry from the bytes embedded at build time.
/// Zero-copy at load (rkyv deserializes lazily); ~5ms for ~15 cards.
pub fn from_embedded() -> Result<CardRegistry, String> {
    CardRegistry::from_pack_bytes(CARDS_PACK)
}

/// Load the card registry from a cache-directory pack file — used by
/// desktop binaries to pick up runtime-downloaded updates.
pub fn from_pack_file(path: &Path) -> Result<CardRegistry, String> {
    CardRegistry::from_pack_file(path)
}
```

- [ ] **Step 2: Register in lib.rs**

In `digimon-engine/src/lib.rs`:

```rust
#[cfg(feature = "dsl-yaml-loader")]
pub mod card_registry;
```

- [ ] **Step 3: Add an integration test**

Create `digimon-engine/tests/dsl/embedded_registry.rs`:

```rust
#[test]
fn embedded_registry_loads_all_15_examples() {
    let registry = digimon_engine::card_registry::from_embedded()
        .expect("embedded cards.pack must load");
    assert_eq!(registry.len(), 15, "expected 15 examples in embedded pack");
    assert!(registry.lookup("ST2-13").is_some());
    assert!(registry.lookup("BT17-015").is_some());
    assert!(registry.lookup("EX11-012").is_some());
}

#[test]
fn embedded_registry_manifest_declares_pack_id() {
    let registry = digimon_engine::card_registry::from_embedded().unwrap();
    assert_eq!(registry.manifest.pack_id, "core");
}
```

Add `mod embedded_registry;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 4: Build + test**

```bash
cargo test --package digimon-engine --test dsl --features dsl-yaml-loader embedded_registry
```

Expected: 2 passed.

Full suite:

```bash
cargo test --package digimon-engine --test dsl --features dsl-yaml-loader
```

Expected: 78 passed (76 prior + 2 new), 0 failed.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/card_registry.rs \
        digimon-engine/src/lib.rs \
        digimon-engine/tests/dsl/embedded_registry.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl(phase1b): from_embedded() + from_pack_file() on engine crate"
```

---

## Task 10: `from_pack_file()` integration test — runtime update simulation

Verifies the cache-dir flow: pack written to a temp file, loaded via `from_pack_file`, matches the embedded registry content.

**Files:**
- Create: `digimon-engine/tests/dsl/pack_file_loader.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the test**

```rust
#[test]
fn round_trip_registry_through_temp_pack_file() {
    use digimon_dsl::{CardPack, loader};
    use std::path::PathBuf;

    // Load the fixtures fresh (not via embedded).
    let examples = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/_examples");
    let (specs, errs) = loader::load_dir_ok(&examples);
    assert!(errs.is_empty());

    let registry = digimon_dsl::CardRegistry::from_specs("test-pack", &specs).unwrap();
    let pack = CardPack {
        manifest: registry.manifest.clone(),
        cards: registry.iter().map(|(_, c)| c.clone()).collect(),
    };

    // Write to a temp file.
    let mut temp = std::env::temp_dir();
    temp.push("digimon-test-pack.pack");
    let bytes = pack.to_bytes().unwrap();
    std::fs::write(&temp, &bytes).unwrap();

    // Read back via from_pack_file.
    let loaded = digimon_engine::card_registry::from_pack_file(&temp).unwrap();
    assert_eq!(loaded.len(), registry.len());
    assert_eq!(loaded.manifest.pack_id, "test-pack");

    // Cleanup.
    let _ = std::fs::remove_file(&temp);
}
```

Add `mod pack_file_loader;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Run + commit**

```bash
cargo test --package digimon-engine --test dsl --features dsl-yaml-loader pack_file_loader
```

Expected: 1 passed. Full suite: 79 passed.

```bash
git add digimon-engine/tests/dsl/pack_file_loader.rs \
        digimon-engine/tests/dsl/main.rs
git commit -m "dsl(phase1b): from_pack_file temp-file round-trip test"
```

---

## Task 11: Verify Tauri desktop binary pulls the embedded pack correctly

The Tauri crate doesn't currently use `card_registry` — `dsl-yaml-loader` is off for Tauri per Phase 0 Task 1 follow-up. But the embedded pack is compiled into `digimon-engine` regardless of features (since `build.rs` always runs). This task checks that the Tauri build still compiles.

**Files:**
- Modify: `src-tauri/Cargo.toml` (maybe; depends on whether Tauri needs cards)

- [ ] **Step 1: Confirm Tauri builds without the feature**

```bash
cargo build --manifest-path src-tauri/Cargo.toml
```

Expected: builds cleanly. If `build.rs` runs for digimon-engine even with the feature off, that's expected — the pack is small and the compile cost is ~100ms. Phase 1b doesn't change Tauri's feature flag.

- [ ] **Step 2: Verify the pack is NOT in the Tauri binary when feature is off**

The `include_bytes!` in `card_registry.rs` is inside `#[cfg(feature = "dsl-yaml-loader")]`, so turning the feature off removes the whole module. The compiled pack file still exists in OUT_DIR but is never referenced by Tauri's build of `digimon-engine`.

Check: `cargo tree --manifest-path src-tauri/Cargo.toml -p digimon-engine` should NOT show `digimon-dsl` or `rkyv` or `serde_yml` in its dependencies.

- [ ] **Step 3: Commit any Cargo.toml / build.rs adjustments if needed**

If no changes were needed, skip commit. If the Tauri build broke, fix the feature-gating and commit.

---

## Task 12: Phase 1b exit integration test

Single end-to-end assertion that the whole pipeline (YAML → parse → compile → serialize → embed → deserialize → lookup) works.

**Files:**
- Create: `digimon-engine/tests/dsl/phase1b_exit.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`
- Create: `digimon-engine/src/card_registry.rs` README section

- [ ] **Step 1: Write the exit test**

```rust
//! Phase 1b exit criteria — the full YAML→pack→embedded pipeline works
//! end-to-end for all 15 worked examples.

#[test]
fn phase_1b_exit_criteria() {
    let registry = digimon_engine::card_registry::from_embedded()
        .expect("embedded registry must load");

    // All 15 fixtures round-tripped through the pack.
    assert_eq!(registry.len(), 15);

    // Spot-check a handful of cards end-to-end.
    let st2_13 = registry.lookup("ST2-13").expect("ST2-13 present");
    assert_eq!(st2_13.name, "Hammer Spark");
    assert_eq!(st2_13.kind, digimon_dsl::compiled::CompiledCardKind::Option);
    assert_eq!(st2_13.effects.len(), 2);

    let war_greymon = registry.lookup("BT17-015").expect("BT17-015 present");
    assert_eq!(war_greymon.name, "WarGreymon");
    assert_eq!(war_greymon.level, Some(6));
    assert!(war_greymon.alt_paths.len() >= 1);

    let nokia = registry.lookup("BT22-084").expect("BT22-084 present");
    assert_eq!(nokia.name, "Nokia Shiramine");
    assert_eq!(nokia.kind, digimon_dsl::compiled::CompiledCardKind::Tamer);

    // Manifest is the one we embedded.
    assert_eq!(registry.manifest.pack_id, "core");
}
```

- [ ] **Step 2: Document Phase 1b completion**

Create `digimon-dsl/README.md`:

```markdown
# digimon-dsl

Leaf crate holding the card-scripting DSL.

## Surface

- `loader` — parse YAML into `CardSpec`
- `validator` — semantic validation
- `compile` — CardSpec → CompiledCard IR lowering
- `pack` — rkyv-serialized pack with manifest + compiled cards
- `registry` — CardRegistry holds the compiled cards keyed by card_id
- `pretty` — canonical YAML pretty-printer
- `schema` — JSON Schema export

## Consumers

- `digimon-engine` — depends on digimon-dsl as runtime + build dep
- `tools/dsl-lint` — CLI linter
- `tools/dsl-schema-export` — JSON Schema exporter

## Phase status

- Phase 0 — schema + parse + validate + round-trip ✓
- Phase 1a — cleanup + real cards.json adapter + dsl-lint --cross-check ✓
- Phase 1b — AOT pipeline: CardSpec → CompiledCard → CardPack → embedded blob ✓
- Phase 1c — engine integration: lower CompiledCard → Effect closures (next)
```

- [ ] **Step 3: Run + commit**

```bash
cargo test --package digimon-engine --test dsl --features dsl-yaml-loader phase1b_exit
```

Expected: 1 passed. Full suite: 80 passed.

```bash
git add digimon-engine/tests/dsl/phase1b_exit.rs \
        digimon-engine/tests/dsl/main.rs \
        digimon-dsl/README.md
git commit -m "dsl(phase1b): exit integration test + README"
```

---

## Phase 1b done

All 12 tasks shipped. Phase 1b delivers:

- `digimon-dsl` leaf crate with full DSL surface + rkyv serialization
- `CompiledCard` IR that captures every `CardSpec` shape in a zero-copy-friendly format
- `CardPack` manifest-plus-cards container
- `CardRegistry` with three constructors: in-memory specs, embedded bytes, cache file
- `build.rs` that compiles `cards/_examples/*.yaml` into `$OUT_DIR/cards.pack` at build time
- `from_embedded()` returns the compiled registry at runtime with ~5ms load cost
- End-to-end test proves 15 fixtures survive the full YAML→pack→embedded pipeline

### What this unblocks

**Phase 1c** can now write lowering code that:
1. Takes a `CompiledCard` (not a `CardSpec` — the compile step has already resolved every shape).
2. Produces `Vec<Effect>` against engine types.
3. Registers each with the engine's card-effect trait object registry.

Phase 1c won't need to touch YAML, serde, or rkyv again.

### Explicitly deferred

- **Runtime update channel** (cache-dir discovery, manifest.json download, SHA verification) — still Phase 3+. Phase 1b ships `from_pack_file(path)` as the mechanism; the download/cache orchestration is a Tauri-side concern.
- **Real-card-pool scaling** — `cards/_examples/` is only 15 cards. The authored pack grows in Phase 2 as `/batch-implement-cards-rust-dsl` starts writing real YAMLs under `digimon-engine/cards/bt17/`, `digimon-engine/cards/bt22/`, etc. Build.rs needs to point at the full `cards/` root then.
- **File splits** (`step.rs`, `clause.rs`) — still deferred; this time there are parallel `compiled.rs` and `compile.rs` files that also have scope for splitting.
