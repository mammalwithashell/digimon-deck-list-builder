# Card Scripting DSL — Phase 2b Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Lower the first wave of selection steps and their binding-consuming zone-move successors, unblocking ~100 cards whose process bodies follow the pattern "prompt the player to pick X, then move X". Biggest architectural change: `run_step` becomes **continuation-passing** so a selection step can park the rest of the process body as its callback.

**Architecture:** Step dispatch splits into two phases — synchronous (memory/draw/etc., already shipped in 2a) and parking (selection steps). A new public `run_steps(&[CompiledStep], ctx, bindings)` iterates; when it encounters a selection step, it builds a callback from the *remainder* of the slice and installs it via `EffectContext::select_*`. The callback cooperatively clones `Bindings` (HashMap clone — cheap), resumes iteration from the split point, and lets subsequent mutation steps (e.g. `AddToHandFromTrash`) resolve their `CompiledBindingRef` against the now-populated bindings.

**Tech Stack:** Rust 2021, `digimon-engine`, `digimon-dsl` leaf crate.

**Scope (strict):**
- `Bindings`: add `Clone`, add `insert_card/perm/hand_idx/trash_idx` typed setters
- `BindingRef` resolver: `SelfRef`, `Source`, `Carrier`, `Named`, plus `EventTarget` / `EventCard` (no-op for Phase 2b — engine event context not yet wired)
- Continuation-passing step dispatcher (`run_steps`)
- 4 selection steps: `SelectHand`, `SelectTrash`, `SelectOwnPermanent`, `SelectOpponentPermanent`
- Binding-consuming zone moves: `AddToHandFromDeck`, `AddToHandFromTrash`, `AddToHandFromReveal`, `TrashFromReveal`, `ReturnToDeckFromReveal`, `RevealTopDeck`, `PlaceRemainderOnDeck`, `TrashFromHandByIndex`, `MarkSecurityFaceUp`
- End-to-end fixture: synthetic DSL card with `select_trash` → `add_to_hand_from_trash`, played through `DebugRunner`

**Non-goals (Phase 2c+):**
- `SelectReveal`, `SelectSecurity`, `SelectMaterial`, `SelectUnionZone`, `SelectCountCappedMulti`, `SelectOrderedPermutation`, `SelectEffectChoice`, `AsSelectingPlayer`
- Permanent mutations (`DeletePermanent`, `ReturnToHand`, `Suspend`, etc.)
- Control flow (`If`, `ForEach`, `PerSelected`, `Optional`, `ScheduleDelayed`)
- Play/digivolve steps
- Modifier steps (`AddDpModifier`, `AddModifier`, step-level `GrantKeyword`)

---

## File structure

- Modify: `digimon-engine/src/dsl_cards/bindings.rs` (add `Clone` + typed inserters)
- Create: `digimon-engine/src/dsl_cards/binding_ref.rs` (`resolve_binding_ref` helper)
- Modify: `digimon-engine/src/dsl_cards/step/mod.rs` (`run_steps` continuation dispatcher)
- Create: `digimon-engine/src/dsl_cards/step/selections.rs`
- Create: `digimon-engine/src/dsl_cards/step/zone_moves.rs`
- Modify: `digimon-engine/src/dsl_cards/lower_triggered.rs` (call `run_steps` instead of `run_step` loop)
- Create: `digimon-engine/tests/dsl/phase2b_binding_ref.rs`
- Create: `digimon-engine/tests/dsl/phase2b_continuation.rs`
- Create: `digimon-engine/tests/dsl/phase2b_zone_moves.rs`
- Create: `digimon-engine/tests/dsl/phase2b_end_to_end.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`

---

## Task 1: Clone-friendly Bindings + typed inserters

Make the binding environment safe to move into a selection callback.

- [ ] **Step 1: Test** — append to an existing test that clones a populated `Bindings` and asserts the clone still answers lookups.

- [ ] **Step 2: Implement** — derive `Clone` on `Bindings`. Add typed helpers:

```rust
impl Bindings {
    pub fn insert_permanent(&mut self, name: &str, h: PermanentHandle) {
        self.insert(name, BindingValue::Permanent(h));
    }
    pub fn insert_card(&mut self, name: &str, h: CardHandle) {
        self.insert(name, BindingValue::Card(h));
    }
    pub fn insert_hand_index(&mut self, name: &str, i: u16) {
        self.insert(name, BindingValue::HandIndex(i));
    }
    pub fn insert_trash_index(&mut self, name: &str, i: u16) {
        self.insert(name, BindingValue::TrashIndex(i));
    }
}
```

Derive `#[derive(Debug, Default, Clone)]` on `Bindings`.

- [ ] **Step 3: Commit** — `dsl phase 2b: Bindings derives Clone + typed inserters`

---

## Task 2: `resolve_binding_ref`

Translate a `CompiledBindingRef` into a concrete value in the current context.

- [ ] **Step 1: Tests** — `phase2b_binding_ref.rs`:

```rust
use digimon_dsl::compiled::CompiledBindingRef;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use digimon_engine::dsl_cards::bindings::{BindingValue, Bindings};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::permanent::PermanentHandle;

#[test]
fn self_ref_resolves_to_source_card() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build();
    let card = runner.game.players[0].hand[0].handle();
    let ctx = EffectContext::new(&mut runner.game, card, None, 0);
    let b = Bindings::new();
    let r = resolve_binding_ref(&CompiledBindingRef::SelfRef, &ctx, &b);
    assert_eq!(r, Some(ResolvedBinding::Card(card)));
}

#[test]
fn source_ref_resolves_to_source_permanent_when_present() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build();
    runner.place_on_field("F", 0);
    let handle = PermanentHandle { player: 0, index: 0 };
    let card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, card, Some(handle), 0);
    let b = Bindings::new();
    let r = resolve_binding_ref(&CompiledBindingRef::Source, &ctx, &b);
    assert_eq!(r, Some(ResolvedBinding::Permanent(handle)));
}

#[test]
fn named_ref_looks_up_in_bindings_as_permanent_first_then_card() {
    let mut runner = DebugRunner::builder().add_card(make_test_card("F", "F")).hand(0, &["F"]).build();
    let card = runner.game.players[0].hand[0].handle();
    let ctx = EffectContext::new(&mut runner.game, card, None, 0);
    let mut b = Bindings::new();
    let perm = PermanentHandle { player: 0, index: 3 };
    b.insert("tgt", BindingValue::Permanent(perm));
    let r = resolve_binding_ref(&CompiledBindingRef::Named("tgt".into()), &ctx, &b);
    assert_eq!(r, Some(ResolvedBinding::Permanent(perm)));
}

#[test]
fn named_ref_missing_returns_none() {
    let mut runner = DebugRunner::builder().add_card(make_test_card("F", "F")).hand(0, &["F"]).build();
    let card = runner.game.players[0].hand[0].handle();
    let ctx = EffectContext::new(&mut runner.game, card, None, 0);
    let b = Bindings::new();
    let r = resolve_binding_ref(&CompiledBindingRef::Named("missing".into()), &ctx, &b);
    assert!(r.is_none());
}
```

- [ ] **Step 2: Implement** — create `digimon-engine/src/dsl_cards/binding_ref.rs`:

```rust
//! Resolve `CompiledBindingRef` variants against the current effect
//! context + named bindings.

use digimon_dsl::compiled::CompiledBindingRef;

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::{BindingValue, Bindings};
use crate::effect_context::EffectContext;
use crate::permanent::PermanentHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedBinding {
    Permanent(PermanentHandle),
    Card(CardHandle),
    HandIndex(u16),
    TrashIndex(u16),
    Literal(i64),
}

pub fn resolve_binding_ref(
    r: &CompiledBindingRef,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
) -> Option<ResolvedBinding> {
    match r {
        CompiledBindingRef::SelfRef => Some(ResolvedBinding::Card(ctx.source_card)),
        CompiledBindingRef::Source | CompiledBindingRef::Carrier => {
            ctx.source_permanent.map(ResolvedBinding::Permanent)
        }
        CompiledBindingRef::Named(name)
        | CompiledBindingRef::Binding(name)
        | CompiledBindingRef::Permanent(name)
        | CompiledBindingRef::OfPermanent(name) => {
            resolve_named(name, bindings)
        }
        CompiledBindingRef::EventTarget | CompiledBindingRef::EventCard => {
            // Phase 2b: engine event context not yet wired to the DSL layer.
            // Returns None so steps relying on these silently no-op.
            None
        }
    }
}

fn resolve_named(name: &str, bindings: &Bindings) -> Option<ResolvedBinding> {
    match bindings.get(name)? {
        BindingValue::Permanent(h) => Some(ResolvedBinding::Permanent(h)),
        BindingValue::Card(h) => Some(ResolvedBinding::Card(h)),
        BindingValue::HandIndex(i) => Some(ResolvedBinding::HandIndex(i)),
        BindingValue::TrashIndex(i) => Some(ResolvedBinding::TrashIndex(i)),
        BindingValue::Literal(v) => Some(ResolvedBinding::Literal(v)),
    }
}
```

Add `pub mod binding_ref;` to `digimon-engine/src/dsl_cards/mod.rs`. Add `mod phase2b_binding_ref;` to `digimon-engine/tests/dsl/main.rs`.

- [ ] **Step 3: Commit** — `dsl phase 2b: resolve_binding_ref (SelfRef/Source/Named + Event* stubs)`

---

## Task 3: Continuation-passing step dispatcher

- [ ] **Step 1: Test** — `phase2b_continuation.rs`:

```rust
//! Continuation dispatcher: a step slice with no selection steps runs
//! straight through. A step slice with a selection step parks the tail
//! as the callback.

use digimon_dsl::compiled::{CompiledPlayerRef, CompiledStep};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

#[test]
fn run_steps_with_no_selections_executes_all_steps_inline() {
    let mut runner = DebugRunner::builder().add_card(make_test_card("F", "F")).hand(0, &["F"]).build();
    let card = runner.game.players[0].hand[0].handle();
    let before = runner.game.memory;
    {
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        let mut b = Bindings::new();
        run_steps(
            &[
                CompiledStep::GainMemory(1),
                CompiledStep::GainMemory(2),
            ],
            &mut ctx,
            &mut b,
        );
    }
    assert_eq!(runner.game.memory, before + 3);
}
```

(Selection-step continuation test lands in Task 4 once `SelectHand` exists.)

- [ ] **Step 2: Implement** — modify `digimon-engine/src/dsl_cards/step/mod.rs`:

```rust
pub fn run_steps(
    steps: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) {
    let mut i = 0;
    while i < steps.len() {
        let step = &steps[i];
        // Selection steps install the remainder as their callback and return.
        if selections::try_install(step, &steps[i + 1..], ctx, bindings.clone()) {
            return;
        }
        // Synchronous families — execute and advance.
        run_step(step, ctx, bindings);
        i += 1;
    }
}
```

`selections::try_install` — new module, returns `true` if the step was a selection and the remainder was installed. Phase 2b handlers for `SelectHand` / `SelectTrash` / `SelectOwnPermanent` / `SelectOpponentPermanent` live here (Task 4 fills in).

Add `pub mod selections;` to `step/mod.rs`. Create `step/selections.rs` with a stub:

```rust
use digimon_dsl::compiled::CompiledStep;
use crate::dsl_cards::bindings::Bindings;
use crate::effect_context::EffectContext;

pub fn try_install(
    _step: &CompiledStep,
    _tail: &[CompiledStep],
    _ctx: &mut EffectContext<'_>,
    _bindings: Bindings,
) -> bool {
    false // Task 4 adds the first handler.
}
```

Add `mod phase2b_continuation;` to `tests/dsl/main.rs`.

- [ ] **Step 3: Commit** — `dsl phase 2b: run_steps continuation dispatcher (synchronous fast-path)`

---

## Task 4: `SelectHand` + `AddToHandFromDeck` / `AddToHandFromTrash`

Ship the first selection-plus-consumer pair, validating the CPS design.

- [ ] **Step 1: Zone-move handlers** — create `digimon-engine/src/dsl_cards/step/zone_moves.rs`:

```rust
//! Binding-consuming zone-move step lowering.

use digimon_dsl::compiled::{CompiledPlayerRef, CompiledStep};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::resolve_player;
use crate::effect_context::EffectContext;

pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &Bindings,
) -> bool {
    match step {
        CompiledStep::AddToHandFromTrash { of, card } => {
            let Some(ResolvedBinding::TrashIndex(idx)) = resolve_binding_ref(card, ctx, bindings) else { return true; };
            let p = resolve_player(ctx, *of);
            ctx.add_to_hand_from_trash(p, idx as usize);
            true
        }
        CompiledStep::AddToHandFromDeck { of, card } => {
            // Phase 2b: `SelectDeck` selection variant doesn't exist yet, so
            // the only way a binding reaches here is via RevealTopDeck's
            // `bind_as`. Treat as TrashIndex for now (reveal pool reuses
            // the trash-index path in some engine code — adapt as needed).
            let Some(resolved) = resolve_binding_ref(card, ctx, bindings) else { return true; };
            let p = resolve_player(ctx, *of);
            if let ResolvedBinding::HandIndex(i) = resolved {
                let _ = ctx.add_to_hand_from_deck(p, i as usize);
            }
            true
        }
        _ => false,
    }
}
```

**Check `EffectContext::add_to_hand_from_trash` signature:** line 589 of `effect_context/mod.rs`. If the real signature is `(&mut self, player: PlayerId, trash_index: usize)`, the above is correct. Adapt if not.

**Check `EffectContext::add_to_hand_from_deck`:** line 580.

- [ ] **Step 2: SelectHand handler** — extend `step/selections.rs`:

```rust
use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledStep};

use crate::dsl_cards::binding_ref::ResolvedBinding;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::step::{resolve_player, run_steps};
use crate::effect_context::EffectContext;
use crate::permanent::PermanentHandle;

pub fn try_install(
    step: &CompiledStep,
    tail: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: Bindings,
) -> bool {
    match step {
        CompiledStep::SelectHand { of, filter, bind_as, prompt, optional, .. } => {
            install_select_hand(ctx, *of, filter.clone(), bind_as.clone(), prompt.clone(), *optional, tail.to_vec(), bindings);
            true
        }
        CompiledStep::SelectTrash { of, filter, bind_as, prompt, optional, .. } => {
            install_select_trash(ctx, *of, filter.clone(), bind_as.clone(), prompt.clone(), *optional, tail.to_vec(), bindings);
            true
        }
        CompiledStep::SelectOwnPermanent { filter, bind_as, prompt, optional, .. } => {
            install_select_own_perm(ctx, filter.clone(), bind_as.clone(), prompt.clone(), *optional, tail.to_vec(), bindings);
            true
        }
        CompiledStep::SelectOpponentPermanent { filter, bind_as, prompt, optional, .. } => {
            install_select_opp_perm(ctx, filter.clone(), bind_as.clone(), prompt.clone(), *optional, tail.to_vec(), bindings);
            true
        }
        _ => false,
    }
}

fn install_select_hand(
    ctx: &mut EffectContext<'_>,
    of: digimon_dsl::compiled::CompiledPlayerRef,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    let target_player = resolve_player(ctx, of);
    let filter_arc = Arc::new(filter);
    let tail_arc = Arc::new(tail);
    let bind_name = bind_as;

    let filter_at_install = filter_arc.clone();
    ctx.select_hand(
        target_player,
        &prompt,
        optional,
        move |game, idx| {
            // Filter at install time — no EffectContext available here,
            // so build a lightweight predicate check against the hand card.
            // Phase 2b: simplest valid implementation — accept all indices
            // if the filter is default; otherwise evaluate via a throwaway
            // EffectReadContext.
            if *filter_at_install == CompiledPredicate::default() {
                return true;
            }
            // For non-trivial filters we'd build an EffectReadContext; the
            // select_hand filter signature is Fn(&Game, usize) -> bool
            // which makes this awkward. Phase 2c will widen the signature.
            // For 2b: accept-all when filter is non-default too, and count
            // on the engine-side predicate evaluation to be adequate.
            true
        },
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_name {
                b.insert_hand_index(name, idx as u16);
            }
            run_steps(&tail_arc, cb_ctx, &mut b);
        },
    );
}

// Analogous install_select_trash / install_select_own_perm / install_select_opp_perm
// — each follows the same pattern: build callback that clones bindings,
// inserts the chosen value under bind_as, and invokes run_steps(tail).
// See the test file for the exact install shape.
```

**Key design note for implementer:** `EffectContext::select_hand`'s filter closure is `Fn(&Game, usize) -> bool`. Evaluating a `CompiledPredicate` here would need access to the predicate evaluator's `EffectReadContext`, which requires a `source_card` / `player` tuple. Phase 2b punts: accept-all filter (`|_, _| true`). Phase 2c widens the filter with a builder that threads the predicate.

This is a known simplification — document it in the module header and call it out in the commit message.

Write installers for `SelectTrash`, `SelectOwnPermanent`, `SelectOpponentPermanent` with the same CPS shape.

- [ ] **Step 3: Test** — `tests/dsl/phase2b_zone_moves.rs`:

Build a fixture where `SelectHand → AddToHand*` would round-trip, and assert the callback ran and bindings were set.

Key test: **manually install a `SelectHand`-like selection via the DSL, provide a synthetic hand with one card, call `Game::resolve_selection(action_id)` to fire the callback, and assert the continuation executed.**

- [ ] **Step 4: Commit** — `dsl phase 2b: SelectHand/SelectTrash/SelectOwn|OpponentPermanent + AddToHandFromDeck|Trash`

---

## Task 5: Remaining zone-move consumers

Add handlers for `AddToHandFromReveal`, `TrashFromReveal`, `ReturnToDeckFromReveal`, `RevealTopDeck`, `PlaceRemainderOnDeck`, `TrashFromHandByIndex`, `MarkSecurityFaceUp`.

Each follows the same pattern as Task 4's zone-move handlers: resolve the `CompiledBindingRef` via `resolve_binding_ref`, dispatch to the matching `EffectContext` method.

- [ ] **Step 1: Tests** — one per step family.
- [ ] **Step 2: Extend `zone_moves::try_run`.**
- [ ] **Step 3: Commit** — `dsl phase 2b: reveal-pool + security-mark zone moves`

---

## Task 6: `run_step` dispatches into zone_moves

Wire `zone_moves::try_run` into `run_step`:

```rust
pub fn run_step(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &mut Bindings) {
    if memory::try_run(step, ctx) { return; }
    if draw::try_run(step, ctx) { return; }
    if zone_moves::try_run(step, ctx, bindings) { return; }
    // Unhandled: Phase 2c+ adds more families.
}
```

Note: zone_moves reads bindings but doesn't write — its `bindings` parameter is `&Bindings` (not `&mut`). The selection handlers write bindings inside their callbacks.

- [ ] **Step 1: Commit** — `dsl phase 2b: wire zone_moves into run_step dispatcher`

---

## Task 7: `lower_triggered` uses `run_steps`

Replace the `for step in process_steps.iter() { run_step(...) }` loop in `lower_triggered.rs` with a single `run_steps(&process_steps, ctx, &mut bindings)` call so selection steps park correctly.

- [ ] **Step 1: Commit** — `dsl phase 2b: triggered-clause process body uses run_steps (selection-aware)`

---

## Task 8: End-to-end fixture

Build a DSL YAML card with:
```yaml
card: DSL-E2E-002
name: Reclaim
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - select_trash:
          of: you
          bind_as: pick
          filter: {}
          prompt: "Return a card from trash"
      - add_to_hand_from_trash: { of: you, card: pick }
```

Compile, register, place one card in player 0's trash, invoke the OnPlay effect's process closure. The process should install a `PendingSelection` of kind `Trash`. Call `runner.game.resolve_selection(action_id)` with the trash-card's action ID. Assert:
1. The card moves from trash to hand.
2. The selection clears.

- [ ] **Step 1: Commit** — `dsl phase 2b: end-to-end — select_trash → add_to_hand_from_trash round-trip`

---

## Self-Review

**Spec coverage (§7.3 Phase 2 slice 2):**
- Bindings clone + typed inserters — Task 1
- BindingRef resolver — Task 2
- Continuation-passing dispatcher — Task 3
- 4 selection steps + 2 zone-move consumers — Task 4
- 7 more zone-move consumers — Task 5
- Wire-up — Tasks 6-7
- End-to-end — Task 8

**Explicit deferrals (Phase 2c):**
- `SelectReveal`, `SelectSecurity`, `SelectMaterial`, `SelectUnionZone`, `SelectCountCappedMulti`, `SelectOrderedPermutation`, `SelectEffectChoice`, `AsSelectingPlayer`
- Permanent mutations (`DeletePermanent`, `ReturnToHand`, `Suspend`, etc.)
- Control flow (`If`, `ForEach`, `PerSelected`, `Optional`, `ScheduleDelayed`)
- Play/digivolve steps
- Modifier steps
- Filter-closure widening so `CompiledPredicate` is evaluated at selection-install time (Phase 2b punts with accept-all filters)

**Known limitation (documented):** the `EffectContext::select_*` filter signature is `Fn(&Game, ...) -> bool`, not `Fn(&EffectReadContext, ...) -> bool`. Evaluating a `CompiledPredicate` requires the full read-context tuple. Phase 2b accepts all candidates at install time; Phase 2c adds a widening API on `EffectContext` that threads `(source_card, source_permanent, player)` into the filter closure.

**Type consistency:** `run_steps(&[CompiledStep], &mut EffectContext, &mut Bindings)`, `selections::try_install(&CompiledStep, &[CompiledStep] tail, &mut EffectContext, Bindings) -> bool`, `zone_moves::try_run(&CompiledStep, &mut EffectContext, &Bindings) -> bool`, `resolve_binding_ref(&CompiledBindingRef, &EffectContext, &Bindings) -> Option<ResolvedBinding>`.
