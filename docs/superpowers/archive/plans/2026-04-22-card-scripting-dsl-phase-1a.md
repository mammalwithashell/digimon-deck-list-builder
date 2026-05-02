# Card-scripting DSL — Phase 1a Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the Phase 0 debt called out in the final-branch review and ship a real `cards.json` → `CardDataDb` adapter so `dsl-lint` can cross-check every authored YAML against the structured-data source of truth. Sets up the ground for Phase 1b's AOT lowering without touching engine code.

**Architecture:** Still no engine integration. Work stays inside `digimon-engine/src/dsl/` and `tools/dsl-lint/`. The new `cards.json` adapter is a thin wrapper around the existing `crate::card_data::CardData` type — it implements the `CardDataDb` trait defined in Task 11 of Phase 0 without pulling DSL types into the engine core. The cleanup items (scope shadow, validator gaps, enum consolidation) shore up schema correctness before lowering code writes against it in Phase 1b.

**Tech Stack:** Rust 2021 / existing deps — no new dependencies.

**Spec reference:** `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` §§ 3, 5. Phase 0 plan: `docs/superpowers/plans/2026-04-21-card-scripting-dsl-phase-0.md`. Final-review carry-over items documented in the Phase 0 close-out message.

**Phase 0 starting commit:** `53196220` (dsl-lint CLI shipped).

---

## File structure

**Modified:**

```
digimon-engine/src/dsl/
├── clause.rs              # Task 1: ClauseScope skip_serializing + CostReductionBody.scope rename
│                          # Task 4: CostReductionBody uses shared PlayerRef instead of String scope
├── predicate.rs           # Task 3: re-export PlayerRef from shared location; drop Owner
├── step.rs                # Task 3: re-export PlayerRef; drop Player; Task 5: AddModifierArgs.target typed
├── spec.rs                # Task 3: introduce PlayerRef (new common type)
├── validator.rs           # Task 2: extend to PredicateSpec.has_keyword + AuraBody.grant_keyword
│                          # Task 6: pass cards_db through ValidationContext
└── loader.rs              # Task 6: RealCardDataAdapter wrapping CardData

tools/dsl-lint/
└── src/main.rs            # Task 7: --cross-check <path> flag wiring the real adapter

digimon-engine/tests/dsl/
├── parse_clauses.rs       # Task 1 regression tests
├── parse_declarative.rs   # Task 4 scope-rename tests
├── validator.rs           # Task 2 new coverage
├── cross_check.rs         # Task 6 new tests against real cards.json
└── phase0_exit.rs         # Task 8: swap CardDataDbStub for the real adapter
```

**Created:**
- `digimon-engine/src/dsl/common.rs` — shared types (`PlayerRef`) used by both `step.rs` and `predicate.rs`.
- `digimon-engine/tests/dsl/real_cards_json.rs` — cross-check tests using `digimon_gym/engine/data/cards.json`.

---

## Task 1: Fix `ClauseScope::FaceUp` default serialization noise

Per Phase 0 final review (Issue 2). Currently `TriggeredClause.scope` and `DeclarativeClause.scope` both have `#[serde(default)]` but no `skip_serializing_if`, so any pretty-print emits `scope: face_up` on every clause. For LLM-authoring diff noise this becomes significant.

**Files:**
- Modify: `digimon-engine/src/dsl/clause.rs`
- Modify: `digimon-engine/tests/dsl/parse_clauses.rs`

- [ ] **Step 1: Add `is_face_up` helper and wire `skip_serializing_if` on both clause structs**

In `digimon-engine/src/dsl/clause.rs`, add a small inherent impl alongside the existing `ClauseScope` enum:

```rust
impl ClauseScope {
    pub(crate) fn is_face_up(&self) -> bool {
        matches!(self, ClauseScope::FaceUp)
    }
}
```

Change the `scope` attribute on both `TriggeredClause` and `DeclarativeClause` from:

```rust
    #[serde(default)]
    pub scope: ClauseScope,
```

to:

```rust
    #[serde(default, skip_serializing_if = "ClauseScope::is_face_up")]
    pub scope: ClauseScope,
```

- [ ] **Step 2: Add regression test confirming pretty-printed output omits `scope: face_up`**

Append to `digimon-engine/tests/dsl/parse_clauses.rs`:

```rust
use digimon_engine::dsl::pretty::format_spec;

#[test]
fn face_up_scope_is_omitted_from_output() {
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
    let spec: digimon_engine::dsl::spec::CardSpec = serde_yml::from_str(yaml).unwrap();
    let formatted = format_spec(&spec);
    assert!(
        !formatted.contains("scope:"),
        "face_up is the default scope and must not serialize; got:\n{formatted}"
    );
}

#[test]
fn inherited_scope_is_preserved_in_output() {
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
    process:
      - trash_top_security: { of: opponent }
"#;
    let spec: digimon_engine::dsl::spec::CardSpec = serde_yml::from_str(yaml).unwrap();
    let formatted = format_spec(&spec);
    assert!(
        formatted.contains("scope: inherited"),
        "non-default scope must serialize; got:\n{formatted}"
    );
}
```

- [ ] **Step 3: Run tests and full suite**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader`
Expected: 68 passed (66 prior + 2 new), 0 failed, no warnings.

Verify fixtures still round-trip (should be automatic since existing round-trip tests compare parsed `CardSpec` values, not YAML strings).

- [ ] **Step 4: Commit**

```bash
git add digimon-engine/src/dsl/clause.rs digimon-engine/tests/dsl/parse_clauses.rs
git commit -m "dsl(phase1a): skip_serializing_if for default ClauseScope"
```

---

## Task 2: Extend validator to cover `has_keyword` and `AuraBody.grant_keyword`

Per Phase 0 final review (Issue 3). `is_known_keyword` is consulted for `StepSpec::GrantKeyword` and `DeclarativeKind::GrantKeyword` but bypassed at two other keyword-string sites. Fixes the uniformity of the validator's namespace enforcement.

**Files:**
- Modify: `digimon-engine/src/dsl/validator.rs`
- Modify: `digimon-engine/tests/dsl/validator.rs`

- [ ] **Step 1: Write failing tests**

Append to `digimon-engine/tests/dsl/validator.rs`:

```rust
#[test]
fn validate_unknown_has_keyword_in_predicate_fails() {
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
    condition:
      has_keyword: NotARealKeyword
    process:
      - gain_memory: 1
"#);
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ctx(&reg)).unwrap_err();
    assert!(errs.iter().any(|e| e.message.contains("NotARealKeyword")),
        "expected keyword typo to be reported, got: {errs:?}");
}

#[test]
fn validate_unknown_aura_grant_keyword_fails() {
    let spec = parse(r#"
card: X-1
name: Test
kind: tamer
color: [red]
cost: 4
effects:
  - kind: aura
    active_when: { your_turn: true }
    target:
      of: you
      zone: [battle_area]
    grant_keyword: { keyword: AlsoNotRealKwrd, value: 1 }
"#);
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ctx(&reg)).unwrap_err();
    assert!(errs.iter().any(|e| e.message.contains("AlsoNotRealKwrd")),
        "expected aura grant_keyword typo to be reported, got: {errs:?}");
}
```

- [ ] **Step 2: Add predicate-tree walker and keyword checks to the validator**

In `digimon-engine/src/dsl/validator.rs`:

Add a helper that walks a `PredicateSpec` subtree and reports unknown keyword strings:

```rust
fn validate_predicate(
    pred: &crate::dsl::predicate::PredicateSpec,
    prefix: &str,
    card_id: &str,
    errors: &mut Vec<ValidationError>,
) {
    if let Some(kw) = &pred.has_keyword {
        if !is_known_keyword(kw) {
            errors.push(ValidationError {
                card_id: card_id.into(),
                path: format!("{prefix}.has_keyword"),
                message: format!("unknown keyword: {kw}"),
            });
        }
    }
    // Recurse into compound forms.
    for (i, sub) in pred.all_of.iter().enumerate() {
        validate_predicate(sub, &format!("{prefix}.all_of[{i}]"), card_id, errors);
    }
    for (i, sub) in pred.any_of.iter().enumerate() {
        validate_predicate(sub, &format!("{prefix}.any_of[{i}]"), card_id, errors);
    }
    for (i, sub) in pred.none_of.iter().enumerate() {
        validate_predicate(sub, &format!("{prefix}.none_of[{i}]"), card_id, errors);
    }
    if let Some(sub) = &pred.not {
        validate_predicate(sub, &format!("{prefix}.not"), card_id, errors);
    }
    if let Some(ex) = &pred.any_permanent {
        validate_predicate(&ex.predicate, &format!("{prefix}.any_permanent"), card_id, errors);
    }
    if let Some(ex) = &pred.no_permanent {
        validate_predicate(&ex.predicate, &format!("{prefix}.no_permanent"), card_id, errors);
    }
    if let Some(ex) = &pred.all_permanents {
        validate_predicate(&ex.predicate, &format!("{prefix}.all_permanents"), card_id, errors);
    }
    if let Some(agg) = &pred.count_lte {
        validate_predicate(&agg.filter, &format!("{prefix}.count_lte.filter"), card_id, errors);
    }
    if let Some(agg) = &pred.count_gte {
        validate_predicate(&agg.filter, &format!("{prefix}.count_gte.filter"), card_id, errors);
    }
}
```

Wire the walker from `validate_triggered`:

```rust
fn validate_triggered(
    t: &TriggeredClause,
    prefix: &str,
    card_id: &str,
    ctx: &ValidationContext<'_>,
    errors: &mut Vec<ValidationError>,
) {
    if let Some(cond) = &t.condition {
        validate_predicate(cond, &format!("{prefix}.condition"), card_id, errors);
    }
    if let Some(aw) = &t.active_when {
        validate_predicate(aw, &format!("{prefix}.active_when"), card_id, errors);
    }
    for (i, step) in t.process.iter().enumerate() {
        let sp = format!("{prefix}.process[{i}]");
        validate_step(step, &sp, card_id, ctx, errors);
    }
}
```

Add the `DeclarativeKind::Aura` arm in `validate`:

```rust
                    DeclarativeKind::Aura => {
                        if let Ok(crate::dsl::clause::TypedDeclarativeBody::Aura(body)) = d.typed_body() {
                            if let Some(gk) = &body.grant_keyword {
                                if !is_known_keyword(&gk.keyword) {
                                    errors.push(ValidationError {
                                        card_id: spec.card.clone(),
                                        path: format!("{prefix}.grant_keyword.keyword"),
                                        message: format!("unknown keyword: {}", gk.keyword),
                                    });
                                }
                            }
                            validate_predicate(&body.target, &format!("{prefix}.target"), &spec.card, &mut errors);
                        }
                    }
```

Also walk `FloodGateBody.target` for has_keyword leaves, by adding a line in the existing `FloodGate` arm:

```rust
                            validate_predicate(&body.target, &format!("{prefix}.target"), &spec.card, &mut errors);
```

- [ ] **Step 3: Run tests**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader`
Expected: 70 passed (68 after Task 1 + 2 new), 0 failed, no warnings.

- [ ] **Step 4: Commit**

```bash
git add digimon-engine/src/dsl/validator.rs digimon-engine/tests/dsl/validator.rs
git commit -m "dsl(phase1a): validator walks predicate tree + AuraBody.grant_keyword"
```

---

## Task 3: Consolidate `Player` and `Owner` into shared `PlayerRef`

Per Phase 0 final review (Suggestion 4). `step::Player { You, Opponent, Any, Active }` and `predicate::Owner { You, Opponent, Any, Active }` are structurally identical enums. Consolidate to a single `PlayerRef` type in a new `common.rs` before Phase 1b lowering code duplicates match arms.

**Files:**
- Create: `digimon-engine/src/dsl/common.rs`
- Modify: `digimon-engine/src/dsl/mod.rs`
- Modify: `digimon-engine/src/dsl/step.rs`
- Modify: `digimon-engine/src/dsl/predicate.rs`

- [ ] **Step 1: Create `common.rs` with `PlayerRef` enum**

```rust
//! Shared types used across multiple DSL submodules.

use serde::{Deserialize, Serialize};

/// Player reference used by both predicate and step modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlayerRef {
    You,
    Opponent,
    Any,
    Active,
}
```

- [ ] **Step 2: Register the module in `mod.rs`**

Add `pub mod common;` alongside the existing module declarations. Also add the re-export:

```rust
pub use common::PlayerRef;
```

- [ ] **Step 3: Replace `Player` in `step.rs` with an import of `PlayerRef`**

Remove the entire `pub enum Player` block. Replace every `Player` usage in argument struct fields (search for `: Player` in `step.rs` — about a dozen sites) with `PlayerRef`. Add `use crate::dsl::common::PlayerRef;` at the top.

- [ ] **Step 4: Replace `Owner` in `predicate.rs` with an import of `PlayerRef`**

Remove the `pub enum Owner` block. Replace every `Owner` usage (same search pattern). Add `use crate::dsl::common::PlayerRef;`.

Note: the `Owner` enum is re-exported in a few test files — those imports need to be updated too. Search the entire repo for `Owner` and replace with `PlayerRef` in any DSL-adjacent context. Also update `ExistentialPredicate.of: Owner` → `of: PlayerRef`.

- [ ] **Step 5: Update test imports that use `Owner` or `Player`**

Files likely needing import updates:
- `digimon-engine/tests/dsl/parse_predicates.rs` — imports `Owner`
- `digimon-engine/tests/dsl/cross_check.rs` — may use `CardDataDbStub` which doesn't touch Owner; verify
- Anywhere else `use digimon_engine::dsl::{predicate::Owner, step::Player}` appears

Replace with `use digimon_engine::dsl::PlayerRef;` (the re-export from `common.rs`) and update enum variant paths.

- [ ] **Step 6: Build and test**

Run: `cargo build --package digimon-engine --features dsl-yaml-loader`
Expected: compiles clean. Any lingering `Owner` or `Player` reference fails compile — fix as found.

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader`
Expected: 70 passed, 0 failed, no warnings.

- [ ] **Step 7: Commit**

```bash
git add digimon-engine/src/dsl/common.rs digimon-engine/src/dsl/mod.rs \
        digimon-engine/src/dsl/step.rs digimon-engine/src/dsl/predicate.rs \
        digimon-engine/tests/dsl/
git commit -m "dsl(phase1a): consolidate Player/Owner into shared PlayerRef"
```

---

## Task 4: Resolve `CostReductionBody.scope` shadow

Per Phase 0 final review (Issue 1). `DeclarativeClause.scope: ClauseScope` is consumed by serde before the `body: IndexMap` flatten, so `CostReductionBody.scope: String` always deserializes to `""`. The validator references it but always sees empty. Two possible fixes:

- **(a)** Drop `CostReductionBody.scope` entirely; rely on the outer `DeclarativeClause.scope`.
- **(b)** Rename `CostReductionBody.scope` to `CostReductionBody.reduction_timing` so the names no longer collide.

Spec text uses `scope: before_pay_cost` inside `cost_reduction` clauses (e.g., BT17-015 sketch in §10.5), but `before_pay_cost` is NOT a `ClauseScope` enum variant — it's a cost-timing discriminator. Option (b) is correct: the two fields encode different concepts that accidentally share a name.

**Files:**
- Modify: `digimon-engine/src/dsl/clause.rs`
- Modify: `digimon-engine/cards/_examples/BT13-007.yaml`
- Modify: `digimon-engine/cards/_examples/BT17-015.yaml`
- Modify: `digimon-engine/tests/dsl/parse_declarative.rs`

- [ ] **Step 1: Write failing test**

Append to `digimon-engine/tests/dsl/parse_declarative.rs`:

```rust
#[test]
fn cost_reduction_reduction_timing_is_populated_independently_of_clause_scope() {
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
    scope: inherited
    reduction_timing: before_pay_cost
    when_playing_this: true
    amount: 3
"#;
    let spec = parse(yaml);
    let d = spec.effects[0].as_declarative().unwrap();
    assert_eq!(d.scope, digimon_engine::dsl::clause::ClauseScope::Inherited);
    match typed_body(&spec, 0) {
        TypedDeclarativeBody::CostReduction(c) => {
            assert_eq!(c.reduction_timing.as_deref(), Some("before_pay_cost"));
        }
        _ => panic!("expected CostReduction"),
    }
}
```

- [ ] **Step 2: Rename the field**

In `digimon-engine/src/dsl/clause.rs`, change `CostReductionBody`:

```rust
pub struct CostReductionBody {
    /// Cost-timing discriminator (e.g., `before_pay_cost`). NOT the
    /// clause's zone scope — that lives on `DeclarativeClause.scope`.
    /// Optional because most cards don't need it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduction_timing: Option<String>,
    // ... rest of fields unchanged (when_playing_this, etc.)
```

Remove the old `pub scope: String` field and its `#[serde(default)]`.

- [ ] **Step 3: Update example fixtures that currently use `scope: before_pay_cost`**

In the two fixtures that had this pattern simplified-away (`BT13-007.yaml` and `BT17-015.yaml` — see Task 15 simplification notes in Phase 0), update the `cost_reduction` clauses to use the new field name. For BT17-015:

```yaml
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    when_playing_this: true
    condition:
      any_permanent:
        of: you
        zone: [battle_area]
        kind: tamer
        name_contains: "Tai Kamiya"
    amount: 3
    summary: "-3 cost with Tai Kamiya"
```

(The spec's example YAML in §10.5 had `scope: before_pay_cost` — now spelled as `reduction_timing: before_pay_cost`.) Apply similarly to BT13-007.

- [ ] **Step 4: Run tests**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader`
Expected: 71 passed (70 prior + 1 new), 0 failed, no warnings. All round-trip / phase0_exit tests continue to pass.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl/clause.rs \
        digimon-engine/cards/_examples/BT13-007.yaml \
        digimon-engine/cards/_examples/BT17-015.yaml \
        digimon-engine/tests/dsl/parse_declarative.rs
git commit -m "dsl(phase1a): rename CostReductionBody.scope to reduction_timing"
```

---

## Task 5: Type `AddModifierArgs.target` as `ModifierTarget` union

Per Phase 0 final review (Suggestion 5). `AddModifierArgs.target: serde_yml::Value` is a loose escape hatch; Phase 1b lowering will need to branch on whether the target is a binding reference or a predicate filter. Formalize as a typed union.

**Files:**
- Modify: `digimon-engine/src/dsl/step.rs`
- Modify: `digimon-engine/tests/dsl/parse_steps.rs`

- [ ] **Step 1: Write failing test**

Append to `digimon-engine/tests/dsl/parse_steps.rs`:

```rust
use digimon_engine::dsl::step::{ModifierTarget};

#[test]
fn add_modifier_target_as_binding_ref() {
    let step = parse_single_step(
        r#"add_modifier: { target: my_target, modifier: CannotAttack, value: 1, expiry: end_of_your_turn }"#,
    );
    match step {
        StepSpec::AddModifier(args) => {
            match args.target {
                ModifierTarget::Binding(BindingRef::Named(n)) => assert_eq!(n, "my_target"),
                other => panic!("expected binding, got {other:?}"),
            }
        }
        _ => panic!("expected AddModifier"),
    }
}

#[test]
fn add_modifier_target_as_predicate_filter() {
    let step = parse_single_step(
        r#"add_modifier: { target: { of: opponent, zone: [battle_area], kind: digimon }, modifier: CannotUnsuspend, value: 1, expiry: end_of_opponents_turn }"#,
    );
    match step {
        StepSpec::AddModifier(args) => {
            match args.target {
                ModifierTarget::Filter(p) => {
                    assert_eq!(p.zone, vec![digimon_engine::dsl::predicate::Zone::BattleArea]);
                }
                other => panic!("expected filter, got {other:?}"),
            }
        }
        _ => panic!("expected AddModifier"),
    }
}
```

- [ ] **Step 2: Define `ModifierTarget` and update `AddModifierArgs`**

In `digimon-engine/src/dsl/step.rs`:

```rust
use crate::dsl::predicate::PredicateSpec;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum ModifierTarget {
    Binding(BindingRef),
    Filter(PredicateSpec),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddModifierArgs {
    pub target: ModifierTarget,
    pub modifier: String,
    pub value: i32,
    pub expiry: String,
}
```

Note: since `BindingRef` is already an untagged enum (`Named(String) | Structured(StructuredBindingRef)`), the nesting `ModifierTarget::Binding(BindingRef::Named(...))` works because serde tries `Binding` (scalar string or structured map with binding-specific keys) before `Filter`. If filter-shape ambiguity arises (e.g. a predicate that only has `kind:` could be confused with `StructuredBindingRef.kind` — but StructuredBindingRef has no `kind` field, so this is fine), the variant order correctly disambiguates.

- [ ] **Step 3: Update the BT13-060 fixture that uses predicate-filter target form**

Fixture at `digimon-engine/cards/_examples/BT13-060.yaml` may use `add_modifier: { target: {...} }` — verify it parses correctly after the schema change.

- [ ] **Step 4: Run tests**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader`
Expected: 73 passed (71 prior + 2 new), 0 failed, no warnings.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl/step.rs digimon-engine/tests/dsl/parse_steps.rs
git commit -m "dsl(phase1a): AddModifierArgs.target as typed ModifierTarget union"
```

---

## Task 6: Real `cards.json` → `CardDataDb` adapter

The `CardDataDbStub` in Phase 0 was hand-crafted for 15 cards. Production use (and `dsl-lint --cross-check`) needs a real adapter that reads `digimon_gym/engine/data/cards.json`. The existing `digimon-engine/src/card_data.rs` already parses that file into `CardData` — this task wraps it to implement `CardDataDb`.

**Files:**
- Modify: `digimon-engine/src/dsl/loader.rs`
- Create: `digimon-engine/tests/dsl/real_cards_json.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write failing test**

Create `digimon-engine/tests/dsl/real_cards_json.rs`:

```rust
use digimon_engine::dsl::loader::{cross_check, RealCardDataAdapter};
use digimon_engine::dsl::spec::CardSpec;
use std::path::PathBuf;

fn cards_json_path() -> PathBuf {
    // Repo-relative to the digimon-engine manifest directory.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("digimon_gym/engine/data/cards.json")
}

#[test]
fn real_adapter_loads_cards_json() {
    let adapter = RealCardDataAdapter::from_path(&cards_json_path())
        .expect("cards.json must load");
    // Should contain at least one known card from the Phase 0 fixtures.
    assert!(adapter.lookup("ST2-13").is_some());
    assert!(adapter.lookup("BT17-015").is_some());
}

#[test]
fn real_adapter_cross_checks_st2_13_fixture() {
    let yaml = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/_examples/ST2-13.yaml")
    ).unwrap();
    let spec: CardSpec = serde_yml::from_str(&yaml).unwrap();
    let adapter = RealCardDataAdapter::from_path(&cards_json_path()).unwrap();
    cross_check(&spec, &adapter).expect("ST2-13 fixture must cross-check against real cards.json");
}

#[test]
fn real_adapter_all_fixtures_cross_check() {
    use digimon_engine::dsl::loader;
    let examples = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/_examples");
    let (specs, errs) = loader::load_dir_ok(&examples);
    assert!(errs.is_empty());
    let adapter = RealCardDataAdapter::from_path(&cards_json_path()).unwrap();
    let mut failures = Vec::new();
    for spec in &specs {
        if let Err(e) = cross_check(spec, &adapter) {
            failures.push(format!("{}: {e}", spec.card));
        }
    }
    assert!(failures.is_empty(), "fixture cross-check failures:\n{}", failures.join("\n"));
}
```

Add `mod real_cards_json;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 2: Implement `RealCardDataAdapter` in `digimon-engine/src/dsl/loader.rs`**

Append:

```rust
use std::path::Path as StdPath;

/// Real `CardDataDb` adapter that reads `cards.json` via the engine's
/// existing `CardData` parser. Maps engine-side enum values to their
/// DSL counterparts at lookup time.
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
                name: data.card_name_eng,
                kind: engine_card_kind_to_dsl(data.card_kind),
                level: if data.level < 0 { None } else { Some(data.level as u8) },
                dp: if data.dp < 0 { None } else { Some(data.dp) },
                cost: Some(data.play_cost),
                colors: data.card_colors.iter().map(|c| engine_color_to_dsl(*c)).collect(),
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

Note: `card_data::CardData::load_from_str` returns a `HashMap<String, CardData>`. Verify the shape in `digimon-engine/src/card_data.rs` before implementing — if the signature differs, adapt.

Also verify that `cards.json` `card_kind` values deserialize to enum variants — if it uses u8 integer indices (which it does per prior code; `type_eng` is an array but `card_kind: 2` = Option), the `crate::enums::CardKind` must have `Serialize/Deserialize` correctly implemented. This should already work since the engine runs against this file today.

- [ ] **Step 3: Run tests — may require fixture updates for dp/level/cost mismatches**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader real_cards_json`
Expected: 3 passed.

If any fixture fails cross-check against the real cards.json, the YAML has the wrong dp/level/cost/color — the fixture is authoritative up to the YAML author's intent but must match cards.json. Fix the fixture to match cards.json, not the other way around. Phase 0's hand-crafted stub was an approximation; real cards.json is the ground truth.

Full suite: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader`
Expected: 76 passed (73 prior + 3 new), 0 failed, no warnings.

- [ ] **Step 4: Commit**

```bash
git add digimon-engine/src/dsl/loader.rs \
        digimon-engine/tests/dsl/real_cards_json.rs \
        digimon-engine/tests/dsl/main.rs \
        digimon-engine/cards/_examples/  # only if any fixtures were corrected
git commit -m "dsl(phase1a): RealCardDataAdapter wrapping engine CardData parser"
```

---

## Task 7: Wire `dsl-lint --cross-check` flag

Expose the real adapter through the CLI so agents and CI can validate both schema and structured-data consistency in one invocation.

**Files:**
- Modify: `tools/dsl-lint/src/main.rs`

- [ ] **Step 1: Add the flag to the arg parser**

In `parse_args()` add:

```rust
            "--cross-check" => {
                let v = iter.next().ok_or("--cross-check requires a path")?;
                cross_check_path = Some(PathBuf::from(v));
            }
```

Add `cross_check_path: Option<PathBuf>` to `Args`.

- [ ] **Step 2: Load the adapter when the flag is set, and run cross_check per file**

In `lint_file`, after `validate` returns, add:

```rust
fn lint_file(path: &Path, adapter: Option<&dyn digimon_engine::dsl::loader::CardDataDb>, diags: &mut Vec<Diagnostic>) {
    // ... existing parse + validate flow ...

    if let Some(db) = adapter {
        if let Err(e) = digimon_engine::dsl::loader::cross_check(&spec, db) {
            diags.push(Diagnostic {
                file: file.clone(),
                severity: Severity::Error,
                path: e.path,
                message: e.message,
            });
        }
    }
}
```

Thread the optional adapter down from `main`:

```rust
fn main() -> ExitCode {
    let args = match parse_args() { /* ... */ };
    let adapter_opt = match &args.cross_check_path {
        Some(p) => Some(digimon_engine::dsl::loader::RealCardDataAdapter::from_path(p)
            .unwrap_or_else(|e| {
                eprintln!("dsl-lint: failed to load cards.json: {e}");
                std::process::exit(3);
            })),
        None => None,
    };
    let adapter_dyn: Option<&dyn digimon_engine::dsl::loader::CardDataDb> =
        adapter_opt.as_ref().map(|a| a as &dyn digimon_engine::dsl::loader::CardDataDb);

    let mut diags = Vec::new();
    for file in walk_yaml(&args.path) {
        lint_file(&file, adapter_dyn, &mut diags);
    }
    // ... existing output rendering + exit code logic ...
}
```

- [ ] **Step 3: Manual smoke tests**

Build:
```
cargo build -p dsl-lint
```

**A. Without cross-check flag (schema only):**
```
cargo run -p dsl-lint -- digimon-engine/cards/_examples
```
Expected: clean, exit 0.

**B. With cross-check against real cards.json:**
```
cargo run -p dsl-lint -- digimon-engine/cards/_examples --cross-check digimon_gym/engine/data/cards.json
```
Expected: clean, exit 0.

**C. Synthesize a fixture with wrong name to force cross-check fail:**

Write to `%TEMP%/bad-crosscheck.yaml`:
```yaml
card: ST2-13
name: "Wrong Name On Purpose"
kind: option
color: [red]
cost: 0
```

Run:
```
cargo run -p dsl-lint -- "%TEMP%\bad-crosscheck.yaml" --cross-check digimon_gym/engine/data/cards.json
```
Expected: diagnostic mentioning "name mismatch", exit 1.

```
cargo run -p dsl-lint -- "%TEMP%\bad-crosscheck.yaml" --cross-check digimon_gym/engine/data/cards.json --format json
```
Expected: JSON with one entry, `severity: "error"`. Exit 1.

Delete the temp file.

**D. Without cross-check flag, the same file with wrong name still parses and validates (only cross-check catches the name mismatch):**
```
cargo run -p dsl-lint -- "%TEMP%\bad-crosscheck.yaml"
```
Expected: exit 0 — demonstrating that cross-check is opt-in and catches distinct issues.

- [ ] **Step 4: Commit**

```bash
git add tools/dsl-lint/src/main.rs
git commit -m "dsl(phase1a): dsl-lint --cross-check wires real cards.json adapter"
```

---

## Task 8: Swap `phase0_exit.rs` stub for the real adapter

The Phase 0 exit integration test used a hand-crafted `CardDataDbStub`. Now that the real adapter exists, replace it — the exit test becomes a genuine end-to-end check against production `cards.json`.

**Files:**
- Modify: `digimon-engine/tests/dsl/phase0_exit.rs`

- [ ] **Step 1: Replace `build_stub_db()` with `RealCardDataAdapter::from_path(...)`**

Delete the `build_stub_db()` function. In `phase_0_exit_criteria`, replace:

```rust
    let db = build_stub_db();
```

with:

```rust
    let cards_json = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("digimon_gym/engine/data/cards.json");
    let db = digimon_engine::dsl::loader::RealCardDataAdapter::from_path(&cards_json)
        .expect("real cards.json adapter must load");
```

Remove the now-unused `CardDataDbStub`, `CardKind`, `ColorSpec` imports if any became dead.

- [ ] **Step 2: Run the exit test**

Run: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader phase0_exit`
Expected: 1 passed. If any fixture fails cross-check, fix the fixture (as in Task 6).

Full suite: `cargo test --package digimon-engine --test dsl --features dsl-yaml-loader`
Expected: 76 passed, 0 failed, no warnings.

- [ ] **Step 3: Commit**

```bash
git add digimon-engine/tests/dsl/phase0_exit.rs
git commit -m "dsl(phase1a): phase0_exit uses real cards.json adapter instead of stub"
```

---

## Phase 1a done

All 8 tasks shipped. Phase 1a resolves the Phase 0 review debt (`ClauseScope` default serialization, keyword validator gaps, `Player`/`Owner` duplication, `CostReductionBody.scope` shadow, loose `AddModifierArgs.target`) and delivers the real `cards.json` adapter behind `dsl-lint --cross-check`.

### What this unblocks
- **Agent authoring loops** — `/batch-implement-cards-rust-dsl` can now cross-check each authored YAML against the real structured data in a single CLI call.
- **Phase 1b preparation** — the consolidated `PlayerRef`, typed `ModifierTarget`, and resolved `CostReductionBody` field names mean lowering code written in Phase 1b can match against stable types instead of loose `serde_yml::Value`.

### Explicitly deferred to later sub-phases

**Phase 1b scope:**
- `CardSpec` → `CompiledCard` intermediate representation (lowering IR).
- rkyv serialization of compiled card packs.
- `build.rs` step that produces `cards.pack` as a build artifact.
- `CardRegistry::from_embedded()` constructor + the corresponding cache-directory loader for runtime-downloaded packs.

**Phase 1c scope:**
- Lowering declarative clauses (`aura`, `cost_reduction`, `flood_gate`, `grant_keyword`, `ace_overflow`) to real `Effect` closures against `EffectContext`.
- Lowering `alt_paths` (digivolve, dna_digivolve) into the engine digivolve-check path.
- Lowering `identity` into the name-overlay registry.
- Parity tests vs hand-written `CardEffect` implementations.
- Retire the first ~50 hand-written cards.

**Phase 3+ scope:**
- Six spec-text idioms simplified in Task 15 fixtures (`lose_count_bound`, `cost_reduction` as alt_path kind, mixed-form on BT10-111, `scope: before_pay_cost` → `reduction_timing: before_pay_cost` handled in Task 4 of this plan, `dp_lte: { formula: {...} }` wrapper mismatch, `condition:` on AltPathSpec).
- File splits (`step.rs` ~808 lines, `clause.rs` ~370 lines).
- DigiEgg `cost: 0` vs spec doc-comment reconciliation.
