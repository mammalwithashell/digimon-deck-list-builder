# Gameplay QA Report — TS Olympos vs BG Imperial

## Test Setup
- **Date**: 2026-03-11
- **Archetypes**: TS Olympos (P1) vs BG Imperial (P2)
- **Deck Lists**: TS Olympos Nationals 2nd (Bartolome) vs BG Imperial Bulgaria 1st (Neonthetheif)
- **Game ID(s)**: ede9a8df-3b6a-4465-861c-4cb536eaf0fc
- **Total Turns Played**: 7
- **Focus Areas**: Play costs, cost reduction, On Play effects, DP auras, "by" cost mechanics, selection phases

## Summary
- **Total Issues Found**: 7
- Critical: 0 | High: 4 | Medium: 2 | Low: 1

## Detailed Findings

### Issue 1: BT24-102 Homeros +1000 DP aura not applying to TS Digimon
- **Card(s)**: BT24-102 — Homeros
- **Severity**: high
- **Category**: effect
- **Expected**: All [TS] trait Digimon should get +1000 DP from Homeros' "[All Turns] All of your [TS] trait Digimon get +1000 DP" aura.
- **Actual**: Tapirmon (TS trait) showed 1000 DP with Homeros on field. Should have been 2000. Tapirmon was subsequently deleted by a 1000 DP security Digimon that it should have survived.
- **Steps to Reproduce**:
  1. Play Homeros (BT24-102) onto the field
  2. Have a [TS] trait Digimon on field (e.g., Tapirmon BT24-043)
  3. Check DP via state endpoint — DP breakdown shows no Homeros contribution
- **Evidence**: dpBreakdown showed base=1000, temporary=0, total=1000 with no modifier from Homeros
- **Rules Reference**: Continuous DP boost effects should apply immediately while the source is on the field

### Issue 2: BT24-034 Aegiomon On Play/When Moving "by" cost auto-pays without player choice
- **Card(s)**: BT24-034 — Aegiomon
- **Severity**: high
- **Category**: effect
- **Expected**: "By adding your top security card to the hand, you **may** play 1 [TS] Tamer" — the "by" cost (adding security to hand) should only be paid if the player opts in. If no valid Tamer targets exist, the cost should not be paid at all.
- **Actual**: The security card was automatically added to hand without player choice, even when no valid Tamer existed to play (all hand Tamers had names matching field Tamers). Security went from 5→4 on first trigger and 3→2 on second trigger without consent.
- **Steps to Reproduce**:
  1. Play Aegiomon with Homeros already on field (no other TS Tamers in hand)
  2. On Play fires and auto-adds security card to hand
  3. No Tamer play selection is shown (correctly, since no valid targets), but the security cost was already paid
- **Evidence**: P1 security count decreased from 4 to 3 after playing Aegiomon with no valid Tamer targets
- **Rules Reference**: "By [cost], you may [effect]" — the cost should only be paid when the player chooses to activate. No valid targets should skip the entire effect.

### Issue 3: BT24-034 Aegiomon When Moving fires when a different Digimon moves
- **Card(s)**: BT24-034 — Aegiomon
- **Severity**: high
- **Category**: effect
- **Expected**: Aegiomon's [When Moving] effect should only trigger when Aegiomon itself moves from breeding to battle area.
- **Actual**: When Elecmon (BT24-031) moved from breeding, Aegiomon's [When Moving] effect fired (adding security to hand). Aegiomon was already on the field.
- **Steps to Reproduce**:
  1. Have Aegiomon on the battle area
  2. Move a different Digimon from breeding to battle area
  3. Aegiomon's When Moving triggers incorrectly
- **Evidence**: Log shows "[Effect] OnMove | Unknown: [When Moving] By adding your top security card to the hand..." after moving Elecmon, not Aegiomon
- **Rules Reference**: [When Moving] effects only trigger for the card they're printed on

### Issue 4: BT24-041 Minervamon On Play skips free Iliad card play
- **Card(s)**: BT24-041 — Minervamon
- **Severity**: high
- **Category**: effect
- **Expected**: "[On Play] You **may** play 1 play cost 5 or lower [Iliad] trait card from your hand without paying the cost. **Then**, De-Digivolve 1 to opponent for each of your Digimon."
- **Actual**: The free play step was completely skipped. The engine went directly to De-Digivolve target selection without offering the optional free play. Hand contained BT24-034 Aegiomon (Iliad, cost 5) which was a valid target.
- **Steps to Reproduce**:
  1. Play Minervamon with Iliad cards in hand (e.g., Aegiomon cost 5)
  2. On Play fires but skips to De-Digivolve targeting
  3. No play-from-hand selection appears
- **Evidence**: No selection phase for play-from-hand; action descriptions jumped to "Select an opponent's Digimon"

### Issue 5: BT24-090 Abyss Sanctuary implementation has multiple errors
- **Card(s)**: BT24-090 — Abyss Sanctuary: Throne Room
- **Severity**: medium
- **Category**: effect
- **Expected**: [Main] Play 1 level 4 or lower blue/yellow [TS] Digimon from hand/trash for free. Then place this card in the battle area. [Security/All Turns] +2000 DP to blue/yellow [TS] Digimon.
- **Actual**: Multiple discrepancies:
  1. Effect log text describes a security-swap mechanic ("Add bottom security to hand, place this card as bottom security") which doesn't match card text
  2. Selection filter allows ALL hand cards instead of level ≤4 blue/yellow [TS] Digimon only
  3. Option was trashed after resolving instead of placed in the battle area
  4. Play said "by paying its cost" with reduction instead of "without paying the cost"
- **Steps to Reproduce**: Play BT24-090 with a Yellow or Blue permanent on field
- **Evidence**: Logs show "trashed after resolving"; selection showed Minervamon (Lv6) and Neptunemon (Lv6) as valid targets

### Issue 6: SelectReveal phase action descriptions are wrong
- **Card(s)**: Multiple (BT24-043, BT24-020, BT24-083)
- **Severity**: low
- **Category**: ui
- **Expected**: During SelectReveal phase, action descriptions should describe selecting from revealed cards (e.g., "Select Gomamon from revealed cards")
- **Actual**: Descriptions show "Trash [card name] from hand" which is misleading. The prompt text is correct ("Select a card from the revealed cards") but the individual action descriptions are wrong.
- **Steps to Reproduce**: Trigger any reveal-and-select effect (e.g., play Tapirmon)
- **Evidence**: Actions showed "Trash Aegiomon from hand" for revealed card selection indices 30-32

### Issue 7: BT24-041 Minervamon De-Digivolve uses attack action IDs
- **Card(s)**: BT24-041 — Minervamon
- **Severity**: medium
- **Category**: effect
- **Expected**: De-Digivolve target selection should use target selection action IDs
- **Actual**: Selection offered action IDs 114 ("Attack player with Tapirmon") and 115 ("Attack Veemon with Homeros") for selecting De-Digivolve target. The descriptions and IDs correspond to attack actions, not target selection.
- **Steps to Reproduce**: Play Minervamon with opponent having Digimon on field
- **Evidence**: Pending selection showed validIndices [114, 115] with attack descriptions

## Cards Tested Successfully
- BT24-043 Tapirmon: On Play reveal ✓, two-pass selection ✓, cost 3 correct ✓
- BT24-020 Gomamon: On Play reveal ✓, two-pass selection ✓, played for free from option effect ✓
- BT24-030 Neptunemon: Cost reduction ✓ (12→7 with 2+ opp Digimon), On Play bounce all lowest evo cards ✓
- BT24-041 Minervamon: Cost reduction ✓ (12→7 with Iliad on field)
- BT24-083 Hiroko Sagisaka: Cost 3 ✓, On Play reveal ✓
- BT24-031 Elecmon: Digivolved onto Wanyamon in breeding ✓
- BT24-102 Homeros: Start of Main Phase +1 memory ✓, 5+ memory suspend/draw check ✓ (correctly didn't fire at <5)

## Engine Gaps Confirmed
- **Protect-other-permanent**: Minervamon's "[Once Per Turn] When this or other [Iliad] Digimon would be deleted, trash security to prevent" did not trigger when Tapirmon was deleted. Known engine gap — WhenRemoveField cannot abort deletion for a different permanent.

## Areas Not Covered
- BT24-004 Wanyamon inherited effect (never moved to battle area with Iliad card played)
- BT24-024 Submarimon, BT24-046 Garurumon, BT24-025 Shellmon (not drawn)
- BT24-028 Divermon, BT24-029 Whamon (not drawn)
- BT24-040 Venusmon, BT24-051 Merukimon (not drawn)
- BT24-085 Dan & Kanan Yuki, BT24-088 Asuna Shiroki (not drawn)
- BT24-091 Tidal Stream, BT24-094 Central Town, BT24-095 Sonic Shot, BT24-100 In-Between Theater (not drawn)
- LM-028 Blue Scramble (not drawn)
- All BG Imperial cards (P2 not heavily tested — focused on TS Olympos)
- Neptunemon unsuspend, protection effects (not tested)
- Homeros End of Turn option use (no TS Options in hand when triggered)
- Security effects of any card
- Digivolution-based effects (When Digivolving)
- Attacking with inherited effects
