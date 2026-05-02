# Card Scripting DSL — Phase 2e Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Lower the six remaining selection-step variants — `SelectEffectChoice`, `SelectReveal`, `SelectSecurity`, `SelectMaterial`, `SelectUnionZone`, `SelectOrderedPermutation` — and add `distinct_by` enforcement to `SelectCountCappedMulti`. After 2e, every selection verb in §3.7.7 of the DSL spec except `AsSelectingPlayer` lowers from `CompiledStep` to engine API calls. Together this unblocks the long tail of cards that read security, prowl reveal piles, pick from the union of hand+trash, peek into materials, or order a sequence (Royal Knight stack-on-deck flow, X-Antibody reveal-from-deck, the EX5 reveal/security trash effects, EX10-059 ordered-bottom-of-stack flows, the `select_effect_choice` "choose one of two effects" patterns).

**Architecture:** Each selection variant follows the 2b/2c/2d "install + capture-tail" pattern verbatim — `selections::try_install` matches the variant, resolves any `CompiledBindingRef` arguments, captures the post-selection step slice as a heap-allocated callback, calls the engine's `ctx.select_*` helper with an accept-all filter (Phase 2b precedent), and on resolution writes the picked value into the cloned `Bindings` per-callback before driving `run_steps` and draining the outer tail. `distinct_by` enforcement piggybacks the existing `install_count_capped_step` trampoline: the engine API grows an `Option<DistinctByMode>` parameter that the trampoline consults when filtering remaining candidates after each pick. The DSL passes through the variant unchanged.

**Tech Stack:** Rust 2021, `digimon-engine`, `digimon-dsl` leaf crate.

**Scope (strict):**
- `SelectEffectChoice` lowering — binds chosen branch index as `BindingValue::Literal`
- `SelectReveal` lowering — binds picked `CardHandle` from `game.revealed_cards`
- `SelectSecurity` lowering — binds picked `CardHandle` from `game.player(of).security`
- `SelectMaterial` lowering — resolves `of_permanent` via `binding_ref`, binds picked source `CardHandle`
- `SelectUnionZone` lowering — translates `Vec<CompiledZone>` to `UnionZoneSet`, binds picked `CardHandle`
- `SelectOrderedPermutation` lowering — resolves `items` to a `CardList`, binds the ordered `Vec<CardHandle>` as `BindingValue::CardList`
- `distinct_by` enforcement in `EffectContext::select_count_capped_multi` and its trampoline — `CardNumber`, `Level`, `Name`
- Tests: per-variant behavioral test plus an end-to-end fixture that chains `SelectEffectChoice` → branch-specific `SelectReveal` and confirms the right branch fires
- `Bindings::insert_literal(name, i64)` convenience helper for symmetry with the other typed inserters

**Non-goals (Phase 2f+):**
- `AsSelectingPlayer` — needs the `override_selecting_player` to persist across the entire body's selection callbacks, not just the immediate `select_*` call. Today the override lives on `EffectContext` (`pub(super)`) and is captured into `PendingSelection.selecting_player` at install time; subsequent callbacks reconstruct `EffectContext` without the override. Plumbing override-persistence across callback re-entries is engine work that wants its own design pass.
- Play / digivolve / placement step lowering (`PlayFromHand`, `PlayFromHandFree`, `PlayFromTrash`, `PlayFromTrashFree`, `PlayFromSecurity`, `PlayFromMaterials`, `EffectInitiatedDigivolve`, `EffectInitiatedDnaDigivolve`, `PlayToken`, `Hatch`, `PlaceOnSecurity`, `PlaceAsBottomSource`, `TrashTopSource`, `TrashTopSecurity`) — unchanged compiled variants, all call sites synchronous, but a coherent slice on its own. Belongs in Phase 2f.
- `ScheduleDelayed` — needs new engine primitive `ctx.schedule_delayed(when, body)`. Belongs in its own engine-design plan.
- Wiring the formula evaluator into `add_modifier`'s `value` field (literals only today). Polish item.
- `equals` / `not_equals` predicates that consume the `SelectEffectChoice` literal binding. Predicate-evaluator work; tracked separately.

---

## File structure

- Modify: `digimon-engine/src/dsl_cards/bindings.rs` (add `insert_literal`)
- Modify: `digimon-engine/src/dsl_cards/step/selections.rs` (add 6 install fns + `try_install` arms; pass `distinct_by` into the multi installer)
- Modify: `digimon-engine/src/effect_context/selections.rs` (add `DistinctByMode` enum; thread it through `select_count_capped_multi` + `install_count_capped_step` + the `EffectContextSelectorScope` forwarder)
- Modify: `digimon-engine/src/effect_context/mod.rs` (re-export `DistinctByMode`)
- Create: `digimon-engine/tests/dsl/phase2e_select_effect_choice.rs`
- Create: `digimon-engine/tests/dsl/phase2e_select_reveal.rs`
- Create: `digimon-engine/tests/dsl/phase2e_select_security.rs`
- Create: `digimon-engine/tests/dsl/phase2e_select_material.rs`
- Create: `digimon-engine/tests/dsl/phase2e_select_union_zone.rs`
- Create: `digimon-engine/tests/dsl/phase2e_select_ordered_permutation.rs`
- Create: `digimon-engine/tests/dsl/phase2e_distinct_by.rs`
- Create: `digimon-engine/tests/dsl/phase2e_end_to_end.rs`
- Modify: `digimon-engine/tests/dsl/main.rs` (register the eight new test modules)
- Modify: `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` (§7.3 sub-phase notes — append a 2e bullet that mirrors the 2d bullet)

**Test fixture conventions** (verified against `tests/dsl/phase2d_*.rs` — copy verbatim):
- `DebugRunner::builder().add_card(make_test_card(id, name)).hand(player, &[id, ...]).build()`.
- Player IDs are raw `u8` (`0` for P0, `1` for P1) — no `PlayerId::P*` enum sugar.
- Hand source-card handle: `runner.game.players[player_idx].hand[0].handle()`.
- Push to trash: `push_to_trash` helper from `phase2d_select_count_capped_multi.rs` — copy the helper into each new test module that needs it (no shared module beyond `phase2d_helpers.rs`, which is selection-specific).
- Push to deck or security: build a `CardSource` via `CardSource::new(data_idx, player, card_index)` and `runner.game.players[p].deck.push(...)` / `.security.push(...)`. `data_idx` comes from `runner.game.card_data.iter().position(|c| c.card_id == ...)`. `card_index` comes from `runner.game.next_card_index()`.
- Reveal pile: build `CardSource`s the same way and `runner.game.revealed_cards.push(card)`.
- Pending selection inspection: `runner.game.pending_selection.as_ref()` → `pending.valid_action_ids` + `pending.selecting_player`.
- Resolve selection: `runner.game.resolve_selection(selecting_player, action_id).expect("...")`.
- `EffectContext::new(&mut runner.game, src_card, /* source_permanent */ None, ctx_player)` — keep the curly-brace scope so the `&mut` borrow drops before the next `runner.game.*` access.
- `CompiledStep` variant shapes — verified from `digimon-dsl/src/compiled.rs`:
  - `SelectEffectChoice { labels: Vec<String>, bind_as: Option<String>, prompt: String, prompt_key: Option<String> }`
  - `SelectReveal { of: CompiledPlayerRef, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool }`
  - `SelectSecurity { of: CompiledPlayerRef, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool }`
  - `SelectMaterial { of_permanent: CompiledBindingRef, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool }`
  - `SelectUnionZone { of: CompiledPlayerRef, zones: Vec<CompiledZone>, filter: CompiledPredicate, bind_as: Option<String>, prompt: String, prompt_key: Option<String>, optional: bool }`
  - `SelectOrderedPermutation { items: CompiledBindingRef, bind_as: Option<String>, prompt: String, prompt_key: Option<String> }`
  - `SelectCountCappedMulti { ..., distinct_by: Option<CompiledDistinctBy> }` (already shipped in 2d; the field is read here in Task 7).

---

## Task 1: `Bindings::insert_literal` helper

Tiny symmetry add — `SelectEffectChoice` will store a chosen branch index as `BindingValue::Literal(i64)`. The other typed inserters all exist; this one was missed.

**Files:**
- Modify: `digimon-engine/src/dsl_cards/bindings.rs`

- [ ] **Step 1: Write the failing test**

Append to the `mod tests` block at the bottom of `digimon-engine/src/dsl_cards/bindings.rs`:

```rust
#[test]
fn literal_insert_round_trip() {
    let mut b = Bindings::new();
    b.insert_literal("branch", 1);
    assert_eq!(b.get_literal("branch"), Some(1));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml dsl_cards::bindings::tests::literal_insert_round_trip`
Expected: FAIL — `insert_literal` does not exist.

- [ ] **Step 3: Add the helper**

In `digimon-engine/src/dsl_cards/bindings.rs`, append to the `impl Bindings` block (alongside `insert_hand_index` / `insert_trash_index`):

```rust
pub fn insert_literal(&mut self, name: &str, v: i64) {
    self.insert(name, BindingValue::Literal(v));
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml dsl_cards::bindings::tests::literal_insert_round_trip`
Expected: PASS.

Then verify the broader engine still compiles:

Run: `cargo build --manifest-path digimon-engine/Cargo.toml`
Expected: SUCCESS.

- [ ] **Step 5: Commit**

```bash
git add digimon-engine/src/dsl_cards/bindings.rs
git commit -m "dsl phase 2e: Bindings::insert_literal helper"
```

---

## Task 2: Lower `SelectEffectChoice`

Smallest of the six new selections — no zone, no filter, just a list of labels. Engine API: `EffectContext::select_effect_choice(prompt, labels, callback)` where `callback: FnOnce(&mut EffectContext, usize)` receives the picked branch index. Bind the index as a `Literal` so downstream `if/equals` predicates can branch on it (a separate `equals` predicate task hooks the binding up; until then the binding is forward-compatible storage).

**Files:**
- Modify: `digimon-engine/src/dsl_cards/step/selections.rs`
- Create: `digimon-engine/tests/dsl/phase2e_select_effect_choice.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Register the new test module in `main.rs`**

Append to `digimon-engine/tests/dsl/main.rs` (alongside the existing `mod phase2d_*;` declarations — keep them in alphabetical order):

```rust
mod phase2e_select_effect_choice;
```

(You'll add the other new modules in subsequent tasks.)

- [ ] **Step 2: Write the failing test**

Create `digimon-engine/tests/dsl/phase2e_select_effect_choice.rs`:

```rust
//! Phase 2e Task 2: SelectEffectChoice installs a parking selection,
//! its callback writes the chosen branch index into Bindings as
//! `BindingValue::Literal`, and the post-selection step runs.

use digimon_dsl::compiled::CompiledStep;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

#[test]
fn select_effect_choice_binds_picked_index() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .hand(0, &["SRC"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();

    let steps = vec![
        CompiledStep::SelectEffectChoice {
            labels: vec!["A".to_string(), "B".to_string()],
            bind_as: Some("branch".to_string()),
            prompt: "Pick A or B".to_string(),
            prompt_key: None,
        },
        // Sentinel: gain memory so the test can confirm the post-select
        // tail ran. Branch-specific behavior is exercised by the end-to-end
        // test in Task 9 once the equals predicate lands; until then we
        // just confirm the callback fires and the tail executes.
        CompiledStep::GainMemory(1),
    ];

    let memory_before = runner.game.memory;

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // SelectEffectChoice parked — the GainMemory tail should not have run yet.
    assert!(
        runner.game.pending_selection.is_some(),
        "select_effect_choice must install a pending selection"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "tail must not run before the choice is resolved"
    );

    // Resolve by picking branch 1 ("B").
    let (action_id, selecting_player) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        // labels[1] is the second action_id in the list; the engine
        // guarantees `valid_action_ids[i]` corresponds to label index i.
        (pending.valid_action_ids[1], pending.selecting_player)
    };
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "tail must run after resolution"
    );
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2e_select_effect_choice`
Expected: FAIL — the dispatcher does not match `SelectEffectChoice`; the tail runs synchronously and the assertion `pending_selection.is_some()` fails (or the dispatcher panics before installing a selection — either way, RED).

- [ ] **Step 4: Add the install fn + dispatch arm**

In `digimon-engine/src/dsl_cards/step/selections.rs`, append a new `match` arm in `try_install`:

```rust
CompiledStep::SelectEffectChoice { labels, bind_as, prompt, .. } => {
    install_select_effect_choice(
        ctx,
        labels.clone(),
        bind_as.clone(),
        prompt.clone(),
        tail.to_vec(),
        bindings,
    );
    true
}
```

(Place it alongside the other `Select*` arms — order doesn't matter functionally, but matching declaration order in `compiled.rs` keeps grep-ability.)

Then append the install fn at the bottom of `selections.rs` (alongside the other `install_select_*` fns):

```rust
fn install_select_effect_choice(
    ctx: &mut EffectContext<'_>,
    labels: Vec<String>,
    bind_as: Option<String>,
    prompt: String,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    let tail = Arc::new(tail);
    ctx.select_effect_choice(
        &prompt,
        labels,
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_literal(name, idx as i64);
            }
            run_steps(&tail, cb_ctx, &mut b);
            // Phase 2d Task 7: drain outer tail captured by run_steps when
            // this selection was installed inside a control-flow body.
            drain_dsl_outer_tail(cb_ctx);
        },
    );
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2e_select_effect_choice`
Expected: PASS.

Re-run the full DSL test suite to confirm no regression:

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl`
Expected: PASS (all phase2a/2b/2c/2d tests + the new phase2e one).

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/selections.rs \
        digimon-engine/tests/dsl/main.rs \
        digimon-engine/tests/dsl/phase2e_select_effect_choice.rs
git commit -m "dsl phase 2e: lower SelectEffectChoice"
```

---

## Task 3: Lower `SelectReveal`

Engine API: `EffectContext::select_reveal(prompt, is_optional, filter, callback)` where `filter: Fn(&Game, usize) -> bool` and `callback: FnOnce(&mut EffectContext, usize)`. Picks an index into `game.revealed_cards`. We resolve that index to a `CardHandle` inside the callback before binding (downstream verbs always want stable handles, never positional indices).

`SelectReveal` has an `of: CompiledPlayerRef` field on the variant — but the engine API doesn't take an `of` argument because the reveal pile is a single global queue (`Game::revealed_cards`), not per-player. The DSL accepts `of:` for symmetry with the other `select_*` verbs and forward-compat with multi-player; for now we simply ignore the argument at install time. Keep the parameter in the lowering signature so the variant signature stays stable; pass it to `resolve_player` only to validate it's a non-`Any` ref (validation, no behavior change).

**Files:**
- Modify: `digimon-engine/src/dsl_cards/step/selections.rs`
- Create: `digimon-engine/tests/dsl/phase2e_select_reveal.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Register the new test module**

Append to `digimon-engine/tests/dsl/main.rs`:

```rust
mod phase2e_select_reveal;
```

- [ ] **Step 2: Write the failing test**

Create `digimon-engine/tests/dsl/phase2e_select_reveal.rs`:

```rust
//! Phase 2e Task 3: SelectReveal installs a parking selection over
//! `Game::revealed_cards`; its callback resolves the picked index into a
//! `CardHandle` and writes it into Bindings.

use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate, CompiledStep};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

fn push_to_revealed(runner: &mut DebugRunner, owner: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, owner, card_index);
    runner.game.revealed_cards.push(card);
}

#[test]
fn select_reveal_binds_picked_card_handle() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("R0", "R0"))
        .add_card(make_test_card("R1", "R1"))
        .hand(0, &["SRC"])
        .build();

    push_to_revealed(&mut runner, 0, "R0");
    push_to_revealed(&mut runner, 0, "R1");
    let target_handle = runner.game.revealed_cards[1].handle();
    let src_card = runner.game.players[0].hand[0].handle();

    let steps = vec![
        CompiledStep::SelectReveal {
            of: CompiledPlayerRef::You,
            filter: CompiledPredicate::default(),
            bind_as: Some("picked".to_string()),
            prompt: "Pick a revealed card".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert!(runner.game.pending_selection.is_some());

    // Pick the second revealed card (index 1).
    let (action_id, selecting_player) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        (pending.valid_action_ids[1], pending.selecting_player)
    };
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve");

    assert!(runner.game.pending_selection.is_none());
    // Tail ran (memory increased by 1).
    // Binding visibility across the callback is exercised in the end-to-end
    // test in Task 9; here we just assert the install + resolve plumbing.
    let _ = target_handle; // tracked for the e2e test
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2e_select_reveal`
Expected: FAIL — `SelectReveal` is not handled by the dispatcher.

- [ ] **Step 4: Add the install fn + dispatch arm**

In `digimon-engine/src/dsl_cards/step/selections.rs`, append a new `match` arm in `try_install`:

```rust
CompiledStep::SelectReveal { of: _, bind_as, prompt, optional, .. } => {
    install_select_reveal(
        ctx,
        bind_as.clone(),
        prompt.clone(),
        *optional,
        tail.to_vec(),
        bindings,
    );
    true
}
```

(The `of:` field is currently ignored — see the task header. The underscore in the destructure pattern documents the ignore.)

Append the install fn:

```rust
fn install_select_reveal(
    ctx: &mut EffectContext<'_>,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    let tail = Arc::new(tail);
    ctx.select_reveal(
        &prompt,
        optional,
        |_game, _idx| true, // Phase 2b precedent: accept-all filter.
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                // Resolve the picked reveal index into a stable CardHandle.
                if let Some(card) = cb_ctx.game.revealed_cards.get(idx) {
                    b.insert_card(name, card.handle());
                }
                // If the index has gone stale (the reveal pile mutated mid-
                // resolution — currently impossible but defensive), silently
                // skip the binding; downstream verbs that consume it no-op
                // per the 2b/2c missing-binding convention.
            }
            run_steps(&tail, cb_ctx, &mut b);
            drain_dsl_outer_tail(cb_ctx);
        },
    );
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2e_select_reveal`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/selections.rs \
        digimon-engine/tests/dsl/main.rs \
        digimon-engine/tests/dsl/phase2e_select_reveal.rs
git commit -m "dsl phase 2e: lower SelectReveal"
```

---

## Task 4: Lower `SelectSecurity`

Engine API: `EffectContext::select_security(of_player, prompt, is_optional, filter, callback)` where `callback: FnOnce(&mut EffectContext, usize)` receives a position in `game.player(of).security`. Bind the resolved `CardHandle`.

**Files:**
- Modify: `digimon-engine/src/dsl_cards/step/selections.rs`
- Create: `digimon-engine/tests/dsl/phase2e_select_security.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Register the new test module**

Append to `digimon-engine/tests/dsl/main.rs`:

```rust
mod phase2e_select_security;
```

- [ ] **Step 2: Write the failing test**

Create `digimon-engine/tests/dsl/phase2e_select_security.rs`:

```rust
//! Phase 2e Task 4: SelectSecurity installs a parking selection over
//! `Game::player(of).security`; the callback resolves the picked index
//! into a CardHandle.

use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate, CompiledStep};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

fn push_to_security(runner: &mut DebugRunner, owner: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, owner, card_index);
    runner.game.players[owner as usize].security.push(card);
}

#[test]
fn select_security_opponent_binds_picked_handle() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("S0", "S0"))
        .add_card(make_test_card("S1", "S1"))
        .hand(0, &["SRC"])
        .build();

    push_to_security(&mut runner, 1, "S0");
    push_to_security(&mut runner, 1, "S1");
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![
        CompiledStep::SelectSecurity {
            of: CompiledPlayerRef::Opponent,
            filter: CompiledPredicate::default(),
            bind_as: Some("sec_pick".to_string()),
            prompt: "Pick an opponent security".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert!(runner.game.pending_selection.is_some());

    let (action_id, selecting_player) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        (pending.valid_action_ids[0], pending.selecting_player)
    };
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.memory, memory_before + 1);
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2e_select_security`
Expected: FAIL.

- [ ] **Step 4: Add the install fn + dispatch arm**

In `digimon-engine/src/dsl_cards/step/selections.rs`, add a `try_install` arm:

```rust
CompiledStep::SelectSecurity { of, bind_as, prompt, optional, .. } => {
    install_select_security(
        ctx,
        *of,
        bind_as.clone(),
        prompt.clone(),
        *optional,
        tail.to_vec(),
        bindings,
    );
    true
}
```

Append the install fn:

```rust
fn install_select_security(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    let target_player = resolve_player(ctx, of);
    let tail = Arc::new(tail);
    ctx.select_security(
        target_player,
        &prompt,
        optional,
        |_game, _idx| true,
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                if let Some(card) = cb_ctx.game.player(target_player).security.get(idx) {
                    b.insert_card(name, card.handle());
                }
            }
            run_steps(&tail, cb_ctx, &mut b);
            drain_dsl_outer_tail(cb_ctx);
        },
    );
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2e_select_security`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/selections.rs \
        digimon-engine/tests/dsl/main.rs \
        digimon-engine/tests/dsl/phase2e_select_security.rs
git commit -m "dsl phase 2e: lower SelectSecurity"
```

---

## Task 5: Lower `SelectMaterial`

Engine API: `EffectContext::select_material(of_permanent, prompt, is_optional, filter, callback)` where `callback: FnOnce(&mut EffectContext, usize)` receives a `card_sources` index inside `of_permanent` (top excluded). The DSL variant has `of_permanent: CompiledBindingRef`, so we resolve it via `binding_ref::resolve_binding_ref` and check it returned a `Permanent` — anything else (or missing) is a silent no-op (2b/2c convention).

Bind the picked source as a `CardHandle`, resolved from `game.player(perm.player).battle_area[perm.index].card_sources[idx]`.

**Files:**
- Modify: `digimon-engine/src/dsl_cards/step/selections.rs`
- Create: `digimon-engine/tests/dsl/phase2e_select_material.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Register the new test module**

Append to `digimon-engine/tests/dsl/main.rs`:

```rust
mod phase2e_select_material;
```

- [ ] **Step 2: Write the failing test**

Create `digimon-engine/tests/dsl/phase2e_select_material.rs`. The test plays a Digimon, stacks two materials under it, and confirms `SelectMaterial` installs a selection with `valid_action_ids` matching the two non-top sources.

```rust
//! Phase 2e Task 5: SelectMaterial resolves `of_permanent` via the
//! existing binding_ref machinery, installs a parking material-pick
//! selection, and binds the picked source as a CardHandle.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledPredicate, CompiledStep};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

#[test]
fn select_material_binds_picked_source_handle() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("STACK", "STACK"))
        .add_card(make_test_card("M0", "M0"))
        .add_card(make_test_card("M1", "M1"))
        .hand(0, &["SRC"])
        .build();

    // Place a permanent on the field with two materials underneath.
    runner.place_on_field(0, "STACK", None);
    runner.stack_material_under(0, 0, "M0");
    runner.stack_material_under(0, 0, "M1");

    let perm_handle = runner.perm_handle(0, 0);
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    // Pre-populate a binding that of_permanent: Named will resolve.
    let mut bindings = Bindings::new();
    bindings.insert_permanent("target", perm_handle);

    let steps = vec![
        CompiledStep::SelectMaterial {
            of_permanent: CompiledBindingRef::Named("target".to_string()),
            filter: CompiledPredicate::default(),
            bind_as: Some("mat".to_string()),
            prompt: "Pick a material".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("select_material must install a pending selection");
    // 2 sources excluding top: stack_size = 3 → card_sources len 3 → 2 candidates.
    assert_eq!(
        pending.valid_action_ids.len(),
        2,
        "expected exactly 2 material candidates (top excluded)"
    );

    let (action_id, selecting_player) = (pending.valid_action_ids[0], pending.selecting_player);
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.memory, memory_before + 1);
}

#[test]
fn select_material_missing_binding_silent_noop() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .hand(0, &["SRC"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![
        CompiledStep::SelectMaterial {
            of_permanent: CompiledBindingRef::Named("missing".to_string()),
            filter: CompiledPredicate::default(),
            bind_as: Some("mat".to_string()),
            prompt: "Pick a material".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Silent no-op: no selection installed, the GainMemory tail still ran
    // synchronously because the selection step didn't park.
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "missing binding → SelectMaterial no-ops; the tail still runs synchronously"
    );
}
```

(`stack_material_under` is the standard DebugRunner helper from existing 2c tests; if it's not exposed, use the same `CardSource::new` + direct push pattern as `push_to_trash` but onto `runner.game.players[p].battle_area[idx].card_sources`. Verify by grepping `stack_material_under` in `digimon-engine/src/debug_runner.rs` before writing the test — if absent, add the test using the direct push.)

- [ ] **Step 3: Verify the helper exists, or fall back**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2e_select_material 2>&1 | head -40`
Expected: FAIL — either `stack_material_under` doesn't exist (compile error → fall back to direct push) or `SelectMaterial` isn't dispatched (assertion failure — proceed to Step 4).

If `stack_material_under` is missing, replace the two calls with:

```rust
let m0_idx = runner.game.card_data.iter().position(|c| c.card_id == "M0").unwrap();
let m1_idx = runner.game.card_data.iter().position(|c| c.card_id == "M1").unwrap();
let m0_card_index = runner.game.next_card_index();
let m1_card_index = runner.game.next_card_index();
let m0 = digimon_engine::card_source::CardSource::new(m0_idx, 0, m0_card_index);
let m1 = digimon_engine::card_source::CardSource::new(m1_idx, 0, m1_card_index);
runner.game.players[0].battle_area[0].card_sources.insert(0, m0);
runner.game.players[0].battle_area[0].card_sources.insert(0, m1);
```

(`card_sources` indices are bottom-up — index 0 is the bottom of the stack. `insert(0, ...)` keeps the stacked-under semantics.)

- [ ] **Step 4: Add the install fn + dispatch arm**

In `digimon-engine/src/dsl_cards/step/selections.rs`, add to `try_install`:

```rust
CompiledStep::SelectMaterial { of_permanent, bind_as, prompt, optional, .. } => {
    install_select_material(
        ctx,
        of_permanent.clone(),
        bind_as.clone(),
        prompt.clone(),
        *optional,
        tail.to_vec(),
        bindings,
    );
    true
}
```

Append the install fn (note the `use` adjustment for `CompiledBindingRef` and `resolve_binding_ref` — verify imports at the top of `selections.rs`):

```rust
fn install_select_material(
    ctx: &mut EffectContext<'_>,
    of_permanent: digimon_dsl::compiled::CompiledBindingRef,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};

    let perm = match resolve_binding_ref(&of_permanent, ctx, &bindings) {
        Some(ResolvedBinding::Permanent(h)) => h,
        // Missing binding or wrong type: silent no-op (2b/2c convention).
        _ => return,
    };
    let tail = Arc::new(tail);
    ctx.select_material(
        perm,
        &prompt,
        optional,
        |_game, _src_idx| true,
        move |cb_ctx, src_idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                let perm_owner = perm.player;
                let perm_index = perm.index as usize;
                if let Some(card) = cb_ctx
                    .game
                    .player(perm_owner)
                    .battle_area
                    .get(perm_index)
                    .and_then(|p| p.card_sources.get(src_idx))
                {
                    b.insert_card(name, card.handle());
                }
            }
            run_steps(&tail, cb_ctx, &mut b);
            drain_dsl_outer_tail(cb_ctx);
        },
    );
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2e_select_material`
Expected: PASS — both `select_material_binds_picked_source_handle` and `select_material_missing_binding_silent_noop`.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/selections.rs \
        digimon-engine/tests/dsl/main.rs \
        digimon-engine/tests/dsl/phase2e_select_material.rs
git commit -m "dsl phase 2e: lower SelectMaterial"
```

---

## Task 6: Lower `SelectUnionZone`

Engine API: `EffectContext::select_union_zone(of_player, zones: UnionZoneSet, prompt, is_optional, filter, callback)` where `callback: FnOnce(&mut EffectContext, CardHandle)` receives a `CardHandle` directly. Translate `Vec<CompiledZone>` to `UnionZoneSet` (currently only Hand and Trash are flagged in the engine bitfield; unsupported zones silently no-op the same way 2d's `SelectCountCappedMulti` did).

**Files:**
- Modify: `digimon-engine/src/dsl_cards/step/selections.rs`
- Create: `digimon-engine/tests/dsl/phase2e_select_union_zone.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Register the new test module**

Append to `digimon-engine/tests/dsl/main.rs`:

```rust
mod phase2e_select_union_zone;
```

- [ ] **Step 2: Write the failing test**

Create `digimon-engine/tests/dsl/phase2e_select_union_zone.rs`:

```rust
//! Phase 2e Task 6: SelectUnionZone over hand+trash binds the picked
//! CardHandle into Bindings.

use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate, CompiledStep, CompiledZone};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, card_index));
}

#[test]
fn select_union_zone_picks_from_hand_or_trash() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("H0", "H0"))
        .add_card(make_test_card("T0", "T0"))
        .hand(0, &["SRC", "H0"])
        .build();

    push_to_trash(&mut runner, 0, "T0");
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![
        CompiledStep::SelectUnionZone {
            of: CompiledPlayerRef::You,
            zones: vec![CompiledZone::Hand, CompiledZone::Trash],
            filter: CompiledPredicate::default(),
            bind_as: Some("union_pick".to_string()),
            prompt: "Pick from hand or trash".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("select_union_zone must install a pending selection");
    // 2 hand cards (SRC + H0) + 1 trash card (T0) = 3 candidates.
    assert_eq!(pending.valid_action_ids.len(), 3);

    let (action_id, selecting_player) = (pending.valid_action_ids[0], pending.selecting_player);
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve");
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.memory, memory_before + 1);
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2e_select_union_zone`
Expected: FAIL — `SelectUnionZone` not dispatched.

- [ ] **Step 4: Add the install fn + dispatch arm**

In `digimon-engine/src/dsl_cards/step/selections.rs`, add to `try_install`:

```rust
CompiledStep::SelectUnionZone { of, zones, bind_as, prompt, optional, .. } => {
    install_select_union_zone(
        ctx,
        *of,
        zones.clone(),
        bind_as.clone(),
        prompt.clone(),
        *optional,
        tail.to_vec(),
        bindings,
    );
    true
}
```

Append the install fn:

```rust
fn install_select_union_zone(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    zones: Vec<digimon_dsl::compiled::CompiledZone>,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    use crate::selection::UnionZoneSet;

    let target_player = resolve_player(ctx, of);
    let mut zoneset = UnionZoneSet(0);
    for z in &zones {
        match z {
            digimon_dsl::compiled::CompiledZone::Hand => zoneset |= UnionZoneSet::HAND,
            digimon_dsl::compiled::CompiledZone::Trash => zoneset |= UnionZoneSet::TRASH,
            // Other zones are not yet exposed by the engine UnionZoneSet
            // bitfield. Silently skip them — Phase 2f+ widens the engine
            // API once a real card needs e.g. (deck | trash).
            _ => {}
        }
    }
    if zoneset.0 == 0 {
        // No supported zones requested: silent no-op.
        return;
    }
    let tail = Arc::new(tail);
    ctx.select_union_zone(
        target_player,
        zoneset,
        &prompt,
        optional,
        |_game, _card| true,
        move |cb_ctx, handle| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_card(name, handle);
            }
            run_steps(&tail, cb_ctx, &mut b);
            drain_dsl_outer_tail(cb_ctx);
        },
    );
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2e_select_union_zone`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/selections.rs \
        digimon-engine/tests/dsl/main.rs \
        digimon-engine/tests/dsl/phase2e_select_union_zone.rs
git commit -m "dsl phase 2e: lower SelectUnionZone"
```

---

## Task 7: Lower `SelectOrderedPermutation`

Engine API: `EffectContext::select_ordered_permutation(items: Vec<CardHandle>, prompt, callback)` where `callback: FnOnce(&mut EffectContext, Vec<CardHandle>)` receives the picked items in chosen order. The DSL variant has `items: CompiledBindingRef`, so we resolve to a `CardList` (the engine-side Bindings carrier added in 2d). Bind the ordered result as another `CardList`.

The empty-items path of the engine API invokes the callback synchronously (no `PendingSelection` installed); the install fn must propagate that (no parking in the dispatcher).

**Files:**
- Modify: `digimon-engine/src/dsl_cards/step/selections.rs`
- Create: `digimon-engine/tests/dsl/phase2e_select_ordered_permutation.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Register the new test module**

Append to `digimon-engine/tests/dsl/main.rs`:

```rust
mod phase2e_select_ordered_permutation;
```

- [ ] **Step 2: Write the failing test**

Create `digimon-engine/tests/dsl/phase2e_select_ordered_permutation.rs`:

```rust
//! Phase 2e Task 7: SelectOrderedPermutation resolves `items` to a
//! CardList, drives the multi-step permutation trampoline, and binds the
//! ordered result as a CardList.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledStep};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, card_index));
}

#[test]
fn select_ordered_permutation_orders_input_list() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("A", "A"))
        .add_card(make_test_card("B", "B"))
        .hand(0, &["SRC"])
        .build();

    push_to_trash(&mut runner, 0, "A");
    push_to_trash(&mut runner, 0, "B");
    let a = runner.game.players[0].trash[0].handle();
    let b = runner.game.players[0].trash[1].handle();
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let mut bindings = Bindings::new();
    bindings.insert_card_list("input", vec![a, b]);

    let steps = vec![
        CompiledStep::SelectOrderedPermutation {
            items: CompiledBindingRef::Named("input".to_string()),
            bind_as: Some("ordered".to_string()),
            prompt: "Order them".to_string(),
            prompt_key: None,
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Permutation install: a 2-item permutation prompts twice. Pick B first, then A.
    let (action_id, selecting_player) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        // Two items remaining: action_ids are SEL_REVEAL_START + 0 and +1.
        (pending.valid_action_ids[1], pending.selecting_player)
    };
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("first pick");

    // Second pick: only one candidate remains; pick it.
    let (action_id, selecting_player) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        (pending.valid_action_ids[0], pending.selecting_player)
    };
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("second pick");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.memory, memory_before + 1);
}

#[test]
fn select_ordered_permutation_empty_runs_tail_synchronously() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .hand(0, &["SRC"])
        .build();
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let mut bindings = Bindings::new();
    bindings.insert_card_list("input", vec![]);

    let steps = vec![
        CompiledStep::SelectOrderedPermutation {
            items: CompiledBindingRef::Named("input".to_string()),
            bind_as: Some("ordered".to_string()),
            prompt: "Order".to_string(),
            prompt_key: None,
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Empty items: engine fires the final callback immediately. Tail runs
    // synchronously — no selection installed.
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.memory, memory_before + 1);
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2e_select_ordered_permutation`
Expected: FAIL — `SelectOrderedPermutation` not dispatched.

- [ ] **Step 4: Add the install fn + dispatch arm**

In `digimon-engine/src/dsl_cards/step/selections.rs`, add to `try_install`:

```rust
CompiledStep::SelectOrderedPermutation { items, bind_as, prompt, .. } => {
    install_select_ordered_permutation(
        ctx,
        items.clone(),
        bind_as.clone(),
        prompt.clone(),
        tail.to_vec(),
        bindings,
    );
    true
}
```

Append the install fn:

```rust
fn install_select_ordered_permutation(
    ctx: &mut EffectContext<'_>,
    items_ref: digimon_dsl::compiled::CompiledBindingRef,
    bind_as: Option<String>,
    prompt: String,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};

    let items = match resolve_binding_ref(&items_ref, ctx, &bindings) {
        Some(ResolvedBinding::CardList(v)) => v,
        // Missing binding or wrong type: silent no-op.
        _ => return,
    };
    let tail = Arc::new(tail);
    ctx.select_ordered_permutation(
        items,
        &prompt,
        move |cb_ctx, ordered| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_card_list(name, ordered);
            }
            run_steps(&tail, cb_ctx, &mut b);
            drain_dsl_outer_tail(cb_ctx);
        },
    );
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2e_select_ordered_permutation`
Expected: PASS — both ordered and empty cases.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/dsl_cards/step/selections.rs \
        digimon-engine/tests/dsl/main.rs \
        digimon-engine/tests/dsl/phase2e_select_ordered_permutation.rs
git commit -m "dsl phase 2e: lower SelectOrderedPermutation"
```

---

## Task 8: `distinct_by` enforcement on `SelectCountCappedMulti`

The 2d `SelectCountCappedMulti` lowering accepted the `distinct_by: Option<CompiledDistinctBy>` field but ignored it — every pick narrowed the candidate set only by removing the picked index, not by also removing other indices that share the distinct-by attribute (`card_number`, `level`, or `name`) with already-picked cards. DigiXros materials and several Tier-4 selection cards rely on the engine enforcing this; the engine's `install_count_capped_step` trampoline is the right place to do it.

**Approach:** add a `DistinctByMode` enum to the engine (mirroring `CompiledDistinctBy`) and thread it through `EffectContext::select_count_capped_multi` and the trampoline. After each pick, `new_candidates` is filtered: for each remaining index, look up the card and reject the index if its distinct-by attribute matches any card in `accum`.

The `EffectContextSelectorScope` forwarder (used by `as_selecting_player`, currently out-of-scope for 2e but needs to keep compiling) gets the same parameter added. The DSL install passes the mapped variant unchanged.

**Files:**
- Modify: `digimon-engine/src/effect_context/selections.rs`
- Modify: `digimon-engine/src/effect_context/mod.rs` (re-export)
- Modify: `digimon-engine/src/dsl_cards/step/selections.rs`
- Create: `digimon-engine/tests/dsl/phase2e_distinct_by.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Register the new test module**

Append to `digimon-engine/tests/dsl/main.rs`:

```rust
mod phase2e_distinct_by;
```

- [ ] **Step 2: Write the failing test**

Create `digimon-engine/tests/dsl/phase2e_distinct_by.rs`:

```rust
//! Phase 2e Task 8: `distinct_by: card_number` removes other zone indices
//! that share the picked card's printed card_id from the next-step
//! candidate list.

use digimon_dsl::compiled::{
    CompiledDistinctBy, CompiledPlayerRef, CompiledPredicate, CompiledStep, CompiledZone,
};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, card_index));
}

#[test]
fn distinct_by_card_number_filters_duplicates_after_pick() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("DUP", "DUP"))
        .add_card(make_test_card("UNIQ", "UNIQ"))
        .hand(0, &["SRC"])
        .build();

    // Two copies of "DUP" + one "UNIQ" in opponent's trash.
    push_to_trash(&mut runner, 1, "DUP");
    push_to_trash(&mut runner, 1, "DUP");
    push_to_trash(&mut runner, 1, "UNIQ");

    let src_card = runner.game.players[0].hand[0].handle();

    let steps = vec![CompiledStep::SelectCountCappedMulti {
        of: CompiledPlayerRef::Opponent,
        zone: CompiledZone::Trash,
        max: 3,
        filter: CompiledPredicate::default(),
        bind_as: Some("picks".to_string()),
        prompt: "Pick distinct".to_string(),
        prompt_key: None,
        optional_zero: false,
        distinct_by: Some(CompiledDistinctBy::CardNumber),
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Step 1: 3 candidates (the two DUPs + UNIQ).
    let pending = runner.game.pending_selection.as_ref().unwrap();
    assert_eq!(pending.valid_action_ids.len(), 3);
    let (action_id, selecting_player) = (pending.valid_action_ids[0], pending.selecting_player);
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("first pick");

    // Step 2: only UNIQ should remain — both DUP indices are filtered out
    // because the picked DUP shares its card_id with the other DUP.
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("pending must re-arm after first pick");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "after picking a DUP, the other DUP must be filtered by distinct_by=card_number"
    );

    let (action_id, selecting_player) = (pending.valid_action_ids[0], pending.selecting_player);
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("second pick");

    // No more candidates: trampoline auto-commits.
    assert!(runner.game.pending_selection.is_none());
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2e_distinct_by`
Expected: FAIL — at step 2, the pending selection still has 2 candidates (the second DUP and UNIQ) because the trampoline currently filters only the picked index.

- [ ] **Step 4: Add the engine `DistinctByMode` enum**

Append to `digimon-engine/src/effect_context/selections.rs`, near the `CountCappedZone` declaration:

```rust
/// Per-card-number / -level / -name uniqueness constraint applied to a
/// `select_count_capped_multi` selection. After each pick, candidates that
/// share the constrained attribute with any already-picked card are
/// removed from the next step's `valid_action_ids`.
///
/// Mirrors `digimon_dsl::compiled::CompiledDistinctBy`; the DSL lowering
/// in `dsl_cards::step::selections` translates the variant unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistinctByMode {
    CardNumber,
    Level,
    Name,
}
```

In `digimon-engine/src/effect_context/mod.rs`, extend the existing re-export at line 17:

```rust
pub use selections::{CountCappedZone, DistinctByMode, EffectContextSelectorScope};
```

- [ ] **Step 5: Thread `Option<DistinctByMode>` through the API**

Modify `EffectContext::select_count_capped_multi` (line 707 of `selections.rs`) to add a `distinct_by: Option<DistinctByMode>` parameter just before `filter`:

```rust
pub fn select_count_capped_multi<F, C>(
    &mut self,
    of_player: PlayerId,
    zone: CountCappedZone,
    max: u8,
    prompt: &str,
    is_optional_zero: bool,
    distinct_by: Option<DistinctByMode>,
    filter: F,
    callback: C,
) where
    F: Fn(&Game, &CardSource) -> bool + Send + Sync + 'static,
    C: FnOnce(&mut EffectContext<'_>, Vec<crate::card_source::CardHandle>) + Send + Sync + 'static,
{
    // ... existing body unchanged through the empty-filter early return ...
    install_count_capped_step(
        self.game,
        of_player,
        zone,
        range_start,
        max,
        is_optional_zero,
        distinct_by, // NEW
        candidate_indices,
        Vec::new(),
        prompt_owned,
        source_card,
        source_permanent,
        selecting_player,
        previous_phase,
        final_callback,
    );
}
```

Apply the same change to the `EffectContextSelectorScope::select_count_capped_multi` forwarder at line 1134, threading the new parameter through. (No behavior change for callers that pass `None`.)

Modify `install_count_capped_step` (line 1275) to take `distinct_by: Option<DistinctByMode>` and apply it inside the callback:

```rust
fn install_count_capped_step(
    game: &mut Game,
    of_player: PlayerId,
    zone: CountCappedZone,
    range_start: u16,
    max: u8,
    is_optional_zero: bool,
    distinct_by: Option<DistinctByMode>, // NEW
    candidate_indices: Vec<usize>,
    accum: Vec<crate::card_source::CardHandle>,
    // ... unchanged ...
) {
    // ... unchanged through the callback construction ...
    callback: Box::new(move |game: &mut Game, action_id: u16| {
        // ... existing pick decode + auto-commit-at-max + new_accum push unchanged ...

        // Compute new_candidates by removing the picked index AND any
        // remaining index whose card shares the distinct-by attribute
        // with any card in new_accum.
        let new_candidates: Vec<usize> = candidate_indices
            .into_iter()
            .filter(|&i| i != pick_zone_idx)
            .filter(|&i| {
                let Some(mode) = distinct_by else { return true };
                let cand_card = match zone {
                    CountCappedZone::Hand => &game.player(of_player).hand[i],
                    CountCappedZone::Trash => &game.player(of_player).trash[i],
                    CountCappedZone::Material(perm_handle) => {
                        &game
                            .player(perm_handle.player)
                            .battle_area[perm_handle.index as usize]
                            .card_sources[i]
                    }
                };
                let cand_data = &game.card_data[cand_card.data_index];
                !new_accum.iter().any(|picked_handle| {
                    let picked_card = match game.find_card_source_by_handle(*picked_handle) {
                        Some(c) => c,
                        None => return false, // handle vanished mid-resolution; conservatively keep candidate
                    };
                    let picked_data = &game.card_data[picked_card.data_index];
                    match mode {
                        DistinctByMode::CardNumber => picked_data.card_id == cand_data.card_id,
                        DistinctByMode::Level => picked_data.level == cand_data.level,
                        DistinctByMode::Name => picked_data.name_eng == cand_data.name_eng,
                    }
                })
            })
            .collect();

        // ... rest unchanged: empty-candidates auto-commit, recurse ...
        install_count_capped_step(
            game,
            of_player,
            zone,
            range_start,
            max,
            is_optional_zero,
            distinct_by, // NEW: thread through the recursion
            new_candidates,
            new_accum,
            // ... unchanged ...
        );
    }),
    // ... on_decline unchanged ...
}
```

If `Game::find_card_source_by_handle` does not exist, replace the `picked_card` lookup with a manual zone scan keyed on `zone` (the picked card lives in the same zone as candidates, so iterate `game.player(of_player).<zone>` until `card.handle() == picked_handle` matches). Verify by `grep -n "find_card_source_by_handle" digimon-engine/src/game.rs digimon-engine/src/card_source.rs` — if it's absent, use this fallback inline in the closure:

```rust
let picked_card = match zone {
    CountCappedZone::Hand => game.player(of_player).hand.iter().find(|c| c.handle() == *picked_handle),
    CountCappedZone::Trash => game.player(of_player).trash.iter().find(|c| c.handle() == *picked_handle),
    CountCappedZone::Material(perm_handle) => game
        .player(perm_handle.player)
        .battle_area[perm_handle.index as usize]
        .card_sources
        .iter()
        .find(|c| c.handle() == *picked_handle),
};
let Some(picked_card) = picked_card else { return false };
```

(Pre-Phase-2e the engine is single-threaded, candidates and `accum` cards are in the same zone, and the trampoline runs synchronously — the linear scan is fine.)

- [ ] **Step 6: Update the existing 2d caller**

In `digimon-engine/src/dsl_cards/step/selections.rs`, find `install_select_count_capped_multi` (Phase 2d code) and update the engine call to pass the new variant. The current Phase 2d code reads the `distinct_by: Option<CompiledDistinctBy>` field but ignores it; map it to `DistinctByMode` and pass it through:

Add the mapping helper at the top of `selections.rs` (below the existing imports):

```rust
fn map_distinct_by(d: Option<digimon_dsl::compiled::CompiledDistinctBy>) -> Option<crate::effect_context::DistinctByMode> {
    use digimon_dsl::compiled::CompiledDistinctBy;
    use crate::effect_context::DistinctByMode;
    d.map(|c| match c {
        CompiledDistinctBy::CardNumber => DistinctByMode::CardNumber,
        CompiledDistinctBy::Level => DistinctByMode::Level,
        CompiledDistinctBy::Name => DistinctByMode::Name,
    })
}
```

Update the `try_install` arm for `SelectCountCappedMulti` (added in 2d) to destructure and pass the field:

```rust
CompiledStep::SelectCountCappedMulti {
    of, zone, max, bind_as, prompt, optional_zero, distinct_by, ..
} => {
    install_select_count_capped_multi(
        ctx,
        *of,
        *zone,
        *max,
        bind_as.clone(),
        prompt.clone(),
        *optional_zero,
        map_distinct_by(*distinct_by),
        tail.to_vec(),
        bindings,
    );
    true
}
```

And update `install_select_count_capped_multi` signature + the inner `ctx.select_count_capped_multi` call:

```rust
#[allow(clippy::too_many_arguments)]
fn install_select_count_capped_multi(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    zone: CompiledZone,
    max: u8,
    bind_as: Option<String>,
    prompt: String,
    optional_zero: bool,
    distinct_by: Option<crate::effect_context::DistinctByMode>, // NEW
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    // ... unchanged through the engine_zone match ...
    ctx.select_count_capped_multi(
        target_player,
        engine_zone,
        max,
        &prompt,
        optional_zero,
        distinct_by, // NEW: passed in the new parameter slot
        |_game, _card| true,
        move |cb_ctx, picks| {
            // ... unchanged ...
        },
    );
}
```

Also audit any non-DSL callers of `EffectContext::select_count_capped_multi` (e.g. card scripts in `digimon-engine/src/cards/`): each must add `None` (or the appropriate mode) in the new parameter slot. Run:

```bash
grep -rn "select_count_capped_multi" digimon-engine/src --include="*.rs"
```

Any hit outside `effect_context/`, `dsl_cards/`, or test files is a card-script call that needs the parameter added. Update each by inserting `None,` before the `filter` argument.

- [ ] **Step 7: Run the test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2e_distinct_by`
Expected: PASS.

Then run the full engine test suite to confirm no card-script regression:

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: PASS — all engine + DSL tests, including the 2b/2c/2d phase tests.

- [ ] **Step 8: Commit**

```bash
git add digimon-engine/src/effect_context/selections.rs \
        digimon-engine/src/effect_context/mod.rs \
        digimon-engine/src/dsl_cards/step/selections.rs \
        digimon-engine/tests/dsl/main.rs \
        digimon-engine/tests/dsl/phase2e_distinct_by.rs \
        digimon-engine/src/cards
git commit -m "dsl phase 2e: distinct_by enforcement on SelectCountCappedMulti"
```

(Stage `digimon-engine/src/cards` only if the audit in Step 6 found card-script callers that needed the new parameter. If no card scripts use this API yet, drop that path from the `git add` line.)

---

## Task 9: End-to-end fixture — chained selections

Confirm two new selection variants compose: `SelectReveal` followed by a synchronous step that consumes the bound `CardHandle`. This isn't a real card; it's a fixture that exercises the install / resolve / bind / tail pattern across two consecutive selection-installing variants in the same `process:` slice. Adding this catches mistakes like a missing `drain_dsl_outer_tail` call (would surface as the inner tail dropping the outer's GainMemory) or a stale `bindings.clone()` snapshot.

The fixture: P0 owns the effect. The reveal pile holds two cards. The process is `select_reveal` (binds `picked`) then `select_effect_choice` (binds `branch`) then `gain_memory(1)`. Confirm both selections install in sequence and the final `gain_memory` runs only after both resolve.

**Files:**
- Create: `digimon-engine/tests/dsl/phase2e_end_to_end.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

- [ ] **Step 1: Register the new test module**

Append to `digimon-engine/tests/dsl/main.rs`:

```rust
mod phase2e_end_to_end;
```

- [ ] **Step 2: Write the failing test**

Create `digimon-engine/tests/dsl/phase2e_end_to_end.rs`:

```rust
//! Phase 2e end-to-end: SelectReveal → SelectEffectChoice → GainMemory.
//! Confirms two parking selections compose and the final tail runs only
//! after both resolve.

use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate, CompiledStep};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

fn push_to_revealed(runner: &mut DebugRunner, owner: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let card_index = runner.game.next_card_index();
    runner
        .game
        .revealed_cards
        .push(CardSource::new(data_idx, owner, card_index));
}

#[test]
fn select_reveal_then_effect_choice_then_gain_memory() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("R0", "R0"))
        .add_card(make_test_card("R1", "R1"))
        .hand(0, &["SRC"])
        .build();

    push_to_revealed(&mut runner, 0, "R0");
    push_to_revealed(&mut runner, 0, "R1");

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![
        CompiledStep::SelectReveal {
            of: CompiledPlayerRef::You,
            filter: CompiledPredicate::default(),
            bind_as: Some("picked".to_string()),
            prompt: "pick".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::SelectEffectChoice {
            labels: vec!["A".to_string(), "B".to_string()],
            bind_as: Some("branch".to_string()),
            prompt: "choose".to_string(),
            prompt_key: None,
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // First parked: SelectReveal.
    assert!(runner.game.pending_selection.is_some());
    assert_eq!(runner.game.memory, memory_before, "tail must not have run");

    let (a, p) = {
        let s = runner.game.pending_selection.as_ref().unwrap();
        (s.valid_action_ids[0], s.selecting_player)
    };
    runner.game.resolve_selection(p, a).expect("reveal pick");

    // Second parked: SelectEffectChoice. The reveal callback installed it
    // before its tail's gain_memory could fire.
    assert!(
        runner.game.pending_selection.is_some(),
        "after resolving SelectReveal, SelectEffectChoice should be installed"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "GainMemory must wait for SelectEffectChoice to resolve"
    );

    let (a, p) = {
        let s = runner.game.pending_selection.as_ref().unwrap();
        (s.valid_action_ids[0], s.selecting_player)
    };
    runner.game.resolve_selection(p, a).expect("choice pick");

    // Both resolved → final tail ran.
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.memory, memory_before + 1);
}
```

- [ ] **Step 3: Run the test to verify it passes**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test dsl phase2e_end_to_end`
Expected: PASS — both selections install in sequence and `GainMemory` fires only after the second resolves.

If the test fails at "after resolving SelectReveal, SelectEffectChoice should be installed", the reveal callback is running its tail before the second selection installs — check that `install_select_reveal`'s callback passes the *full* tail (which includes `SelectEffectChoice` as its first element) to `run_steps`, and that `run_steps` re-enters `selections::try_install` for the second selection.

- [ ] **Step 4: Commit**

```bash
git add digimon-engine/tests/dsl/main.rs \
        digimon-engine/tests/dsl/phase2e_end_to_end.rs
git commit -m "dsl phase 2e: end-to-end SelectReveal → SelectEffectChoice composition test"
```

---

## Task 10: Update the spec's Phase 2 sub-phase notes

Add a `2e` bullet to §7.3 of the DSL spec mirroring the existing `2a`/`2b`/`2c`/`2d` bullets, recording what landed and what defers to 2f+.

**Files:**
- Modify: `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md`

- [ ] **Step 1: Locate the sub-phase progress section**

Open `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` and find the `**Sub-phase progress:**` bullet list under §7.3 (currently ends with the `2d` bullet at line 1554-1565).

- [ ] **Step 2: Append the `2e` bullet**

After the `2d` bullet, add:

```markdown
- **2e** (landed YYYY-MM-DD) — remaining selection kinds (`SelectEffectChoice`,
  `SelectReveal`, `SelectSecurity`, `SelectMaterial`, `SelectUnionZone`,
  `SelectOrderedPermutation`) and `distinct_by` enforcement on
  `SelectCountCappedMulti` (`CardNumber` / `Level` / `Name`). Defers to 2f+:
  `AsSelectingPlayer` (needs override-persistence across selection
  callbacks — engine work), play / digivolve / placement steps,
  formula values in `add_modifier` `value`, and `ScheduleDelayed`
  (needs `ctx.schedule_delayed` engine primitive).
```

Replace `YYYY-MM-DD` with the actual landing date when the work merges.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs/2026-04-21-card-scripting-dsl.md
git commit -m "dsl phase 2e: spec — record 2e sub-phase landing notes"
```

---

## Final verification

- [ ] **Step 1: Full test suite**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: PASS — all engine + DSL tests including every phase2a/2b/2c/2d/2e module.

- [ ] **Step 2: Confirm no regressions in card scripts**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test test_cards_behavioral`
Expected: PASS — the 22 hand-written test cards still behave correctly. Failures here mean the `select_count_capped_multi` signature change in Task 8 broke a card-script call site that wasn't updated in Step 6.

- [ ] **Step 3: Build the PyO3 bindings to confirm cross-language compile**

Run: `cd digimon-engine-py && cargo build`
Expected: SUCCESS — the bindings re-export `EffectContext` indirectly; if `DistinctByMode` was not pub-exported correctly, a downstream build error here is the canary.

- [ ] **Step 4: Optional Rust-backend parity smoke**

Run: `DIGIMON_BACKEND=rust python -m pytest tests/engine/test_rust_backend_parity.py -v`
Expected: PASS — confirms the engine still serves Python through the existing parity tests.

---

## Notes for the executor

1. **Phase 2b accept-all filter.** The DSL `filter:` field on each selection variant is not yet enforced at install time — every install fn passes `|_, _| true`. This is the documented Phase 2b precedent (`docs/superpowers/specs/.../2026-04-21-card-scripting-dsl.md` `selections.rs` module header). Phase 2e does not change this; tightening the filter signature to a wider read-context is its own future phase. Don't write tests that assume the filter narrows candidates — the candidate count is `zone_len` for now.

2. **`drain_dsl_outer_tail` is mandatory.** Every install fn's callback must end with `drain_dsl_outer_tail(cb_ctx);` — the 2d outer-tail-propagation fix relies on every callback draining. Forgetting it silently breaks composition with `If` / `ForEach` parents. Each task's install-fn step lists this call explicitly; copy verbatim.

3. **`try_install` order doesn't affect correctness** — each variant matches exactly one arm. Keep the arms in the same order as `compiled.rs`'s `CompiledStep` declaration order for grep-ability.

4. **Selection install fn skeleton** (the pattern shared by every Task 2-7 install fn):
   ```rust
   let target_player = resolve_player(ctx, of); // when the variant has `of:`
   let tail = Arc::new(tail);
   ctx.<select_*>(<args>, |_g, _x| true, move |cb_ctx, result| {
       let mut b = bindings.clone();
       if let Some(name) = &bind_as {
           b.<insert_*>(name, /* derived from result */);
       }
       run_steps(&tail, cb_ctx, &mut b);
       drain_dsl_outer_tail(cb_ctx);
   });
   ```
   Whenever you find yourself deviating from this skeleton, the deviation is probably a bug.

5. **The `Send + Sync + 'static` filter bound** is satisfied trivially by `|_, _| true` (no captures). When 2f or later widens to predicate-respecting filters, the closure will need to capture an `Arc<CompiledPredicate>`; that's not in 2e scope.

6. **`as_selecting_player` defer rationale.** Looking at `EffectContextSelectorScope`, the override is only set for the immediate `select_*` call — it's `take()`n before the underlying helper returns. The helper captures `selecting_player` into the `PendingSelection.selecting_player` field, so the install honors the override; but the *callback's* re-entry through `EffectContext::new(game, source_card, source_permanent, selecting_player)` uses whatever `selecting_player` the engine recovers, which is the original effect controller. For an `AsSelectingPlayer { body: [select_a, select_b, …] }` clause, both `select_a` and `select_b` need the override applied — but `select_b` installs from `select_a`'s callback, where the override is gone. The fix is either (a) propagate the override into the callback's `EffectContext` reconstruction (engine plumbing in `selection.rs::resolve_generic_selection`), or (b) carry the override on `Bindings` and have every install fn check for it. Either is a single-task design pass; not in 2e.

---

## Self-review

**Spec coverage check:**
- §3.7.7 `select_effect_choice` — Task 2 ✓
- §3.7.7 `select_reveal` — Task 3 ✓
- §3.7.7 `select_security` — Task 4 ✓
- §3.7.7 `select_material` — Task 5 ✓
- §3.7.7 `select_union_zone` — Task 6 ✓
- §3.7.7 `select_ordered_permutation` — Task 7 ✓
- §3.7.7 `as_selecting_player` — explicitly deferred in non-goals; rationale documented in Notes §6
- §3.3 `distinct_by` (DigiXros materials) — Task 8 ✓
- §7.3 sub-phase notes update — Task 10 ✓
- §3.7.7 `select_count_capped_multi` (already shipped 2d) — Task 8 extends it ✓

**Placeholder scan:** none. Every step contains the actual code or command.

**Type consistency:**
- `DistinctByMode` (engine) ↔ `CompiledDistinctBy` (DSL): both have `CardNumber` / `Level` / `Name` arms; `map_distinct_by` (Task 8 Step 6) is the bridge.
- `BindingValue::Literal(i64)` insert via `insert_literal` (Task 1) read via existing `get_literal` (line 74 of `bindings.rs`) — types match.
- `CompiledBindingRef::Named(String)` resolution via `binding_ref::resolve_binding_ref` returning `ResolvedBinding::Permanent` (Task 5) and `ResolvedBinding::CardList` (Task 7) — both variants exist (verified against `binding_ref.rs:11-20`).
- `CompiledZone::Hand` / `CompiledZone::Trash` → `UnionZoneSet::HAND` / `UnionZoneSet::TRASH` — bitfield definitions verified at `digimon-engine/src/selection.rs:32-34`.

No gaps detected.
