## Context

When a Digimon enters battle area via play, the engine dispatches three distinct trigger broadcasts:

1. `EffectTiming::OnPlay` — the played card's own `[On Play]` clause (timing `OnPlay`). Fired via `Game::fire_on_play(player, field_index)`.
2. `EffectTiming::OnEnterFieldAnyone` — observer effects on other permanents that react to any card entering the field (e.g. BT17-081 Tai & Matt's `[All Turns] When one of your Digimon is played`).
3. `EffectTiming::OnAllyPlayed` — observer effects narrowed to own-ally plays.

The current play-flow pattern at all five call sites is:

```rust
self.fire_on_play(player_id, field_index);                 // enqueues OnPlay + drains immediately
self.enqueue_triggered(OnEnterFieldAnyone, EnteredField {...});
self.enqueue_triggered(OnAllyPlayed,       EnteredField {...});
self.drain_effect_queue();                                 // drains the remaining two
```

`fire_on_play` internally calls `maybe_drain_effect_queue()`, which respects the `draining_deferred` counter but defaults to inline-draining when the counter is zero. Because the play sites do NOT open a deferred-drain scope, `fire_on_play`'s drain runs immediately, before `OnEnterFieldAnyone` is enqueued. Consequently, the played card's `[On Play]` always resolves before any observer's `[All Turns]` trigger gets a chance to enter the queue — they can never share a TriggerOrder bundle.

The 2026-05-23 deferred-drain sweep (`engine-gaps.md` G-DSL-OUTER-TAIL-NESTED-PARK) added `enter_deferred_drain()` / `exit_deferred_drain_and_flush()` to fix nested-park crashes in `fire_on_*` observer helpers. The play flow specifically was not wrapped because the panic-mode crashes the sweep was fixing did not surface from `fire_on_play`. This proposal completes that work for the play flow.

DCGO (`BT17_027.cs` MetalGarurumon, `BT17_081.cs` Tai & Matt) registers both `[On Play]` and `[All Turns]` as `EffectTiming.OnEnterFieldAnyone` ICardEffects and batches them, surfacing a single ordering choice to the turn player. The Rust engine's two-batch flow diverges from this and was identified as Gap 4 in the May 24 2026 engine-MCP QA pass.

## Goals / Non-Goals

**Goals:**

- When a play event fires `OnPlay`, `OnEnterFieldAnyone`, and `OnAllyPlayed` triggers simultaneously, all three queue into a single bundle before any drain, so the turn player's `TriggerOrder` selection covers all of them.
- The default player choice (pick the played card's own `[On Play]` first) preserves the previous observable sequence; the new freedom is the option to pick observer triggers first.
- All five `fire_on_play` callers move to a single `fire_play_event_triggers` helper that encapsulates the deferred-drain wrapper, eliminating the four duplicated `enqueue_triggered(OnEnterFieldAnyone, ...) + enqueue_triggered(OnAllyPlayed, ...) + drain` sequences.
- The change is internally consistent — existing inline-drain semantics for non-play `fire_on_*` helpers (combat, leave-field, place-security, etc.) are not altered.

**Non-Goals:**

- Removing or deprecating `Game::fire_on_play`. It remains a `pub` function for callers that explicitly want OnPlay-only firing (currently no production caller does, but the surface is part of the public API).
- Touching `OnDigivolve` / `OnDnaDigivolve` / `WhenDigivolving` trigger dispatch. Those paths are separate and their ordering is governed by the digivolve / DNA digivolve code, not the play code.
- Changing the `EffectTiming` enum or the trigger source shapes. `OnPlay`, `OnEnterFieldAnyone`, and `OnAllyPlayed` remain three distinct timings; what changes is whether they share a drain.
- Aligning Rust's play-event timing taxonomy with DCGO's single `OnEnterFieldAnyone` everything-bucket. The trade-off was made deliberately when the timing map was authored; this proposal stays inside the existing taxonomy.
- Modifying `EffectTiming::OnAllyPlayed`'s narrowing predicate (own ally only). That filter is correct and unchanged.

## Decisions

**Decision 1: Introduce a single helper `Game::fire_play_event_triggers(player_id, field_index, effect_initiated)` rather than wrapping each call site individually.**

All five call sites have identical structure: fire OnPlay → enqueue OnEnterFieldAnyone with `TriggerSource::EnteredField{..., effect_initiated}` → enqueue OnAllyPlayed with the same source → drain. Extracting a helper:

- Removes ~25 lines of duplication across the five sites.
- Makes the deferred-drain wrapper a single source of truth (the helper's enter/exit pair).
- Makes future changes to the play-event trigger broadcast (e.g. adding a fourth trigger source) a single-site edit.

Alternative considered: wrap each call site individually with `self.enter_deferred_drain()` / `self.exit_deferred_drain_and_flush()`. Rejected because (a) it leaves the duplication in place; (b) it's easier to forget the wrapper at a future new call site; (c) the helper consolidates the `TriggerSource::EnteredField` construction which currently duplicates the `permanent` + `card` field extraction logic.

**Decision 2: The helper does NOT replace `fire_on_play`.**

`fire_on_play` continues to exist as a `pub` API. Its body remains unchanged (enqueues OnPlay, calls `maybe_drain_effect_queue`). When called from inside the new helper's deferred-drain scope, its `maybe_drain` no-ops — exactly the existing deferred-drain semantics.

Alternative considered: inline OnPlay enqueueing into the helper and remove `fire_on_play` entirely. Rejected because (a) it's used by `debug_runner.rs:124` for test scaffolding that may want OnPlay-only behavior; (b) removing a `pub` function is a breaking surface change that pulls additional risk into this proposal without value; (c) it's better refactored in a follow-up that audits callers.

**Decision 3: The helper signature takes `effect_initiated: bool` to disambiguate between player-action plays (false) and effect-initiated plays (true).**

The existing call sites already pass this distinction into the `TriggerSource::EnteredField { effect_initiated }` shape — the helper preserves that. Effect-initiated plays surface via the `event_dna_origin` machinery for some triggers; making the flag a helper parameter keeps the semantic explicit and easy to grep.

**Decision 4: The fifth call site (`debug_runner.rs:124`) collapses to the helper too.**

Test scaffolding benefits from the same trigger-bundle semantics as production code. Keeping `debug_runner.rs:124` on the old inline pattern would mean tests observe different ordering than the engine produces in play. Collapsing it to the helper aligns test scaffolding with production semantics.

## Risks / Trade-offs

**[Risk] Existing behavioral tests assert OnPlay resolves before OnEnterFieldAnyone observers.** → Mitigation: run the full `cards_behavioral` suite as part of the verification step. The bounded blast radius is the 6 cards listed in the proposal's risk section (BT17-007/-019/-081, BT22-008/-017, BT12-021/-047). Test assertions that depend on the old order need updating to either (a) match the new TriggerOrder shape, or (b) explicitly pick the played card's own `[On Play]` first in the test to preserve the resolution sequence.

**[Risk] Agents trained on the previous policy see new legal actions at TriggerOrder bundles.** → Mitigation: the policy surface expansion is bounded and conservative — agents that always pick the played card's `[On Play]` first (a reasonable default) produce the same downstream sequence as before. No forced retraining; downstream pipelines may want to flag the expanded action mask in evaluation reports for transparency.

**[Risk] An observer's `[All Turns]` trigger that depends on the played card having already entered the field (e.g. count-based predicates) may resolve too eagerly.** → All `OnEnterFieldAnyone` triggers ALREADY fire after the card enters the field; this is unchanged. The only thing that changes is the relative order between the playing card's own `[On Play]` and observer triggers, not whether the played card is in the battle area at observer-trigger time.

**[Risk] The helper introduces a subtle behavior change for partial broadcasts.** → The current `selections.rs:1639` call site only invokes `fire_on_play` (no OnEnterFieldAnyone / OnAllyPlayed enqueues). If this is intentional (security-effect plays may have a deliberately different broadcast set), the helper's three-trigger broadcast would be a behavior change at that site. Mitigation: audit `selections.rs:1639` specifically before consolidating. If the partial broadcast is intentional, expose a helper variant or leave that site on the inline pattern.

## Migration Plan

1. **Add the helper.** In `code/digimon-engine/src/game.rs` (near the existing `fire_on_play` definition), add:
   ```rust
   pub fn fire_play_event_triggers(
       &mut self,
       player_id: PlayerId,
       field_index: usize,
       effect_initiated: bool,
   )
   ```
   Body:
   - `enter_deferred_drain()`
   - Compute `entered: PermanentHandle` and `entered_card: CardHandle` from the player/field_index.
   - `fire_on_play(player_id, field_index)` — `maybe_drain` no-ops while deferred.
   - `enqueue_triggered(OnEnterFieldAnyone, EnteredField { player, permanent, card, effect_initiated })`
   - `enqueue_triggered(OnAllyPlayed, EnteredField { ... })`
   - `exit_deferred_drain_and_flush()` — drains all three at once.
2. **Replace inline patterns at the four production call sites:**
   - `game_actions.rs:857` (`play_card_to_battle_area`) — replace lines 857-877 with `self.fire_play_event_triggers(player_id, field_index, false)`.
   - `game.rs:1164` — replace the equivalent block with `self.fire_play_event_triggers(player_id, field_index, true)`.
   - `game.rs:1276` — replace the per-entered-card loop's trigger-broadcast block with the helper call.
   - `selections.rs:1639` — **audit first** (see Risks). If the partial broadcast at this site is unintentional, switch to the helper. If intentional, leave the inline `fire_on_play` and document why in a comment.
3. **Update the test scaffolding** at `debug_runner.rs:124` to use the helper instead of bare `fire_on_play` for clean test ↔ production parity.
4. **Add the new behavioral test** in `code/digimon-engine/tests/cards_behavioral/bt17/bt17_081.rs` (or a new file) covering the MG-play + T&M-observer ordering case. Assert the TriggerOrder bundle exposed after MG's play contains BOTH MG's `[On Play]` mandatory trigger AND T&M's `[All Turns]` optional trigger. Pick T&M first → assert T&M's effect runs before MG's effect-choice prompt.
5. **Run the cards_behavioral suite.** Fix any test assertions that pinned the old ordering. Each fix is small (typically swapping the assertion order of two effect events) but the count is bounded to the 6 cards in the risk section.
6. **Verify via engine-MCP QA replay** of the original Omnimon scenario: confirm picking T&M trigger BEFORE MG's effect choice surfaces in TriggerOrder, then resolves T&M's All Turns BEFORE MG's effect-choice prompt.

**Rollback:** revert the helper extraction and inline-pattern replacements. The `enter_deferred_drain` / `exit_deferred_drain_and_flush` machinery stays in place (used by combat / other paths).

## Open Questions

- Should `fire_play_event_triggers` also handle the `mark_until_condition_dirty` + `reevaluate_until_condition_modifiers_if_dirty` calls that follow the trigger broadcasts at lines 878-879 / 1184-1185 / 1299-1300? → Probably yes, since they are part of the post-play state finalization. Implementer should fold them in unless it interferes with a specific call site's needs.
- Should the helper return a value (e.g. the entered permanent handle) to consolidate the post-trigger-broadcast code paths further? → The play helpers currently track `entered` independently for security-played card detection; check during implementation whether the helper can absorb that or whether keeping the return narrow is cleaner.
- Is there a reason the `OnAllyPlayed` broadcast survives separately from `OnEnterFieldAnyone`, given they fire on the same event with the same `TriggerSource::EnteredField`? → Likely the `OnAllyPlayed` filter (own ally only) is the differentiator. Out of scope to merge them; this proposal preserves the existing structure.
