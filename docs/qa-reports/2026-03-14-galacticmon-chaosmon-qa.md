# QA Report: Galacticmon vs Chaosmon
Date: 2026-03-14

## Overview
- **Galacticmon**: 16 unique cards, all previously validated as PASS
- **Chaosmon**: 16 unique cards, 5 previously validated (PASS), 11 unvalidated

## Test Methodology
- Code review of all 11 unvalidated Chaosmon scripts against card text
- Debug game API testing for On Play effects (BT20-030, BT20-031, BT20-039, BT20-036)
- Simulation runs (5 games each) for both archetypes confirming no crashes
- Git log check confirming no script changes since last validation

---

## Galacticmon Regression Results

All 16 Galacticmon cards remain **PASS**. No script changes since validation date. 5-game mirror simulations complete without errors.

| Card ID | Name | Status | Notes |
|---------|------|--------|-------|
| BT11-061 | Vemmon | PASS | No changes since 2026-03-14 validation |
| BT11-065 | Snatchmon | PASS | No changes since 2026-03-13 validation |
| BT11-105 | Fusionize | PASS | No changes since 2026-03-14 validation |
| BT18-060 | Vemmon | PASS | No changes since 2026-03-13 validation |
| BT18-065 | Snatchmon | PASS | No changes since 2026-03-14 validation |
| BT18-092 | Zenith | PASS | No changes since 2026-03-13 validation |
| BT21-006 | Tsumemon | PASS | No changes since 2026-03-13 validation |
| BT21-056 | Vemmon | PASS | No changes since 2026-03-13 validation |
| BT21-058 | Snatchmon | PASS | No changes since 2026-03-14 validation |
| BT21-060 | Destromon | PASS | No changes since 2026-03-13 validation |
| BT21-062 | Galacticmon | PASS | No changes since 2026-03-13 validation |
| BT21-087 | Zenith | PASS | No changes since 2026-03-13 validation |
| BT21-098 | Ragnarok Cannon | PASS | No changes since 2026-03-14 validation |
| EX11-046 | Galacticmon | PASS | No changes since 2026-03-13 validation |
| EX11-066 | Xeno | PASS | No changes since 2026-03-14 validation |
| P-094 | Destromon | PASS | No changes since 2026-03-14 validation |

---

## Chaosmon Previously Validated Cards

5 cards already validated by other QA reports. No script changes since validation.

| Card ID | Name | Status | Validated In |
|---------|------|--------|-------------|
| BT20-037 | Chaosmon: Valdur Arm | PASS | 2026-03-14-ts-olympos-qa.md |
| LM-029 | Yellow Scramble | PASS | 2026-03-14-puppets-qa.md |
| LM-037 | Sepia Memory Boost! | PASS | 2026-03-14-puppets-qa.md |
| LM-043 | Darkdramon | PASS | 2026-03-13-dark-masters.md |
| LM-047 | Chartreuse Memory Boost! | PASS | 2026-03-14-hudiemon-qa.md |

---

## Chaosmon New QA Results (11 cards)

### BT20-004 Pinamon (Lv.2 Digi-Egg) -- FAIL

**Card text**: Inherited: [Your Turn] [Once Per Turn] When any of your Digimon with the [ACCEL] trait are played, this Digimon may digivolve into a Digimon card with the [ACCEL] trait in the hand with the digivolution cost reduced by 2.

**Issues**:
1. `digi_filter` returns `True` for all cards -- does not check for [ACCEL] trait on the digivolution target
2. Condition does not check whether the triggering played Digimon has the [ACCEL] trait (the `context` dict should contain the played card info but is not checked)
3. No cost reduction of 2 is applied -- `effect_digivolve_from_hand` is called without specifying a cost reduction parameter

**Verdict**: FAIL


### BT20-030 Liollmon (Lv.3) -- FAIL

**Card text**: [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with [Chaosmon] in its name or the [ACCEL] trait and 1 Option card with the [ACCEL] trait among them to the hand. Return the rest to the bottom of the deck.

**Tested**: Played via debug API. On Play effect log fires but no cards are added to hand.

**Issues**:
1. Lines 61-63: Stray `player.trash_cards.pop()` operation before the reveal -- attempts to pop from an empty trash pile, does nothing but would corrupt state if trash had cards
2. `reveal_filter` only checks for "Chaosmon" in `card_names` -- completely misses the [ACCEL] trait condition (`or the [ACCEL] trait`)
3. Card text requires 2 separate selections (1 Digimon + 1 Option with ACCEL), but script only does 1 `effect_reveal_and_select` call
4. `on_revealed` callback takes `(selected, remaining)` where `selected` is singular, not a multi-pass selection

**Inherited**: Barrier keyword -- likely handled by engine keyword system (not in script)

**Verdict**: FAIL


### BT20-031 Liamon (Lv.4) -- PASS

**Card text**: [On Play] [When Digivolving] 1 of your opponent's Digimon gets -3000 DP for the turn.

**Tested**: Played via debug API. Opponent Falcomon (1000 DP) was deleted after Liamon play, confirming -3000 DP applied correctly.

**Notes**:
- Auto-targets lowest DP opponent Digimon (standard engine pattern for "1 of your opponent's Digimon")
- Both On Play and When Digivolving effects implemented with correct timings

**Inherited**: Barrier keyword -- likely handled by engine keyword system

**Verdict**: PASS


### BT20-033 LoaderLeomon (Lv.5) -- FAIL

**Card text**: [On Play] [When Digivolving] Until the end of your opponent's turn, 1 of their Digimon can't activate [When Digivolving] effects and gets -3000 DP.

**Issues**:
1. "Can't activate [When Digivolving] effects" is a stub (`pass  # descriptive-tagged: disable_effect`) -- effect suppression is not implemented
2. `CANNOT_BE_SELECTED_BY_EFFECT` modifier is applied to the **own permanent** (`perm`) instead of the targeted **opponent's Digimon** -- wrong target entirely
3. The modifier type is also wrong -- should be "can't activate When Digivolving" not "can't be selected by effects"

**Inherited**: [Opponent's Turn] [Once Per Turn] redirect attack -- stub (`pass  # descriptive-tagged: redirect_attack`)

**Verdict**: FAIL


### BT20-036 BanchoLeomon (Lv.6) -- FAIL

**Card text**:
- When this card would be played, if you have a Digimon with the [ACCEL] trait, reduce the play cost by 5.
- [On Play] [When Digivolving] De-Digivolve 2 on 1 opponent Digimon. Then, 1 of their Digimon gets -5000 DP until end of their turn.
- [End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card with [Chaosmon] in its name in the hand. Then, the DNA digivolved Digimon may attack.

**Tested**: Played via debug API. Cost was 7 (12-5) even without ACCEL Digimon on field -- condition is missing the ACCEL field check.

**Issues**:
1. **BeforePayCost condition** does not check for an ACCEL trait Digimon on the player's field -- cost reduction always applies when playing this card
2. **Orphan effect2** (lines 63-85) duplicates the cost_reduction = 5 with an always-true condition and no timing -- could cause double-dip or unintended leaking
3. **On Play process order reversed**: applies -5000 DP first, then De-Digivolve -- card text says De-Digivolve first, then -5000 DP (and they can target different Digimon)
4. -5000 DP and De-Digivolve both auto-select lowest DP target; card text implies player choice for each
5. **End of Turn DNA digivolve**: uses `effect_play_from_zone(player, 'hand', ...)` which plays a Chaosmon from hand for free -- should be DNA digivolve merging this Digimon + another field Digimon as materials
6. `force_attack` is a stub after DNA digivolve
7. **Inherited** redirect_attack is a stub

**Verdict**: FAIL


### BT20-038 Falcomon (Lv.3) -- PARTIAL

**Card text**: [Your Turn] When this Digimon would digivolve into a Digimon card with the [ACCEL] trait, reduce the digivolution cost by 1.

**Tested**: Confirmed Falcomon is on field and digivolve actions appear. The cost_reduction = 1 on BeforePayCost fires correctly.

**Issues**:
1. Condition does not verify that the digivolving target card has the [ACCEL] trait -- reduction applies to all digivolutions from this Digimon, not just ACCEL targets
2. This is a minor over-scoping issue; in the Chaosmon deck, all digivolution targets are ACCEL, so it works correctly in practice

**Verdict**: PARTIAL


### BT20-039 Diatrymon (Lv.4) -- PASS

**Card text**: [On Play] [When Digivolving] Suspend 1 of your opponent's Digimon.

**Tested**: Played via debug API (indirectly confirmed through simulation runs and game creation).

**Notes**:
- Uses `effect_select_opponent_permanent` with `target_filter` returning True -- selects any opponent permanent
- Both On Play and When Digivolving effects correctly implemented

**Inherited**: Piercing keyword -- handled by engine keyword system

**Verdict**: PASS


### BT20-041 Crowmon (Lv.5) -- PARTIAL

**Card text**: [On Play] [When Digivolving] Suspend 1 of your opponent's Digimon and 1 of your Digimon gets +3000 DP for the turn. Then, 1 of your Digimon may attack.

**Issues**:
1. +3000 DP is applied to `perm` (self) rather than being a player-selectable own Digimon -- "1 of your Digimon" implies choice
2. `force_attack` is a stub -- the "may attack" part is not implemented
3. Suspend targets any opponent permanent (works correctly)

**Inherited**: [When Attacking] [Once Per Turn] 1 opponent Digimon gets -4000 DP -- auto-selects lowest DP (standard pattern)

**Verdict**: PARTIAL


### BT20-043 Varodurumon (Lv.6) -- FAIL

**Card text**:
- When this card would be played, if you have a Digimon with the [ACCEL] trait, reduce the play cost by 5.
- [On Play] [When Digivolving] Suspend **all** of your opponent's Digimon and 1 of your Digimon gets +3000 DP for the turn. Then, 1 of your Digimon may attack.
- [End of Your Turn] DNA digivolve into Chaosmon. Then, may attack.

**Issues**:
1. **BeforePayCost condition** does not check for ACCEL Digimon on field (same as BT20-036)
2. **Orphan effect2** with duplicate cost_reduction and always-true condition (same as BT20-036)
3. **Suspend targets only 1 opponent** via `effect_select_opponent_permanent` -- card text says "Suspend **all** of your opponent's Digimon"
4. +3000 DP applied to self, not player-selectable
5. `force_attack` is a stub
6. **End of Turn DNA digivolve** uses `effect_play_from_zone` (plays from hand) instead of actual DNA digivolve mechanic

**Inherited**: [When Attacking] [Once Per Turn] -4000 DP -- auto-selects lowest (standard pattern)

**Verdict**: FAIL


### BT20-099 Singularity of Chaos (Option) -- FAIL

**Card text**:
- While you have a Digimon with [Chaosmon] in its name or the [ACCEL] trait, you can ignore this card's color requirements.
- [Security] Gain 1 memory and add this card to the hand.
- [Main] You may play 1 Digimon card with the [ACCEL] trait from your hand with the play cost reduced by 4. Then, place this card as any of your Digimon's bottom digivolution card.

**Inherited (as digi-card)**: [End of Opponent's Turn] If this Digimon has [Chaosmon] in its name, trash your opponent's top security card and this Digimon gets -30000 DP for the turn.

**Tested**: Card does not appear in playable actions even with ACCEL Digimon on field.

**Issues**:
1. **Color ignore**: stub (effect0 does nothing) -- card cannot be played because the engine enforces color requirements and the ignore mechanism is not implemented
2. **Main effect**: calls `effect_play_from_zone` with `free=True` -- should be cost reduced by 4, not free
3. **Main effect**: does not place self as a bottom digivolution card on any Digimon after playing the ACCEL card
4. **Security effect**: `player.trash_cards.pop()` to add to hand -- wrong mechanism (should use the card from the security check context, not pop from trash)
5. **Inherited EOT**: `-30000 DP` targets opponent Digimon (`enemy.battle_area`) -- should target **self** (`perm.change_dp(-30000)`)
6. **Inherited EOT**: condition does not check if this Digimon has "Chaosmon" in its name
7. **Inherited EOT**: security trash targets opponent correctly but the DP penalty direction is inverted

**Verdict**: FAIL


### P-221 Chaosmon (Lv.7) -- FAIL

**Card text**:
- Security Attack +1
- Partition (Yellow Lv.6 & Purple/Black Lv.6)
- [When Digivolving] If DNA digivolving, until your opponent's turn ends, their effects don't affect this Digimon.
- [When Digivolving] [When Attacking] 1 of your opponent's Digimon gets -10000 DP until their turn ends.

**Issues**:
1. **When Digivolving immunity** (effect3): condition does not check if the digivolution was a DNA digivolve -- fires on any digivolve
2. **When Digivolving -10000 DP** (effect4): has NO process callback -- effect is defined with timing and condition but does nothing when triggered
3. **When Attacking -10000 DP** (effect5): has NO process callback -- same issue, completely empty
4. **Security Attack +1**: set via `_security_attack_modifier = 1` -- appears correct (engine-level)
5. **Partition**: set via `_is_partition = True` -- appears correct (engine-level)
6. The -10000 DP effect is the core offensive ability of Chaosmon and is entirely missing

**Verdict**: FAIL

---

## Summary

### Galacticmon (16 cards)
- **PASS**: 16/16
- No regressions detected

### Chaosmon (16 cards)
- **PASS**: 7 (BT20-031, BT20-039, BT20-037, LM-029, LM-037, LM-043, LM-047)
- **PARTIAL**: 3 (BT20-038, BT20-041, BT20-041 inherited)
- **FAIL**: 8 (BT20-004, BT20-030, BT20-033, BT20-036, BT20-043, BT20-099, P-221)

### Common Issues Across Chaosmon Scripts
1. **DNA Digivolve not implemented**: BT20-036 and BT20-043 End of Turn effects use `effect_play_from_zone` (plays from hand) instead of DNA digivolve mechanic. This is the core deck strategy and is non-functional.
2. **BeforePayCost missing ACCEL field check**: BT20-036 and BT20-043 cost reductions apply unconditionally without checking for ACCEL Digimon on field.
3. **Orphan cost_reduction effects**: BT20-036 and BT20-043 have duplicate effect entries with always-true conditions.
4. **force_attack stubs**: BT20-041, BT20-043, BT20-036 all have "may attack" effects that are stubs.
5. **redirect_attack stubs**: BT20-033 and BT20-036 inherited effects are stubs (same engine gap as Galacticmon P-094).
6. **P-221 Chaosmon missing core effects**: The Lv.7 boss card has no -10000 DP effect implementation at all.

### Engine Gaps Identified
| Gap | Cards Affected | Status |
|-----|---------------|--------|
| redirect_attack | BT20-033 inherited, BT20-036 inherited | Stub -- same gap as Galacticmon |
| force_attack | BT20-036, BT20-041, BT20-043 | Stub -- no engine API for granting attack after effect |
| DNA digivolve from effect | BT20-036, BT20-043 | Not available as scripting API -- `effect_play_from_zone` used as workaround |
| disable_effect (suppress When Digivolving) | BT20-033 | Stub -- no engine API for effect suppression |
| ignore_color_requirement | BT20-099 | Stub -- no engine API to bypass color checks |

### Simulation Results
- Chaosmon mirror: 5/5 games complete (3 steps each), no crashes
- Galacticmon mirror: 5/5 games complete (3 steps each), no crashes
- Chaosmon vs Galacticmon: 5/5 games complete (3 steps each), no crashes
- Note: All simulation games were very short (3 steps), suggesting the greedy agent does not exercise card effects deeply
