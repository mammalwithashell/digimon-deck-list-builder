# Diaboromon Archetype QA Report

**Date**: 2026-03-01
**Archetype**: Diaboromon (Token/Swarm)
**Cards Tested**: 22 (of 26 unique; 4 already validated)
**Best Deck**: `digimonmeta_10363349c033` (1st Place, 19 unique cards)
**Games Run**: 3 debug games
**Method**: Debug games with skip_shuffle, manual action sequences, script review

---

## Summary

| Status | Count | Cards |
|--------|-------|-------|
| PASS | 7 | BT17-053, BT17-055, BT17-060, BT2-053, BT2-059, BT5-059, BT5-063 |
| PARTIAL | 15 | BT22-053, BT22-057, BT22-059, BT22-064, BT22-091, BT24-052, BT24-065, BT5-085, BT5-090, EX6-036, EX6-039, EX6-041, EX6-043, BT24-064, BT19-101 |

22 cards tested. 7 PASS, 15 PARTIAL. Combined with 4 previously validated (BT22-005 PASS, LM-031 PASS, P-039 PASS, P-107 PASS): **26/26 cards have validation status.**

---

## Issues Found

### Issue 1: Diaboromon Token play callbacks stubbed in 8 scripts (HIGH)

Multiple scripts have `pass  # descriptive-tagged: play_token` instead of calling `game.effect_play_token(player, 'diaboromon')`. The engine method `effect_play_token` is fully implemented and works (verified via BT17-053 and BT5-090 which use it correctly).

**Affected scripts:**
- `EX6-043` (bt22/ex6_043.py) -- Start of Main Phase token + When Digivolving token (lines 41, 70)
- `BT22-064` (bt22/bt22_064.py) -- When Digivolving token + When Attacking token (lines 72, 101)
- `BT24-052` (bt24/bt24_052.py) -- When Moving token + When Digivolving token (lines 55, 83)
- `BT22-059` (bt22/bt22_059.py) -- Inherited On Deletion token (line 155)
- `EX6-036` (ex6/ex6_036.py) -- Inherited On Deletion token (line 87)
- `EX6-039` (ex6/ex6_039.py) -- Inherited On Deletion token (line 144)

**Fix**: Replace `pass  # descriptive-tagged: play_token` with `game.effect_play_token(player, 'diaboromon')` in each process callback.

### Issue 2: BT22-053 On Play process has spurious trash pop (HIGH)

`process1` in bt22_053.py (line 54) pops a card from trash before performing the reveal:
```python
if player and player.trash_cards:
    card_to_add = player.trash_cards.pop()
    player.hand_cards.append(card_to_add)
```
This incorrectly steals a card from trash every time the On Play effect fires. The reveal filter is also too broad (`return True` instead of filtering for Arata Sanada + Unidentified/CS trait).

### Issue 3: EX6-036 On Play condition incorrectly blocks effect (HIGH)

The condition for EX6-036's On Play reveal effect checks `if not ('Diaboromon' in text)` on the permanent's top card text. Since EX6-036 Keramon's card text does not literally contain "Diaboromon", the condition always returns False and the On Play reveal never fires.

**Observed**: Playing EX6-036 produced no reveal prompt (Game 3).
**Expected**: Reveal top 3, add 1 Tamer/Option with Diaboromon text + 1 Unidentified to hand.

EX6-036 also has the same spurious trash pop in its process callback (line 47-49).

### Issue 4: EX6-039 cost reduction not functional (MED)

EX6-039 has a `BeforePayCost` effect with `cost_reduction = 3` but:
1. The process callback is `pass` (line 38) -- no actual deletion of Unidentified Digimon occurs
2. The cost reduction property may not be read by the engine's play-cost logic
3. On Play delete filter does not check play cost <= 3 (uses generic `p.is_digimon`)

**Observed**: Playing EX6-039 with a Keramon (Unidentified) on field still charged full 5 cost.

### Issue 5: EX6-041 On Play/When Digivolving missing deletion cost (MED)

EX6-041's effect says "By deleting 1 of your Digimon with [Diaboromon] in its name, this Digimon may digivolve into [Diaboromon] in your hand without paying the cost." The script calls `game.effect_digivolve_from_hand()` without first deleting a Diaboromon -- the deletion cost is skipped entirely.

Also, the inherited De-Digivolve trigger condition doesn't verify the trigger Digimon has "Diaboromon" in its name.

### Issue 6: BT22-057 missing tamer count check (LOW)

BT22-057's When Digivolving condition should check "if you have 1 or fewer Tamers" before allowing Arata Sanada play. The condition is missing this check -- it always allows the play regardless of tamer count.

**Observed**: In Game 2, the free Arata Sanada play worked correctly (1 tamer in play, correct per card text), but would also work with 2+ tamers (incorrect).

### Issue 7: BT22-091 attack redirect not functional (MED)

BT22-091 Arata Sanada's Opponent's Turn effect to redirect attacks is stubbed:
```python
# Redirect attack target (SwitchDefender) -- not yet in engine
pass  # descriptive-tagged: redirect_attack
```
The process callback only suspends a target but doesn't actually redirect the attack.

### Issue 8: Overclock keyword not triggering at end of turn (MED)

BT24-065 has `_is_overclock = True` but the Overclock end-of-turn attack did not trigger in Game 1 when passing turn with a Keramon (Unidentified) on field as potential deletion target.

### Issue 9: BT19-101 On Play/When Digivolving/When Attacking uses bounce instead of deck-bottom return (MED)

The card text says "return 1 of their Digimon to the bottom of the deck" but the script uses `enemy.bounce_permanent_to_hand(target_perm)` which returns to hand. Also, the "by returning 1 Digimon card from opponent's trash to deck top" cost is not implemented.

### Issue 10: BT24-065 When Digivolving effect incorrect (MED)

The card text says "De-Digivolve 1 for each of your Digimon. Then, delete all opponent's Digimon with the highest play cost." The script instead does a single delete + single de-digivolve-1, not a scaled de-digivolve per own Digimon count.

### Issue 11: BT5-085 cost reduction may not be consumed by engine (LOW)

BT5-085 uses `_temp_play_cost_reduction` attribute on player. This pattern matches BT17-060 which also uses it. Functionality depends on whether the engine's play logic reads this attribute.

### Issue 12: EX6-043 Jamming/Blocker grant is self-only (LOW)

EX6-043's card text says "All of your other Digimon with [Diaboromon] in their names gain Jamming and Blocker." The script applies `_is_blocker` and `_is_jamming` to itself (the EX6-043 permanent) rather than granting them to other Diaboromon-named Digimon.

---

## Card-by-Card Results

### BT17-053 Keramon -- PASS
- Inherited On Deletion token play uses `game.effect_play_token(player, 'diaboromon')` -- correctly implemented
- Main effect (reactive digivolve into Infermon) has proper condition checking (opponent's turn, Lv.5+ trigger)
- Process correctly finds Infermon in hand and adds to card_sources

### BT17-055 Infermon -- PASS
- When Digivolving: De-Digivolve 1 + attack restriction works
- Inherited: De-Digivolve on Diaboromon play has correct name check and once-per-turn limit
- Tested in Game 1: digivolution cost 3 correct, When Digivolving triggered

### BT17-060 Armageddemon -- PASS
- Cost reduction from trash via BeforePayCost timing implemented
- Rush, Blocker, Reboot keywords all present
- On Play/When Digivolving delete with total cost 15 budget -- multi-select pattern correct
- Can attack unsuspended Digimon effect present
- Note: cost reduction uses `_temp_play_cost_reduction` which requires engine support

### BT2-053 Keramon -- PASS
- Inherited: Draw 1 on same-name play has correct condition (checks trigger name matches top card name, excludes self)
- Process correctly calls `player.draw_cards(1)`

### BT2-059 Kurisarimon -- PASS
- Inherited: Gain 1 memory on same-name play has correct condition (same pattern as BT2-053)
- Process correctly calls `game.memory += 1`

### BT5-059 Keramon -- PASS
- On Play: Reveal 5, add 1 Unidentified Digimon + 1 Arata Sanada -- fully implemented
- Non-interactive reveal (auto-selects first matching cards) -- acceptable for engine

### BT5-063 Kurisarimon -- PASS
- When Digivolving: Play Arata Sanada from hand -- condition correctly checks no Arata Sanada in play AND checks hand for one
- Inherited Rush grant has correct condition (own turn check)

### BT5-085 Armageddemon -- PARTIAL
- Cost reduction: BeforePayCost deletes own Diaboromon, sets `_temp_play_cost_reduction += 12` (Issue 11)
- Rush keyword present
- Suppress Lv.7 When Digivolving effects: declarative flag `_suppresses_when_digivolving_lv7` -- untested whether engine checks this

### BT5-090 Arata Sanada -- PARTIAL
- Start of Turn: Gain 1 memory if Unidentified in trash -- condition and process correct
- Your Turn: On digivolve into Diaboromon, suspend tamer + play token -- uses `game.effect_play_token(player, 'diaboromon')` (correctly implemented!)
- Security Effect: Play this card for free -- implemented
- PARTIAL because the digivolve-into-Diaboromon trigger was not observed firing in Game 2 despite a Diaboromon digivolution occurring (auto-chain timing may have bypassed the check)

### BT22-053 Keramon -- PARTIAL
- On Play reveal: Works but has spurious trash pop (Issue 2) and overly broad filter
- Inherited leave protection: Condition correctly checks Diaboromon in text + Diaboromon in name
- Alt digivolve from Lv.2 for cost 0: Present

### BT22-057 Kurisarimon -- PARTIAL
- When Digivolving: Free play Arata Sanada works (tested in Game 2)
- Missing tamer count check (Issue 6)
- Inherited leave protection: Same pattern as BT22-053

### BT22-059 Infermon -- PARTIAL
- On Play/When Digivolving delete cost <= 5: Filter too broad (generic `p.is_digimon`, doesn't check cost)
- Immunity (Arata Sanada check not implemented in condition)
- Inherited On Deletion token play: Stubbed (Issue 1)
- Alt digivolve from Lv.4 for cost 3: Present

### BT22-064 Diaboromon -- PARTIAL
- Alliance keyword present
- When Digivolving token: Stubbed (Issue 1)
- When Attacking token: Stubbed (Issue 1)
- When Unidentified played, delete lowest cost: Trigger has once-per-turn + correct timing, but delete filter is too broad
- Alt digivolve from Infermon for cost 3: Present

### BT22-091 Arata Sanada -- PARTIAL
- Security play: Present
- Set memory to 3: Correct timing and condition
- Attack redirect: Stubbed (Issue 7)
- Inherited Eater Adam redirect: Stubbed, requires Eater Adam condition

### BT24-052 Keramon (X Antibody) -- PARTIAL
- When Moving token: Stubbed (Issue 1)
- When Digivolving token: Stubbed (Issue 1)
- Inherited leave protection: Condition checks Diaboromon in text -- correct pattern
- Alt digivolve from Keramon for cost 0: Present

### BT24-064 Ouryumon -- PARTIAL
- Piercing: NOT present in script (missing keyword)
- Blocker: Present
- When Digivolving reveal: Uses both `effect_play_from_zone` and `effect_reveal_and_select` -- may double-fire
- De-Digivolve 2 on suspend: Correct timing (`OnTappedAnyone`), once-per-turn
- Alt digivolve from Lv.5 for cost 3: Present

### BT24-065 Diaboromon (X Antibody) -- PARTIAL
- Overclock: Flag present but not triggering at EOT (Issue 8)
- Blocker: Present
- When Digivolving: De-Digivolve + delete highest cost -- implementation is single target, not scaled per own Digimon (Issue 10)
- Leave protection play: Uses `effect_play_from_zone` for hand only (missing digivolution cards source)
- Alt digivolve from Diaboromon for cost 2: Present

### BT19-101 ZeedMillenniummon -- PARTIAL
- Overclock: Flag present (same issue as BT24-065)
- On Play/When Digivolving/When Attacking: Uses hand bounce instead of deck-bottom return (Issue 9)
- Missing trash-to-deck-top cost
- Immunity + can't suspend: Declarative flags present but no process callbacks
- Alt digivolve from MoonMillenniummon for cost 2: Present

### EX6-036 Keramon -- PARTIAL
- On Play reveal: Condition blocks effect due to wrong text check (Issue 3)
- Inherited On Deletion token: Stubbed (Issue 1)

### EX6-039 Kurisarimon -- PARTIAL
- Cost reduction: Not functional (Issue 4)
- On Play delete cost <= 3: Filter too broad
- Inherited On Deletion token: Stubbed (Issue 1)

### EX6-041 Infermon -- PARTIAL
- On Play/When Digivolving: Free digivolve into Diaboromon from hand works via `effect_digivolve_from_hand`, but deletion cost skipped (Issue 5)
- Inherited De-Digivolve: Trigger condition doesn't check for Diaboromon name

### EX6-043 Diaboromon -- PARTIAL
- Start of Main Phase token: Stubbed (Issue 1)
- When Digivolving token: Stubbed (Issue 1)
- Opponent play trigger: Once-per-turn, correct timing
- Blocker + Jamming on self only, not granted to other Diaboromon (Issue 12)

---

## Archetype Mechanic Assessment

### Token Generation (Core Mechanic)
The `effect_play_token(player, 'diaboromon')` engine method works correctly and the Diaboromon token definition in `token_registry.py` is complete (Cost 14, Lv.6, White, Mega, Unknown, Unidentified, 3000 DP). However, **8 of 12 token play callbacks across 6 scripts are stubbed**. Only BT17-053 (inherited) and BT5-090 (tamer) have working token generation. This severely limits the archetype's swarm capability.

### Same-Name Triggers
BT2-053 (Draw 1) and BT2-059 (Gain 1 memory) have correct same-name detection logic. These work correctly for the Diaboromon multiplication strategy.

### De-Digivolve
BT17-055, EX6-041, BT24-065 all implement De-Digivolve using `target_perm.de_digivolve(N)` which trashes removed cards. The mechanic is functional.

### Leave Protection (Substitute Diaboromon)
BT22-053, BT22-057, BT24-052 inherited effects use `WhenRemoveField` timing with correct once-per-turn limits. Condition requires "Diaboromon" in top card text and name, which is appropriate since this effect only activates when inherited by a Diaboromon.

### Overclock
Not triggering at end of turn despite `_is_overclock = True` flag. Needs engine investigation.

---

## Test Details

### Game 1 (901fed7a)
- Played BT22-053 Keramon: On Play reveal triggered correctly (3 cards shown, selection prompt)
- Digivolved BT22-057 onto Keramon: Cost 2, drew card, When Digivolving fired
- Digivolved BT22-059 Infermon: Cost 3, When Digivolving fired (no targets to delete)
- Auto-chained to BT24-065 Diaboromon X: Cost 5, Blocker + Overclock keywords shown
- Attacked with Diaboromon X: Security check worked (13000 vs 7000 DP, attacker survived)
- Overclock did NOT fire at end of turn

### Game 2 (969d7935)
- Played BT5-090 Arata Sanada: Cost 3, deployed to battle area
- Played BT22-053 Keramon: On Play reveal triggered
- Digivolved BT22-057: When Digivolving fired, free-played Arata Sanada from hand (correct)
- Digivolved BT22-059 then auto-chained to EX6-043 Diaboromon: Blocker + Jamming keywords
- EX6-043 When Digivolving token play: No token created (stubbed)
- BT5-090 Arata Sanada token-on-digivolve: Did not trigger during auto-chain

### Game 3 (462d342b)
- Played EX6-036 Keramon: On Play reveal did NOT trigger (condition bug)
- Played EX6-039 Kurisarimon: Full cost 5 charged (no cost reduction despite Unidentified on field)
