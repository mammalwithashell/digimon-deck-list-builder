# Phase 2 Track B — Generic `.activation_cost(...)` Builder for Triggered Abilities

You are adding a new `EffectBuilder::activation_cost(predicate)` hook plus two concrete cost helpers (`ctx.suspend_self_as_cost() -> bool`, `ctx.return_self_to_deck_bottom_as_cost() -> bool`) to the Rust engine, and threading their cost-failure short-circuit through `effect_queue::run_queued_effect_inner`. This closes engine gap **#8 from `docs/RUST_ENGINE_GAPS.md`** ("Generic `.activation_cost(...)` builder hook for triggered abilities").

This track is fully independent of Phase 2 Track A (DSL eval-arm sweep) and Track C (OPT / inherited dispatch). The only file-level conflict risk is `effect_context/mod.rs` if Track C lands first.

## Why this matters

A huge family of Tamer-driven triggered abilities pays a cost on the *trigger* (not on a play action). Examples:

- "by suspending this Tamer, gain 1 memory" (BT4-097, BT8-090, ST6-14, BT8-094, RB1-035, BT13-101, P-136, …)
- "by suspending this Tamer to <Draw 1>" (EX9-068, BT13-102, P-136)
- "By returning this Tamer to the bottom of the deck, <Draw 2>" (BT22-088, BT22-094, BT17-093, EX11-071)

There's no engine surface for this today. Card authors either skip the cost (over-fires) or hand-roll an inline `if !perm.suspended { suspend; gain_memory(1); }` that breaks on already-suspended (no failure-path), can't surface a player decline distinct from a cost-impossible state, and re-implements the pattern per-card.

The substrate cluster this unblocks is the largest pilot-archetype unblock available — five of six piloted archetypes have at least one card waiting on it:

| Consumer | Refs / cards |
|---|---|
| **DNA Omnimon** | ~7 Tamer triggered cards (Tai Kamiya, Matt Ishida, Davis Motomiya & Ken, etc.) |
| **Puppets PUPPETS-G023** | 6 test refs (BT13-101 Miki Kurosaki & Megumi Shirakawa, P-136 Arisa Kinosaki) |
| **Puppets PUPPETS-G028** | BT22-088 Arisa Kinosaki return-this-Tamer cost |
| **Royal Knights RK-G002** | EX11-071 Cool Boy return-self-to-deck → reduced-cost hand play |
| **Medusamon / BG Imperial** | Sibling shapes call out the same builder |

Expected pilot-unblock: 5 archetypes touched, ~10–14 cards re-promotable from `PARTIAL` to `IMPLEMENTED` after this lands, plus ~15 ignored tests un-blockable across the test tree.

## Read these first (in order)

1. `CLAUDE.md` — Working Rules §17 (no-approximations, esp. cost-failure must NOT be hidden auto-selection — failures collapse the effect, no decline-vs-fail elision), §18 (write the failing DebugRunner test first), §20 (PyO3 binding boundary unchanged), §21 (no new Python card scripts for cards being moved into Rust here).
2. `docs/RUST_ENGINE_GAPS.md` — full entry "Generic `.activation_cost(...)` builder hook for triggered abilities (suspend-self / pay-as-cost on triggered abilities)". Notes the sibling relationship with "Dynamic cost reduction at BeforePayCost" (the existing `.pay_cost_fn` builder, scoped to cost-reduction usage on plays/digivolves). **This new builder must NOT collapse with `.pay_cost_fn` — that one runs during cost calculation for a play/digivolve; this one runs on triggered-ability resolution before the body process.**
3. `docs/RUST_ENGINE_API.md` — §11 (EffectBuilder), §12 (EffectContext), §13 (cost helpers). Read end-to-end before touching `effect.rs`.
4. `code/digimon-engine/src/effect.rs` — `EffectBuilder` struct + the existing `.pay_cost_fn(...)` / `.pay_cost(...)` methods at lines ~893–910. Your `.activation_cost(...)` mirrors this shape but stores on a *different* field and is consulted by `effect_queue::run_queued_effect_inner`, not by the cost-payment path for plays.
5. `code/digimon-engine/src/effect_context/mod.rs` — search for `decline_pending_pay_cost` (~line 984), `source_permanent`, and existing self-helpers (`ctx.suspend_self`, etc., if any). Note the pattern for source-handle access.
6. `code/digimon-engine/src/effect_queue.rs` — `run_queued_effect_inner` is the queued-effect dispatcher. Trace how a triggered effect goes from `enqueue_from_permanent` → drain → `process` closure invocation. You need to insert an `activation_cost` consultation between "trigger predicate gate" and "body process".
7. `code/digimon-engine/src/selection.rs` — `PendingSelection` and the cost-failure decline path. Cost failure must NOT install a pending selection — it's silent collapse. The "may you accept" prompt is `.optional()` (already exists); cost failure happens AFTER accept.
8. `qa/dsl-vocab-gaps.md` § "BT13-101 / P-136 — event predicates with suspend-this-Tamer cost [PUPPETS-G023]" and § "BT22-088 — return-this-Tamer cost before branch free-play [PUPPETS-G028]" — the user-facing DSL syntax expected by card authors. Mirror this exactly in the DSL step you add.
9. `qa/dsl-vocab-gaps.md` § "Royal Knights — source-bound return-self cost into reduced-cost hand play [RK-G002]" — slightly more complex consumer; reads cost-paid result to gate a downstream reduced play.
10. Existing card test files for the first-test target: `code/digimon-engine/tests/cards_behavioral/bt13/bt13_101.rs`, `code/digimon-engine/tests/cards_behavioral/p/p_136.rs`. These already have ignored regressions you'll un-ignore last.
11. DCGO behavioral reference only (printed text wins on disagreements): `DCGO/Assets/Scripts/CardEffect/BT13/Red/BT13_101.cs` (BT13-101 Miki Kurosaki & Megumi Shirakawa) — confirms the `isSuspend` cost gate happens after the trigger-fire `canActivate` check but before the body coroutine. Use as tiebreaker only for ordering.

## Work to be done

### 1. New `EffectBuilder::activation_cost(...)`

In `code/digimon-engine/src/effect.rs`:

- Add `pub activation_cost_fn: Option<ActivationCostFn>` to the `Effect` struct (alongside `pay_cost_fn`).
- `type ActivationCostFn = Box<dyn Fn(&mut EffectContext) -> bool + Send + Sync>;`
- Add builder method:
  ```rust
  pub fn activation_cost<F>(mut self, f: F) -> Self
  where F: Fn(&mut EffectContext) -> bool + Send + Sync + 'static
  ```
  that stores onto `inner.activation_cost_fn`.
- Initialize `activation_cost_fn: None` in every `Effect` constructor path (use `cargo check` to find them all — there are ~10 builder entry points).
- Docstring must clearly contrast against `.pay_cost_fn` — different timing, different scope, different failure semantics.

### 2. Cost helpers on `EffectContext`

In `code/digimon-engine/src/effect_context/mod.rs`:

- `pub fn suspend_self_as_cost(&mut self) -> bool` — returns `false` if `self.source_permanent()` is already suspended, otherwise suspends it and returns `true`. Calls into the existing suspension primitive used by `ctx.suspend(permanent)`. Must fire `OnSuspend` observers (per CLAUDE.md rule that all suspensions surface the same payload).
- `pub fn return_self_to_deck_bottom_as_cost(&mut self) -> bool` — moves the source permanent's top card to the controller's deck bottom (Tamer return-to-deck-bottom semantics). Returns `false` if source has been removed (extremely unlikely mid-trigger but possible if prior chain destroyed it). Trash the rest of the digivolution stack per standard return-to-deck rules. Fires `OnLeaveField` for the source.
- Both helpers respect the source-permanent escape hatch (Working Rule 17): no-op + return `false` if the source permanent is no longer valid. Do NOT panic.

### 3. Cost-failure short-circuit in `effect_queue.rs`

In `code/digimon-engine/src/effect_queue.rs::run_queued_effect_inner`:

- After the existing condition-gate check (the `condition` closure) and after the optional-prompt acceptance, but BEFORE invoking the body `process` closure: if `effect.activation_cost_fn.is_some()`, run it. If it returns `false`, log the failure (debug-only), drop the effect from the queue, and do NOT run the body. **Crucially: cost failure must consume the trigger slot the same way as a successful firing for OPT-lockout purposes** (so a "by suspending this, do X — but I'm already suspended" doesn't get to retry next trigger event). Cite the OPT-slot accounting code (mostly lives in `effect_queue.rs` already; Track C may reshape it but the relationship is the same).
- Cost successes drop through to body invocation as today.

### 4. DSL surface

In `code/digimon-engine/src/dsl_cards/step/` (find the file that holds triggered-clause step definitions) and `code/digimon-dsl/src/step.rs`:

- New `CompiledStep::ActivationCost { kind: ActivationCostKind }` where `ActivationCostKind` is initially `SuspendSelf` or `ReturnSelfToDeckBottom` (extensible enum).
- DSL syntax (mirror `qa/dsl-vocab-gaps.md` PUPPETS-G023):
  ```yaml
  - activation_cost:
      suspend_self: true
  - gain_memory: 1
  ```
  or
  ```yaml
  - activation_cost:
      return_self_to_deck_bottom: true
  - draw: 2
  ```
- The DSL lowering must emit one `Effect` per triggered clause where the activation-cost step is the FIRST sub-step, lifting onto `EffectBuilder::activation_cost(...)`. Downstream sub-steps land in the body process closure.
- Add a coverage arm for `CompiledStep::ActivationCost` in the step dispatcher (the variant-coverage lint from PR #475 will require this — that's working as intended).
- **Do not** allow `activation_cost:` to appear anywhere except as the leading step in a triggered clause body. Validator should reject mid-body uses with a clear error message.

### 5. Failing-test first (TDD)

Write `code/digimon-engine/tests/cards_behavioral/bt13/bt13_101.rs` extension or new behavioral test:

- BT13-101 Miki Kurosaki & Megumi Shirakawa: trigger condition met, suspend-self cost paid, body fires. Memory delta verified.
- Same card: pre-suspended (e.g., used MayAttackPlayerOnly suspend earlier), trigger condition met, cost fails, body does NOT fire, OPT slot consumed.
- BT22-088 Arisa Kinosaki: return-self-to-deck-bottom cost paid; permanent gone from battle area; deck size +1; body's free-play branch fires with the source-Tamer reference still resolvable as a `provenance_token` (the helper must bind provenance before the source leaves).

Confirm tests FAIL with the right shape before adding any production code (write test → run → see failure → write code → re-run → see pass).

### 6. Migrate one consumer card YAML

Pick BT13-101 (Puppets, smallest scope) and rewrite its current YAML to use the new `activation_cost:` step. Confirm the existing test passes. Update `qa/qa-reports/validated_cards_dsl.json` to advance BT13-101 from `PARTIAL` to `IMPLEMENTED` if and only if all printed text is now covered (do not advance if a *different* gap remains).

**Do not** migrate the other ~10 consumer cards in this PR — that's card-author work for Wave 4 / Phase 3. List them in the PR description so the next batch-implement run picks them up.

## Acceptance gates

- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt13_101` passes (new + pre-existing).
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_088` passes (new return-self-as-cost test).
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage` passes (new `CompiledStep::ActivationCost` variant covered).
- `cargo test --manifest-path code/digimon-engine/Cargo.toml` smoke: no regressions, total pass count rises.
- New `EffectContext::suspend_self_as_cost` / `return_self_to_deck_bottom_as_cost` are documented in `docs/RUST_ENGINE_API.md` §13.
- Cost-failure does NOT install a pending selection (verify by inspecting `game.pending_selection` after a forced-failure scenario).
- OPT-slot accounting: a cost-failed triggered effect consumes the OPT slot for the same activation key as a successful firing (Track C may revisit OPT keying; coordinate if Track C lands first).

## Constraints

- No-approximations: the prompt for accepting the trigger (`.optional()`) and the cost-failure path are distinct mechanisms. A player accepting `.optional()` and then having the cost fail is NOT a player decline — log it as a cost failure. (Card authors will frequently chain `.optional().activation_cost(...)` — both must fire in order.)
- Working Rule 1: no `ACTION_SPACE_SIZE` change. The activation-cost prompt does not surface through a new action bit — it reuses the existing optional-prompt action when paired with `.optional()`, and consumes silently when used without.
- Working Rule 9: WebSocket / state filter unchanged.
- Working Rule 17: cost failure cannot hide a player decision. The body that the cost gates may surface choices — those still flow through `pending_selection`.
- Working Rule 20: PyO3 binding boundary unchanged.
- Working Rule 21–22: do NOT touch `code/engine_py_legacy/`. This is Rust-only work.
- Source priority: printed text → Rules Manual → fandom wiki → DCGO. DCGO confirms ordering, NOT optionality.
- Do NOT couple with `.pay_cost_fn` rename / refactor — that's a sibling builder, must coexist.
- Do NOT remove `raw_rust` escapes for other consumer cards in this PR. That's card-author scope.

## Verification

```
# Targeted card behavioral tests
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt13_101
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_088

# DSL surface
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- activation_cost
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage

# Effect queue + cost-failure semantics
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- activation_cost
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_queue
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch

# Full smoke
cargo test --manifest-path code/digimon-engine/Cargo.toml

# Python parity (sanity)
cd code/digimon-engine-py && maturin develop
DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v
```

## Tracker discipline

- `docs/RUST_ENGINE_GAPS.md` — move entry "Generic `.activation_cost(...)` builder hook for triggered abilities" to `qa/resolved-gaps.md` under "Phase 2 Track B closure — 2026-05-XX". Cite the PR # and test names.
- `docs/RUST_ENGINE_API.md` — add `Effect::activation_cost(predicate)`, `EffectContext::suspend_self_as_cost`, `EffectContext::return_self_to_deck_bottom_as_cost` to §11 / §13 reference sections.
- `qa/dsl-vocab-gaps.md` — close PUPPETS-G023, PUPPETS-G028, and RK-G002 entries (substrate now landed; card-author migration is the remaining sliver — note explicitly).
- `qa/archetype-qa/engine-gaps.md` — no shadow entries should require touching.
- `qa/qa-reports/validated_cards_dsl.json` — advance BT13-101 if migration is complete. Do not touch other cards.

## Order of operations

1. **Read the no-approximations rule and the existing `.pay_cost_fn` builder.** Understand the contrast before writing code.
2. **Write the BT13-101 failing test first.** Includes the cost-paid happy path and the pre-suspended cost-failure path. Run, see fail.
3. **Add the `Effect::activation_cost_fn` field + `EffectBuilder::activation_cost(...)` method.** `cargo check` to find every `Effect` constructor — initialize the field everywhere.
4. **Add `EffectContext::suspend_self_as_cost`.** Re-use the existing suspend primitive; ensure `OnSuspend` fires.
5. **Wire the cost-failure short-circuit in `effect_queue::run_queued_effect_inner`.** OPT-slot accounting parity with success path.
6. **Run the BT13-101 happy path test — should pass.** Then the cost-failure test — should pass.
7. **Add `EffectContext::return_self_to_deck_bottom_as_cost`.** Bind provenance before the source leaves field. Fire `OnLeaveField`.
8. **Write the BT22-088 test, run, pass.**
9. **DSL surface:** `CompiledStep::ActivationCost`, lowering, validator rejection of mid-body uses, variant-coverage lint compliance.
10. **Migrate BT13-101 YAML** to use the new step. Re-run all BT13-101 tests.
11. **Tracker hygiene + PR write-up.** List the ~9 other consumer cards that are now ready for card-author migration (do not migrate them in this PR).

## Out of scope (do NOT do in this PR)

- Migrating other consumer cards' YAML beyond BT13-101 (BT4-097, BT8-090, ST6-14, BT8-094, EX9-068, BT13-102, RB1-035, P-136, BT22-094, BT17-093 — all defer to Phase 3 batch-implement).
- The G-OPT-TRIGGERED enforcement at run_queued_effect_inner — that's Track C and may reshape adjacent code. Coordinate if Track C lands first.
- The G-INHERITED-DISPATCH residue — separate Track C work.
- Any new selection kind, action bit, or tensor profile change.
- BeforePayCost-time cost-reduction interactions on plays/digivolves — those stay on `.pay_cost_fn`. Do not unify the two builders.
- `RK-G002`'s downstream "reduced-cost hand play" gate — that's a card-author concern over the new substrate, not engine work.

## Discovery rider

If, while wiring the cost-failure short-circuit, you find that the OPT-slot accounting in `run_queued_effect_inner` doesn't currently consume the slot on cost failure (i.e., a card could retry next trigger), document it as a Track C overlap and consume the slot consistently anyway in this PR. Flag in the PR description so Track C reviewer knows.

If the existing `.pay_cost_fn` builder's failure path conflicts in some subtle way with the new one — e.g., shared `decline_pending_pay_cost` plumbing — call it out explicitly in design notes. Do NOT silently re-use the existing decline mechanism; cost-failure is conceptually distinct.
