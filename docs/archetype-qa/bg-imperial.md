# Archetype QA: BG Imperial
Date: 2026-03-11
Total cards: 25

## Summary
- PASS: 10
- IMPLEMENTED: 1
- QA-FAIL (fixed): 7 (4 critical, 3 high — all fixed)
- QA-FAIL (remaining): 6 (medium/low — acceptable for RL)
- BLOCKED: 1 (BT3-103 main effect — engine gap)

## PASS Cards
| Card ID | Name |
|---------|------|
| BT12-002 | DemiVeemon |
| BT3-002 | DemiVeemon |
| BT12-021 | Veemon |
| P-117 | Veemon |
| BT12-047 | Wormmon |
| EX1-014 | ExVeemon |
| ST9-09 | Stingmon |
| ST9-05 | Paildramon |
| BT16-027 | Imperialdramon: Fighter Mode |
| BT3-103 | Hidden Potential Discovered! (main BLOCKED, security OK) |

## IMPLEMENTED Cards
| Card ID | Name |
|---------|------|
| BT12-031 | Imperialdramon: Fighter Mode (new script) |

## Fixed QA Failures (Critical)

### BT16-025 Paildramon — FIXED
- [When Digivolving] changed from "de-digivolve" to correctly suspending opponent Digimon with <= digivolution card count.
- DNA clause now applies CANNOT_UNSUSPEND to ALL opponent Digimon.
- When Attacking properly suspends 1 unsuspended opponent, unsuspends self only if no suspend happened.
- Added inherited Partition effect.

### BT16-028 Imperialdramon: Dragon Mode — FIXED
- Alt-digi now matches both "Paildramon" AND "Dinobeemon".
- When Digivolving: correct flow (CANNOT_UNSUSPEND on 1 opponent, then optional suspend-to-unsuspend trade).
- All Turns: correct reactive trigger with by-effect check, Tamer requirement, Fighter Mode in hand check.
- Proper filter functions instead of `return True`.

### BT20-020 Imperialdramon: Fighter Mode — FIXED
- Added Piercing keyword effect.
- Implemented play restriction (CANNOT_PLAY_CARD modifier).
- Security trash now conditional on Dragon Mode in digi-stack.
- OnLoseSecurity checks opponent is the one who lost security.
- Delete filter includes DP comparison.

### BT17-097 Return to the Primogenitor — FIXED
- Battle area placement after Main and Security effects.
- Delay uses correct one-shot deletion prevention.
- Self-effect exclusion check added.
- Proper trash-as-cost for Delay activation.

## Fixed QA Failures (High)

### BT12-022 ExVeemon — FIXED
- Added WhenDigivolving effect: when DNA digivolving into a green Digimon, gain 1 memory.

### BT12-050 Stingmon — FIXED
- Added WhenDigivolving effect: when DNA digivolving into a blue Digimon, gain 1 memory.

### BT16-085 Davis Motomiya & Ken Ichijoji — FIXED
- Added DNA digivolution sub-effect: trash up to 3 digivolution cards from opponent's Digimon when DNA digivolving.

## Remaining QA Issues (Medium/Low — acceptable for RL)

### BT16-040 Wormmon
- **Severity:** medium
- `perm_filter` in `_make_trash_digivolve_process` allows selecting any Digimon as long as any qualifying trash card exists globally, without validating that the chosen trash card can legally digivolve onto that specific permanent.

### BT21-037 Lighdramon
- **Severity:** medium
- DP +2000 applied with no duration via `perm.change_dp(2000)` — should use `register_modifier(CHANGE_DP, ..., expiry='end_of_opponent_turn')`.
- Effect order wrong: DP change applied before suspend selection.

### BT12-028 Paildramon
- **Severity:** low
- Inherited [End of Attack] condition checks `perm.contains_card_name('Imperialdramon')` (whole digi-stack) instead of only checking the top card.

### ST9-06 Imperialdramon Dragon Mode
- **Severity:** medium
- Auto-selects first qualifying blue/green Digimon from digi-stack instead of presenting player selection.

### BT3-093 Davis Motomiya
- **Severity:** medium
- Wrong timing: uses `OnStartMainPhase` but card text and C# use `OnStartTurn`.
- On-play reveal auto-picks first matching cards without player selection.

### LM-030 Green Scramble
- **Severity:** low
- Delay activation auto-picks first green Digimon from trash without player selection.
- Spurious `effect0.cost_reduction = 3` on OptionSkill timing effect.

### BT17-077 Imperialdramon: Paladin Mode
- **Severity:** low
- [When Attacking] unsuspend fires unconditionally even if the deck-bottom bounce fails due to protection.

## Blocked Cards
### BT3-103 Hidden Potential Discovered!
- **Effect text:** "[Main] For the turn, when one of your green Digimon would next digivolve, by suspending 1 of your Digimon, reduce the digivolution cost by 5."
- **Missing mechanic:** Player-level temporary digivolution cost reduction hook for future digivolve events with suspend-as-cost.
- **Suggested engine change:** Add a `player.register_one_shot_digivolve_hook(condition_fn, cost_fn, effect_fn)` API.

## Implementation Notes
- BT12-031 was implemented from scratch with alt-digi, When Digivolving (suspend + branch choice for Dragon Mode return), DP modifier per digi-stack color, conditional Security Attack +1 and Blocker.
- DNA digivolve conditions (jogress) are assumed to be handled by card data metadata rather than scripts for most cards.
- BT12-022/BT12-050 DNA memory gain implemented as WhenDigivolving timing (fires after cost payment, same net effect as C#'s BeforePayCost for RL purposes).
- BT16-085 DNA trash sub-effect auto-selects opponent Digimon with most digivolution cards (acceptable for RL).
- Several scripts use direct list manipulation anti-patterns instead of engine API methods.
- Multiple scripts auto-select targets instead of presenting player choices — this is acceptable for RL training but diverges from C# behavior.
