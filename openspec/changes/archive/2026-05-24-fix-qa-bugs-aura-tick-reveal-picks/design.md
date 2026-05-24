## Context

Three bugs were uncovered during a May 24 2026 MCP-driven QA session against the freshly-landed TS Olympos and Rocks Rust ports. Each sits at a different layer of the stack:

- **LiveGame action wrappers** (`code/digimon-engine/src/live_game.rs`) — the typed API used by the MCP and `code/digimon-engine-py` bindings. `play()` and `move_from_breeding()` call into `Game::play_from_hand` / `Game::move_from_breeding` directly. Wrappers that route through `decode_action` / `step` (`pass_turn`, `digivolve`, `attack`, `step`) get `tick_declarative_effects()` for free via `decode_action`'s pre+post tick discipline (`code/digimon-engine/src/action/decode.rs:36,39,72`). The direct callers do not — filter-auras like Homeros's `[All Turns] +1000 DP to TS Digimon` therefore install one MCP action late.

- **DSL card-scripting vocabulary** (`code/digimon-dsl/` + `code/digimon-engine/src/dsl_cards/`) — the YAML primitives available to card authors. Two distinct primitives exist for the "reveal top 3, pick by trait" pattern:
  - `select_reveal_buckets` (per-bucket `min`/`max`, no `optional` flag — mandatory by default; used correctly by BT24-031 Elecmon)
  - `choose_from_reveal` (one selection per pick, supports an `optional` flag — used by EX8-047 Sunarizamon and P-167 Landramon, both incorrectly setting `optional: true` on what printed text marks as mandatory adds)

- **Specific card YAMLs** — EX8-047 and P-167 are the only two cards in the repo currently using `choose_from_reveal` (`grep -l "choose_from_reveal" code/digimon-engine/cards/` returns those two files). Both author mis-uses of the `optional` flag.

The QA session also confirmed the engine's tick discipline DOES correctly produce buffed effective DP during combat — because attack resolution flows through `decode_action`, which ticks. So the LiveGame gap is invisible during normal gameplay but corrupts every same-tick observability and ANY [On Play] effect that queries opponent DP via `effective_dp` before the next decode_action boundary.

## Goals / Non-Goals

**Goals:**

- Close the LiveGame action-wrapper tick gap so static-aura state is consistent at every API boundary, not just every `decode_action` boundary.
- Restore the no-approximations contract on EX8-047 and P-167 — eligible reveal-search picks become mandatory, matching printed text.
- Add a spec rule + DSL guidance so future `choose_from_reveal` mis-use is caught in review instead of in QA.
- Add behavioral regression tests so the three bugs cannot silently regress.

**Non-Goals:**

- Refactoring the `choose_from_reveal` vs `select_reveal_buckets` primitive split. Two primitives is fine; the bugs are author-side mis-use, not a missing primitive. Migrating EX8-047 to `select_reveal_buckets` is the correct fix because it's the closer semantic match (two buckets, one selection prompt).
- Reworking `tick_declarative_effects()` itself. The tick machinery is correct; we just need to invoke it consistently.
- Tracking down EVERY card YAML that might have a similar `optional` mis-use. The grep result for `choose_from_reveal` shows only EX8-047 and P-167 today. A broader audit is worth doing but out of scope for this fix.
- Changing combat DP resolution. It already calls `effective_dp` correctly through the ticked path.

## Decisions

**Decision 1 — fix `LiveGame::play` by adding a post-action tick rather than routing through `decode_action`.**

Considered: routing `LiveGame::play` through `decode_action` like `digivolve` does. Rejected because `play` validates phase + decision-player + hand bounds at the wrapper layer (lines 638–668) before dispatching, which `decode_action` deliberately does not (per the doc-comment at lines 623–633, the wrapper IS the sole enforcement point for phase). Re-routing through `decode_action` would duplicate validation or require lifting the wrapper's checks into the action decoder, both worse than a one-line tick call.

The fix is: append `self.game.tick_declarative_effects();` after `self.game.play_from_hand(player, hand_idx)` returns successfully (line 671) and before `make_result`. Mirror the same pattern in `move_from_breeding` (line 764).

For `resolve_selection` (line 682) and `end_turn` (line 707): audit whether the underlying engine methods already invoke `tick_declarative_effects` (e.g., `Game::end_turn` likely already ticks as part of phase transition). If they do, no change needed. If not, add post-action ticks. The audit lands as a separate task in tasks.md so a check happens even if the fix is a no-op.

**Decision 2 — fix EX8-047 by migrating to `select_reveal_buckets`, fix P-167 by setting `optional: false`.**

EX8-047 is structurally "Add 1 X and 1 Y from top-3" — exactly what `select_reveal_buckets` was designed for. Migrating it brings the implementation in line with BT24-031 Elecmon (the working reference) and lets the engine surface one unified bucket selection prompt with `no_duplicate_cards: true`, which matches the printed-text intent.

P-167 is structurally different — there's a player-chosen destination (hand vs bottom-source), so the YAML branches on `dest_choice` and uses `choose_from_reveal` per branch. Migrating to `select_reveal_buckets` would require a primitive extension. The minimal correct fix is to drop `optional: true` from both `choose_from_reveal` calls at lines 66 and 80 of the YAML. The top-level effect's `optional: true` (line 35) is correct and stays — the source-trash cost is itself optional, only the post-cost-paid reveal pick becomes mandatory.

**Decision 3 — add a single new requirement to `dsl-card-scripting-vocabulary` covering `choose_from_reveal` optional semantics.**

The spec delta says: `choose_from_reveal { optional: true }` is permissible ONLY when the printed card text explicitly grants the player a "may" at that specific pick. When the printed card text is "Add 1 card with the [X] trait...", the pick is mandatory and `optional` MUST be `false` (or omitted, since `false` is the default). The natural fizzle behavior (zero eligible candidates → bucket auto-skips) handles the "no candidates" case without needing a player-driven decline.

This rule is asymmetric with `choose_from_reveal`'s `optional` field default, but matches the printed-text reading and the no-approximations policy in `CLAUDE.md`.

**Decision 4 — behavioral tests live in `tests/cards_behavioral/` under each card's set folder.**

Tests for EX8-047 and P-167 follow the established `bt24/bt24_046.rs` pattern (DebugRunner setup, place card on field, execute trigger, assert pending selection state). The Homeros aura tick test is a fresh integration test asserting that calling `LiveGame::play(Homeros)` while a TS Digimon is on field makes `modifiers(handle)` reflect the ChangeDp +1000 entry within the same call's return — no follow-up action needed to materialize it.

## Risks / Trade-offs

- **[Risk] Other LiveGame wrappers may have the same gap and go unnoticed.** → Mitigation: tasks.md explicitly schedules an audit of `resolve_selection()` and `end_turn()`. If those are also buggy, the same one-line fix applies and tests are added.
- **[Risk] `tick_declarative_effects` is heavier than expected at high-volume call sites.** → Mitigation: the tick already runs on every `decode_action` (which is every gameplay action in normal flow), so adding it to `play` and `move_from_breeding` is at most a 2× increase in tick frequency for those specific call paths, and only for callers that bypass `decode_action`. RL training uses `step()` → `decode_action`, unchanged. No production hot-path is affected.
- **[Risk] Migrating EX8-047 to `select_reveal_buckets` changes the player-visible prompt shape from two sequential `choose_from_reveal` prompts to one combined bucket prompt.** → Mitigation: that's the correct UX per BT24-031 Elecmon precedent. The MCP `pending_selection` callers (humans + RL agents) already handle the bucket shape; no agent retraining needed.
- **[Trade-off] We're leaving the broader `choose_from_reveal { optional: true }` audit out of scope.** Acceptable because today only EX8-047 and P-167 use this primitive, and the new spec rule + author guidance prevents future mis-use.

## Migration Plan

No migration is required. The fixes are:

1. LiveGame: additive (new tick calls) — zero behavior change for callers that don't observe modifiers state immediately after a play/move.
2. EX8-047 / P-167: behavior change for cards in-flight, but only in the sense that ineligible "decline the mandatory pick" inputs now produce a fizzle / engine-rejection instead of a silent skip. No saved state to migrate.
3. Spec deltas: documentation-only.

Rollback: revert the YAMLs and the `live_game.rs` edits. No persistent state to undo.

## Open Questions

- Do `LiveGame::resolve_selection()` and `LiveGame::end_turn()` need post-action ticks too? Answer is one focused audit task in tasks.md.
- Should we add a lint pass over all YAMLs that flags `choose_from_reveal { optional: true }` without a documented "may" annotation in the printed-text comment? Probably yes, but as a follow-up change once this fix lands and the new spec rule is the canonical reference.
