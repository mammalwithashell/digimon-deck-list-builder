# QA Report: BG Imperial vs ExMaquinamon
Date: 2026-03-14

## Matchup Overview
- **Player 1**: BG Imperial (25 unique cards, 55-card deck with eggs)
- **Player 2**: ExMaquinamon (16 unique cards, 50-card deck with eggs)
- **Method**: Debug API games with targeted card injection, code review of all scripts, and automated simulation batches

## Simulation Results
- BG Imperial vs ExMaquinamon: 5/5 games completed, no crashes
- ExMaquinamon mirror: 5/5 games completed, no crashes
- BG Imperial mirror: 5/5 games completed, no crashes

---

## BG Imperial Card Results

### PASS (21 cards)

| Card ID | Name | Notes |
|---------|------|-------|
| BT3-002 | DemiVeemon | Inherited OnUseAttack draw 1 if Jamming, once-per-turn limiter correct |
| BT12-002 | DemiVeemon | Inherited OnUseAttack draw 1 if green Digimon in play, once-per-turn correct |
| P-117 | Veemon | BeforePayCost digivolution -1 for Free trait with tamer check, leak guard present. Inherited draw if 2+ colors correct |
| EX1-014 | ExVeemon | Unconditional Jamming keyword + inherited conditional Jamming for Imperialdramon/Free trait |
| ST9-09 | Stingmon | Cost reduction verified: paid 3 instead of 4 with blue Digimon on field. Inherited draw 1 if blue Digimon correct |
| BT12-022 | ExVeemon | WhenDigivolving DNA memory +1 for green correct. Inherited Jamming for Imperialdramon/Free correct |
| BT12-050 | Stingmon | WhenDigivolving DNA memory +1 for blue correct. Inherited Piercing for Imperialdramon/Free correct |
| BT16-025 | Paildramon | Previously validated. Partition, WD suspend by digi-card count, DNA cannot unsuspend all, WA suspend/unsuspend |
| BT16-027 | Imperialdramon: Fighter Mode (ACE) | Alt-digi from Dragon Mode cost 2, Blast Digivolve, On Play/WD bottom deck by digi-card count, End of Attack unsuspend + conditional bottom deck |
| BT16-028 | Imperialdramon: Dragon Mode | Alt-digi Paildramon + Dinobeemon cost 3, WD freeze + optional suspend trade, All Turns reactive trigger for Fighter Mode digivolve |
| BT20-020 | Imperialdramon: Fighter Mode | Alt-digi Dragon Mode cost 2, Raid, Piercing, WD play restriction + conditional security trash, OnLoseSecurity delete by DP comparison |
| BT12-028 | Paildramon | WD trash top 3 digi-cards from all opponent Digimon + DNA cannot attack. Inherited End of Attack memory gain |
| BT12-031 | Imperialdramon: Fighter Mode | Alt-digi Dragon Mode cost 2, WD suspend no-digi opponents + branch (Dragon Mode return for deck-bottom-all vs bounce-1), dynamic DP per digi-stack color, conditional SA+1/Blocker at 2+ colors |
| BT21-037 | Lighdramon | Alt-digi from Veemon cost 2, Piercing, Armor Purge, WD suspend + DP change |
| BT16-040 | Wormmon | Alt-digi from Minomon cost 0, inherited suspend 1 opponent once-per-turn, Start of Main + On Play trash digivolve into Lv.4 Insectoid/Free |
| ST9-06 | Imperialdramon Dragon Mode | WD play 1 blue Lv4 and 1 green Lv4 from digi-stack free |
| BT16-085 | Davis Motomiya & Ken Ichijoji | Play cost 4 verified. Security play, Start of Main play Veemon/Wormmon free with end-of-opponent-turn bounce, digivolve memory trigger + DNA trash sub-effect |
| BT3-093 | Davis Motomiya | Previously validated and fixed |
| BT3-103 | Hidden Potential Discovered! | Previously validated |
| BT17-077 | Imperialdramon: Paladin Mode | Previously validated |
| BT17-097 | Return to the Primogenitor | Previously validated and fixed |
| LM-030 | Green Scramble | Previously validated |

### FAIL (2 cards)

#### BT12-021 Veemon -- FAIL (On Play reveal not firing)
- **Severity**: High
- **Card text**: "[On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with [Imperialdramon] in its name or a [Free] trait and 1 Tamer card with [Davis Motomiya] in its name among them to your hand. Place the remaining cards at the bottom of your deck in any order."
- **Observed**: After playing BT12-021, library size remains unchanged (40 cards). No selection phase triggered, no cards added to hand. The On Play effect is silently not executing.
- **Script analysis**: Effect uses `EffectTiming.OnEnterFieldAnyone` with `is_on_play = True` and has a valid process callback calling `game.effect_reveal_and_select_multi()`. The condition checks `card.permanent_of_this_card() is None` which should find the permanent after play. The script structure matches other working On Play effects.
- **Root cause hypothesis**: Unclear. The effect metadata (timing, flags, condition, process callback) all appear correct. The engine's `_effect_matches_timing` and `_collect_triggered_effects` flow should pick it up. May be an issue with effect ordering or the condition check failing silently.

#### BT12-047 Wormmon -- FAIL (On Play reveal not firing)
- **Severity**: High
- **Card text**: "[On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with [Imperialdramon] in its name or a [Free] trait and 1 Tamer card with [Ken Ichijoji] in its name among them to your hand. Place the remaining cards at the bottom of your deck in any order."
- **Observed**: Same behavior as BT12-021 -- no reveal, no cards added. Library unchanged.
- **Script analysis**: Identical structure to BT12-021 (same `effect_reveal_and_select_multi` call pattern). Same root cause.
- **Note**: Both BT12-021 and BT12-047 share the exact same On Play pattern. Fixing one should fix both.

### Known Issues (from previous QA)

| Card ID | Severity | Issue |
|---------|----------|-------|
| BT16-040 | Medium | perm_filter allows selecting any Digimon regardless of whether the chosen trash card legally digivolves onto it |
| BT21-037 | Medium | DP +2000 applied without duration (should expire end of opponent turn); effect order wrong (DP before suspend) |
| BT12-028 | Low | Inherited End of Attack condition checks whole digi-stack via contains_card_name instead of only top card |
| ST9-06 | Medium | Auto-selects first qualifying blue/green Digimon from digi-stack instead of player selection |
| BT3-093 | Medium | Uses OnStartMainPhase instead of OnStartTurn; auto-picks first matching cards |
| LM-030 | Low | Delay activation auto-picks first green Digimon; spurious cost_reduction on OptionSkill |
| BT17-077 | Low | When Attacking unsuspend fires unconditionally even if bounce fails |

---

## ExMaquinamon Regression Results

All 16 ExMaquinamon cards maintain their previously validated PASS status. No crashes or regressions detected in simulation testing.

| Card ID | Name | Status |
|---------|------|--------|
| EX11-006 | Flickmon | PASS |
| EX11-027 | Maquinamon | PASS |
| EX11-029 | Turbomon | PASS |
| EX11-033 | Maneuvermon | PASS |
| EX11-036 | Dalphomon | PASS |
| EX11-040 | Mulemon | PASS |
| EX11-042 | MockingBirdmon | PASS |
| EX11-045 | Metatromon | PASS |
| EX11-062 | Shoto Kazama | PASS |
| EX11-070 | Unchained | PASS |
| EX11-071 | Cool Boy | PASS |
| EX11-073 | ExMaquinamon | PASS |
| EX6-072 | Mega Digimon Assembly! | PASS |
| LM-048 | Chrome Memory Boost! | PASS |
| P-151 | Digimon Liberator | PASS |

**Blocked engine gaps** (unchanged):
- EX11-036: force_attack -- no engine API
- EX11-042: redirect_attack -- no engine API
- EX11-070: DP floor -- registered but not enforced

---

## Summary

| Archetype | PASS | FAIL | Total |
|-----------|------|------|-------|
| BG Imperial | 23 | 2 | 25 |
| ExMaquinamon | 15 | 0 | 15 (regression) |

**New failures found**: BT12-021 and BT12-047 both have non-firing On Play reveal effects. These are the same bug (identical script pattern) and should be investigated as a single engine-level or script-level issue.

**Action items**:
1. Investigate why BT12-021/BT12-047 On Play `effect_reveal_and_select_multi` effects do not fire despite correct metadata
2. Once fixed, both cards should transition to PASS
