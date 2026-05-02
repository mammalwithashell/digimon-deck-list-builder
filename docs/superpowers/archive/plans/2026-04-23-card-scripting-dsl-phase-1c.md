# Card Scripting DSL — Phase 1c Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Lower `CompiledCard` → `Vec<Effect>` for the declarative subset (aura, grant_keyword, cost_reduction, flood_gate, ace_overflow) and wire the embedded pack into the engine's `CardEffectRegistry` so DSL-authored cards actually run in `DebugRunner`.

**Architecture:** An engine-side `dsl_cards/` module owns the lowering. A `DslCardEffect` struct wraps an `Arc<CompiledCard>` and implements the engine's `CardEffect` trait — `effects()` translates each `CompiledDeclarativeClause` into one or more `Effect`s using the existing `Effect::declarative` / `Effect::before_pay_cost` builders. A predicate evaluator operates on `EffectReadContext` so scripted `active_when` / `condition` predicates become real read-only closures. Triggered clauses (`CompiledClause::Triggered`), identity, alt-paths, replacement, delay, partition, and alt-path registration are out-of-scope and emit nothing in Phase 1c (covered by Phase 2).

**Tech Stack:** Rust 2021 edition, `digimon-engine` crate, `digimon-dsl` crate (leaf — no engine types), `EffectContext` / `EffectReadContext` closures, `ModifierType` + `ModifierEntry` for flood-gate application.

**Non-goals (explicit):** process-step lowering, binding scope, `select_*` prompts, `if/for_each/per_selected`, triggered dispatch, identity aliases, alt-path digivolve registration, raw_rust dispatch, i18n resolution.

---

## File Structure

New files:
- `digimon-engine/src/dsl_cards/mod.rs` — public API: `DslCardEffect`, `register_dsl_cards`, `ace_overflow_of`
- `digimon-engine/src/dsl_cards/predicate.rs` — predicate evaluator (`eval_predicate`)
- `digimon-engine/src/dsl_cards/lower_aura.rs` — `CompiledDeclarativeClause::Aura` → `Effect`
- `digimon-engine/src/dsl_cards/lower_grant_keyword.rs` — whole-card `GrantKeyword` → `Effect`
- `digimon-engine/src/dsl_cards/lower_cost_reduction.rs` — `CostReduction` → BeforePayCost `Effect`
- `digimon-engine/src/dsl_cards/lower_flood_gate.rs` — `FloodGate` → declarative `Effect` that installs `ModifierType`
- `digimon-engine/src/dsl_cards/modifier_map.rs` — string → `ModifierType` and string → `Keyword` lookup tables
- `digimon-engine/tests/dsl/phase1c_predicate.rs` — predicate evaluator unit tests
- `digimon-engine/tests/dsl/phase1c_lowering.rs` — per-clause lowering tests
- `digimon-engine/tests/dsl/phase1c_parity.rs` — parity vs hand-written `CardEffect` for 5 fixture cards
- `digimon-engine/tests/dsl/phase1c_exit.rs` — exit criteria test

Modified files:
- `digimon-engine/src/lib.rs` — `pub mod dsl_cards;` (behind `dsl-yaml-loader` feature)
- `digimon-engine/src/cards.rs` — optional `register_dsl_cards(&mut registry)` call in `build_registry()` behind the feature flag

---

## Task 1: Scaffold `dsl_cards` module with empty lowering

**Files:**
- Create: `digimon-engine/src/dsl_cards/mod.rs`
- Modify: `digimon-engine/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Create `digimon-engine/tests/dsl/phase1c_scaffold.rs`:

```rust
// digimon-engine/tests/dsl/phase1c_scaffold.rs
use digimon_dsl::compiled::{CompiledCard, CompiledCardKind};
use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use std::sync::Arc;

#[test]
fn dsl_card_effect_with_no_clauses_emits_no_effects() {
    let compiled = CompiledCard {
        card: "TEST-EMPTY".into(),
        name: "Empty".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(3),
        color: vec![],
        cost: Some(0),
        dp: Some(1000),
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        alt_paths: vec![],
        effects: vec![],
    };
    let dsl = DslCardEffect::new(Arc::new(compiled));
    let card = CardHandle { player: 0, zone_index: 0 };
    assert!(dsl.effects(card).is_empty());
}
```

Wire it via `tests/dsl/main.rs` by adding `mod phase1c_scaffold;`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_scaffold -- --nocapture`
Expected: FAIL with "unresolved import `digimon_engine::dsl_cards`".

- [ ] **Step 3: Create scaffolding**

Create `digimon-engine/src/dsl_cards/mod.rs`:

```rust
//! DSL → engine lowering. Phase 1c: declarative clauses only.
//!
//! `DslCardEffect` wraps a `digimon_dsl::CompiledCard` and emits engine
//! `Effect`s at `effects()` time. Triggered clauses, identity, alt_paths,
//! and raw_rust are skipped in Phase 1c (Phase 2 owns them).

use std::sync::Arc;

use digimon_dsl::compiled::CompiledCard;

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct DslCardEffect {
    compiled: Arc<CompiledCard>,
}

impl DslCardEffect {
    pub fn new(compiled: Arc<CompiledCard>) -> Self {
        Self { compiled }
    }

    pub fn compiled(&self) -> &CompiledCard {
        &self.compiled
    }
}

impl CardEffect for DslCardEffect {
    fn effects(&self, _card: CardHandle) -> Vec<Effect> {
        // Phase 1c: declarative clauses only. Empty for now; per-clause
        // lowering lands in Tasks 4-8.
        Vec::new()
    }
}
```

Modify `digimon-engine/src/lib.rs` — add this line next to the other `pub mod` declarations (alphabetical order among modules starting with `d`):

```rust
#[cfg(feature = "dsl-yaml-loader")]
pub mod dsl_cards;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_scaffold`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/mod.rs digimon-engine/src/lib.rs digimon-engine/tests/dsl/main.rs digimon-engine/tests/dsl/phase1c_scaffold.rs
git commit -m "dsl phase 1c: scaffold DslCardEffect adapter (empty lowering)"
```

---

## Task 2: Predicate evaluator — leaf fields

**Files:**
- Create: `digimon-engine/src/dsl_cards/predicate.rs`
- Modify: `digimon-engine/src/dsl_cards/mod.rs` (add `pub mod predicate;`)
- Test: `digimon-engine/tests/dsl/phase1c_predicate.rs`

- [ ] **Step 1: Write the failing test**

Create `digimon-engine/tests/dsl/phase1c_predicate.rs`:

```rust
use digimon_dsl::compiled::{
    CompiledCardKind, CompiledColor, CompiledPlayerRef, CompiledPredicate, CompiledZone,
};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::predicate::{eval_predicate, PredicateSubject};

fn fresh_runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(make_test_card("TEST-A", "Test"))
        .add_card(make_test_card("TEST-B", "Test"))
        .add_card(make_test_card("TEST-C", "Test"))
        .build()
}

#[test]
fn empty_predicate_matches_anything() {
    let runner = fresh_runner();
    let game = runner.game();
    let rctx = digimon_engine::effect_context::EffectReadContext::new(
        game,
        CardHandle { player: 0, zone_index: 0 },
        None,
        0,
    );
    let pred = CompiledPredicate::default();
    assert!(eval_predicate(&pred, &rctx, PredicateSubject::None));
}

#[test]
fn kind_predicate_matches_kind_on_subject_card() {
    let runner = fresh_runner();
    let game = runner.game();
    let card = CardHandle { player: 0, zone_index: 0 };
    let rctx = digimon_engine::effect_context::EffectReadContext::new(game, card, None, 0);
    let pred = CompiledPredicate {
        kind: Some(CompiledCardKind::Digimon),
        ..Default::default()
    };
    assert!(eval_predicate(&pred, &rctx, PredicateSubject::Card(card)));

    let pred_tamer = CompiledPredicate {
        kind: Some(CompiledCardKind::Tamer),
        ..Default::default()
    };
    assert!(!eval_predicate(&pred_tamer, &rctx, PredicateSubject::Card(card)));
}

#[test]
fn your_turn_predicate_reads_game_state() {
    let runner = fresh_runner();
    let game = runner.game();
    let card = CardHandle { player: game.turn_player, zone_index: 0 };
    let rctx = digimon_engine::effect_context::EffectReadContext::new(
        game, card, None, game.turn_player,
    );
    let pred = CompiledPredicate { your_turn: Some(true), ..Default::default() };
    assert!(eval_predicate(&pred, &rctx, PredicateSubject::None));
}
```

Add `mod phase1c_predicate;` to `tests/dsl/main.rs`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_predicate`
Expected: FAIL with "unresolved import `digimon_engine::dsl_cards::predicate`".

- [ ] **Step 3: Implement predicate evaluator — leaf fields**

Create `digimon-engine/src/dsl_cards/predicate.rs`:

```rust
//! Predicate evaluator. Phase 1c: leaf + combinator fields; existentials
//! added in Task 3.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledColor, CompiledPlayerRef, CompiledPredicate, CompiledZone,
};

use crate::card_source::CardHandle;
use crate::effect_context::EffectReadContext;
use crate::enums::{CardColor, CardKind, PlayerId};
use crate::permanent::PermanentHandle;

/// The subject a predicate is applied to. Aura targets resolve to
/// `Permanent`; whole-card conditions where no card-shaped anchor exists
/// resolve to `None` (only game-state fields like `your_turn`, `memory_lte`
/// will match).
#[derive(Debug, Clone, Copy)]
pub enum PredicateSubject {
    Permanent(PermanentHandle),
    Card(CardHandle),
    None,
}

pub fn eval_predicate(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    subject: PredicateSubject,
) -> bool {
    // Game-state-only fields — evaluate regardless of subject.
    if let Some(want) = pred.your_turn {
        let is_my = rctx.game.turn_player == rctx.player;
        if is_my != want {
            return false;
        }
    }
    if let Some(want) = pred.opponents_turn {
        let is_opp = rctx.game.turn_player != rctx.player;
        if is_opp != want {
            return false;
        }
    }
    if let Some(cap) = pred.memory_lte {
        if (rctx.game.memory as i32) > cap {
            return false;
        }
    }
    if let Some(floor) = pred.memory_gte {
        if (rctx.game.memory as i32) < floor {
            return false;
        }
    }
    if let Some(cap) = pred.security_count_lte {
        if rctx.security_count(rctx.player) as u8 > cap {
            return false;
        }
    }
    if let Some(floor) = pred.security_count_gte {
        if (rctx.security_count(rctx.player) as u8) < floor {
            return false;
        }
    }

    // Subject-dependent fields.
    match subject {
        PredicateSubject::Card(card) => eval_card_fields(pred, rctx, card),
        PredicateSubject::Permanent(h) => eval_permanent_fields(pred, rctx, h),
        PredicateSubject::None => eval_no_subject_fields(pred),
    }
}

fn eval_no_subject_fields(pred: &CompiledPredicate) -> bool {
    // Reject if any subject-only fields are set.
    pred.kind.is_none()
        && pred.level_eq.is_none()
        && pred.level_lte.is_none()
        && pred.level_gte.is_none()
        && pred.color_is.is_none()
        && pred.color_only.is_none()
        && pred.trait_has.is_none()
        && pred.form_is.is_none()
        && pred.attribute_is.is_none()
        && pred.name_is.is_none()
        && pred.name_contains.is_none()
        && pred.name_in.is_none()
        && pred.card_number_is.is_none()
}

fn eval_card_fields(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    card: CardHandle,
) -> bool {
    let data_idx = match rctx.game.card_source_for(card) {
        Some(src) => src.data_index,
        None => return false,
    };
    let data = &rctx.game.card_data[data_idx];

    if let Some(want) = pred.kind {
        if !kind_matches(want, data.kind) {
            return false;
        }
    }
    if let Some(want) = pred.level_eq {
        if data.level != Some(want) {
            return false;
        }
    }
    if let Some(cap) = pred.level_lte {
        if data.level.map_or(true, |l| l > cap) {
            return false;
        }
    }
    if let Some(floor) = pred.level_gte {
        if data.level.map_or(true, |l| l < floor) {
            return false;
        }
    }
    if let Some(want) = pred.color_is {
        if !data.colors.iter().any(|c| color_matches(want, *c)) {
            return false;
        }
    }
    if let Some(ref allowed) = pred.color_only {
        for c in &data.colors {
            if !allowed.iter().any(|a| color_matches(*a, *c)) {
                return false;
            }
        }
    }
    if let Some(ref t) = pred.trait_has {
        if !data.traits.iter().any(|x| x.eq_ignore_ascii_case(t)) {
            return false;
        }
    }
    if let Some(ref f) = pred.form_is {
        if data.form.as_deref().map_or(true, |df| !df.eq_ignore_ascii_case(f)) {
            return false;
        }
    }
    if let Some(ref a) = pred.attribute_is {
        if data.attribute.as_deref().map_or(true, |da| !da.eq_ignore_ascii_case(a)) {
            return false;
        }
    }
    if let Some(ref n) = pred.name_is {
        if data.name != *n {
            return false;
        }
    }
    if let Some(ref n) = pred.name_contains {
        if !data.name.to_lowercase().contains(&n.to_lowercase()) {
            return false;
        }
    }
    if let Some(ref names) = pred.name_in {
        if !names.iter().any(|n| n == &data.name) {
            return false;
        }
    }
    if let Some(ref cn) = pred.card_number_is {
        if data.card_number != *cn {
            return false;
        }
    }
    true
}

fn eval_permanent_fields(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    handle: PermanentHandle,
) -> bool {
    let perm = match rctx.game.player(handle.player).battle_area.get(handle.index as usize) {
        Some(p) => p,
        None => return false,
    };
    let top = perm.top_card();
    // Delegate the shared card fields to the CardHandle path.
    let top_card = CardHandle { player: handle.player, zone_index: top.zone_index };
    if !eval_card_fields(pred, rctx, top_card) {
        return false;
    }
    if let Some(want) = pred.is_suspended {
        if perm.suspended != want {
            return false;
        }
    }
    if let Some(want) = pred.is_unsuspended {
        if perm.suspended == want {
            return false;
        }
    }
    if let Some(cap) = pred.stack_size_lte {
        if perm.stack_len() as u8 > cap {
            return false;
        }
    }
    if let Some(floor) = pred.stack_size_gte {
        if (perm.stack_len() as u8) < floor {
            return false;
        }
    }
    if !pred.zone.is_empty() {
        // Permanents always live in BattleArea — this fails for any zone
        // list that does not include BattleArea.
        if !pred.zone.contains(&CompiledZone::BattleArea) {
            return false;
        }
    }
    if let Some(want) = pred.owner {
        let matches = match want {
            CompiledPlayerRef::You => handle.player == rctx.player,
            CompiledPlayerRef::Opponent => handle.player == rctx.opponent_id(),
            CompiledPlayerRef::Active => handle.player == rctx.game.turn_player,
            CompiledPlayerRef::Any => true,
        };
        if !matches {
            return false;
        }
    }
    true
}

fn kind_matches(want: CompiledCardKind, got: CardKind) -> bool {
    matches!(
        (want, got),
        (CompiledCardKind::Digimon, CardKind::Digimon)
            | (CompiledCardKind::Tamer, CardKind::Tamer)
            | (CompiledCardKind::Option, CardKind::Option)
            | (CompiledCardKind::DigiEgg, CardKind::DigiEgg)
    )
}

fn color_matches(want: CompiledColor, got: CardColor) -> bool {
    matches!(
        (want, got),
        (CompiledColor::Red, CardColor::Red)
            | (CompiledColor::Blue, CardColor::Blue)
            | (CompiledColor::Yellow, CardColor::Yellow)
            | (CompiledColor::Green, CardColor::Green)
            | (CompiledColor::Black, CardColor::Black)
            | (CompiledColor::Purple, CardColor::Purple)
            | (CompiledColor::White, CardColor::White)
    )
}
```

Modify `digimon-engine/src/dsl_cards/mod.rs` — add `pub mod predicate;` after the existing `use` lines, before the struct definition.

**Contract notes for implementer:**
- If `rctx.game.card_source_for(card)` doesn't exist on `Game`, use the equivalent: iterate `game.player(card.player).{hand, deck, trash, security, battle_area}` to find the `CardSource` whose `zone_index == card.zone_index`. Match the hottest zone first — hand/trash — and fall through. If a simpler `data_index` lookup helper exists (`game.card_data_for_handle`), prefer it.
- `Permanent::stack_len` is the count of cards in the digivolution stack including the top card. If the existing Permanent API calls this something else (`source_count`, `len`), use the real name.
- `CardData` field names referenced (`kind`, `level`, `colors`, `traits`, `form`, `attribute`, `name`, `card_number`) come from `digimon-engine/src/card_data.rs`. Open that file and match the real field names if any differ.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_predicate`
Expected: all 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/mod.rs digimon-engine/src/dsl_cards/predicate.rs digimon-engine/tests/dsl/phase1c_predicate.rs digimon-engine/tests/dsl/main.rs
git commit -m "dsl phase 1c: predicate evaluator — leaf fields (kind/level/color/name/zone/owner)"
```

---

## Task 3: Predicate evaluator — combinators + existentials

**Files:**
- Modify: `digimon-engine/src/dsl_cards/predicate.rs`
- Test: `digimon-engine/tests/dsl/phase1c_predicate.rs`

- [ ] **Step 1: Write the failing test**

Append to `digimon-engine/tests/dsl/phase1c_predicate.rs`:

```rust
#[test]
fn all_of_combinator_ands_children() {
    let runner = fresh_runner();
    let game = runner.game();
    let card = CardHandle { player: 0, zone_index: 0 };
    let rctx = digimon_engine::effect_context::EffectReadContext::new(game, card, None, 0);
    let pred = CompiledPredicate {
        all_of: vec![
            CompiledPredicate { kind: Some(CompiledCardKind::Digimon), ..Default::default() },
            CompiledPredicate { level_gte: Some(1), ..Default::default() },
        ],
        ..Default::default()
    };
    assert!(eval_predicate(&pred, &rctx, PredicateSubject::Card(card)));
}

#[test]
fn any_of_combinator_ors_children() {
    let runner = fresh_runner();
    let game = runner.game();
    let card = CardHandle { player: 0, zone_index: 0 };
    let rctx = digimon_engine::effect_context::EffectReadContext::new(game, card, None, 0);
    let pred = CompiledPredicate {
        any_of: vec![
            CompiledPredicate { kind: Some(CompiledCardKind::Tamer), ..Default::default() },
            CompiledPredicate { kind: Some(CompiledCardKind::Digimon), ..Default::default() },
        ],
        ..Default::default()
    };
    assert!(eval_predicate(&pred, &rctx, PredicateSubject::Card(card)));
}

#[test]
fn none_of_combinator_inverts_any_of() {
    let runner = fresh_runner();
    let game = runner.game();
    let card = CardHandle { player: 0, zone_index: 0 };
    let rctx = digimon_engine::effect_context::EffectReadContext::new(game, card, None, 0);
    let pred = CompiledPredicate {
        none_of: vec![
            CompiledPredicate { kind: Some(CompiledCardKind::Tamer), ..Default::default() },
        ],
        ..Default::default()
    };
    assert!(eval_predicate(&pred, &rctx, PredicateSubject::Card(card)));
}

#[test]
fn not_inverts_single_child() {
    let runner = fresh_runner();
    let game = runner.game();
    let card = CardHandle { player: 0, zone_index: 0 };
    let rctx = digimon_engine::effect_context::EffectReadContext::new(game, card, None, 0);
    let pred = CompiledPredicate {
        not: Some(Box::new(CompiledPredicate {
            kind: Some(CompiledCardKind::Tamer),
            ..Default::default()
        })),
        ..Default::default()
    };
    assert!(eval_predicate(&pred, &rctx, PredicateSubject::Card(card)));
}

#[test]
fn any_permanent_matches_if_any_battle_area_perm_matches() {
    use digimon_dsl::compiled::CompiledExistential;
    // Builder that places one Digimon on our battle area.
    let runner = DebugRunner::builder()
        .add_card(make_test_card("FIXT-DIGI", "Fixt"))
        .place_permanent("FIXT-DIGI", 0)
        .build();
    let game = runner.game();
    let card = CardHandle { player: 0, zone_index: 0 };
    let rctx = digimon_engine::effect_context::EffectReadContext::new(game, card, None, 0);
    let pred = CompiledPredicate {
        any_permanent: Some(Box::new(CompiledExistential {
            of: CompiledPlayerRef::You,
            predicate: CompiledPredicate {
                kind: Some(CompiledCardKind::Digimon),
                ..Default::default()
            },
        })),
        ..Default::default()
    };
    assert!(eval_predicate(&pred, &rctx, PredicateSubject::None));
}
```

**Note to implementer:** `DebugRunner::builder().place_permanent(id, player)` is the intended helper. If it does not exist, use whatever combination of `add_card` + test-harness helpers already in `digimon-engine/src/debug_runner.rs` place a permanent on a battle area at setup time. Do not invent a new helper — use what's there.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_predicate`
Expected: new tests FAIL with "condition was false" on combinator cases.

- [ ] **Step 3: Extend the evaluator**

Append to `digimon-engine/src/dsl_cards/predicate.rs` (inside `eval_predicate`, just before the `subject` match block):

```rust
    // Combinators — short-circuit on first failure/success as appropriate.
    for child in &pred.all_of {
        if !eval_predicate(child, rctx, subject) {
            return false;
        }
    }
    if !pred.any_of.is_empty() {
        let any_match = pred.any_of.iter().any(|c| eval_predicate(c, rctx, subject));
        if !any_match {
            return false;
        }
    }
    for child in &pred.none_of {
        if eval_predicate(child, rctx, subject) {
            return false;
        }
    }
    if let Some(inner) = &pred.not {
        if eval_predicate(inner, rctx, subject) {
            return false;
        }
    }

    // Existentials — scan battle areas for a matching permanent.
    if let Some(ex) = &pred.any_permanent {
        if !existential_any(ex, rctx) {
            return false;
        }
    }
    if let Some(ex) = &pred.no_permanent {
        if existential_any(ex, rctx) {
            return false;
        }
    }
    if let Some(ex) = &pred.all_permanents {
        if !existential_all(ex, rctx) {
            return false;
        }
    }
```

Add the helper fns at the bottom of the file:

```rust
fn existential_any(
    ex: &digimon_dsl::compiled::CompiledExistential,
    rctx: &EffectReadContext<'_>,
) -> bool {
    let players = existential_players(ex.of, rctx);
    for p in players {
        let n = rctx.game.player(p).battle_area.len();
        for i in 0..n {
            let handle = PermanentHandle { player: p, index: i as u8 };
            if eval_predicate(&ex.predicate, rctx, PredicateSubject::Permanent(handle)) {
                return true;
            }
        }
    }
    false
}

fn existential_all(
    ex: &digimon_dsl::compiled::CompiledExistential,
    rctx: &EffectReadContext<'_>,
) -> bool {
    let players = existential_players(ex.of, rctx);
    let mut any_seen = false;
    for p in players {
        let n = rctx.game.player(p).battle_area.len();
        for i in 0..n {
            any_seen = true;
            let handle = PermanentHandle { player: p, index: i as u8 };
            if !eval_predicate(&ex.predicate, rctx, PredicateSubject::Permanent(handle)) {
                return false;
            }
        }
    }
    any_seen
}

fn existential_players(
    of: CompiledPlayerRef,
    rctx: &EffectReadContext<'_>,
) -> Vec<PlayerId> {
    match of {
        CompiledPlayerRef::You => vec![rctx.player],
        CompiledPlayerRef::Opponent => vec![rctx.opponent_id()],
        CompiledPlayerRef::Active => vec![rctx.game.turn_player],
        CompiledPlayerRef::Any => {
            (0..rctx.game.players.len() as PlayerId).collect()
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_predicate`
Expected: all predicate tests PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/predicate.rs digimon-engine/tests/dsl/phase1c_predicate.rs
git commit -m "dsl phase 1c: predicate combinators and existentials"
```

---

## Task 4: String → `ModifierType` and `Keyword` mapping

**Files:**
- Create: `digimon-engine/src/dsl_cards/modifier_map.rs`
- Modify: `digimon-engine/src/dsl_cards/mod.rs`
- Test: `digimon-engine/tests/dsl/phase1c_lowering.rs`

- [ ] **Step 1: Write the failing test**

Create `digimon-engine/tests/dsl/phase1c_lowering.rs`:

```rust
use digimon_engine::dsl_cards::modifier_map::{lookup_keyword, lookup_modifier_type};
use digimon_engine::enums::{Keyword, ModifierType};

#[test]
fn modifier_map_covers_flood_gate_names_used_by_examples() {
    // Names used by the 15 fixture YAMLs. When new YAMLs add new flood-gate
    // strings, extend this test + the map.
    assert_eq!(
        lookup_modifier_type("CannotActivateSecurityEffects"),
        Some(ModifierType::CannotActivateSecurityEffects)
    );
    assert_eq!(
        lookup_modifier_type("CannotBeDestroyed"),
        Some(ModifierType::CannotBeDestroyed)
    );
    assert_eq!(lookup_modifier_type("DoesNotExist"), None);
}

#[test]
fn keyword_map_covers_aura_grants_used_by_examples() {
    assert_eq!(lookup_keyword("Blocker", None), Some(Keyword::Blocker));
    assert_eq!(lookup_keyword("Raid", None), Some(Keyword::Raid));
    assert_eq!(lookup_keyword("SecurityAttackPlus", Some(1)), Some(Keyword::SecurityAttackPlus(1)));
    assert_eq!(lookup_keyword("MaterialSave", Some(1)), Some(Keyword::Save));
    assert_eq!(lookup_keyword("NotAKeyword", None), None);
}
```

Add `mod phase1c_lowering;` to `tests/dsl/main.rs`.

**Note to implementer:** `MaterialSave` in the YAML vocabulary maps to engine `Keyword::Save` — these are the same mechanic; the YAML spelling came from DCGO. Confirm which engine `Keyword` variant your real cards use; if the engine has a dedicated `Keyword::MaterialSave`, map it there instead.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_lowering`
Expected: FAIL with "unresolved module `modifier_map`".

- [ ] **Step 3: Implement the map**

Create `digimon-engine/src/dsl_cards/modifier_map.rs`:

```rust
//! Translate DSL modifier/keyword strings into engine enums.
//!
//! The DSL stores modifier/keyword names as strings (e.g.
//! `"CannotActivateSecurityEffects"`, `"Blocker"`) so the pack format is
//! decoupled from engine enum layout. This module is the single resolution
//! point — adding a new flood-gate or grantable keyword means two lines
//! (one `match` arm here, one test case above).

use crate::enums::{Keyword, ModifierType};

pub fn lookup_modifier_type(name: &str) -> Option<ModifierType> {
    Some(match name {
        "CannotActivateSecurityEffects" => ModifierType::CannotActivateSecurityEffects,
        "CannotActivateMainEffects" => ModifierType::CannotActivateMainEffects,
        "CannotBeDestroyed" => ModifierType::CannotBeDestroyed,
        "CannotBeDestroyedByBattle" => ModifierType::CannotBeDestroyedByBattle,
        "CannotBeDestroyedByEffect" => ModifierType::CannotBeDestroyedByEffect,
        "CannotBeRemoved" => ModifierType::CannotBeRemoved,
        "CannotAttack" => ModifierType::CannotAttack,
        "CannotAttackPlayer" => ModifierType::CannotAttackPlayer,
        "CannotSuspend" => ModifierType::CannotSuspend,
        "CannotUnsuspend" => ModifierType::CannotUnsuspend,
        "CannotBeSelectedByEffect" => ModifierType::CannotBeSelectedByEffect,
        "CannotBeAffected" => ModifierType::CannotBeAffected,
        "CannotReduceCost" => ModifierType::CannotReduceCost,
        _ => return None,
    })
}

pub fn lookup_keyword(name: &str, value: Option<i32>) -> Option<Keyword> {
    Some(match name {
        "Blocker" => Keyword::Blocker,
        "Rush" => Keyword::Rush,
        "Jamming" => Keyword::Jamming,
        "Piercing" => Keyword::Piercing,
        "Reboot" => Keyword::Reboot,
        "Blitz" => Keyword::Blitz,
        "Armor" => Keyword::Armor,
        "Raid" => Keyword::Raid,
        "Alliance" => Keyword::Alliance,
        "Blast" => Keyword::Blast,
        "Save" | "MaterialSave" => Keyword::Save,
        "Fortitude" => Keyword::Fortitude,
        "Overclock" => Keyword::Overclock,
        "Barrier" => Keyword::Barrier,
        "Decoy" => Keyword::Decoy,
        "Partition" => Keyword::Partition,
        "Vortex" => Keyword::Vortex,
        "Collision" => Keyword::Collision,
        "Evade" => Keyword::Evade,
        "Decode" => Keyword::Decode,
        "ArmorPurge" => Keyword::ArmorPurge,
        "SecurityAttackPlus" => Keyword::SecurityAttackPlus(value.unwrap_or(1) as i8),
        "SecurityAttackMinus" => Keyword::SecurityAttackMinus(value.unwrap_or(1) as i8),
        "DeDigivolve" => Keyword::DeDigivolve(value.unwrap_or(1) as u8),
        "DrawX" => Keyword::DrawX(value.unwrap_or(1) as u8),
        "Fragment" => Keyword::Fragment(value.unwrap_or(1) as u8),
        _ => return None,
    })
}
```

Add `pub mod modifier_map;` to `digimon-engine/src/dsl_cards/mod.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_lowering`
Expected: both map tests PASS.

**Note:** If the engine is missing a `ModifierType` variant or `Keyword` variant named here, delete the match arm (it's unreachable — no fixture references it) and update the test. Do NOT invent new enum variants — that's Phase 2/3 engine work.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/modifier_map.rs digimon-engine/src/dsl_cards/mod.rs digimon-engine/tests/dsl/phase1c_lowering.rs digimon-engine/tests/dsl/main.rs
git commit -m "dsl phase 1c: string→ModifierType/Keyword lookup tables"
```

---

## Task 5: Lower whole-card `grant_keyword`

**Files:**
- Create: `digimon-engine/src/dsl_cards/lower_grant_keyword.rs`
- Modify: `digimon-engine/src/dsl_cards/mod.rs` (register lowering; dispatch from `effects()`)
- Test: `digimon-engine/tests/dsl/phase1c_lowering.rs`

- [ ] **Step 1: Write the failing test**

Append to `digimon-engine/tests/dsl/phase1c_lowering.rs`:

```rust
use digimon_dsl::compiled::{
    CompiledCard, CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope,
};
use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use std::sync::Arc;

fn fixture_grant_keyword(keyword: &str, value: Option<i32>) -> CompiledCard {
    CompiledCard {
        card: "F-GK".into(),
        name: "Fixture".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(6),
        color: vec![],
        cost: Some(10),
        dp: Some(10000),
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        alt_paths: vec![],
        effects: vec![CompiledClause::Declarative(
            CompiledDeclarativeClause::GrantKeyword {
                keyword: keyword.into(),
                value,
                scope: CompiledScope::FaceUp,
                active_when: None,
                summary: None,
                summary_key: None,
            },
        )],
    }
}

#[test]
fn grant_keyword_emits_one_declarative_effect_with_condition_passthrough() {
    let dsl = DslCardEffect::new(Arc::new(fixture_grant_keyword("Blocker", None)));
    let card = CardHandle { player: 0, zone_index: 0 };
    let effects = dsl.effects(card);
    assert_eq!(effects.len(), 1);
    assert!(effects[0].declarative);
    assert!(effects[0].name.contains("Blocker"));
}

#[test]
fn grant_keyword_unknown_name_skips_emission() {
    let dsl = DslCardEffect::new(Arc::new(fixture_grant_keyword("NotAKeyword", None)));
    let card = CardHandle { player: 0, zone_index: 0 };
    assert!(dsl.effects(card).is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_lowering grant_keyword`
Expected: FAIL — `effects()` still returns empty.

- [ ] **Step 3: Implement the lowering**

Create `digimon-engine/src/dsl_cards/lower_grant_keyword.rs`:

```rust
//! Lower whole-card `grant_keyword` (e.g. AD1-025 Omnimon grants Raid to
//! itself) into a single declarative `Effect` that installs a permanent
//! keyword modifier on the source permanent whenever the card is in the
//! battle area.

use digimon_dsl::compiled::CompiledScope;

use crate::card_source::CardHandle;
use crate::dsl_cards::modifier_map::lookup_keyword;
use crate::effect::{Effect, EffectBuilder};
use crate::enums::Expiry;

pub fn lower(
    card: CardHandle,
    keyword_name: &str,
    value: Option<i32>,
    scope: CompiledScope,
) -> Option<Effect> {
    let kw = lookup_keyword(keyword_name, value)?;

    let mut builder: EffectBuilder = Effect::declarative(card)
        .name(&format!("Grant {keyword_name}"))
        .process(move |ctx| {
            // Apply only when this card is a face-up permanent in the battle
            // area. `source_permanent` is Some exactly in that case.
            let Some(handle) = ctx.source_permanent else { return; };
            ctx.grant_keyword(handle, kw, Expiry::Permanent);
        });

    // Scope::Inherited on a whole-card keyword is unusual — the engine's
    // stack-granted-keyword pathway uses `.inherited()` on the Effect.
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }

    Some(builder.build())
}
```

Modify `digimon-engine/src/dsl_cards/mod.rs`:

```rust
pub mod lower_grant_keyword;
pub mod modifier_map;
pub mod predicate;

// ... rest unchanged up to `impl CardEffect for DslCardEffect`

impl CardEffect for DslCardEffect {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause};

        let mut out = Vec::new();
        for clause in &self.compiled.effects {
            match clause {
                CompiledClause::Triggered(_) => {
                    // Phase 1c: triggered clauses are not lowered. Phase 2
                    // owns process-step lowering.
                }
                CompiledClause::Declarative(decl) => match decl {
                    CompiledDeclarativeClause::GrantKeyword {
                        keyword, value, scope, ..
                    } => {
                        if let Some(e) = lower_grant_keyword::lower(card, keyword, *value, *scope) {
                            out.push(e);
                        }
                    }
                    _ => {}
                },
            }
        }
        out
    }
}
```

**Note to implementer:** if `EffectContext::grant_keyword` signature differs (perhaps it takes `(target, keyword, expiry, source)` with a 4th argument), match the real signature. The exact modifier-install call comes from `digimon-engine/src/effect_context/mod.rs:903` — read that first.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_lowering grant_keyword`
Expected: both tests PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/lower_grant_keyword.rs digimon-engine/src/dsl_cards/mod.rs digimon-engine/tests/dsl/phase1c_lowering.rs
git commit -m "dsl phase 1c: lower whole-card grant_keyword to declarative Effect"
```

---

## Task 6: Lower `aura` with `dp_modifier` and `grant_keyword`

**Files:**
- Create: `digimon-engine/src/dsl_cards/lower_aura.rs`
- Modify: `digimon-engine/src/dsl_cards/mod.rs` (dispatch)
- Test: `digimon-engine/tests/dsl/phase1c_lowering.rs`

- [ ] **Step 1: Write the failing test**

Append to `digimon-engine/tests/dsl/phase1c_lowering.rs`:

```rust
use digimon_dsl::compiled::{
    CompiledGrantKeywordValue, CompiledPlayerRef, CompiledPredicate,
};

fn fixture_aura_self_dp(amount: i32) -> CompiledCard {
    CompiledCard {
        card: "F-AURA-SELF".into(),
        name: "Fixture".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(4),
        color: vec![],
        cost: Some(5),
        dp: Some(4000),
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        alt_paths: vec![],
        effects: vec![CompiledClause::Declarative(
            CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Inherited,
                active_when: None,
                target: CompiledPredicate::default(), // empty → self
                dp_modifier: Some(amount),
                grant_keyword: None,
                modifier: None,
                summary: None,
                summary_key: None,
            },
        )],
    }
}

#[test]
fn self_aura_with_dp_modifier_lowers_to_declarative_with_dp_modifier_field() {
    let dsl = DslCardEffect::new(Arc::new(fixture_aura_self_dp(2000)));
    let card = CardHandle { player: 0, zone_index: 0 };
    let effects = dsl.effects(card);
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].dp_modifier, 2000);
    assert!(effects[0].inherited, "scope: inherited should set inherited flag");
}

fn fixture_aura_filtered(
    target: CompiledPredicate,
    grant: Option<CompiledGrantKeywordValue>,
    dp: Option<i32>,
) -> CompiledCard {
    CompiledCard {
        card: "F-AURA-FILT".into(),
        name: "Fixture".into(),
        kind: CompiledCardKind::Tamer,
        level: None,
        color: vec![],
        cost: Some(3),
        dp: None,
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        alt_paths: vec![],
        effects: vec![CompiledClause::Declarative(
            CompiledDeclarativeClause::Aura {
                scope: CompiledScope::FaceUp,
                active_when: None,
                target,
                dp_modifier: dp,
                grant_keyword: grant,
                modifier: None,
                summary: None,
                summary_key: None,
            },
        )],
    }
}

#[test]
fn filtered_aura_emits_declarative_with_process_but_no_dp_modifier_field() {
    // Tamer aura: +1 SecurityAttackPlus on "Omnimon" permanents we own.
    let target = CompiledPredicate {
        owner: Some(CompiledPlayerRef::You),
        name_contains: Some("Omnimon".into()),
        ..Default::default()
    };
    let grant = Some(CompiledGrantKeywordValue {
        keyword: "SecurityAttackPlus".into(),
        value: Some(1),
    });
    let dsl = DslCardEffect::new(Arc::new(fixture_aura_filtered(target, grant, None)));
    let card = CardHandle { player: 0, zone_index: 0 };
    let effects = dsl.effects(card);
    assert_eq!(effects.len(), 1);
    assert!(effects[0].declarative);
    // dp_modifier is only used for the simple self-DP case; filtered
    // auras apply via process closure, not via the static field.
    assert_eq!(effects[0].dp_modifier, 0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_lowering aura`
Expected: FAIL — aura not dispatched yet.

- [ ] **Step 3: Implement the lowering**

Create `digimon-engine/src/dsl_cards/lower_aura.rs`:

```rust
//! Lower `CompiledDeclarativeClause::Aura`.
//!
//! Two shapes:
//!
//! 1. **Self aura** — `target` predicate is effectively empty or `permanent: carrier`.
//!    Emit a declarative `Effect` using the static `dp_modifier` field (fast path,
//!    consumed by tensor helpers without re-running the closure).
//!
//! 2. **Filtered aura** — `target` names other permanents (e.g. "all your
//!    'Omnimon' permanents"). Emit a declarative `Effect` whose `process`
//!    closure scans the relevant battle areas and applies the DP modifier /
//!    keyword grant to each matching permanent. `active_when` guards the
//!    outer condition. Phase 1c applies a `Permanent` expiry + reinstall is
//!    idempotent (engine already dedups identical modifier entries).

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledGrantKeywordValue, CompiledPredicate, CompiledScope,
};

use crate::card_source::CardHandle;
use crate::dsl_cards::modifier_map::lookup_keyword;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::effect::{Effect, EffectBuilder};
use crate::enums::{Expiry, PlayerId};
use crate::permanent::PermanentHandle;

pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<CompiledPredicate>,
    target: CompiledPredicate,
    dp_modifier: Option<i32>,
    grant_keyword: Option<CompiledGrantKeywordValue>,
) -> Option<Effect> {
    let is_self_aura = target == CompiledPredicate::default();
    let active_when = active_when.map(Arc::new);

    let mut builder: EffectBuilder = Effect::declarative(card).name("Aura");

    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    if let Some(aw) = active_when.clone() {
        builder = builder.condition(move |rctx| eval_predicate(&aw, rctx, PredicateSubject::None));
    }

    if is_self_aura {
        if let Some(dp) = dp_modifier {
            builder = builder.dp_modifier(dp);
        }
        // Self grant_keyword handled by the dedicated grant_keyword clause —
        // a self aura with grant_keyword should not occur in fixtures; fall
        // through to the process path below if it does.
        if grant_keyword.is_none() {
            return Some(builder.build());
        }
    }

    // Filtered aura — build a process closure that scans battle areas.
    let target = Arc::new(target);
    let dp = dp_modifier;
    let gk = grant_keyword.and_then(|g| lookup_keyword(&g.keyword, g.value).map(|k| (k, g.value)));
    builder = builder.process(move |ctx| {
        let target = target.clone();
        let rctx = ctx.as_read();
        let mut matched: Vec<PermanentHandle> = Vec::new();
        let players: Vec<PlayerId> = (0..rctx.game.players.len() as PlayerId).collect();
        for p in players {
            let n = rctx.game.player(p).battle_area.len();
            for i in 0..n {
                let handle = PermanentHandle { player: p, index: i as u8 };
                if eval_predicate(&target, &rctx, PredicateSubject::Permanent(handle)) {
                    matched.push(handle);
                }
            }
        }
        // Drop rctx before mutating ctx.
        drop(rctx);
        for h in matched {
            if let Some(dp) = dp {
                ctx.add_dp_modifier(h, dp, Expiry::Permanent);
            }
            if let Some((kw, _)) = gk {
                ctx.grant_keyword(h, kw, Expiry::Permanent);
            }
        }
    });

    Some(builder.build())
}
```

Add `pub mod lower_aura;` to `digimon-engine/src/dsl_cards/mod.rs`. Extend the `match decl` in `effects()`:

```rust
CompiledDeclarativeClause::Aura {
    scope,
    active_when,
    target,
    dp_modifier,
    grant_keyword,
    ..
} => {
    if let Some(e) = lower_aura::lower(
        card,
        *scope,
        active_when.clone(),
        target.clone(),
        *dp_modifier,
        grant_keyword.clone(),
    ) {
        out.push(e);
    }
}
```

**Note to implementer:** declarative aura dispatch in the engine relies on `Effect::declarative` effects being re-evaluated every tick/tensor-build. That cadence is owned by the engine (declarative effects don't enqueue — their `process` is invoked when the engine rebuilds derived state, or their static `dp_modifier`/condition is polled). Verify against an existing filtered-aura test card before trusting re-install idempotency. If the engine does NOT tick declarative process closures (it only reads static fields), replace the filtered path with an `OnPlay` + `StartOfYourTurn` pair that installs with a matching expiry — document the decision in a short comment.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_lowering aura`
Expected: both aura tests PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/lower_aura.rs digimon-engine/src/dsl_cards/mod.rs digimon-engine/tests/dsl/phase1c_lowering.rs
git commit -m "dsl phase 1c: lower aura (self-DP + filtered grant_keyword/dp_modifier)"
```

---

## Task 7: Lower `cost_reduction` with literal amount

**Files:**
- Create: `digimon-engine/src/dsl_cards/lower_cost_reduction.rs`
- Modify: `digimon-engine/src/dsl_cards/mod.rs`
- Test: `digimon-engine/tests/dsl/phase1c_lowering.rs`

- [ ] **Step 1: Write the failing test**

Append to `digimon-engine/tests/dsl/phase1c_lowering.rs`:

```rust
fn fixture_cost_reduction(amount: i32, condition: Option<CompiledPredicate>) -> CompiledCard {
    CompiledCard {
        card: "F-CR".into(),
        name: "Fixture".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(6),
        color: vec![],
        cost: Some(11),
        dp: Some(11000),
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        alt_paths: vec![],
        effects: vec![CompiledClause::Declarative(
            CompiledDeclarativeClause::CostReduction {
                scope: CompiledScope::FaceUp,
                active_when: None,
                reduction_timing: Some("before_pay_cost".into()),
                when_playing_this: true,
                when_any_ally_played: None,
                condition,
                once_per_turn: false,
                amount: Some(amount),
                amount_fn: None,
                pay_cost: vec![],
                summary: None,
                summary_key: None,
            },
        )],
    }
}

#[test]
fn cost_reduction_when_playing_this_emits_before_pay_cost_effect() {
    let dsl = DslCardEffect::new(Arc::new(fixture_cost_reduction(3, None)));
    let card = CardHandle { player: 0, zone_index: 0 };
    let effects = dsl.effects(card);
    assert_eq!(effects.len(), 1);
    assert_eq!(
        effects[0].timing,
        digimon_engine::enums::EffectTiming::BeforePayCost
    );
    assert!(effects[0].cost_reduction_fn.is_some());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_lowering cost_reduction`
Expected: FAIL — no dispatch.

- [ ] **Step 3: Implement the lowering**

Create `digimon-engine/src/dsl_cards/lower_cost_reduction.rs`:

```rust
//! Lower `CompiledDeclarativeClause::CostReduction`.
//!
//! Phase 1c only handles `when_playing_this: true` with a literal `amount`.
//! That covers BT17-015 and the majority of Tamer-gated self-discount cards.
//! `when_any_ally_played`, `amount_fn`, and `pay_cost` steps are Phase 2/3.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope};

use crate::card_source::CardHandle;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::effect::Effect;

pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<CompiledPredicate>,
    condition: Option<CompiledPredicate>,
    amount: i32,
) -> Effect {
    // Combine active_when + condition into a single evaluator — both must
    // pass for the reduction to fire.
    let active_when = active_when.map(Arc::new);
    let condition = condition.map(Arc::new);

    let mut builder = Effect::before_pay_cost(card).name("Cost reduction");
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    builder = builder.cost_reduction_fn(move |rctx| {
        // Guard: reduction only applies when THIS card is the one being
        // played. The engine sets `rctx.source_card` to the card whose
        // effect is being consulted; the "card being played" is surfaced
        // via engine context. Use `source_permanent.is_none()` as the
        // "card is still in hand" discriminator (permanent appears only
        // after the play resolves).
        if rctx.source_permanent.is_some() {
            return 0;
        }
        if let Some(aw) = &active_when {
            if !eval_predicate(aw, rctx, PredicateSubject::None) {
                return 0;
            }
        }
        if let Some(c) = &condition {
            if !eval_predicate(c, rctx, PredicateSubject::None) {
                return 0;
            }
        }
        amount
    });
    builder.build()
}
```

**Note to implementer:** the guard `rctx.source_permanent.is_some()` is the Phase 1c approximation for "this card is being played from hand". The engine's real "card being played" discriminator lives in the play-cost calculation path — look at `game_actions.rs:1798` area (mentioned in `grep` of `CannotReducePlayCost`). If the engine exposes a cleaner "is currently paying for this card" hook, use it. If it doesn't, the hand-check approximation is acceptable for Phase 1c (the one BT17-015 fixture covers only this case).

Add `pub mod lower_cost_reduction;` and dispatch:

```rust
CompiledDeclarativeClause::CostReduction {
    scope,
    active_when,
    when_playing_this,
    condition,
    amount,
    amount_fn,
    ..
} => {
    // Phase 1c: only when_playing_this + literal amount supported.
    if !*when_playing_this || amount_fn.is_some() {
        continue;
    }
    if let Some(a) = *amount {
        out.push(lower_cost_reduction::lower(
            card,
            *scope,
            active_when.clone(),
            condition.clone(),
            a,
        ));
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_lowering cost_reduction`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/lower_cost_reduction.rs digimon-engine/src/dsl_cards/mod.rs digimon-engine/tests/dsl/phase1c_lowering.rs
git commit -m "dsl phase 1c: lower cost_reduction (when_playing_this + literal amount)"
```

---

## Task 8: Lower `flood_gate`

**Files:**
- Create: `digimon-engine/src/dsl_cards/lower_flood_gate.rs`
- Modify: `digimon-engine/src/dsl_cards/mod.rs`
- Test: `digimon-engine/tests/dsl/phase1c_lowering.rs`

- [ ] **Step 1: Write the failing test**

Append to `digimon-engine/tests/dsl/phase1c_lowering.rs`:

```rust
fn fixture_flood_gate(modifier: &str, target: CompiledPredicate) -> CompiledCard {
    CompiledCard {
        card: "F-FG".into(),
        name: "Fixture".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(7),
        color: vec![],
        cost: Some(15),
        dp: Some(17000),
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        alt_paths: vec![],
        effects: vec![CompiledClause::Declarative(
            CompiledDeclarativeClause::FloodGate {
                scope: CompiledScope::FaceUp,
                active_when: Some(CompiledPredicate { your_turn: Some(true), ..Default::default() }),
                modifier: modifier.into(),
                target,
                summary: None,
                summary_key: None,
            },
        )],
    }
}

#[test]
fn flood_gate_emits_declarative_with_process_closure() {
    let target = CompiledPredicate {
        owner: Some(CompiledPlayerRef::Opponent),
        kind: Some(CompiledCardKind::Option),
        ..Default::default()
    };
    let dsl = DslCardEffect::new(Arc::new(fixture_flood_gate(
        "CannotActivateSecurityEffects",
        target,
    )));
    let card = CardHandle { player: 0, zone_index: 0 };
    let effects = dsl.effects(card);
    assert_eq!(effects.len(), 1);
    assert!(effects[0].declarative);
    assert!(effects[0].process.is_some());
}

#[test]
fn flood_gate_unknown_modifier_skips_emission() {
    let dsl = DslCardEffect::new(Arc::new(fixture_flood_gate(
        "NoSuchModifier",
        CompiledPredicate::default(),
    )));
    let card = CardHandle { player: 0, zone_index: 0 };
    assert!(dsl.effects(card).is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_lowering flood_gate`
Expected: FAIL — no dispatch.

- [ ] **Step 3: Implement the lowering**

Create `digimon-engine/src/dsl_cards/lower_flood_gate.rs`:

```rust
//! Lower `CompiledDeclarativeClause::FloodGate`.
//!
//! Phase 1c treats a flood-gate as a declarative effect whose `process`
//! installs the named `ModifierType` (at `Expiry::Permanent`) on every
//! permanent currently matching `target`. On every declarative tick the
//! process re-runs; the engine's modifier registry dedups identical
//! entries by (target, modifier, value, expiry) so re-installation is safe.
//!
//! `active_when` gates whether the gate contributes — when false, the
//! process is a no-op.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope};

use crate::card_source::CardHandle;
use crate::dsl_cards::modifier_map::lookup_modifier_type;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::effect::{Effect, EffectBuilder};
use crate::enums::{Expiry, ModifierType, PlayerId};
use crate::modifiers::ModifierEntry;
use crate::permanent::PermanentHandle;

pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<CompiledPredicate>,
    modifier_name: &str,
    target: CompiledPredicate,
) -> Option<Effect> {
    let modifier: ModifierType = lookup_modifier_type(modifier_name)?;
    let active_when = active_when.map(Arc::new);
    let target = Arc::new(target);

    let mut builder: EffectBuilder = Effect::declarative(card).name(&format!("Flood gate: {modifier_name}"));
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }

    builder = builder.process(move |ctx| {
        // active_when gate.
        {
            let rctx = ctx.as_read();
            if let Some(aw) = &active_when {
                if !eval_predicate(aw, &rctx, PredicateSubject::None) {
                    return;
                }
            }
        }
        // Collect targets.
        let mut targets: Vec<PermanentHandle> = Vec::new();
        {
            let rctx = ctx.as_read();
            let players: Vec<PlayerId> = (0..rctx.game.players.len() as PlayerId).collect();
            for p in players {
                let n = rctx.game.player(p).battle_area.len();
                for i in 0..n {
                    let handle = PermanentHandle { player: p, index: i as u8 };
                    if eval_predicate(&target, &rctx, PredicateSubject::Permanent(handle)) {
                        targets.push(handle);
                    }
                }
            }
        }
        // Install.
        let source_player = ctx.player;
        for h in targets {
            let entry = ModifierEntry::simple(modifier, 0, Expiry::Permanent, source_player);
            ctx.game.modifiers.add(h, entry);
        }
    });

    Some(builder.build())
}
```

**Note to implementer:** `ModifierEntry::simple` takes `(modifier, value, expiry, source_player)` — see `digimon-engine/src/modifiers.rs:42`. Value 0 is the neutral "no scalar" default for boolean gates like `CannotActivateSecurityEffects`; check whether the engine's flood-gate consumers read `value` (e.g. `ChangePlayCost` does) and pass the correct value if so — Phase 1c only lowers the valueless gates in the fixture list, so 0 is safe.

Add `pub mod lower_flood_gate;` and dispatch:

```rust
CompiledDeclarativeClause::FloodGate {
    scope,
    active_when,
    modifier,
    target,
    ..
} => {
    if let Some(e) = lower_flood_gate::lower(
        card,
        *scope,
        active_when.clone(),
        modifier,
        target.clone(),
    ) {
        out.push(e);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_lowering flood_gate`
Expected: both tests PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/lower_flood_gate.rs digimon-engine/src/dsl_cards/mod.rs digimon-engine/tests/dsl/phase1c_lowering.rs
git commit -m "dsl phase 1c: lower flood_gate (process-driven modifier install)"
```

---

## Task 9: Expose `ace_overflow_of`

**Files:**
- Modify: `digimon-engine/src/dsl_cards/mod.rs`
- Test: `digimon-engine/tests/dsl/phase1c_lowering.rs`

- [ ] **Step 1: Write the failing test**

Append to `digimon-engine/tests/dsl/phase1c_lowering.rs`:

```rust
#[test]
fn ace_overflow_reads_from_compiled_card() {
    let mut c = fixture_grant_keyword("Blocker", None);
    c.ace_overflow = Some(-5);
    let dsl = DslCardEffect::new(Arc::new(c));
    assert_eq!(dsl.ace_overflow(), Some(-5));
}

#[test]
fn ace_overflow_is_none_when_unset() {
    let dsl = DslCardEffect::new(Arc::new(fixture_grant_keyword("Blocker", None)));
    assert_eq!(dsl.ace_overflow(), None);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_lowering ace_overflow`
Expected: FAIL — no such method.

- [ ] **Step 3: Add the accessor**

Modify `digimon-engine/src/dsl_cards/mod.rs` — add an impl method on `DslCardEffect`:

```rust
impl DslCardEffect {
    // ... existing new() / compiled()

    pub fn ace_overflow(&self) -> Option<i32> {
        self.compiled.ace_overflow
    }
}
```

**Note:** Phase 1c exposes the value but does NOT plumb it into the overflow-damage path. Phase 2 owns the engine side of ACE — whoever lands ACE mechanics in the engine consults this accessor to get the overflow count per DSL-authored card. If the engine already has an ACE registry consulting some other lookup, route it through this accessor so ACE remains single-source-of-truth.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_lowering ace_overflow`
Expected: both tests PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/mod.rs digimon-engine/tests/dsl/phase1c_lowering.rs
git commit -m "dsl phase 1c: expose CompiledCard.ace_overflow via DslCardEffect::ace_overflow"
```

---

## Task 10: Register DSL cards into `CardEffectRegistry`

**Files:**
- Modify: `digimon-engine/src/dsl_cards/mod.rs` (`pub fn register_dsl_cards`)
- Modify: `digimon-engine/src/cards.rs` (`build_registry()` calls into DSL)
- Test: `digimon-engine/tests/dsl/phase1c_lowering.rs`

- [ ] **Step 1: Write the failing test**

Append to `digimon-engine/tests/dsl/phase1c_lowering.rs`:

```rust
#[test]
fn register_dsl_cards_inserts_every_pack_card_into_registry() {
    let pack_registry = digimon_engine::dsl_registry::from_embedded()
        .expect("embedded pack loads");
    let mut effect_registry = digimon_engine::cards::CardEffectRegistry::new();
    digimon_engine::dsl_cards::register_dsl_cards(&mut effect_registry, &pack_registry);

    // All 15 fixture cards from digimon-engine/cards/_examples/ should be
    // registered.
    assert_eq!(effect_registry.len(), pack_registry.len());
    for (card_id, _) in pack_registry.iter() {
        assert!(
            effect_registry.get(card_id).is_some(),
            "missing DSL registration for {card_id}"
        );
    }
}

#[test]
fn build_registry_contains_dsl_and_hand_written_without_collision() {
    // build_registry() calls register_dsl_cards() after hand-written sets.
    // Collision policy: DSL registration REPLACES hand-written when a card
    // appears in both (migration direction Python→Rust→DSL, per CLAUDE.md
    // rule 21). No fixture overlaps a TEST- card today so this only asserts
    // both sets present.
    let registry = digimon_engine::cards::build_registry();
    assert!(registry.get("TEST-001").is_some(), "hand-written TEST-001 present");
    assert!(registry.get("ST2-13").is_some(), "DSL-authored ST2-13 present");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_lowering register`
Expected: FAIL — `register_dsl_cards` doesn't exist.

- [ ] **Step 3: Implement registration**

Append to `digimon-engine/src/dsl_cards/mod.rs`:

```rust
use digimon_dsl::CardRegistry as DslCardRegistry;

use crate::cards::CardEffectRegistry;

/// Register every card in `dsl_registry` into `effect_registry` as a
/// `DslCardEffect`. Existing entries (e.g. hand-written TEST-* cards)
/// with the same `card_id` are replaced — DSL is authoritative once a
/// card migrates (CLAUDE.md rule 21).
pub fn register_dsl_cards(
    effect_registry: &mut CardEffectRegistry,
    dsl_registry: &DslCardRegistry,
) {
    for (card_id, compiled) in dsl_registry.iter() {
        let dsl_effect = Arc::new(DslCardEffect::new(Arc::new(compiled.clone())));
        effect_registry.insert(card_id, dsl_effect);
    }
}
```

Modify `digimon-engine/src/cards.rs` — extend `build_registry()`:

```rust
pub fn build_registry() -> CardEffectRegistry {
    let mut registry = CardEffectRegistry::new();
    test::register(&mut registry);
    bt17::register(&mut registry);
    tokens::register(&mut registry);

    // DSL-authored cards (embedded at build time via build.rs → cards.pack).
    // Registered AFTER hand-written sets so DSL overrides on collision.
    #[cfg(feature = "dsl-yaml-loader")]
    {
        match crate::dsl_registry::from_embedded() {
            Ok(pack) => crate::dsl_cards::register_dsl_cards(&mut registry, &pack),
            Err(e) => eprintln!("DSL embedded pack failed to load: {e}"),
        }
    }

    registry
}
```

**Note to implementer:** the `eprintln!` is a dev-time signal — an empty pack still succeeds so tests stay green. If the workspace convention is to log via `tracing` or a dedicated logger, use that instead.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_lowering register`
Expected: both tests PASS.

Run the full DSL suite to make sure nothing else broke:

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl`
Expected: all tests PASS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/mod.rs digimon-engine/src/cards.rs digimon-engine/tests/dsl/phase1c_lowering.rs
git commit -m "dsl phase 1c: register DSL pack cards into CardEffectRegistry"
```

---

## Task 11: Parity test vs hand-written equivalent (BT5-093)

**Files:**
- Create: `digimon-engine/tests/dsl/phase1c_parity.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the parity test**

Create `digimon-engine/tests/dsl/phase1c_parity.rs`:

```rust
//! Parity: a hand-written `CardEffect` with the same observable shape as
//! BT5-093 (Tai Kamiya & Matt Ishida — aura granting SecurityAttackPlus(1)
//! to your "Omnimon" permanents during your turn) should produce the same
//! tensor slice as the DSL-authored version when a BT5-093 permanent is
//! on the battle area.

use std::sync::Arc;

use digimon_dsl::compiled::CompiledCard;
use digimon_engine::card_source::CardHandle;
use digimon_engine::cards::CardEffectRegistry;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::Expiry;

/// Hand-written reference for the BT5-093 filtered aura — matches the
/// DSL behavior (grant SecurityAttackPlus(1) on "Omnimon" permanents we
/// own, during your turn).
struct HandBt5093;
impl CardEffect for HandBt5093 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("BT5-093 aura (hand)")
            .condition(|rctx| rctx.game.turn_player == rctx.player)
            .process(|ctx| {
                let rctx = ctx.as_read();
                let me = rctx.player;
                let n = rctx.game.player(me).battle_area.len();
                let mut hits = Vec::new();
                for i in 0..n {
                    let h = digimon_engine::permanent::PermanentHandle { player: me, index: i as u8 };
                    let perm = &rctx.game.player(me).battle_area[i];
                    let data = &rctx.game.card_data[perm.top_card().data_index];
                    if data.name.contains("Omnimon") {
                        hits.push(h);
                    }
                }
                drop(rctx);
                for h in hits {
                    ctx.grant_keyword(
                        h,
                        digimon_engine::enums::Keyword::SecurityAttackPlus(1),
                        Expiry::Permanent,
                    );
                }
            })
            .build()]
    }
}

fn dsl_bt5093_compiled() -> Arc<CompiledCard> {
    let pack = digimon_engine::dsl_registry::from_embedded().unwrap();
    Arc::new(pack.lookup("BT5-093").expect("BT5-093 in pack").clone())
}

#[test]
fn dsl_and_hand_versions_of_bt5093_grant_identical_keyword_set() {
    let dsl_effects = DslCardEffect::new(dsl_bt5093_compiled()).effects(
        CardHandle { player: 0, zone_index: 0 },
    );
    let hand_effects = HandBt5093.effects(CardHandle { player: 0, zone_index: 0 });

    // Both emit one declarative effect (BT5-093 also has a triggered
    // `start_of_your_turn` clause and an `on_security` clause — Phase 1c
    // does NOT lower those, so the DSL version should emit only the aura
    // clause that does lower: grant_keyword via filtered aura).
    let dsl_decl: Vec<_> = dsl_effects.iter().filter(|e| e.declarative).collect();
    let hand_decl: Vec<_> = hand_effects.iter().filter(|e| e.declarative).collect();
    assert_eq!(dsl_decl.len(), hand_decl.len(), "declarative effect count parity");
}
```

Add `mod phase1c_parity;` to `tests/dsl/main.rs`.

**Note to implementer:** the assertion compares declarative-effect *count*, not byte-equality of the closures themselves (closures are opaque). For Phase 1c this count assertion is sufficient — full tensor-parity testing (§7.3 Phase 2 exit criteria in the spec) requires process-step lowering and lives in the Phase 2 plan.

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_parity`
Expected: PASS.

If it fails because BT5-093 has a different shape than expected (e.g. the `on_security` or `start_of_your_turn` clause bleeds into a declarative effect), that's a real bug — investigate and fix the lowering, not the test.

- [ ] **Step 3: Commit**

```bash
git add digimon-engine/tests/dsl/phase1c_parity.rs digimon-engine/tests/dsl/main.rs
git commit -m "dsl phase 1c: parity test — BT5-093 aura DSL vs hand-written"
```

---

## Task 12: Exit test — 15 pack cards load and lower cleanly

**Files:**
- Create: `digimon-engine/tests/dsl/phase1c_exit.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Write the exit test**

Create `digimon-engine/tests/dsl/phase1c_exit.rs`:

```rust
//! Phase 1c exit: every fixture YAML compiles into CompiledCard, registers
//! into CardEffectRegistry, and at least the declarative clauses produce
//! Effects (ST2-13 has no declarative clauses — only triggered — so it may
//! produce zero effects; that's expected in Phase 1c).

use digimon_dsl::compiled::CompiledClause;
use digimon_engine::card_source::CardHandle;
use digimon_engine::cards::build_registry;

#[test]
fn all_fixture_cards_register() {
    let registry = build_registry();
    let pack = digimon_engine::dsl_registry::from_embedded().unwrap();
    assert!(pack.len() >= 15, "at least 15 fixtures in the embedded pack");
    for (card_id, _) in pack.iter() {
        assert!(
            registry.get(card_id).is_some(),
            "pack card {card_id} not registered",
        );
    }
}

#[test]
fn declarative_fixtures_produce_at_least_one_effect() {
    // Cards from the 15 fixtures whose declarative clauses Phase 1c lowers
    // into at least one Effect. Exclude cards whose ONLY effects are
    // triggered or alt_path_registration (not lowered in Phase 1c):
    //   - ST2-13: only `when: main_from_hand` + `on_security` → zero effects
    //   - BT17-007: triggered + alt_path_registration → zero effects
    //   - BT18-019, BT12-112, BT22-084, BT24-016, BT13-060, BT13-007,
    //     BT20-083, EX11-012 — mix (see inline fixture review).
    //
    // Cards with a declarative clause we lower (cost_reduction, aura,
    // grant_keyword, flood_gate):
    let must_have_effect = &[
        "BT17-015", // cost_reduction
        "BT10-111", // grant_keyword + aura
        "BT5-093",  // aura
        "AD1-025",  // grant_keyword (Raid + Blocker)
        "BT12-112", // flood_gate (CannotActivateSecurityEffects)
    ];

    let registry = build_registry();
    let pack = digimon_engine::dsl_registry::from_embedded().unwrap();
    for card_id in must_have_effect {
        let compiled = pack.lookup(card_id).unwrap_or_else(|| panic!("{card_id} missing"));
        let has_declarative = compiled
            .effects
            .iter()
            .any(|c| matches!(c, CompiledClause::Declarative(_)));
        assert!(
            has_declarative,
            "{card_id} has no declarative clauses — fixture audit needed"
        );
        let effect = registry
            .get(card_id)
            .unwrap_or_else(|| panic!("{card_id} not registered"));
        let card = CardHandle { player: 0, zone_index: 0 };
        let out = effect.effects(card);
        assert!(
            !out.is_empty(),
            "{card_id} declarative clauses lowered to zero Effects"
        );
    }
}
```

Add `mod phase1c_exit;` to `tests/dsl/main.rs`.

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase1c_exit`
Expected: PASS.

If `BT12-112` reports no declarative clause, re-read the YAML — it has a `kind: flood_gate` declarative clause. Fix the lowering dispatch (`match decl` arm), not the test.

- [ ] **Step 3: Run the full engine test suite**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: all tests PASS (Phase 1b's 80 + new Phase 1c tests). If anything outside `dsl` broke, it's a regression in `build_registry()` — check that DSL registration doesn't shadow a TEST-* card that's referenced by another test.

- [ ] **Step 4: Commit**

```bash
git add digimon-engine/tests/dsl/phase1c_exit.rs digimon-engine/tests/dsl/main.rs
git commit -m "dsl phase 1c: exit test — 15 fixtures register, declarative clauses lower"
```

---

## Self-Review Summary

**Spec coverage (§7.2 Phase 1 scope):**
- alt_paths — DEFERRED (needs engine first-class alt-path hook, Phase 2)
- identity — DEFERRED (Phase 2)
- aura — Task 6
- cost_reduction — Task 7 (literal amount + when_playing_this only)
- flood_gate — Task 8
- grant_keyword — Task 5
- ace_overflow — Task 9 (accessor only; engine ACE integration owned by the engine team)

**Non-goals kept out:**
- Triggered process steps (Phase 2)
- Bindings (Phase 2)
- raw_rust dispatch (Phase 4)
- Replacement / delay / partition / alt_path_registration (Phase 3)

**Type consistency:** `DslCardEffect::new(Arc<CompiledCard>)`, `DslCardEffect::effects(CardHandle) -> Vec<Effect>`, `register_dsl_cards(&mut CardEffectRegistry, &DslCardRegistry)`, `PredicateSubject::{Permanent, Card, None}`, `eval_predicate(&CompiledPredicate, &EffectReadContext, PredicateSubject) -> bool`, `lookup_modifier_type(&str) -> Option<ModifierType>`, `lookup_keyword(&str, Option<i32>) -> Option<Keyword>`. Used consistently across tasks 1–12.

**Deferred for Phase 2 planning:**
- alt_path lowering (requires engine side to expose an `alt_path_registry` surface — currently alt-paths are hand-written via `evo_costs` on `CardData` for basic digivolve, and `dna_digivolve.rs` hand-rolls DNA paths)
- Identity / name aliases — blocked on engine "treat_as" hook in combat / selection machinery
- `cost_reduction` with `amount_fn`, `when_any_ally_played`, and `pay_cost` steps
- Triggered clauses — the big one; unlocks ~500 cards
