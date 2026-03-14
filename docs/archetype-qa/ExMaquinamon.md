# Archetype QA: ExMaquinamon
Date: 2026-03-14 (rev 2)
Total cards: 16

## Summary
- PASS: 3 (EX11-006, EX11-045, EX11-071)
- FIXED: 13

## Card Verdicts

| Card ID   | Name                         | Verdict     | Notes |
|-----------|------------------------------|-------------|-------|
| BT3-103   | Hidden Potential Discovered! | FIXED       | Cost reduction now only applies to green Digimon (was applying to ALL) |
| EX11-006  | Flickmon                     | PASS        | Linked-with-Maquinamon check and digivolve-from-hand correct |
| EX11-027  | Maquinamon                   | FIXED       | Link-after-reveal now offers 3-way choice (self/hand/decline) per C#; WhenRemoveField lets player choose which linked card |
| EX11-029  | Turbomon                     | FIXED       | Digi card source now uses player selection instead of auto-select; zone choice (hand vs digi) through effect_choose_branch |
| EX11-033  | Maneuvermon                  | FIXED       | Source is "hand or link cards" (was digi cards); full zone choice; inherited battle-win condition checks battle context |
| EX11-036  | Dalphomon                    | FIXED       | Suspends 2 Digimon/Tamers (was 1 Digimon-only); cannot-unsuspend targets Digimon or Tamers; inherited uses FORCE_ATTACK modifier (engine API now available) |
| EX11-040  | Mulemon                      | FIXED       | Digi card source now uses player selection; zone choice through effect_choose_branch |
| EX11-042  | MockingBirdmon               | FIXED       | Source is "hand or link cards" (was hand-only); full zone choice; WhenLinked adds Your Turn guard; redirect uses redirect_attack (engine API now available) |
| EX11-045  | Metatromon                   | PASS        | De-digivolve + cannot-digivolve correct; end-of-turn digivolve targets OTHER Digimon correctly; inherited delete-lowest correct |
| EX11-062  | Shoto Kazama                 | FIXED       | register_modifier argument order fixed (was target/type swapped for CHANGE_DP and VORTEX_CAN_ATTACK_PLAYERS) |
| EX11-070  | Unchained                    | FIXED       | Play Unchained from digi cards uses selection instead of auto-select; DP floor and stack-trash immunity conditions check permanent directly |
| EX11-071  | Cool Boy                     | PASS        | Reveal+select and Main deck-bounce effect correct |
| EX11-073  | ExMaquinamon                 | FIXED       | Link-up-to-3 uses full zone-choice loop (hand/trash/digi) with selection per C#; was auto-selecting trash and digi cards |
| EX6-072   | Mega Digimon Assembly!       | FIXED       | Security trash-to-hand uses player selection instead of auto-select |
| LM-048    | Chrome Memory Boost!         | FIXED       | Reveal uses effect_reveal_and_select for player choice (was auto-selecting first match) |
| P-151     | Digimon Liberator            | FIXED       | Security effect activates Main effects (was incorrectly playing the card instead) |

## Fix Details

### Auto-selection removals (zero auto-selections rule)
- **EX11-029, EX11-040**: Digivolution card source selection now uses `effect_choose_branch` for zone choice + `request_selection` for card pick
- **EX11-033, EX11-042**: Link card source selection now uses `effect_choose_branch` for zone choice (hand vs link cards)
- **EX11-073**: Full zone-choice loop (hand/trash/digi) with `effect_choose_branch` and proper selection APIs for each zone
- **EX11-070**: Unchained from digi cards uses `request_selection` with `SelectEffectChoice`
- **EX6-072**: Security trash selection uses `request_selection` with `SelectTrash`
- **LM-048**: Reveal uses `effect_reveal_and_select` for agent choice
- **EX11-027**: WhenRemoveField link card selection when multiple exist; link-after-reveal 3-way zone choice

### Faithfulness fixes
- **EX11-036**: Suspend count 1 -> 2; target filter Digimon-only -> Digimon or Tamers; cannot-unsuspend targets Digimon or Tamers; inherited force-attack uses `game.register_modifier(perm, ModifierType.FORCE_ATTACK, ...)` (previously blocked, now engine API available)
- **EX11-033**: Source changed from "digi cards" to "link cards" per C# `LinkedCards` reference
- **EX11-042**: Source changed from "hand only" to "hand or link cards" per C# `LinkedCards` reference; added Your Turn guard to WhenLinked delete effect; redirect uses `game.redirect_attack(perm)`
- **BT3-103**: Cost reduction restricted to green Digimon only (was applying to all Digimon)
- **P-151**: Security effect changed from "play this card" to "activate this card's [Main] effects"
- **EX11-062**: `register_modifier` calls had swapped argument order (ModifierType, target) -> corrected to (target, ModifierType)

### Previously blocked, now resolved
- **EX11-036 force_attack**: `game.register_modifier(perm, ModifierType.FORCE_ATTACK, ...)` now available
- **EX11-042 redirect_attack**: `game.redirect_attack(new_target_perm)` now available
