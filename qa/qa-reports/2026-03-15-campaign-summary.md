# Cross-Archetype Matchup QA Campaign Summary
Date: 2026-03-15

## Overview

8 matchup pairs covering all 17 priority archetypes tested via automated regression (160+ games) and targeted debug testing (30+ debug games). Each matchup was handled by a dedicated QA agent.

## Results at a Glance

| # | Matchup | Regression | Crashes | Debug Games | New Bugs |
|---|---------|-----------|---------|-------------|----------|
| 1 | TS Neptunemon vs Millenniummon | 30/30* | 0* | 6 | 2 (1 CRITICAL, 1 MEDIUM) |
| 2 | Hudiemon vs Zephagamon | 11/20 | 9 | 5 | 2 (1 CRITICAL, 1 LOW) |
| 3 | Puppets vs TS Olympos | 50/50 | 0 | 3 | 0 |
| 4 | BG Imperial vs Galacticmon | 20/20* | 0* | 4 | 1 CRITICAL (shared) |
| 5 | Jesmon vs Dark Masters | 8/20 | 12 | 5 | 4 (2 CRASH, 2 FAIL) |
| 6 | Rocks vs Chaos Control | 20/20 | 0 | 3 | 7 (5 HIGH, 2 MEDIUM) |
| 7 | DNA Omnimon vs Medusamon | 20/20 | 0 | 4 | 3 (1 CRITICAL, 2 FAIL) |
| 8 | TS Jupitermon vs Royal Knights | 20/20 | 0 | 6 | 4 (11 individual bugs) |

*After removing BT18-100 which crashes on load

**Totals: ~190 automated games, ~36 debug games, 23 distinct card-level issues found**

## Critical / Crash Issues (Fix Immediately)

### 1. BT18-100 "Gospel of the Fallen Angel" — `EffectTiming.DelaySkill` Missing
- **File:** `digimon_gym/engine/data/scripts/bt18/bt18_100.py` line 87
- **Impact:** Crashes every game where card is drawn/loaded. Blocks Millenniummon + Galacticmon matchups entirely.
- **Found by:** Agent 1, Agent 4
- **Fix:** Use `EffectTiming.OnStartMainPhase` with `_is_delay_effect = True` and `_is_field_main = True`

### 2. BT24-056 Dezipmon — `ModifierType.CANNOT_BE_RETURNED` Missing
- **File:** `digimon_gym/engine/data/scripts/bt24/bt24_056.py` lines 73, 110
- **Impact:** 45% crash rate in Zephagamon matchups. 25 scripts across bt10-bt24 reference this nonexistent modifier.
- **Found by:** Agent 2
- **Fix:** Add `CANNOT_BE_RETURNED` to `ModifierType` enum, or batch-replace with `CANNOT_BE_REMOVED` in all 25 scripts

### 3. BT15-069 Candlemon — `player.is_player_one` Missing
- **File:** `digimon_gym/engine/data/scripts/bt15/bt15_069.py` line 42
- **Impact:** Crashes 30-40% of Dark Masters games
- **Found by:** Agent 5
- **Fix:** Replace `player.is_player_one` with `player.player_id == 1`

### 4. BT14-044 Palmon — `card_name` vs `card_names`
- **File:** `digimon_gym/engine/data/scripts/bt14/bt14_044.py` line 80
- **Impact:** Crashes when granted OnTappedAnyone effect fires (50% of random games)
- **Found by:** Agent 5
- **Fix:** Change `top_card.card_name` to `top_card.card_names[0]`

### 5. 7 Uppercase Script Filenames — Silent Import Failure
- **Files:** `BT24_016.py`, `BT24_082.py`, `BT24_089.py`, `BT21_072.py`, `EX8_074.py`, `EX9_013.py`, `P_206.py`
- **Impact:** Cards operate as vanilla with zero effects (3 in Medusamon deck: Lamiamon, Owen Dreadnought, Unique Emblem)
- **Found by:** Agent 7
- **Fix:** `git mv` each file to lowercase

## QA-FAIL Cards (Script Bugs, Not Crashes)

| Card ID | Card Name | Archetype | Issue | Agent |
|---------|-----------|-----------|-------|-------|
| BT18-073 | Machinedramon | Millenniummon | Cost reduction without deletion cost | 1 |
| BT23-097 | Seventh Penetration | Hudiemon | Missing level >= hand size filter | 2 |
| BT20-013 | BaoHuckmon | Jesmon | [Main] effect unreachable (systemic) | 5 |
| BT23-076 | Sistermon Blanc | Jesmon | On Play completely wrong (wrong zone, order, harms opponent) | 5 |
| BT23-057 | Gankoomon | Jesmon | Process callback never executes (systemic BeforePayCost) | 5 |
| P-216 | WaruMonzaemon | Dark Masters | OnEnterFieldAnyone fires on ALL plays, causes mass deletion | 5 |
| BT21-029 | Medusamon | Medusamon | Petrification tokens never generated (2 sub-bugs) | 7 |
| BT24-071 | Raidramon | Chaos Control | SA+1 is no-op (`pass`), On Deletion filter missing | 6 |
| BT24-079 | Hadesmon | Chaos Control | When Digivolving no timing, link not implemented | 6 |
| EX10-054 | VenomMyotismon | Chaos Control | Suspend 1 not 2, cannot-unsuspend on self not target | 6 |
| BT20-073 | MetalPhantomon | Chaos Control | Skips self-deletion cost, no level filter | 6 |
| EX8-057 | DemiDevimon | Chaos/Rocks | On Play pops from trash not deck, wrong filter, 1 not 2 | 6 |
| BT24-097 | Soul Fear | Chaos Control | Fabricated When Attacking effect | 6 |
| BT24-074 | SkullSeadramon | Chaos Control | Wrong mechanic (delete vs trash digi), plays from hand not trash | 6 |
| BT13-086 | Gizmon: XT | Royal Knights | Free cost reduction, overbroad play filters (3 bugs) | 8 |
| BT24-098 | Invasion of the Titans | TS Jupitermon | Trashes 1 not 2, missing Titan filter (4 bugs) | 8 |
| BT13-100 | Yoshino Fujieda | Royal Knights | Wrong trigger type, suspends opponent not self | 8 |
| BT13-036 | Liollmon | Royal Knights | Auto-selects DP target, missing security condition | 8 |

## Systemic Engine Issues

### BeforePayCost Process Callbacks Never Execute
- `calculate_play_cost()` reads `cost_reduction` but never calls `on_process_callback`
- Affects: BT23-057 Gankoomon, BT13-086 Gizmon:XT, BT18-073 Machinedramon, potentially many more
- Players get cost reductions for free without paying the intended deletion/trash costs

### BT20-013 [Main] Effect Unreachable
- Despite `_is_field_main = True`, action mask doesn't present [Main] activation
- Systemic OnDeclaration timing issue (97+ scripts affected per prior report)

## Positive Findings

### RecursionError RESOLVED
- Puppets token deletion chain: **0/50 games (0%)** vs previous 45% baseline
- Stress test with `sys.setrecursionlimit(200)`: still 0% — fully fixed

### BT12-021/BT12-047 Upgraded FAIL → PASS
- Previous false positive from bad test setup (skip_shuffle placed no valid targets in top 3)
- Both cards' On Play reveal-and-select effects work correctly

### Name Aliasing Working
- BT17-015 WarGreymon inherited effect fires correctly under Omnimon
- `contains_card_name('Omnimon')` and `also_treated_as_names` system confirmed working

### Stable Cross-Archetype Interactions
- TS tamer auras + RK keyword grants coexist without conflicts
- DigiXros mechanics in Rocks work correctly
- Alliance mechanic in Hudiemon/Puppets working
- Ignore-color-requirement working (BT23-094 Nanomachine Break)

## Win Rate Summary

| Matchup | Dominant Archetype | Win Rate | Policy |
|---------|-------------------|----------|--------|
| Puppets vs TS Olympos | Puppets | 67.5% | Both |
| TS Neptunemon vs Millenniummon | Neptunemon | ~80% | Greedy |
| Chaos Control vs Rocks | Chaos Control | 90% | Both |
| DNA Omnimon vs Medusamon | Balanced | ~50% | Both |
| BG Imperial vs Galacticmon | Balanced | ~55% | Both |
| Royal Knights vs TS Jupitermon | Royal Knights | 80% | Both |
| Jesmon vs Dark Masters | ~Even | ~57% Jesmon | Greedy (low N) |
| Hudiemon vs Zephagamon | Hudiemon | 100% | Greedy |

## Reports Generated
1. `2026-03-15-ts-neptunemon-millenniummon-qa.md`
2. `2026-03-15-hudiemon-zephagamon-qa.md`
3. `2026-03-15-puppets-ts-olympos-qa.md`
4. `2026-03-15-bg-imperial-galacticmon-qa.md`
5. `2026-03-15-jesmon-dark-masters-qa.md`
6. `2026-03-15-rocks-chaos-control-qa.md`
7. `2026-03-15-dna-omnimon-medusamon-qa.md`
8. `2026-03-15-ts-jupitermon-royal-knights-qa.md`

## Priority Fix Order
1. **BT18-100** — unblocks Millenniummon/Galacticmon (1-line fix)
2. **BT24-056 / CANNOT_BE_RETURNED** — unblocks 25 scripts (enum addition)
3. **7 uppercase filenames** — unblocks 7 scripts (git mv)
4. **BT15-069 + BT14-044** — unblocks Dark Masters (2 simple fixes)
5. **P-216 WaruMonzaemon** — stop mass self-deletion
6. **BT23-076 Sistermon Blanc** — complete rewrite needed
7. **Chaos Control batch** — 7 cards need varying levels of rewrite
8. **Royal Knights batch** — 4 cards need fixes
9. **Systemic: BeforePayCost process callbacks** — engine-level fix
