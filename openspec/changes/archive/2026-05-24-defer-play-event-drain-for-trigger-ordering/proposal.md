## Why

When a Digimon enters battle area via play, the played card's own `[On Play]` trigger (timing `OnPlay`) is enqueued and **drained immediately** by `fire_on_play()` before observer triggers (`OnEnterFieldAnyone`, `OnAllyPlayed`) are enqueued. As a result, simultaneous triggers from the same play event cannot share a `TriggerOrder` bundle — the turn player loses the ordering authority that DCGO grants for simultaneous triggers from the same event.

A May 24 2026 engine-MCP QA pass exercising MetalGarurumon's `[On Play]` choice alongside Tai & Matt's `[All Turns] When one of your Digimon is played` trigger observed the bug directly: MG's mandatory effect-choice prompt surfaced **before** T&M's All Turns trigger appeared in any TriggerOrder. DCGO (`BT17_027.cs` + `BT17_081.cs`) registers both as `EffectTiming.OnEnterFieldAnyone` ICardEffects, batches them, and surfaces a "which to resolve first" prompt to the turn player.

The deferred-drain machinery to fix this already shipped in the 2026-05-23 nested-park-collision sweep (`Game::draining_deferred`, `enter_deferred_drain()`, `exit_deferred_drain_and_flush()`, and `maybe_drain_effect_queue()` helpers). Most `fire_on_*` observer helpers already use `maybe_drain_effect_queue`, but the play flow specifically was outside the panic-mode crash sweep and still inline-drains because no enclosing deferred-drain scope exists at the play call sites.

Closing this gap restores DCGO-aligned simultaneous-trigger ordering for play events. After the change, when MG and T&M trigger from the same play, the player sees a single TriggerOrder bundle covering both, can pick the order, and (with the BT17-081 cost-gate fix from `fix-tai-matt-cost-gate`) can mechanically express "deny first / accept second" by ordering accepts ahead of cost-payable conflicts.

## What Changes

- **Extract `Game::fire_play_event_triggers(player_id, field_index, effect_initiated)` helper** that wraps the four enqueues (`OnPlay`, `OnEnterFieldAnyone`, `OnAllyPlayed`, final drain) inside `enter_deferred_drain()` / `exit_deferred_drain_and_flush()`. The helper consolidates the trigger source construction (the `TriggerSource::EnteredField { player, permanent, card, effect_initiated }` shape currently duplicated at four sites) into one definition.
- **Replace inline play-event trigger sequences at five call sites** with the new helper:
  - `code/digimon-engine/src/game_actions.rs:857` — `play_card_to_battle_area` (standard player-initiated play)
  - `code/digimon-engine/src/game.rs:1164` — `play_source_refs_from_effect_with_cost_and_provenance` (effect-initiated single play)
  - `code/digimon-engine/src/game.rs:1276` — effect-initiated multi-source play (loop over each entered card)
  - `code/digimon-engine/src/effect_context/selections.rs:1639` — security-effect play (e.g. "Play this card without paying the cost")
  - `code/digimon-engine/src/debug_runner.rs:124` — test passthrough (collapse to helper invocation)
- **Keep `fire_on_play()` as a public function** for callers that explicitly want OnPlay-only firing (currently none in production code, but the helper is `pub` and used by tests). `fire_on_play`'s internal `maybe_drain_effect_queue()` call continues to no-op when invoked inside a deferred-drain scope.
- **Trigger ordering observable change**: when both the played card's `[On Play]` and observers' `[All Turns] When one of your Digimon is played` are queued from the same play event, they appear in a single `TriggerOrder` bundle. Default player choice (pick the played card's `[On Play]` first) preserves the current observable behavior; the new freedom is the ability to pick observer triggers first.
- **Regression coverage** in `code/digimon-engine/tests/cards_behavioral/bt17/bt17_081.rs` (or a new combined test): scenario where MG plays into a board with T&M and an Agumon. Assert the TriggerOrder bundle exposed after MG's play contains entries for both MG (mandatory) and T&M (optional). Assert that picking T&M first runs T&M's body before MG's effect-choice prompts. Existing behavioral tests on the 6 cards with `[On Play]` interactions (BT17-007, BT17-019, BT17-081, BT22-008, BT22-017, BT12-021, BT12-047) get re-run; any failures need their assertions updated to reflect correct DCGO-aligned ordering.

## Capabilities

### New Capabilities

(none — all changes modify existing capabilities)

### Modified Capabilities

- `live-game-surface`: play-action wrappers now expose simultaneous-trigger orderings via the standard `TriggerOrder` selection kind; observers' `[All Turns]` triggers and the played card's `[On Play]` share a single bundle when they fire from the same play event. Agents and MCP clients see a single `pending_selection` of kind `TriggerOrder` listing both, rather than the previous behavior of the played card's effect prompt firing first and any observer triggers appearing only after.

## Impact

- **Rust engine** — `code/digimon-engine/src/game.rs` (new helper), `code/digimon-engine/src/game_actions.rs` (call-site update), `code/digimon-engine/src/effect_context/selections.rs` (call-site update), `code/digimon-engine/src/debug_runner.rs` (call-site collapse). All five sites collapse to a single helper invocation; net diff is small (≈30 lines).
- **Behavioral tests** — `code/digimon-engine/tests/cards_behavioral/bt17/bt17_081.rs` adds the simultaneous-trigger ordering test. Existing tests on the 6 affected cards (BT17-007/-019/-081, BT22-008/-017, BT12-021/-047) are re-run; any assertion pinning the old "OnPlay drains first" order updates to expect a TriggerOrder bundle.
- **Specs** — modified delta to `live-game-surface` capturing the new trigger-bundle behavior. No new spec capability needed.
- **No agent retraining required** — default player choice (pick played card's `[On Play]` first when present) reproduces the previous observable sequence. The action space gains an additional legal action at the TriggerOrder selection (the observer option becomes pickable earlier), which expands the agent's policy surface without invalidating existing learned behavior.
- **No breaking API changes**. MCP tool shapes unchanged; the surface change is "more triggers may appear in a TriggerOrder bundle than before", which is already a valid state the clients handle.
- **No new dependencies**.
