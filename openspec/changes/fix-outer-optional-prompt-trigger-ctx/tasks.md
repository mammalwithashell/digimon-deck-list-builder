## 1. Add failing regression tests first (TDD)

- [x] 1.1 In [code/digimon-engine/tests/cards_behavioral/bt16/bt16_085.rs](code/digimon-engine/tests/cards_behavioral/bt16/bt16_085.rs), add `bt16_085_optional_outer_prompt_installs_on_normal_digivolve` — places Davis & Ken + a Lv.3 base on player 0's field, puts a blue Lv.4 Digimon in hand, calls `digivolve_from_hand` then `drain_effect_queue`, and asserts `runner.pending_selection().is_some()`, `pending_selection_view().kind == SelectionKind::Replacement`, `pending_selection_view().is_optional == true`, `pending_selection_view().selecting_player == 0`, and Davis & Ken is still unsuspended at this point. Test MUST fail on `main` (i.e. before the fix).
- [x] 1.2 Add `bt16_085_optional_outer_prompt_installs_on_dna_digivolve` — extends 1.1 to the DNA path: places two blue/green Lv.4 materials on field, Paildramon in hand, triggers the DNA digivolve via `EffectContext::effect_initiated_dna_digivolve` with `ignore_requirements=true`, asserts the same pending-selection invariants after the queue drains.
- [x] 1.3 Add `bt16_085_optional_outer_prompt_decline_skips_body` — drives the prompt to decline (via `runner.execute_action(view.selecting_player, view.on_decline_action_id_or_pass)`) and asserts Davis & Ken is NOT suspended and no `MemoryChange` events from this clause were emitted.
- [x] 1.4 Add `bt16_085_optional_outer_prompt_accept_runs_body_with_trigger_ctx` — drives the prompt to accept and asserts Davis & Ken IS suspended and exactly one `MemoryChange { delta: 1, player: 0 }` event was emitted by this clause.
- [x] 1.5 In a new file [code/digimon-engine/tests/cards_behavioral/bt17/bt17_081.rs](code/digimon-engine/tests/cards_behavioral/bt17/bt17_081.rs) (or extend an existing one if present), add `bt17_081_optional_outer_prompt_installs_on_own_digivolve` covering the sibling Tamer's analogous clause. This test MUST also fail on `main`.
- [x] 1.6 Run `cargo test --manifest-path code/digimon-engine/Cargo.toml --test bt16_085 --test bt17_081` and confirm all five new tests FAIL with assertions complaining `pending_selection` is `None` after drain. **CONFIRMED:** all 5 tests fail on `main` with "outer optional accept/decline prompt MUST install" panics. Note: the test target is `cards_behavioral` not per-card; ran via `cargo test --test cards_behavioral bt16_085_optional_outer_prompt` and `bt17_081_optional_outer_prompt`.

## 2. Refactor `queued_effect_wants_outer_optional_prompt` signature

- [x] 2.1 In [code/digimon-engine/src/effect_queue.rs](code/digimon-engine/src/effect_queue.rs), change the signature of `queued_effect_wants_outer_optional_prompt` from `&self` to `&mut self`.
- [x] 2.2 At the single call site (line 857, inside `drain_effect_queue`), no syntactic change is needed because `self` is already `&mut Game` — but verify the borrow checker accepts the change. **CONFIRMED:** no call-site change needed; `effects_for_card` returns an owned `Vec` so the inner borrow ends before the guard installation.
- [x] 2.3 Run `cargo check --manifest-path code/digimon-engine/Cargo.toml` and resolve any borrow-checker fallout before proceeding. **CONFIRMED:** clean check.

## 3. Install the trigger context guard inside the function

- [x] 3.1 Inside `queued_effect_wants_outer_optional_prompt`, immediately after the existing early-return guards (granted-effect, effects-not-found, `needs_outer_optional_prompt` flag, source-live, OPT counter), and BEFORE the `EffectReadContext::new_with_source_kind` construction at line 2866, install the trigger context via `let trigger_guard = TriggerContextGuard::install(self, qe.trigger_context.clone());` and rebind `rctx` to use `&*trigger_guard.game`.
- [x] 3.2 Ensure both the condition closure AND the `outer_optional_guard` closure evaluate while `trigger_guard` is in scope. **CONFIRMED:** both `cond(&rctx)` and `outer_guard(&rctx)` are called before the function returns; `guard` remains in scope through the final `true`, and `Drop` restores the previous context.
- [x] 3.3 Borrow checker accepted the straightforward form because `effects_for_card` returns an owned `Vec<Effect>` — no need to hoist closure clones.

## 4. Verify the fix

- [x] 4.1 Re-run `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt16_085_optional_outer_prompt` and `bt17_081_optional_outer_prompt` — **all 5 PASS** with the fix applied.
- [x] 4.2 Run the full Rust engine test suite — **10 fix-induced regressions** surfaced and were resolved by updating tests:
  - BT17-081 / EX4-061 / EX9-066 "gains memory" tests (8 total): root cause was tests using `memory(10)` (at the +10 cap) — the +1 gain was a no-op pre-fix because no prompt installed and `if !accepted { return; }` early-exited, masking the issue. Updated each to `runner.game.set_memory(5)` and removed the lenient early-return.
  - BT24-082 clause 2 tests (2 total): synchronous body-effect assertions now intercepted by outer prompt. Added `runner.accept_optional_trigger()` + drain between the trigger and the assertions.
  - **3 pre-existing failures** remain on `main` (unrelated to this fix): `bt24_008_on_play_decline_does_not_trash_or_draw`, `ex9_024_decline_discard_does_not_return_trash_card`, `st19_04_on_play_decline_does_not_trash_or_draw`. Out of scope for this change.
- [x] 4.3 Re-ran the external MCP repro. Trace now shows `drain iter 3: PENDING: kind=Replacement optional=True prompt="You may activate BT16-085's triggered effect"` with options `action_id=59` (accept) and `action_id=62` (Pass / decline). Two MemoryChange events fire on material-2 resolve (ExVeemon's and Stingmon's mandatory `before_pay_cost_observe` gains); the third MemoryChange fires only after the player accepts the Replacement prompt. Final memory 6 from starting 3 = Δ+3 as expected.
- [x] 4.4 Rebuilt the digimon-engine-mcp binary; `cargo build -p digimon-engine-mcp` succeeds and the binary launches.

## 5. Documentation + archive

- [x] 5.1 Affected cards surfaced organically through the full-suite run: BT16-085, BT17-081, EX4-061, EX9-066 ([All Turns]/[Your Turn] suspend-self observer Tamers), BT24-082 Owen (Reptile/Dragonkin digivolve observer). All 10 affected tests were updated and now pass. The existing `qa/archetype-qa/engine-gaps.md` entry `G-OUTER-OPTIONAL-COND-NO-TRIGGER-CONTEXT` already documented the pattern; updated to RESOLVED with the affected-card list.
- [x] 5.2 Updated the comment at [code/digimon-engine/tests/cards_behavioral/bt16/bt16_085.rs](code/digimon-engine/tests/cards_behavioral/bt16/bt16_085.rs) — replaced the "if an outer accept/decline prompt installs" hedge with a definitive statement, cross-referencing the new sharper Section 7 tests.
- [x] 5.3 Marked [`G-OUTER-OPTIONAL-COND-NO-TRIGGER-CONTEXT`](qa/archetype-qa/engine-gaps.md) RESOLVED with the fix description and verification notes.
- [x] 5.4 `openspec validate fix-outer-optional-prompt-trigger-ctx --strict` reports `Change 'fix-outer-optional-prompt-trigger-ctx' is valid`.
- [ ] 5.5 Archive via `/opsx:archive` after this change is committed and the change PR merges. (Deferred — archive runs on the user's call.)
