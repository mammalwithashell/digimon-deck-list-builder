## Why

A May 24 2026 MCP-driven QA pass on the TS Olympos and Rocks archetypes surfaced three faithfulness regressions:

1. **Filter-aura DP buffs don't install immediately** when their source enters play through `LiveGame::play()` — Homeros's `[All Turns] +1000 DP to TS Digimon` aura only materializes on the *next* MCP action's tick. Any [On Play] effect that queries `effective_dp` / `modifiers` between the play and the next `decode_action` boundary reads stale state. This is a real engine bug, not just an MCP visibility quirk.
2. **EX8-047 Sunarizamon** searcher lets the player decline picking eligible Mineral/Rock or LIBERATOR cards from the revealed top-3 — the YAML uses `choose_from_reveal { optional: true }` for both buckets. Printed card text says "Add 1 card..." with no "may", so per the no-approximations policy these picks are mandatory when eligible cards exist.
3. **P-167 Landramon** has the same shape after the source-trash cost is paid: both `choose_from_reveal` branches (add-to-hand vs place-as-bottom-source) are marked `optional: true`, letting players decline the mandatory reveal-and-place step.

Fixing all three preserves the no-approximations contract every RL agent and human player relies on, and closes a latent observability gap that could mask future static-aura regressions.

## What Changes

- **LiveGame action wrappers tick declarative effects post-action.** Add `tick_declarative_effects()` after the underlying engine mutation in `LiveGame::play()` (line 671) and `LiveGame::move_from_breeding()` (line 764). Audit `resolve_selection()` (line 682) and `end_turn()` (line 707) for the same gap and add ticks where missing. The wrappers that already route through `decode_action` / `step` (`pass_turn`, `digivolve`, `attack`, `step`) are unchanged.
- **EX8-047 Sunarizamon YAML migrates to `select_reveal_buckets`**, matching the pattern Elecmon BT24-031 uses correctly: a single two-bucket selection with `min: 1, max: 1` per bucket and no `optional` flag. Both picks become mandatory when eligible cards are present; the bucket auto-skips gracefully when no candidates match.
- **P-167 Landramon YAML drops `optional: true`** from both `choose_from_reveal` branches inside the post-cost-paid reveal-and-route step. The top-level effect remains `optional: true` (the source-trash cost is a "by trashing..." optional activation), but the reveal pick that *follows* cost payment becomes mandatory.
- **DSL vocabulary specification clarifies the rule.** Add a requirement to `dsl-card-scripting-vocabulary` that `choose_from_reveal { optional: true }` is only correct when the printed card text grants the player an explicit "may" at that pick. Mandatory adds default to `optional: false`; mandatory adds with no eligible candidates rely on natural fizzle, not a player-driven decline.
- **Regression coverage** in `code/digimon-engine/tests/`: behavioral tests pinning (a) Homeros aura installs on the same tick its source lands, (b) EX8-047 with one eligible Mineral/Rock candidate in top-3 forces the pick (PASS rejected), (c) P-167 with one eligible Mineral/Rock candidate post-cost-paid forces the pick.

## Capabilities

### New Capabilities

(none — all changes modify existing capabilities)

### Modified Capabilities

- `live-game-surface`: action wrapper requirement now mandates `tick_declarative_effects()` is invoked after every state-mutating action method, so filter-aura observers (modifiers view, combat DP, [On Play] queries) see consistent state at every MCP / API boundary.
- `dsl-card-scripting-vocabulary`: new requirement governing when `choose_from_reveal { optional: true }` is permissible, plus the guidance to prefer `select_reveal_buckets` for "Add 1 X and 1 Y" reveal-search patterns.

## Impact

- **Rust engine** — `code/digimon-engine/src/live_game.rs` (action wrappers gain post-tick calls); behavioral tests under `code/digimon-engine/tests/cards_behavioral/bt24/` (new Homeros aura test) and `code/digimon-engine/tests/cards_behavioral/ex8/` and `.../p/` (new EX8-047 + P-167 mandatory-pick tests).
- **Card YAMLs** — `code/digimon-engine/cards/ex8/EX8-047.yaml` (rewrite the On Play clause to `select_reveal_buckets`); `code/digimon-engine/cards/p/P-167.yaml` (drop `optional: true` on the two post-cost-paid `choose_from_reveal` calls).
- **Specs** — modified deltas to `live-game-surface` and `dsl-card-scripting-vocabulary` capturing the new requirements.
- **No frontend / API / training contract changes** — the LiveGame fix is internal observability hardening; combat DP resolution already correctly read the buff at attack-time through `decode_action`'s tick, so no agent retraining is needed.
- **No new dependencies.** No breaking changes to MCP tool shapes.
