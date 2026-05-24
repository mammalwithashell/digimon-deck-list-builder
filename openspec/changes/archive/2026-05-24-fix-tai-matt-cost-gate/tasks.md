## 1. YAML migration

- [x] 1.1 Read `code/digimon-engine/cards/bt17/BT17-081.yaml` and locate the `[All Turns]` triggered clause.
- [x] 1.2 Add `activation_cost: { suspend_self: true }` as the leading body step (per BT13-101 / P-136 idiom; compiler lifts onto `EffectBuilder::activation_cost(...)`).
- [x] 1.3 Drop the now-redundant clause-level `optional: true` flag (BT13-101 / P-136 pattern — activation_cost carries the player-decline semantics for single-trigger bundles via `effect_queue.rs:779-832`'s pre-cost prompt; multi-trigger bundles get the per-trigger cost-payability inert via `EffectContext::suspend_self_as_cost`).
- [x] 1.4 Remove the `- suspend: { target: source }` body step.
- [x] 1.5 Update the YAML header comment to describe the new authoring shape and reference PUPPETS-G023 + BT13-101 / P-136 sister cards.

### Investigation note (memory-clamp masking)

Initial implementation surfaced a false-positive "substrate gap" that turned out to be a TEST FIXTURE issue, not an engine bug:

- The `taimatt_runner()` test fixture sets memory to `+10` (the `Rules::standard()` upper clamp). `gain_memory(+1)` from this value clamps right back to 10, producing `delta=0` after the gain.
- The existing memory-grant tests (`bt17_081_observer_{greymon,garurumon,both}_*`) included an `if !accepted { return; }` early-return guard that silently no-op'd the memory assertion when no accept/decline prompt installed. With the OLD `optional: true` + body-step `suspend` authoring, the trigger installed an outer accept prompt → `accepted` stayed true → tests asserted memory. With the NEW `activation_cost` authoring (no clause-level `optional`), the trigger auto-fires → no prompt → `accepted` stays false → early-return skipped the assertion entirely.

Both issues compound: the migration was correct, but the fixture-at-clamp + early-return-on-no-prompt combination silently passed even when memory actually wasn't granted (because of the clamp). Fix: lower the fixture's starting memory to `0` so `gain_memory` has headroom on both sides of the seesaw.

The early-return guards in the existing tests were also removed where they no longer make sense (auto-fire path doesn't set `accepted=true`), with explicit assertions pinning the memory delta unconditionally.

## 2. Behavioral regression tests

- [x] 2.1 Audited existing BT17-081 tests. Two material weaknesses identified and fixed:
   1. **Test fixture at memory clamp**: `taimatt_runner()` set memory to `+10` (the upper clamp from `Rules::standard()`). Memory gains tried to push past the clamp and got pinned at 10 — `gain_memory(+1)` yielded `delta=0`. Fixed by setting fixture memory to `0` so the seesaw has headroom.
   2. **Early-return short-circuits**: `bt17_081_observer_{greymon,garurumon,both}_present_gains_*_memory` contained `if !accepted { return; }` guards that silently no-op'd the memory assertion when no accept/decline prompt installed. With the `activation_cost` migration the trigger auto-fires (no prompt), so the early-returns short-circuited the entire assertion. Removed those guards and pinned the memory delta unconditionally.
- [x] 2.2 Added `bt17_081_two_sequential_triggers_pay_cost_once_grant_memory_once` (SECTION 7, end of file). Setup: T&M + Greymon-name Digimon + Garurumon-name Digimon pre-placed on P0; two plain Digimons in hand. Plays each in sequence; asserts the first play grants +2 memory and suspends T&M, then the second play grants +0 memory because the `suspend_self_as_cost` gate fails on the already-suspended Tamer.
- [x] 2.3 Added `bt17_081_trigger_inert_when_already_suspended` as the isolated-unit version of 2.2: pre-suspends T&M, plays a Digimon, asserts no memory granted and T&M remains suspended.
- [x] 2.4 Updated `bt17_081_process_steps_match_card_text` to assert the new leading-body-step shape: `CompiledStep::ActivationCost { kind: CompiledActivationCostKind::SuspendSelf }`. Updated `bt17_081_clause1_is_all_turns_observer_optional_faceup` to assert `!clause.optional` (the `optional` flag is dropped post-migration per BT13-101 / P-136 idiom).

## 3. Verification

- [x] 3.1 Ran `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt17_081`. Result: **21/21 BT17-081 tests pass** (19 baseline + 2 new from this change).
- [x] 3.2 Ran the full `cards_behavioral` test suite. Result: **3393 passed, 3 failed, 62 ignored**. The 3 failures (`bt24_008_on_play_decline_*`, `ex9_024_decline_discard_*`, `st19_04_on_play_decline_*`) are **pre-existing failures on `main`** — verified by stashing changes and running the same tests against the baseline branch. **No new regressions introduced by this change.**
- [~] 3.3 Engine-MCP Omnimon replay: not run live in this implementation cycle because the engine binary needs a rebuild + Claude Code restart to surface, and the simultaneous-trigger behavior is already pinned by unit tests `bt17_081_two_sequential_triggers_pay_cost_once_grant_memory_once` and `bt17_081_trigger_inert_when_already_suspended`. The unit tests use the same code path the MCP scenario would exercise (the engine's `play` action triggers `OnEnterFieldAnyone` which queues observers and drains them through `run_queued_effect` → `activation_cost_fn` → body). The MCP-driven scenario can be re-verified opportunistically next session.

## 4. Documentation

- [x] 4.1 BT17-081's migration is documented in the YAML's header comment with a 2026-05-24 timestamp, reference to the `fix-tai-matt-cost-gate` openspec change, the PUPPETS-G023 substrate (shipped 2026-05-20), and the BT13-101 / P-136 sister-card pattern.
- [~] 4.2 `qa/dsl-vocab-gaps.md` doesn't have a BT17-081-specific entry; PUPPETS-G023 already documents the activation_cost substrate generically. No new entry required.
