# Gameplay QA Report — Royal Knights vs Medusamon

## Test Setup
- **Date**: 2026-03-22
- **Archetypes**: Royal Knights (P1) vs Medusamon (P2)
- **Deck Sources**: deck_library.json best-placement decklists
- **Game IDs**:
  - Game 1: `310f22c9-ded0-4239-af51-ef5999cc79e9` (organic play, 6 turns)
  - Game 2: `5cb75de8-e8e2-4ec8-85df-781c8bf572e6` (injected cards, 6 turns)
  - Game 3: `1edf236b-06a5-4fb5-9d57-ff1531524164` (high initial memory, 6 turns)
  - Game 4: `af0798b1-845d-4cac-ae29-819317af8d00` (remaining cards, 6 turns)
- **Total Turns Played**: ~24
- **Focus Areas**: King Drasil_7D6 breeding, Royal Knight play effects, Medusamon attack/delete, digivolve validation, "by" cost optionality

## Summary
- **Total Issues Found**: 5
- Critical: 2 | High: 2 | Medium: 1 | Low: 0

## Detailed Findings

### Issue 1: King Drasil_7D6 (BT13-007) immediately trashed from breeding area
- **Card(s)**: BT13-007 — King Drasil_7D6
- **Severity**: critical
- **Category**: game_flow
- **Expected**: King Drasil_7D6 is a DigiEgg designed to stay in the breeding area. It has `[Breeding]` timing effects that operate from that zone, including cost reduction for Royal Knights, absorbing Royal Knight Digimon as evo cards, and generating memory. The card should remain in breeding and function as the linchpin of the Royal Knights archetype.
- **Actual**: Upon hatching, the engine logs `[Rule Process] Non-Digimon in breeding area — trashed` and immediately removes the card. This is because King Drasil_7D6 has `Level: None` and `DP: None`, causing the engine to classify it as a non-Digimon.
- **Steps to Reproduce**:
  1. Create a game with BT13-007 in the digitama deck
  2. Select Hatch (action 60)
  3. Observe: "Player 1 hatched King Drasil_7D6" followed by "Non-Digimon in breeding area — trashed"
- **Evidence**: Confirmed across all 4 games, every hatch attempt trashes the card
- **Impact**: This breaks the ENTIRE Royal Knights archetype. Cascading effects include:
  - EX11-053/BT20-083 Omekamon On Play "tuck under King Drasil_7D6" has no target
  - BT13-110 Royal Knights of the Purge Delay "play from breeding evo cards" has no target
  - BT20-100 The Last Guardian "Omnimon-name leaving" Delay has no breeding support
  - No White presence on field → White Options (BT20-100, BT13-110) unplayable via color restriction
  - BT13-007's cost reduction, memory generation, and Royal Knight absorption never function

### Issue 2: Dynasmon BT23-035 "by trashing security" cost auto-accepted without player choice
- **Card(s)**: BT23-035 — Dynasmon
- **Severity**: critical
- **Category**: effect
- **Expected**: Card text says "[On Play] **By** trashing your top security card, all of your opponent's Digimon get -6000 DP for the turn." The keyword "by" indicates a cost the player must choose to pay. The engine should present an optional selection: pay the cost (trash own security) or decline.
- **Actual**: The effect auto-fires without presenting any choice. The player's security is automatically trashed and the -6000 DP is applied with no option to decline.
- **Steps to Reproduce**:
  1. Play BT23-035 Dynasmon from hand
  2. Observe: On Play effect fires automatically, security trashed, no selection offered
- **Evidence**: Game 3, Turn 3. Memory 3→-9, no selection phase appeared. LOG shows both the "by" effect and the cascading OnLoseSecurity effect firing sequentially.
- **Rules Reference**: "By [doing X]" in card text indicates a cost the player optionally pays to activate the effect.

### Issue 3: Styracomon BT24-018 digivolve validator allows Lv7 onto Lv5
- **Card(s)**: BT24-018 — Styracomon
- **Severity**: high
- **Category**: digivolution
- **Expected**: Styracomon is Lv7 with evolution requirement "Red Lv.6 for 4". It should only be able to digivolve onto Lv6 Digimon.
- **Actual**: In Game 2 Turn 4, action 416 offered "Digivolve Styracomon onto Lamiamon" where Lamiamon (BT24-016) is a Lv5 Digimon. This should not be a valid digivolve target.
- **Steps to Reproduce**:
  1. Have Lamiamon (Lv5) on field
  2. Have Styracomon (Lv7) in hand
  3. Check available actions — digivolve onto Lamiamon appears
- **Evidence**: Game 2 (`5cb75de8`), Turn 4 actions. Action 416 listed alongside valid action 401 (Medusamon Lv6 onto Lamiamon Lv5).
- **Notes**: The action was never executed, so it's unknown if the engine would actually process the digivolve. But appearing in the action mask is itself a bug.

### Issue 4: BT20-091 Cool Boy "by suspending this Tamer" auto-fires without choice
- **Card(s)**: BT20-091 — Cool Boy
- **Severity**: high
- **Category**: effect
- **Expected**: Card text says "[Your Turn] When any of your Digimon are played, if any of them have the [Royal Knight] trait, **by suspending** this Tamer, <Draw 1> and gain 1 memory." The "by suspending" is a cost the player should choose to pay.
- **Actual**: When LordKnightmon (Royal Knight) was played via Cool Boy EX11-071's effect, BT20-091's trigger auto-fired without presenting a choice to suspend or decline.
- **Steps to Reproduce**:
  1. Have BT20-091 Cool Boy on field (unsuspended)
  2. Play a Digimon with [Royal Knight] trait
  3. Observe: Cool Boy auto-suspends, draw + memory gain happen without choice
- **Evidence**: Game 1 (`310f22c9`), Turn 5. Memory went 10→1→2 (play cost then auto +1 from Cool Boy). No selection phase appeared.
- **Rules Reference**: "By [doing X]" costs must be optional per card text.

### Issue 5: debug/set-memory doesn't recalculate action mask
- **Card(s)**: N/A — debug endpoint
- **Severity**: medium
- **Category**: game_flow
- **Expected**: After setting memory via `POST /debug/games/{id}/set-memory`, available actions should be recalculated to reflect the new memory value.
- **Actual**: After setting memory to -10 (P2 gets 10), P2's action list only showed Pass and Delay — no play actions despite having sufficient memory. The action mask was computed with the old memory value.
- **Steps to Reproduce**:
  1. Create a game where P2 has 1 memory in main phase
  2. `POST /debug/games/{id}/set-memory` with `{"memory": -10}`
  3. `GET /games/{id}/actions` — play actions are missing
- **Evidence**: Game 1 Turn 4.

## Cards Tested Successfully

### Royal Knights
| Card | Name | Status | Notes |
|------|------|--------|-------|
| BT13-093 | Omekamon | PASS | On Play Draw 1 correct |
| BT23-054 | Magnamon | PASS | On Play Draw + Royal Knight/CS protection works. "Attack target" label in SelectTarget is cosmetic (known WONTFIX). |
| EX11-071 | Cool Boy | PASS | On Play reveal top 3, add matching cards. Active effect: return to deck, play Royal Knight/LIBERATOR at -2 cost. Both work correctly. |
| BT20-017 | Jesmon | PASS | On Play creates Atho/Rene/Por Token (6000 DP, Reboot/Blocker/Decoy). Your Turn trigger correctly auto-skips when no valid targets. |
| BT23-058 | Craniamon | PASS | Play cost correct. No On Play effects (correct). Reboot/Blocker passive keywords present. |
| BT13-110 | Royal Knights of Purge | PASS | Option: Draw 1, placed as Delay. Tuck-under-KD7D6 has no target (cascading from Issue 1). |
| BT19-072 | LordKnightmon | PASS | Played at reduced cost via Cool Boy EX11-071 effect. On Play "play Lv4 from trash" auto-skipped (no valid targets). |

### Medusamon
| Card | Name | Status | Notes |
|------|------|--------|-------|
| BT21-029 | Medusamon | PASS | WhenDigivolving: delete opponent's lowest DP works. Petrification Token placement works. Progress correctly blocks security effects. Security Attack +1 present but attacker deleted before 2nd check (correct). |
| BT24-016 | Lamiamon | PASS | Play cost 7 correct. No On Play effects (correct — effects are When Digivolving/Attacking). |
| BT18-087 | Owen Dreadnought | PASS | Start of Turn "memory to 3 if 2 or less" works correctly (tested with 4 memory — no change, correct). Security removal trigger fires correctly. |
| BT24-082 | Owen Dreadnought | PASS | Play cost 3 correct. Start of Main effect auto-skips when no Owen in hand (correct). |
| BT24-089 | Unique Emblem | PASS | Main effect: plays Elizamon/Owen from hand/trash free. Placed in battle area as Delay. |
| BT24-008 | Dimetromon | PASS | On Play "by trashing Reptile/Dragonkin/LIBERATOR, Draw 2" correctly presents decline option (action 62). Card trashed, 2 drawn. |
| BT21-008 | Elizamon | PASS | Digivolves onto Lv2 egg correctly (evo cost 0). |

## Cards Not Covered
Due to time constraints and the King Drasil_7D6 breeding bug blocking Royal Knights strategy, the following cards were not fully tested in gameplay:

### Royal Knights (not tested)
- BT13-075 Alphamon, BT13-112 Omnimon, BT20-060 Alphamon: Ouryuken, BT20-102 Omnimon (X Antibody) — Lv6/7 too expensive without cost reduction from King Drasil_7D6
- BT22-052 Leopardmon — action mix-up prevented testing
- EX4-065 Trident Gaia — not played (no Red color on field for Option)
- BT20-100 The Last Guardian — couldn't play (no White on field due to Issue 1)
- BT20-083 Omekamon — in hand but not played
- EX11-053 Omekamon — On Play only triggered "Also Treated As" (cascading from Issue 1)
- 40+ additional Royal Knights cards from broader archetype pool

### Medusamon (not tested)
- BT16-082, BT17-018, BT20-102, BT21-093, BT23-014, BT8-097, EX11-012, EX4-006, EX8-074, EX9-008, P-206

## Key Observations

1. **King Drasil_7D6 is the #1 blocker**: The Royal Knights archetype cannot function without this card staying in breeding. Fixing Issue 1 should be the top priority before further Royal Knights QA.

2. **"By" cost auto-acceptance is systemic**: Both Dynasmon (BT23-035) and Cool Boy (BT20-091) auto-accept "by" costs without presenting choices. This likely affects other cards with "by" cost patterns across the entire card pool.

3. **SelectTarget/SelectReveal descriptions remain cosmetic**: Action descriptions in selection phases are mislabeled (e.g., "Attack player with X" during a SelectTarget for a delete effect). This is a known WONTFIX from prior QA reports.

4. **Medusamon archetype mostly working**: Core mechanics (Petrification Tokens, Progress, delete-on-security-removal) function correctly. The archetype is in better shape than Royal Knights.
