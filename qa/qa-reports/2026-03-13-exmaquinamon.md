# Gameplay QA Report -- ExMaquinamon

## Test Setup
- **Date**: 2026-03-13
- **Archetype**: ExMaquinamon
- **Game IDs**: 6e286c17 (Game 1), 318bfb14 (Game 2), e790b311 (Game 3), 4a0b5484 (Game 5), c5db0899 (Game 6)
- **Total Turns Played**: ~15 across 5 games

## Summary
- **Total Issues Found**: 14
- Critical: 1 | High: 7 | Medium: 5 | Low: 1

## Detailed Findings

### Issue 1: EX11-027 Maquinamon On Play effect never triggers (Critical)
- **Card**: EX11-027 Maquinamon
- **Expected**: On Play reveals top 3 cards, adds 1 [Maquinamon] and 1 card with [Maquinamon] in text to hand, then may link
- **Actual**: Effect never fires. Hand unchanged after play (confirmed in Games 1, 2, 6).
- **Root Cause**: condition1 checks `effect.effect_source_permanent` which is never set on the ICardEffect instance. When `hasattr(effect, 'effect_source_permanent')` is True but the value is None, the condition falls to the else branch and returns False. Additionally, the `permanent.top_card.card_text` check is redundant since the card IS Maquinamon.
- **Secondary Issue**: The reveal logic (`effect_reveal_and_select`) only selects 1 card total, but the card text says to add TWO separate cards (1 named Maquinamon + 1 with Maquinamon in text).
- **Missing**: The "Then, you may link this Digimon or 1 [Maquinamon] in your hand to 1 of your other Digimon" part is completely absent from the script.

### Issue 2: EX11-029 Turbomon On Play uses wrong timing (High)
- **Card**: EX11-029 Turbomon
- **Expected**: On Play links 1 [Maquinamon] from hand or digivolution cards to a Digimon
- **Actual**: On Play effect uses `EffectTiming.OnMove` (effect1) instead of `EffectTiming.OnEnterFieldAnyone`. The OnMove timing does not fire during play, so the On Play effect never triggers.
- **Note**: The When Digivolving effect (effect2) correctly uses `OnEnterFieldAnyone` and works (confirmed via Mulemon test in Game 6).

### Issue 3: EX11-033 Maneuvermon On Play plays Maquinamon instead of linking (High)
- **Card**: EX11-033 Maneuvermon
- **Expected**: On Play/When Digivolving links 1 [Maquinamon] to an existing Digimon
- **Actual**: Uses `effect_play_from_zone` which plays Maquinamon as a separate Digimon on the field. Card text says "play 1 [Maquinamon]... to 1 of your Digimon" which means linking in Digimon TCG context.
- **Verified**: In Game 2, Maneuvermon's When Digivolving effect played a Maquinamon as a standalone Digimon in slot 2 instead of linking it to an existing one.

### Issue 4: EX11-036 Dalphomon suspends 1 instead of 2 (High)
- **Card**: EX11-036 Dalphomon
- **Expected**: On Play/When Digivolving/When Attacking suspends **2** opponent Digimon/Tamers, then **1** can't unsuspend
- **Actual**: Script only calls `effect_select_opponent_permanent` once per trigger, suspending only 1 target and immediately granting cannot_unsuspend to that same target. Should suspend 2 separately then select 1 for cannot_unsuspend.
- **Verified**: In Game 5, Dalphomon's On Play suspended and locked P2's single Maquinamon. Correct behavior for 1 target, but would be wrong with 2+ targets.

### Issue 5: EX11-036 Dalphomon End-of-Turn condition uses broken effect_source_permanent pattern (High)
- **Card**: EX11-036 Dalphomon
- **Expected**: End of Your Turn, 1 of your other Digimon may digivolve into a black card with [Maquinamon] in text
- **Actual**: condition5 checks `effect.effect_source_permanent` which is never set, causing the condition to always return False. Effect never fires.

### Issue 6: EX11-036 Dalphomon inherited WhenLinked missing "this Digimon may attack" (Medium)
- **Card**: EX11-036 Dalphomon (inherited)
- **Expected**: When linked, suspend 1 opponent's Digimon. Then, this Digimon may attack.
- **Actual**: Suspends correctly, but grants `cannot_unsuspend` (not in card text for inherited) and the "this Digimon may attack" part is BLOCKED (comment in code: "engine cannot force a Digimon to attack").

### Issue 7: EX11-045 Metatromon grants wrong modifier (High)
- **Card**: EX11-045 Metatromon
- **Expected**: De-Digivolve 2 an opponent Digimon. Then, 1 of their Digimon or Tamers can't **digivolve** until their turn ends.
- **Actual**: Grants `CANNOT_BE_SELECTED_BY_EFFECT` instead of "can't digivolve". The anti-digivolve restriction is not the same as effect targeting immunity.

### Issue 8: EX11-045 Metatromon End-of-Turn condition broken (High)
- **Card**: EX11-045 Metatromon
- **Expected**: End of Your Turn, 1 of your other Digimon may digivolve into a green card with [Maquinamon] in text
- **Actual**: Same `effect_source_permanent` condition bug as Issues 1 and 5. Effect never fires.

### Issue 9: EX11-006 Flickmon inherited condition broken + missing cost reduction (Medium)
- **Card**: EX11-006 Flickmon (inherited)
- **Expected**: When Attacking (once per turn), this Digimon linked with [Maquinamon] may digivolve into a Digimon with [Maquinamon] in text from hand with digivolution cost reduced by 2
- **Actual**: (a) Same `effect_source_permanent` condition bug. (b) Does not check for "linked with [Maquinamon]" requirement. (c) Does not apply the -2 digivolution cost reduction.

### Issue 10: EX11-073 ExMaquinamon When Digivolving missing DNA check (Medium)
- **Card**: EX11-073 ExMaquinamon
- **Expected**: When Digivolving, **if DNA digivolving**, link up to 3 [Maquinamon] from hand/trash/digi cards
- **Actual**: condition4 does not check whether the digivolve was a DNA digivolve. The effect would trigger on any digivolve, not just DNA.

### Issue 11: EX11-070 Unchained missing inherited effects (High)
- **Card**: EX11-070 Unchained (inherited)
- **Expected**: Two inherited effects: (a) "This Digimon with [Maquinamon] in text can't have less than 1000 DP, and opponent's effects can't trash its stacked cards." (b) "[End of All Turns] You may play 1 [Unchained] from this Digimon's digivolution cards without paying the cost."
- **Actual**: Only the DP floor effect (effect3) is present but has `effect_source_permanent` bug. The "can't trash stacked cards" protection and the "[End of All Turns] play Unchained from digi cards" effect are both missing entirely.

### Issue 12: EX6-072 Mega Digimon Assembly DNA digivolve effect does not fire (Medium)
- **Card**: EX6-072 Mega Digimon Assembly!
- **Expected**: 1 of your level 6 Digimon and 1 card in the hand may DNA digivolve into a level 7 Digimon card in your hand
- **Actual**: `effect_dna_digivolve_from_hand` requires the hand card to have `dna_costs` with valid field targets matching both requirements simultaneously (2 field Digimon). The card's intent is to use 1 field Lv6 + 1 hand card as materials, which is a non-standard DNA pattern not supported by the engine method.

### Issue 13: LM-048 Chrome Memory Boost duplicated on field (Low)
- **Card**: LM-048 Chrome Memory Boost!
- **Expected**: Play option, reveal top 3, add 1 green/black Digimon to hand, place this card in battle area
- **Actual**: Two copies of LM-048 appeared on the field after playing one. Reveal+add effect works correctly. The duplication may be a display issue or the card being placed twice.

### Issue 14: P-151 Digimon Liberator "ignore color requirements" not implemented (Medium)
- **Card**: P-151 Digimon Liberator
- **Expected**: "While you have [LIBERATOR] trait Digimon or Tamer, you can ignore this card's color requirements" should allow playing this White option without a White Digimon/Tamer on field
- **Actual**: The ignore-color effect is a no-op stub (`pass  # descriptive-tagged`). The card can only be played when a White Digimon/Tamer is on field.

## Cards Tested
| Card ID | Name | Status | Notes |
|---------|------|--------|-------|
| EX11-006 | Flickmon | FAIL | Inherited condition broken (effect_source_permanent bug); missing linked-with-Maquinamon check; missing -2 evo cost reduction |
| EX11-027 | Maquinamon | FAIL | On Play effect never triggers (condition bug); reveal selects only 1 card instead of 2; missing link-to-Digimon step |
| EX11-029 | Turbomon | PARTIAL | On Play uses wrong timing (OnMove); When Digivolving works correctly; WhenLinked (play Unchained) untested |
| EX11-033 | Maneuvermon | PARTIAL | On Play/When Digivolving plays Maquinamon as standalone instead of linking; WhenLinked suspend+cannot_unsuspend structurally correct; inherited unsuspend structurally correct |
| EX11-036 | Dalphomon | PARTIAL | On Play suspends 1 instead of 2; cannot_unsuspend applied correctly to 1; End-of-Turn condition broken; inherited WhenLinked missing force-attack; Vortex keyword present |
| EX11-040 | Mulemon | PASS | On Play link mechanic works correctly (uses effect_link_to_permanent); When Digivolving link works; WhenLinked plays Unchained for free; Reboot inherited present |
| EX11-042 | MockingBirdmon | PARTIAL | On Play/When Digivolving structurally correct (same play-vs-link issue as Maneuvermon); WhenLinked delete structurally correct; inherited attack redirect is a no-op stub |
| EX11-045 | Metatromon | FAIL | Grants CANNOT_BE_SELECTED instead of can't-digivolve; End-of-Turn condition broken; Blocker keyword present; De-Digivolve 2 structurally correct; inherited delete-lowest structurally correct |
| EX11-062 | Shoto Kazama | PASS | Set memory to 3 correct; On Play fires (continuous Vortex modifier); suspend trigger with draw+DP buff well-structured; Security play correct |
| EX11-070 | Unchained | PARTIAL | Security play correct; Set memory to 3 works (verified in Game 6); End-of-Turn DNA+Mind Link structurally present; missing 2 inherited effects (stack protection + End-of-All-Turns Unchained play) |
| EX11-071 | Cool Boy | PASS | On Play reveal+add works with multi-filter selection; Main return-to-deck + reduced-cost play structurally correct; Security play present |
| EX11-073 | ExMaquinamon | PARTIAL | Link +2 markers present; When Digivolving link up to 3 structurally correct but missing DNA-only condition; End-of-Opponent's-Turn trash-security-to-bounce structurally correct but auto-trashes (should be optional "by trashing") |
| EX6-072 | Mega Digimon Assembly! | PARTIAL | Ignore color requirement is stub; DNA digivolve effect does not fire (engine limitation with 1 field + 1 hand DNA pattern); Security effect structurally correct |
| LM-048 | Chrome Memory Boost! | PARTIAL | Reveal+add works; Delay marker present; field placement duplicated; auto-selects first matching card without player choice |
| P-151 | Digimon Liberator | PARTIAL | Ignore color requirement is stub; reveal+add structurally correct; play-from-hand structurally correct but trait check uses correct attribute |
