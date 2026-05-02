---
title: Keyword Parity Phase C — Nested-selection-in-replacement substrate
date: 2026-04-25
status: design
area: digimon-engine
parent: docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md
related:
  - docs/superpowers/specs/2026-04-21-card-scripting-dsl.md
  - docs/superpowers/plans/2026-04-24-card-scripting-dsl-phase-2d.md
  - docs/RUST_ENGINE_API.md
  - docs/RUST_PYTHON_PARITY.md
---

# Phase C — Nested-selection-in-replacement substrate

## 0. TL;DR

Phase C delivers the engine substrate that lets a `WhenWouldBe*` replacement-process closure install a nested player-selection (e.g., "pick a Tamer to slide under") and resume cleanly after the player picks, with the resulting outcome routed back into the replacement dispatcher. Without this, Phase D's keyword auto-installs (Save, Decoy, Fortitude, Fragment, ArmorPurge, Partition) cannot be authored — they all share the pattern *fire replacement → prompt for selection → mutate state → cancel/redirect/substitute the original event*.

The substrate is **engine-only**:
- New `Game.parked_replacement: Option<ParkedReplacement>` slot.
- Four new `EffectContext` outcome-setter methods (`cancel_leave`, `redirect_replacement`, `substitute_replacement`, `handle_replacement`).
- Two dispatcher hooks in `replacement.rs` (post-process, post-callback).
- Hand-rolled test cards proving end-to-end behavior for single-pick (Save-like), multi-pick (Fragment-like), and substitute (Decoy-like) flows.

**Explicit non-goals** (deferred to DSL Phase 3): wiring `digimon-engine/src/dsl_cards/lower_replacement.rs`'s no-op body, adding DSL step verbs (`cancel_leave`, `redirect_leave_to`, etc.). Phase D's hand-rolled keyword auto-installs consume the engine substrate directly without the DSL.

## 1. Motivation

The Phase B Phase-7 replacement framework supports two synchronous patterns:
1. **Optional accept dialog**: `install_optional_selection` prompts "may you accept this replacement?" — single binary selection. Today's Barrier / Evade / Decode auto-installs.
2. **Mandatory synchronous process**: `replacement_process` closure runs to completion, sets `outcome`. No nested player input.

What's missing is the **third pattern**: a replacement process that *itself* needs a player selection (pick a Tamer, pick N sources, pick a redirect target). DCGO's `SaveClass.cs`, `FragmentClass.cs`, `DecoyClass.cs`, etc. all use this pattern. The current Rust framework documents the gap as the **Task 3 limitation** at `combat.rs:2213-2229`: "if an optional replacement installs a `PendingSelection::Replacement` at EITHER dispatch stage, this method early-returns without committing."

Phase D depends on Phase C. The alpha-tier auto-installs (Save, Decoy, Fortitude, Fragment, ArmorPurge, Partition) cannot be implemented without the substrate.

## 2. Scope

### 2.1 In scope

- `Game.parked_replacement: Option<ParkedReplacement>` slot + invariants.
- `EffectContext::cancel_leave() / redirect_replacement(zone) / substitute_replacement(subject) / handle_replacement()`.
- Dispatcher post-process hook in `replacement.rs::run_candidate_inner`: install `parked_replacement` when process closure parks a selection.
- Dispatcher post-callback hook (in the selection-resolution path): drain `parked_replacement` after the callback completes; route through `commit_deferred_outcome`.
- Panic-safe RAII guard around the parked-commit (mirrors `replacement.rs::run_commit_with_flag`).
- Hand-rolled test cards covering single-pick, multi-pick, and substitute flows.
- Substrate-level tests (default-None outcome, last-write-wins, single-outstanding invariant, panic recovery).
- Removal of the `Task 3 limitation` doc comment at `combat.rs:2213-2229`.
- Documentation updates: `RUST_ENGINE_API.md` (new outcome-setters), `RUST_ENGINE_GAPS.md` (mark `WhenWouldBeDeleted framework extensions` resolved), parity tracker (Phase C landed note).

### 2.2 Out of scope

- **DSL Phase 3 work**. `lower_replacement.rs::lower` stays a no-op stub; new DSL step verbs (`cancel_leave` etc.) are deferred. The DSL author picks up the wiring when Phase 3 lands.
- **Phase D auto-installs**. Save / Decoy / Fortitude / Fragment / ArmorPurge / Partition keyword `keyword_to_auto_effect` emissions are Phase D's job; Phase C only delivers the substrate they consume.
- **Move-to-stack and trash-source-from-stack primitives**. These are Phase D mutation primitives; Phase C's test cards substitute inline mutations to avoid coupling.
- **Multi-stage parking** (a callback whose body itself parks another replacement). Single-outstanding invariant only; if a real card surfaces nested-park, escalate to a follow-up plan that converts the slot to a `Vec`-stack.
- **Generalization beyond `WhenWouldBeDeleted`**. The substrate works for every `WhenWouldBe*` timing because the dispatcher hooks live in shared `replacement.rs` paths; Phase C tests only `WhenWouldBeDeleted` end-to-end. `WhenWouldBeReturnedToHand` / `WhenWouldBeDeDigivolved` / etc. inherit the substrate without per-timing wiring.

## 3. Approach — DSL-aligned closure continuation

The DSL Phase 2b shipped a continuation-passing dispatcher: `run_steps` walks a step list; when it hits `select_*`, the tail is captured as the callback. This is **already the right pattern** for replacement-process bodies. Phase C extends the engine to make it work cleanly across the replacement boundary:

- The replacement-process closure calls existing `ctx.select_*` helpers from inside its body.
- The dispatcher detects `pending_selection.is_some()` after the process returns (same trick `make_accept_callback` already uses for the optional-accept dialog).
- The dispatcher snapshots replacement-context state (`subject`, `cause`, `original_destination`, `source_card`, `source_permanent`, `controller`) into `Game.parked_replacement`.
- The user's `select_*` callback closure IS the continuation — Phase C adds NO new continuation storage. The existing `PendingSelection::callback` slot carries it.
- Inside the callback body, the user calls `ctx.cancel_leave()` etc. — these write to `Game.parked_replacement.outcome`.
- After the callback returns, the dispatcher post-callback hook reads `parked.outcome`, calls `commit_deferred_outcome`, clears the slot.

For Rust hand-authored cards (Phase D Save):

```rust
.replacement_process(|rctx| {
    rctx.effect.select_own_permanent(
        "pick a Tamer to slide under",
        /* optional = */ false,
        |g, h| g.is_tamer(h),
        |ctx, tamer| {
            ctx.move_self_under(tamer);
            ctx.cancel_leave();
        },
    );
})
```

For the eventual DSL Phase 3:

```yaml
- kind: replacement
  trigger: would_be_deleted
  process:
    - select_own_permanent: { filter: { is_tamer: true }, bind_as: tamer, prompt: "Pick a Tamer to slide under" }
    - move_self_under: { target: $tamer }
    - cancel_leave: {}
```

DSL Phase 3 lowers the step list via the existing `run_steps` machinery; the `cancel_leave` step lowers to `EffectContext::cancel_leave`. **Phase C delivers the engine pieces that DSL Phase 3 will compile against.**

## 4. Components

### 4.1 `ParkedReplacement` (new struct on `Game`)

```rust
// digimon-engine/src/replacement.rs (or game.rs — see §6.1)
pub(crate) struct ParkedReplacement {
    pub subject: ReplacementSubject,
    pub cause: ReplacementCause,
    pub original_destination: Option<Zone>,
    pub source_card: CardHandle,
    pub source_permanent: Option<PermanentHandle>,
    pub controller: PlayerId,
    /// Outcome the in-flight callback writes via `ctx.cancel_leave()` etc.
    /// Read by the dispatcher post-callback hook after the callback returns.
    pub outcome: ReplacementOutcome,
}
```

Slot: `Game.parked_replacement: Option<ParkedReplacement>` (pub(crate), `#[doc(hidden)]`). Initialised to `None` in `Game::new`.

**Single-outstanding invariant**: at most one parked replacement at any time. The dispatcher post-process hook `debug_assert!`s on entry. If a real card surfaces nested-park, escalate.

**Coexistence with `Game.dsl_outer_tail`** (Phase 2d): independent slots for independent concerns. Both can be `Some(_)` simultaneously without interaction. Cross-reference both fields' docs.

### 4.2 `EffectContext` outcome-setters

```rust
impl<'a> EffectContext<'a> {
    /// Cancel the parked leave-the-field event. The would-be-deleted /
    /// would-be-bounced / etc. permanent stays on the field; the original
    /// caller's deletion / return / etc. is suppressed.
    ///
    /// Phase C §C2. Writes `Cancelled` to `Game.parked_replacement.outcome`.
    /// Calling this outside a replacement-process callback is a
    /// `debug_assert!` panic in dev builds; release no-ops silently.
    pub fn cancel_leave(&mut self);

    /// Mark the parked replacement as custom-handled — the process body
    /// has already mutated state and the original event should be skipped.
    /// Distinct from `cancel_leave` only at the doc level; both result
    /// in `commit_deferred_outcome` taking the no-op arm.
    pub fn handle_replacement(&mut self);

    /// Redirect the parked event to a different zone. Honored by
    /// `commit_deferred_outcome` for `(Trash, Redirected(Deck))` →
    /// `return_to_deck`, `(Trash, Redirected(Hand))` → `return_to_hand`,
    /// etc. Per-arm semantics defined in `replacement.rs::commit_deferred_outcome`.
    pub fn redirect_replacement(&mut self, zone: Zone);

    /// Substitute a different subject for the parked event. Phase 7 already
    /// handles `Substituted(Permanent(other))` by recursively dispatching
    /// the original event against `other`. Used by Decoy.
    pub fn substitute_replacement(&mut self, subject: ReplacementSubject);
}
```

The same accessors mirror onto `EffectReadContext` only as `deletion_cause()` / `was_deleted_by_*()` already do (Phase B §B5) — but `EffectReadContext` cannot mutate, so outcome-setters live on `EffectContext` only.

### 4.3 Dispatcher post-process hook

In `replacement.rs::run_candidate_inner`, after the `process(&mut rep_ctx)` call:

```rust
// (existing code that runs the process closure)
process(&mut rep_ctx);

// Phase C: detect nested-select park.
if game.pending_selection.is_some() {
    debug_assert!(
        game.parked_replacement.is_none(),
        "nested replacement park; outer outcome would be lost"
    );
    game.parked_replacement = Some(ParkedReplacement {
        subject,
        cause,
        original_destination,
        source_card,
        source_permanent,
        controller,
        outcome: ReplacementOutcome::None,
    });
    // Caller (e.g. make_accept_callback) already handles the
    // pending_selection.is_some() yield — return None here.
    return ReplacementOutcome::None;
}

rep_ctx.outcome
```

### 4.4 Dispatcher post-callback hook

Selection resolution flows through `Game::resolve_selection` → `resolve_generic_selection` (in `effect_queue.rs`). After the user callback returns, before any further dispatch, check the parked slot:

```rust
// In the selection-callback completion path (resolve_generic_selection),
// after invoking the user's callback:
if let Some(parked) = game.parked_replacement.take() {
    // Panic-safe commit, mirroring run_commit_with_flag.
    let prior = game.in_replacement_commit;
    game.in_replacement_commit = true;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        commit_deferred_outcome(
            game,
            parked.subject,
            parked.cause,
            parked.original_destination,
            parked.outcome,
        );
    }));
    game.in_replacement_commit = prior;
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}
```

This hook fires *after* the user's callback completes (whether by accept or by inner-decline-default). It does NOT fire for selections that were never associated with a parked replacement (`parked_replacement.is_none()` short-circuits to no-op).

### 4.5 `commit_deferred_outcome` extension

Today the function in `replacement.rs:794-908` handles outcomes from optional-accept callbacks. Extension: it already takes `subject` / `cause` / `original_destination` / `outcome` as parameters — Phase C invokes it the same way. **No new arms needed**; the existing (Trash/Hand/Deck × Cancelled/Redirected/Substituted) switch is complete.

### 4.6 Test cards

Hand-rolled in `digimon-engine/tests/replacements/`:

- **`nested_select_save.rs`** — single-pick + Cancelled.
- **`nested_select_fragment.rs`** — multi-pick (count-capped) + Cancelled, including the empty-stack edge case.
- **`nested_select_decoy.rs`** — single-pick + Substituted.
- **`nested_select_regression.rs`** — Barrier / Evade / Decode synchronous-process unchanged-behavior verification.
- **`nested_select_substrate.rs`** — substrate-level tests (default-None, last-write-wins, single-outstanding invariant, panic recovery).

## 5. Data flow

### 5.1 Single-pick (Save) — accept path

```
T0:  ctx.delete_permanent(saved_handle)
       → progress_excludes? no → game.delete_permanent_with_effects(saved_handle)
       → infer_deletion_cause → OpponentEffect
       → delete_permanent_with_cause(saved_handle, OpponentEffect)
T1:  try_replace(WhenWouldLeaveBattleArea, ...) → None
T2:  try_replace(WhenWouldBeDeleted, Permanent(saved_handle), OpponentEffect, Some(Trash))
       → Save candidate found, optional
       → install_optional_selection: PendingSelection::Replacement{REPLACEMENT_ACCEPT}
       → return; outer caller yields
T3:  resolve_selection(controller, REPLACEMENT_ACCEPT)
       → make_accept_callback fires
       → run_candidate_inner builds rep_ctx, runs Save's process closure:
           rctx.effect.select_own_permanent("pick a Tamer", false, filter, |ctx, tamer| {
               ctx.move_self_under(tamer);
               ctx.cancel_leave();
           });
       → select_own_permanent installs PendingSelection::OwnField + user closure
       → process returns to run_candidate_inner
T4:  POST-PROCESS HOOK: pending_selection.is_some()
       → install Game.parked_replacement = Some(ParkedReplacement {
             subject: Permanent(saved_handle),
             cause: OpponentEffect,
             original_destination: Some(Trash),
             outcome: None, ... })
       → run_candidate_inner returns None; outer caller yields
T5:  resolve_selection(controller, encode_attack(0, tamer_slot))
       → nested OwnField selection's callback fires
       → fresh EffectContext keyed to source
       → user body runs: move_self_under(tamer); cancel_leave()
       → cancel_leave() writes Game.parked_replacement.outcome = Cancelled
T6:  POST-CALLBACK HOOK: parked_replacement.is_some()
       → take parked, run_parked_commit_with_guard:
         commit_deferred_outcome(Permanent(saved_handle), OpponentEffect, Some(Trash), Cancelled)
       → (Trash, Cancelled) arm: no-op (deletion cancelled)
T7:  pending_selection cleared, parked_replacement cleared, deletion fully resolved.
```

### 5.2 Single-pick (Save) — outer-decline path

```
T0..T2: same as 5.1 up to install_optional_selection.
T3:  resolve_selection(controller, REPLACEMENT_DECLINE)
       → make_decline_callback fires
       → commit_deferred_outcome(..., None) → original deletion proceeds
       → parked_replacement NEVER set
       → carrier dies. Done.
```

### 5.3 Multi-pick (Fragment(2)) — accept path

```
T0..T2: same as 5.1 (Fragment is also optional).
T3:  resolve_selection(controller, REPLACEMENT_ACCEPT)
       → process closure installs select_count_capped_multi(N=2)
       → post-process hook installs parked_replacement (Cancelled-pending)
T4..T4+N:  count-capped trampoline runs N pick steps internally
T4+N: trampoline final_callback runs user body:
       for src_idx in picks { trash source }; cancel_leave();
T4+N+1: POST-CALLBACK HOOK detects parked_replacement;
       commit_deferred_outcome → (Trash, Cancelled) → no-op
```

The count-capped trampoline (Phase 8 Task 6) is self-contained; Phase C does not modify it. The trampoline's final_callback is what runs the user body; the user's `cancel_leave()` writes to the parked slot. The post-callback hook drains the slot once per replacement, regardless of how many internal pick steps the trampoline ran.

### 5.4 Empty-stack edge case (Fragment(2) on a 1-source carrier)

`select_count_capped_multi` with empty candidates fires its final_callback immediately with `picks=Vec::new()`. The user body runs (zero loop iterations + `cancel_leave()`) — which would incorrectly cancel deletion when the cost was unpayable.

**Phase D's auto-install responsibility**: Fragment's `Effect::condition` closure must check `permanent.card_sources.len() > N` so the candidate isn't even offered when the cost is unpayable. Matches DCGO's `FragmentClass.cs::CanReplace`.

**Phase C's responsibility**: provide a test (`nested_select_fragment.rs::fragment_n_too_few_sources_does_not_offer`) that exercises a hand-rolled Fragment-like card with a `condition` closure gating on stack size. Confirms the outer prompt is suppressed entirely; deletion proceeds normally; `parked_replacement` stays None.

### 5.5 Substitute (Decoy) — accept path

```
T0:  ctx.delete_permanent(ally_handle)
T1..T2: try_replace(WhenWouldBeDeleted, Permanent(ally_handle)) → Decoy candidate found
       (Decoy is an OnAlly replacement — fired for ally's deletion, not self's)
T3:  resolve_selection(controller, REPLACEMENT_ACCEPT)
       → process closure installs select_own_permanent (filter: own Digimon other than ally)
T4:  POST-PROCESS HOOK installs parked_replacement
T5:  resolve_selection(controller, decoy_self_slot)
       → user body: ctx.substitute_replacement(Permanent(decoy_self_handle));
T6:  POST-CALLBACK HOOK: commit_deferred_outcome (Trash, Substituted(Permanent(decoy_self)))
       → existing arm: delete_permanent_with_cause(decoy_self, OpponentEffect) recursively
       → ally survives; decoy_self is deleted instead.
```

## 6. API surface changes

### 6.1 `Game` struct additions

- `parked_replacement: Option<ParkedReplacement>` — `pub(crate)`, `#[doc(hidden)]`. Initialized to `None` in `Game::new`.
- `ParkedReplacement` struct — defined in `replacement.rs` (next to `ReplacementContext`) and re-exported `pub(crate)`.

No public Game-level methods added; the substrate is consumed via `EffectContext`.

### 6.2 `EffectContext` additions

- `cancel_leave(&mut self)`
- `handle_replacement(&mut self)`
- `redirect_replacement(&mut self, zone: Zone)`
- `substitute_replacement(&mut self, subject: ReplacementSubject)`

All four `pub`. Doc-comment cross-reference Phase C §C2 + the parked-replacement scope contract.

### 6.3 `replacement.rs` modifications

- New private function `try_install_parked_replacement(game, subject, cause, original_destination, source_card, source_permanent, controller)` — called by `run_candidate_inner` post-process.
- New private function `try_drain_parked_replacement_with_guard(game)` — called by selection-callback completion path in `effect_queue.rs::resolve_generic_selection`.
- Existing `commit_deferred_outcome` unchanged.
- Existing `Task 3 limitation` doc comment at `combat.rs:2213-2229` deleted; replaced with a one-liner pointing at this spec.

### 6.4 `effect_queue.rs::resolve_generic_selection` modification

Single new call site: after the user callback completes, before existing post-callback observer dispatch:

```rust
// existing: invoke user callback
(callback)(game, action_id);

// Phase C: drain parked replacement if any.
crate::replacement::try_drain_parked_replacement_with_guard(game);

// existing: drain effect queue, etc.
```

### 6.5 No DSL changes

`digimon-engine/src/dsl_cards/lower_replacement.rs` stays a no-op stub through Phase C. DSL Phase 3 picks up the wiring (a separate plan, not Phase C scope).

## 7. Testing strategy

All TDD. Per §5.5 in the brainstorming summary:

### 7.1 Test cards (hand-rolled, in `tests/replacements/`)

| File | Test | What it proves |
|---|---|---|
| `nested_select_save.rs` | `save_picks_tamer_and_cancels_deletion` | T0..T7 single-pick happy path |
| `nested_select_save.rs` | `save_outer_decline_proceeds_with_deletion` | §5.2 outer-decline path |
| `nested_select_save.rs` | `save_with_no_tamers_does_not_offer` | Empty-filter outer-suppression |
| `nested_select_fragment.rs` | `fragment_2_picks_two_sources_and_cancels` | Multi-pick happy path |
| `nested_select_fragment.rs` | `fragment_n_too_few_sources_does_not_offer` | §5.4 empty-stack edge case |
| `nested_select_decoy.rs` | `decoy_substitutes_self_for_ally_deletion` | Substitute outcome arm |
| `nested_select_regression.rs` | `barrier_synchronous_process_unchanged` | Barrier auto-install still works |
| `nested_select_regression.rs` | `evade_synchronous_process_unchanged` | Evade auto-install still works |
| `nested_select_regression.rs` | `decode_synchronous_process_unchanged` | Decode auto-install still works |

### 7.2 Substrate-level tests (`nested_select_substrate.rs`)

| Test | What it proves |
|---|---|
| `cancel_leave_outside_parked_scope_panics_in_dev` | `debug_assert!` fires |
| `last_write_wins_on_outcome` | Multiple setters: last wins |
| `default_none_when_callback_skips_outcome` | No setter call → outcome = None → original event proceeds |
| `single_outstanding_park_panics_on_double_install` | `debug_assert!` for nested-park |
| `panic_in_callback_clears_parked_slot` | `take()` before commit + AssertUnwindSafe → no leak |

### 7.3 Final-verification surfaces

Mirroring Phase A and B's exit verification:
1. `cargo test --manifest-path digimon-engine/Cargo.toml` — full sweep, all green except the two pre-existing main inheritances (`phase0_exit::phase_0_exit_criteria`, `real_cards_json::real_adapter_all_fixtures_cross_check` color mismatches).
2. `maturin build --release --manifest-path digimon-engine-py/Cargo.toml` — clean.
3. `DIGIMON_BACKEND=rust python -m pytest tests/engine/test_rust_backend_parity.py -v` — 13/13 pass.
4. `cargo test --manifest-path src-tauri/Cargo.toml` — all green.

## 8. Documentation updates

- **`docs/RUST_ENGINE_API.md`** — document the four new `EffectContext` outcome-setters (`cancel_leave` / `handle_replacement` / `redirect_replacement` / `substitute_replacement`), with one usage example each (Save / Decoy / Evade-style redirect / opaque-handled).
- **`docs/RUST_ENGINE_GAPS.md`** — mark `WhenWouldBeDeleted framework extensions` resolved 2026-04-25.
- **`docs/superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md`** — Phase C §5 entry marked landed; deviations noted (engine-only substrate, DSL Phase 3 picks up `lower_replacement.rs` wiring later).
- **`docs/RUST_PYTHON_PARITY.md`** — no change (Phase C is Rust-internal substrate; no Python-side observable behavior change).
- **`combat.rs:2213-2229`** — delete the `Task 3 limitation` doc block; replace with a pointer to this spec.

## 9. Risks and trade-offs

| Risk | Mitigation |
|---|---|
| **Single-outstanding-park is too restrictive.** A real card might surface a callback whose body itself fires another deletion that parks. | Phase C `debug_assert!`s the invariant. If a real card surfaces nested-park during Phase D / E authoring, escalate to a follow-up plan that converts the slot to a `Vec`-stack. The substrate still works for every alpha-tier keyword (Save, Decoy, Fortitude, Fragment, ArmorPurge, Partition) under the single-outstanding model. |
| **Coexistence with `Game.dsl_outer_tail` (Phase 2d).** Two ambient-state slots with separate invariants — could one's hook accidentally clobber the other? | The slots are independent. `dsl_outer_tail` carries DSL step-list tails; `parked_replacement` carries replacement-context outcome routing. Cross-reference both fields' docs. Tests in Phase C don't exercise DSL replacement clauses (DSL Phase 3 work). |
| **Breaking the existing optional-replacement flow.** The post-process hook fires after every replacement-process call, including the existing Barrier / Evade / Decode synchronous processes. | Synchronous processes don't install a `pending_selection`, so the hook's `pending_selection.is_some()` check short-circuits. Regression coverage (`nested_select_regression.rs`) exercises all three keywords. |
| **Panic during a parked continuation leaks the slot.** | `take()` before commit + `AssertUnwindSafe` + `resume_unwind`. Mirror the existing `replacement.rs::run_commit_with_flag`. Test (`panic_in_callback_clears_parked_slot`) verifies. |
| **DSL Phase 3 might want a different shape** (e.g., a builder API instead of free-form outcome-setters). | The DSL spec already maps step verbs to `EffectContext::*` calls. Phase C's outcome-setters are the natural targets for `cancel_leave` / `redirect_leave_to` / `substitute_leave_with` / `handle_replacement` step verbs. The DSL author can add a thin wrapper if Phase 3 wants a different idiom. |
| **`Task 3 limitation` doc comment removal could surprise future readers.** | Replace with a one-liner pointing at this spec; preserve the breadcrumb. |

## 10. Open questions

- **Outcome-setter naming.** `cancel_leave` matches the existing spec wording (RUST_ENGINE_API.md mentions `ctx.cancel_leave()` already). `redirect_replacement` is parallel. `handle_replacement` vs `handled` — going with `handle_replacement` for consistency with the prefix family. **Lock in `cancel_leave / handle_replacement / redirect_replacement / substitute_replacement` unless feedback during plan-writing surfaces a better shape.**
- **Edge case: callback never installs the inner select.** A buggy process closure might not call any `select_*` after returning. Today: `pending_selection.is_none()` post-process means no park, no callback, outcome stays at whatever the rep_ctx had (which is None unless the closure called `rctx.cancel()` etc. directly). This degrades gracefully — the original event proceeds. No special-casing needed.
- **Where does `ParkedReplacement` live?** `replacement.rs` is the natural home (next to `ReplacementContext`). The slot on `Game` is `pub(crate)`. Phase C plan should pick the file. **Default: `replacement.rs` — same module as the rest of the replacement framework.**

## 11. Deliverable sequencing

Single phase, ~12 tasks (estimated). Substrate is small enough that subagent-driven-development with one task per slot/method/test suite produces tight feedback loops. The plan splits roughly:
- 1 task for the `ParkedReplacement` struct + `Game` slot.
- 1 task per outcome-setter (4 tasks).
- 1 task for the post-process dispatcher hook.
- 1 task for the post-callback dispatcher hook.
- 1 task per test card (4 tasks: save / fragment / decoy / regression).
- 1 task for substrate-level tests.
- 1 task for docs.
- 1 task for final verification.

Total: ~14 tasks. Plan file at `docs/superpowers/plans/2026-04-25-keyword-parity-phase-c.md`.

## 12. Out-of-scope follow-ups

Tracked but not owned by this spec:
- **DSL Phase 3** — `lower_replacement.rs` body wiring + new step verbs (`cancel_leave`, `redirect_leave_to`, `substitute_leave_with`, `handle_replacement`, `move_self_under`, `trash_source_from_stack`).
- **Phase D auto-installs** — Save, Decoy, Fortitude, Fragment, ArmorPurge, Partition `keyword_to_auto_effect` emissions consuming Phase C's substrate.
- **Vec-stack `parked_replacement`** if a real card surfaces nested-park during Phase D / E authoring.
- **Phase E (Retaliation, Scapegoat) cause-discriminator consumption** — already enabled by Phase B; not blocked by Phase C.
