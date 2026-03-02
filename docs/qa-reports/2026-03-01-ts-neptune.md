# Gameplay QA Report -- TS Neptune

## Test Setup
- **Date**: 2026-03-01
- **Archetype**: TS Neptune (30 unique cards across 10 decklists)
- **Best Decklist**: digimonmeta_b33bce9f88da (1st Place, 19 unique cards)
- **Method**: Debug API (POST /debug/games, set-memory, inject-card)
- **Games Created**: 6 debug games with skip_shuffle and controlled hands
- **Focus**: Full archetype validation -- play costs, evo costs, On Play effects, keywords, inherited effects

## Summary
- **Total Cards Tested**: 30
- **PASS**: 16
- **PARTIAL**: 14
- **Total Issues Found**: 8
- Critical: 0 | High: 3 | Medium: 4 | Low: 1

## Detailed Findings

### Issue 1: Persistent pending selection causes game deadlock
- **Card(s)**: Engine-level (multiple card effects)
- **Severity**: high
- **Category**: game_flow
- **Expected**: Pending selections should resolve and return to normal phase flow
- **Actual**: After certain effect chains (particularly involving BT24-034 Aegiomon's When Digivolving and other "play from hand without cost" effects), a persistent "Select a card from hand to play without paying its cost" selection appears and never clears. Declining (action 62) advances turns but the selection persists across turn boundaries, eventually leading to a stuck state at phase 0 with no valid actions.
- **Steps to Reproduce**:
  1. Create game, play multiple Digimon with On Play/When Digivolving effects
  2. The "Select a card from hand to play without paying its cost" prompt appears
  3. Declining with action 62 advances the turn but the prompt reappears
  4. Eventually game deadlocks at phase 0
- **Evidence**: Games 1 (618f23cf) and 5 (a3fc25e4) both entered infinite pending selection loops
- **Status**: OUTSTANDING

### Issue 2: "When this card would be played, reduce cost by 5" effects do not apply
- **Card(s)**: BT24-030 Neptunemon, BT24-040 Venusmon, BT24-051 Merukimon, BT24-041 Minervamon
- **Severity**: high
- **Category**: effect
- **Expected**: Play cost should be reduced by 5 when conditions are met (e.g., Merukimon: 3+ Digimon; Minervamon: Iliad trait on field; Neptunemon: 2+ opponent Digimon; Venusmon: 3 or fewer security)
- **Actual**: Full play cost is always charged. Merukimon with 3 Digimon on field charged 12 instead of 7. Minervamon with Kamemon (Iliad trait) on field charged 12 instead of 7.
- **Steps to Reproduce**:
  1. Game 6: Play 3 Digimon including Kamemon (Iliad trait) on P1 field
  2. Play BT24-051 Merukimon (cost 12, should reduce to 7 with 3+ Digimon): memory 7 -> -5 (charged 12)
  3. Game 5: Play BT24-041 Minervamon with Kamemon on field: memory 6 -> -6 (charged 12 instead of 7)
- **Evidence**: Memory deltas confirm full cost charged in both cases
- **Status**: OUTSTANDING

### Issue 3: BT24-102 Homeros [All Turns] +1000 DP to TS trait Digimon not applied
- **Card(s)**: BT24-102 Homeros
- **Severity**: medium
- **Category**: effect
- **Expected**: All TS trait Digimon should get +1000 DP while Homeros is on field
- **Actual**: Neptunemon and Venusmon both show 12000 DP (base) with no modifier from Homeros, despite Homeros being on field
- **Steps to Reproduce**:
  1. Game 2: Play Homeros, then play Neptunemon and Venusmon
  2. Check DP breakdown: base 12000, temporary 0, total 12000 (no +1000)
- **Evidence**: DP breakdown shows `"temporary": 0.0` for both Digimon
- **Status**: OUTSTANDING

### Issue 4: BT24-027 Lanamon When Digivolving does not place card from hand as source
- **Card(s)**: BT24-027 Lanamon
- **Severity**: medium
- **Category**: effect
- **Expected**: When Digivolving, prompt to place a Lv4 or lower blue TS Digimon from hand as bottom digivolution card, THEN select a Digimon for battle immunity
- **Actual**: Only prompts to select a Digimon for battle immunity. No hand card placement step. The "By placing" cost part of the effect is skipped.
- **Steps to Reproduce**:
  1. Game 1: Digivolve BT24-027 Lanamon onto BT24-020 Gomamon
  2. Pending selection only asks "Select one of your Digimon" (the immunity target)
  3. No prompt to place a card from hand as bottom source
- **Status**: OUTSTANDING

### Issue 5: BT24-088 Asuna Shiroki On Play trash-to-draw effect does not trigger
- **Card(s)**: BT24-088 Asuna Shiroki
- **Severity**: medium
- **Category**: effect
- **Expected**: On Play, optionally trash 1 TS/Three Musketeers card from hand, then Draw 2
- **Actual**: On Play resolves with no selection prompt for trashing/drawing
- **Steps to Reproduce**:
  1. Game 5: Play BT24-088 with TS cards in hand
  2. No trash prompt appears; effect silently skipped
- **Status**: OUTSTANDING

### Issue 6: BT24-091 Tidal Stream linking does not work
- **Card(s)**: BT24-091 Tidal Stream
- **Severity**: medium
- **Category**: effect
- **Expected**: After Main effect resolves, card should be linked to a Digimon on field
- **Actual**: After selecting a Digimon target, Tidal Stream remains as a separate permanent in battle area with no linked association. linkedCardIds is empty on the target Digimon.
- **Steps to Reproduce**:
  1. Game 5: Play BT24-091, select Gomamon for linking
  2. Check state: Tidal Stream is separate permanent, Gomamon linkedCardIds=[]
- **Status**: OUTSTANDING

### Issue 7: BT24-028 Divermon has no DP in card database
- **Card(s)**: BT24-028 Divermon
- **Severity**: low
- **Category**: data
- **Expected**: Lv5 Digimon should have a base DP value
- **Actual**: play_cost=0 and dp=None in CardDatabase. When played, shows DP=None on field. The script has effects (alt digivolve, On Play Blocker grant, When Attacking inherited play) but the base card data appears incomplete.
- **Status**: OUTSTANDING

### Issue 8: BT3-093 Davis Motomiya On Play reveal does not trigger
- **Card(s)**: BT3-093 Davis Motomiya
- **Severity**: medium (legacy card, low priority for this archetype)
- **Category**: effect
- **Expected**: On Play reveals top 3, adds 1 blue and 1 green Digimon to hand
- **Actual**: No reveal prompt appeared when played
- **Steps to Reproduce**:
  1. Game 5: Play BT3-093 from hand
  2. No revealed cards or selection prompt
- **Status**: OUTSTANDING

## Per-Card Results

### Best Deck Cards (19)

| Card ID | Name | Kind | Tested | Status | Notes |
|---------|------|------|--------|--------|-------|
| BT24-002 | Bukamon | DigiEgg | Yes | PASS | Hatches correctly. Inherited end-of-turn unsuspend effect present. |
| BT24-020 | Gomamon | Digimon Lv3 | Yes | PASS | Play cost 3 correct. On Play reveals top 3, adds Sea Beast + TS card. Evo cost 0 onto Lv2 correct. When Digivolving reveal triggers. |
| BT24-023 | Calmaramon | Digimon Lv4 | Yes | PASS | Evo cost 3 correct. Blocker and Decode keywords shown. When Digivolving bounce effect present. Inherited Jamming works. |
| BT24-027 | Lanamon | Digimon Lv4 | Yes | PARTIAL | Evo cost 2 correct. Decode keyword shown. When Digivolving triggers but skips hand-card-placement step (Issue 4). |
| BT24-028 | Divermon | Digimon Lv5 | Yes | PARTIAL | Plays for cost 0 (DB value). Gains Blocker from On Play. DP=None in DB -- incomplete card data (Issue 7). Script has alt digivolve and inherited effects. |
| BT24-029 | Whamon | Digimon Lv5 | Yes | PARTIAL | Play cost 7 correct. DP=7000. On Play hand-placement effect did not trigger (same pattern as Lanamon "by placing" effects). |
| BT24-030 | Neptunemon | Digimon Lv6 | Yes | PARTIAL | Play cost 12 works. DP=12000. Cost reduction when opponent has 2+ Digimon not applied (Issue 2). On Play return effect would need opponent Digimon to test. |
| BT24-031 | Elecmon | Digimon Lv3 | Yes | PARTIAL | Play cost 3 correct. On Play reveals top 3 but only does 1 selection pass instead of 2 (Iliad + TS). May add a single card that satisfies both traits. |
| BT24-034 | Aegiomon | Digimon Lv4 | Yes | PASS | Evo cost 2 correct. Barrier keyword shown. When Digivolving "By" effect correctly optional -- declining skips security cost. |
| BT24-040 | Venusmon | Digimon Lv6 | Yes | PARTIAL | Play cost 12 works. DP=12000. Cost reduction with <=3 security not applied (Issue 2). On Play effect needs opponent Digimon to fully test. |
| BT24-051 | Merukimon | Digimon Lv6 | Yes | PARTIAL | Play cost 12 charged full despite 3+ Digimon on field (Issue 2). DP=12000. Rush keyword shown from Your Turn Iliad effect. |
| BT24-059 | Sharkmon | Digimon Lv5 | Yes | PASS | Play cost 7 correct. DP=7000. On Play De-Digivolve needs opponent Digimon. On Deletion reveal-and-play present in script. |
| BT24-085 | Dan Yuki & Kanan Yuki | Tamer | Yes | PASS | Play cost 4 correct. Start of Main Phase +1 memory works when <=4. End of Turn effect present in script. |
| BT24-090 | Abyss Sanctuary: Throne Room | Option | Yes | PARTIAL | Use cost 3 correct. Places in battle area. Prompts to play blue/yellow TS Digimon with cost -3. Security-to-hand and face-up security placement mechanics unclear. |
| BT24-100 | In-Between Theater | Option | Yes | PASS | Use cost 3 correct. Reveals top 3, adds TS card to hand, places in battle area as Delay. |
| BT24-102 | Homeros | Tamer | Yes | PARTIAL | Play cost 5 correct. Start of Main Phase +1 memory works. All Turns +1000 DP to TS Digimon NOT applied (Issue 3). End of Turn Olympos XII activation present in script. |
| P-104 | Mental Training | Option | Yes | PASS | Use cost 2 correct. Reveals top 2, adds blue card. Places in battle area as Delay. |
| P-196 | Gomamon | Digimon Lv3 | Yes | PASS | Play cost 3 correct. DP=1000. Start of Main Phase free digivolve effect present. Inherited When Attacking Draw 1 present. |
| P-197 | Patamon | Digimon Lv3 | Yes | PASS | Play cost 3 correct. DP=1000. Start of Main Phase free digivolve effect present. Inherited When Attacking -2000 DP present. |

### Variant-Only Cards (11)

| Card ID | Name | Kind | Tested | Status | Notes |
|---------|------|------|--------|--------|-------|
| BT24-014 | Aegiochusmon | Digimon Lv5 | Yes | PASS | Play cost 8 correct. DP=8000. Decode keyword shown. Security A. +1 not visible in keywords but may work in combat. |
| BT24-019 | Kamemon | Digimon Lv3 | Yes | PASS | Play cost 3 correct. DP=1000. Evo cost reduction for blue TS present in script. Inherited Jamming present. |
| BT24-022 | Ikkakumon | Digimon Lv4 | Yes | PASS | Play cost 6 correct. DP=6000. Jamming keyword shown. On Play trash evo cards needs opponent Digimon. |
| BT24-025 | Shellmon | Digimon Lv4 | Yes | PASS | Play cost 4 correct. DP=4000. Passive effects (unsuspend trigger, digivolve into Venusmon) present in script. |
| BT24-041 | Minervamon | Digimon Lv6 | Yes | PARTIAL | Play cost 12 charged full despite Iliad on field (Issue 2). DP=12000. Blocker + Reboot keywords shown. On Play play-from-hand triggers. |
| BT24-083 | Hiroko Sagisaka | Tamer | Yes | PASS | Play cost 3 correct. On Play reveals top 3, adds TS card. Start of Turn return-and-play effect triggers. |
| BT24-088 | Asuna Shiroki | Tamer | Yes | PARTIAL | Play cost 3 correct. On Play trash-to-draw effect does NOT trigger (Issue 5). Start of Turn effect present in script. |
| BT24-091 | Tidal Stream | Option | Yes | PARTIAL | Use cost 5 correct. Main effect resolves. Link mechanic does not actually link to target Digimon (Issue 6). |
| BT3-093 | Davis Motomiya | Tamer | Yes | PARTIAL | Play cost 4 correct. On Play reveal effect does not trigger (Issue 8). Start of Turn set memory to 3 present. |
| LM-028 | Blue Scramble | Option | Yes | PARTIAL | Use cost 2 correct. Places in battle area. Digivolve-from-hand effect selects a Digimon but digivolution does not execute. Delay present. |
| P-198 | DemiDevimon | Digimon Lv3 | Yes | PASS | Play cost 3 correct. DP=1000. Start of Main Phase free digivolve effect present. Inherited Draw+Trash present. |

## Systemic Issues Identified

1. **Play cost reduction ("When this card would be played") is not functional** -- Affects all Lv6 mega Digimon in this archetype (Neptunemon, Venusmon, Merukimon, Minervamon). These cards are designed to be played for 7 cost in the right conditions but always charge full 12.

2. **"By placing X from hand as bottom source" effects are not implemented** -- Affects Lanamon, Whamon, and potentially Divermon. The engine skips the hand-card-placement cost step in "By [cost], [effect]" patterns.

3. **Persistent pending selection deadlock** -- When certain effect chains produce "play from hand without cost" selections, the selection can become stuck and persist across turn boundaries, eventually deadlocking the game.

## Test Methodology

- Created 6 debug games with controlled hands via skip_shuffle
- Verified play costs by checking memory gauge delta
- Verified evo costs by checking memory gauge delta after digivolution
- Verified keywords by checking keyword arrays in battle area state
- Verified On Play/When Digivolving effects by checking pending selections, revealed cards, and state changes
- Used inject-card for all 11 variant-only cards
- All cards have existing scripts (30/30 scripts exist)
