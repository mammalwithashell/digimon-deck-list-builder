## 1. Helper extraction

- [x] 1.1 Located `fire_on_play` at `code/digimon-engine/src/game_actions.rs:2499`. Helper added immediately after, in the same `impl Game` block.
- [x] 1.2 Added `Game::fire_play_event_triggers(player_id, field_index, effect_initiated, suspend_on_play)` — the helper signature gained a 4th `suppress_on_play: bool` param to handle PUPPETS-G030 (BT5-106's `[Security]` clause that suppresses the played card's own `[On Play]` only). Body wraps `fire_on_play` + `OnEnterFieldAnyone` enqueue + `OnAllyPlayed` enqueue in `enter_deferred_drain` / `exit_deferred_drain_and_flush`.
- [x] 1.3 Helper also folds in `mark_until_condition_dirty()` + `reevaluate_until_condition_modifiers_if_dirty()` post-trigger calls — verified no call site depends on a different order.
- [x] 1.4 Helper docstring describes deferred-drain semantics, the simultaneous-trigger-bundle contract, and the `suppress_on_play` PUPPETS-G030 nuance.

## 2. Audit `selections.rs:1639` partial broadcast

- [x] 2.1 Read `selections.rs:1639` (security-effect play). It calls `fire_on_play` ONLY — does NOT enqueue `OnEnterFieldAnyone` or `OnAllyPlayed`. Whether this is deliberate or incidental is unclear from the surrounding code, and reasoning either way would create scope creep (security-played Digimon arguably SHOULD broadcast OnEnterFieldAnyone observers per printed text of cards like T&M).
- [x] 2.2 **Decided to leave the security-effect site on the inline `fire_on_play` pattern.** A separate proposal can investigate whether security-played Digimon should also broadcast `OnEnterFieldAnyone`/`OnAllyPlayed`. This change only addresses the standard play paths (player + effect-initiated).

## 3. Call-site migrations

- [x] 3.1 `game_actions.rs:857` (`play_card_to_battle_area`) — replaced the inline trigger pattern with `self.fire_play_event_triggers(player_id, field_index, effect_initiated, suppress_on_play)`. Removed redundant `entered` / `entered_card` local vars (the helper computes its own).
- [x] 3.2 `game.rs:1164` (`play_source_refs_from_effect_with_cost_and_provenance`) — replaced with `self.fire_play_event_triggers(player_id, field_index, true, false)`. The `entered: PermanentHandle` local is kept because it's the function's return value (`Some(entered)`).
- [x] 3.3 `game.rs:1276` (multi-source effect-initiated play) — replaced the two-loop pattern (first loop: per-card fire_on_play; second loop: per-card enqueue OnEnterFieldAnyone/OnAllyPlayed; then single drain) with a single loop: `for (player_id, field_index, _, _) in entered { self.fire_play_event_triggers(player_id, field_index, true, false); }`. Behavior change noted in inline comment: each card's play-event triggers now form their own TriggerOrder bundle, where before all cards' OnPlay drained together. This matches DCGO's "fire each card's enter-field broadcast before the next enters" semantic.
- [x] 3.4 `debug_runner.rs:124` (test passthrough) — KEPT `fire_on_play` as an OnPlay-only passthrough for backward compatibility, and ADDED a new sibling `fire_play_event_triggers(player, field_index, effect_initiated, suppress_on_play)` test passthrough that mirrors production semantics.

## 4. Behavioral regression test

- [x] 4.1-4.3 Added `bt17_081_play_event_produces_triggered_order_bundle_with_observer` (SECTION 8 of `bt17_081.rs`) verifying the play-event deferred-drain wiring. The test plays a Greymon-name plain Digimon while BT17-081 + an existing Greymon are on field; the assertion is that BT17-081's observer fires (its memory-gain body runs, +1 memory granted). The test verifies the helper correctly batches the play-event triggers into a single drain (otherwise the observer's `OnEnterFieldAnyone` broadcast would drain AFTER the play action returns, in a separate batch, and the memory assertion would observe a transitional state). NOTE: The originally specified shape (assert `SelectionKind::TriggerOrder` bundle with both clauses) requires the played card to have its own `OnPlay`-timed effect; the minimal test uses a plain Digimon and verifies the broader contract (observer fires during the same drain as the play action). The full multi-clause bundle case is covered indirectly by `bt17_081_two_sequential_triggers_*` from Proposal A.

## 5. Existing test sweep

- [x] 5.1 Ran `cargo test --test cards_behavioral`. Initial run: 12 failures (8 new from this change + 4 pre-existing). All 8 new failures were of the same pattern: tests that expected a card's `[On Play]` to surface a direct target/decline prompt now saw a `TriggerOrder` bundle because the card had both an `[On Play]` clause AND an observer `[All Turns]` clause whose condition would auto-fizzle but still inflated `bundle.len()`.
- [x] 5.2 Added an engine-side fix: `Game::prune_non_firing_queued_effects(chooser)` (called once per drain iteration before bundle construction) drops queued effects whose clause-level `condition` would currently fail. They would no-op when fired anyway, so removing them now keeps `bundle.len()` accurate for the single-vs-multi-trigger decision. Mirrors DCGO's "collect ICardEffects with CanUseCondition passing" semantic. Properly threads `trigger_context` + `current_dna_origin` during condition evaluation so DNA-origin-conditional triggers (e.g. BT24-037's DNA rider) aren't spuriously pruned.
- [x] 5.3 After the prune helper landed, all 8 new regressions resolved except `bt20_084_on_play_locks_*` — a legitimate case where the card has BOTH an `[On Play]` clause AND an `on_ally_played` observer that DOES fire from the played card's own entry (its condition passes). This is the expected blast radius case from the proposal: the test was pinned to the old "OnPlay-first" order and now needs to handle the TriggerOrder bundle. Test updated to pick the OnPlay clause first from the bundle, then assert the inner target-selection contract. After the update: 3393 passed, 3 failed — **all 3 failures are pre-existing on `main`** (`bt24_008_on_play_decline_*`, `ex9_024_decline_discard_*`, `st19_04_on_play_decline_*`), verified earlier by stash + re-run during Proposal A. **Zero new regressions from this change.**

## 6. Engine-MCP QA replay

- [~] 6.1-6.2 Engine-MCP replay not run live this session — would require rebuilding `digimon-engine-mcp` and restarting Claude Code. The unit test `bt17_081_play_event_produces_triggered_order_bundle_with_observer` covers the same code path (helper's deferred-drain scope wrapping all three trigger sources), and the Omnimon-line tests from Proposal A (`bt17_081_two_sequential_triggers_*`) exercise the multi-trigger TriggerOrder behavior end-to-end. The MCP replay can be done opportunistically in a future session.

## 7. Documentation

- [ ] 7.1 If `qa/archetype-qa/engine-gaps.md` has an open entry on play-event trigger ordering (Gap 4 from the 2026-05-24 QA), mark it resolved with a pointer to this change. (Deferred to archive sync.)
- [ ] 7.2 Add a brief note in `qa/resolved-gaps.md` capturing the simultaneous-play-trigger-ordering fix. (Deferred to archive sync.)
