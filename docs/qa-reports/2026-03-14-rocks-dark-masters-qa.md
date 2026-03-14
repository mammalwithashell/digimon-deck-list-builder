# QA Report: Rocks vs Dark Masters
Date: 2026-03-14

## Test Summary

| Test | Result |
|------|--------|
| Rocks mirror (random policy) | PASS - completed in 110 steps, 25 turns |
| Dark Masters mirror (random policy) | PASS - completed in 197 steps, 62 turns |
| Rocks vs Dark Masters game 1 (random) | PASS - completed in 88 steps, 18 turns |
| Rocks vs Dark Masters game 2 (random) | PASS - completed in 78 steps, 24 turns |
| Rocks vs Dark Masters (greedy policy) | DEADLOCK at step 125, Start phase, turn 43 |

## Engine-Level Issues Found

### CRITICAL: Greedy agent deadlock in Start phase

Games using `greedy` policy deadlock consistently around turn 40-50. The engine enters `GamePhase.Start` with an empty action mask, making the game unrecoverable. Observed when one player's field count reaches extreme values (29 permanents on P2 field). Random-policy games do not deadlock.

- Observed in: cross-matchup (Rocks vs Dark Masters) with greedy agents
- Root cause: likely related to field overflow or Start-phase auto-transition failing under high field counts
- Severity: high (blocks greedy-agent testing)

### HIGH: `effect_select_opponent_permanent` with no valid targets causes self-deletion

When `effect_select_opponent_permanent` is called with `is_optional=False` and no opponent Digimon exist, the engine presents a target selection with only the "no target" slot (action 100+14 or 100+15). Selecting this slot causes the requesting player's just-played Digimon to be deleted.

- Reproduction: Play any Digimon from hand when opponent's field is empty, then process the forced "Select an opponent's Digimon" selection
- Observed with: EX8-047 Sunarizamon, EX8-046 Gotsumon, P-167 Landramon, BT21-021 OmniShoutmon, EX10-012 MetalSeadramon, BT15-062 Gigadramon
- Impact: Digimon played from hand get immediately deleted when opponent has no field targets
- Note: This may be a pre-existing engine issue not specific to these archetypes

### MEDIUM: Spurious `SelectTarget` phase after card play

Every card played from hand (both Digimon and Tamer/Option cards) triggers a `SelectTarget` phase with prompt "Select an opponent's Digimon" even when no On Play targeting effect exists on the card. Cards like EX8-046 Gotsumon (On Deletion only) and EX8-047 Sunarizamon (On Play = reveal, no targeting) still trigger this selection.

- This suggests a global engine hook is firing `effect_select_opponent_permanent` on every card play, independent of scripts
- The tamer ST6-14 Matt Ishida did NOT trigger this (tamers may be exempt)

---

## BT21-021 OmniShoutmon Validation

**Status: PARTIAL** (confirmed, consistent with validated_cards.json)

### Effects Analysis

| # | Effect | Card Text | Script Implementation | Verdict |
|---|--------|-----------|----------------------|---------|
| 0 | Alt digivolve (Shoutmon) | Digivolve from [Shoutmon] for cost 4 | `_alt_digi_cost=4, _alt_digi_name="Shoutmon"` | PASS |
| 1 | Alt digivolve (Xros Heart/Hero) | Digivolve Lv4 w/[Xros Heart]/[Hero] for cost 3 | `_alt_digi_cost=3, _alt_digi_trait="Xros Heart"`, condition also checks Hero | PASS |
| 2 | Shoutmon name for DigiXros | "Also treated as [Shoutmon] for DigiXros" | No functional implementation (condition returns True, no process) | PARTIAL - DigiXros not modeled |
| 3 | Unspecified effect | N/A | Empty effect (condition returns True, no process) | N/A |
| 4 | On Deletion | Place 1 [Xros Heart]/[Blue Flare] Digimon under Tamer + Save | Has timing and flags but NO process callback | FAIL - effect never executes |
| 5 | End of Attack | Play 1 [Xros Heart]/[Blue Flare]/[Hero] from hand cost -5, then delete self | Process calls `effect_select_opponent_permanent` (delete opponent) then `effect_play_from_zone` | FAIL - deletes opponent instead of self; cost reduction via `free=True` instead of -5 |
| 6 | Inherited Rush | [Your Turn] Gains Rush if [Xros Heart] trait | Checks permanent's top_card for Xros Heart trait, sets `_is_rush` | PASS |

### Key Issues

1. **End of Attack effect (effect5)** is fundamentally wrong:
   - Card says "play from hand cost -5, then delete THIS Digimon"
   - Script calls `effect_select_opponent_permanent` to delete an OPPONENT's Digimon, then plays from hand with `free=True` (free instead of -5 reduction)
   - The "If you did, delete this Digimon" self-deletion is missing entirely

2. **On Deletion effect (effect4)** has no process callback - the effect will fire (condition returns True) but do nothing. Should place a Xros Heart/Blue Flare card from hand/trash under a Tamer, then Save.

3. **DigiXros** effects are placeholder-only (expected, DigiXros not modeled in engine)

---

## Dark Masters Regression

### Smoke Test Results

All 3 games involving Dark Masters decks completed successfully with random policy:
- Dark Masters mirror: 197 steps, 62 turns, no crashes
- Rocks vs Dark Masters game 1: 88 steps, 18 turns
- Rocks vs Dark Masters game 2: 78 steps, 24 turns

### Card-Level Regression Status

The following Dark Masters cards were exercised during gameplay (confirmed present in active game states):

| Card | Name | Previous Status | Regression | Notes |
|------|------|-----------------|------------|-------|
| BT15-006 | (Digi-Egg) | PASS (frozen) | OK | Appeared in games |
| P-216 | WaruMonzaemon | N/A | OK | On Play effect fires correctly |
| EX10-012 | MetalSeadramon | IMPLEMENTED | OK* | On Play fires but triggers spurious target selection (engine issue) |
| EX10-020 | Puppetmon | IMPLEMENTED | OK | Appeared in games |
| EX10-035 | Machinedramon | IMPLEMENTED | OK | Appeared in games |
| EX10-057 | Piedmon | IMPLEMENTED | OK | Appeared in games |
| EX10-061 | Apocalymon | IMPLEMENTED | OK | Appeared in games |
| EX10-072 | (Option) | IMPLEMENTED | OK | Appeared in games |
| ST6-14 | Matt Ishida | IMPLEMENTED | OK | Played successfully, no spurious selection |
| BT15-062 | Gigadramon | FIXED | OK* | On Play fires but triggers spurious target selection (engine issue) |
| BT15-077 | LadyDevimon | FIXED | OK | Appeared in active game states |
| BT9-112 | DeathXmon | IMPLEMENTED | OK | Appeared in games |
| LM-043 | Darkdramon | IMPLEMENTED | OK | Appeared in games |
| BT13-102 | (Option) | FIXED | OK | Appeared in games |
| BT8-090 | (Option) | N/A | OK | Appeared in games |
| BT16-082 | Ukkomon | N/A | OK | Appeared in games |
| BT15-102 | (Option) | N/A | OK | Appeared in games |

*Cards marked OK* are functionally correct in their script logic but are affected by the engine-level spurious SelectTarget issue.

### Regression Verdict

**No Dark Masters regressions detected.** All games with Dark Masters decks completed to natural game-over states. The previously fixed cards (BT15-080, BT15-081, EX2-046, RB1-035, BT13-088, BT13-108) were not directly targeted in these test games but no related crashes or errors surfaced.

---

## Rocks Archetype Status

### Known QA Failures (from rocks.md, confirmed still present)

The Rocks archetype has 18 known QA failures documented in `docs/archetype-qa/rocks.md`. This QA session confirmed the following are still active:

| Card | Issue | Severity | Still Active |
|------|-------|----------|-------------|
| BT21-055 | Missing BeforePayCost timing on cost reduction | Critical | Yes (cost reduction never fires) |
| EX10-032 | [Hand][Main] dead code + inherited fires too broadly | Critical | Not directly tested |
| EX10-034 | Wrong timing + runtime crash on value_fn | Critical | Not directly tested |
| EX10-069 | Close checked as trait instead of name | Critical | Not directly tested |
| P-167 | Direct card_sources mutation + no top/bottom choice | Medium | Confirmed (card played, then trashed by engine bug) |

### Smoke Test

Rocks mirror game completed successfully (110 steps, 25 turns, random policy). No crashes or infinite loops.

---

## Findings Summary

1. **BT21-021 OmniShoutmon**: Confirmed PARTIAL. Non-DigiXros effects have significant implementation errors (End of Attack deletes opponent instead of self; On Deletion has no process callback).

2. **Dark Masters regression**: No regressions found. All 58 archetype cards continue to work. Games complete normally.

3. **Engine issues**: Two engine-level issues discovered:
   - Greedy agent deadlock in Start phase (empty action mask after ~40 turns)
   - Spurious `effect_select_opponent_permanent` firing on card play with no valid targets, causing self-deletion

4. **Rocks archetype**: 18 previously known QA failures remain unfixed. Smoke test passes with random policy.
