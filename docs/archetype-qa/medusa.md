# Archetype QA: Medusa
Date: 2026-03-12
Total cards: 30

## Summary
- PASS: 5 (existing frozen, QA-verified: BT24-011, BT24-001, BT24-008, BT21-017, P-189)
- QA-FAIL found and fixed: 24
  - SIMPLE-FIX (tech lead / orchestrator fixed): 11
  - COMPLEX-FIX (Sonnet agents revised): 12
  - STILL-BROKEN: 0
- BLOCKED: 1 (partial — BT5-008 evo cost prevention)

## FALSE-POSITIVE Cards (5)
| Card ID | Reason |
|---------|--------|
| BT24-001 | `is_my_turn` is the accepted proxy for opponent security check |
| BT24-008 | Engine creates permanent before firing OnEnterFieldAnyone; `is_my_turn` proxy valid |
| BT21-017 | Card text has no "1 or fewer Tamers" gate — QA agent misread |
| P-189 | `is_my_turn` proxy for opponent security check is valid |
| BT21-093 (partial) | "Missing battle area placement" — engine handles via `_is_delay` |

## SIMPLE-FIX Applied (11)
| Card ID | Fix Summary |
|---------|-------------|
| BT21-001 | Added `cost_reduction=1` to `effect_digivolve_from_hand` |
| BT21-093 | Added opponent security check (`ctx_player is card.owner`) to delay condition |
| BT18-087 | Added opponent security check + tamer suspend check to OnLoseSecurity |
| BT20-102 | Fixed X Antibody check: `card_traits` → `card_names` |
| BT23-005 | Added Reptile/Dragonkin trait filter to BeforePayCost condition |
| BT8-097 | Fixed `register_modifier` arg order (was `ModifierType, enemy` → per-perm iteration) |
| EX11-008 | Added `game.turn_count` duration to `grant_keyword('_is_raid')` for turn expiry |
| EX10-010 | Fixed `register_modifier` arg order + changed expiry to `'permanent'` |
| LM-027 | Added `cost_reduction=3` to digivolve call; fixed `deck_cards` → `library_cards` |
| P-035 | Added missing SecuritySkill effect (play self from security) |
| BT24-082 | Added FORCE_ATTACK modifier for "may attack" after DP grant |

## COMPLEX-FIX Revised (12)
| Card ID | Problem | Resolution |
|---------|---------|------------|
| BT24-012 | WhenRemoveField missing opponent-effect check, wrong self-bounce | Rewrote condition with `is_opponent_effect` + trait check; process returns self to hand + registers CANNOT_BE_REMOVED |
| BT24-016 | Broken [Hand][Main] condition, extra attack gate, missing ESS turn guard | Converted to alt-digi pattern (`_alt_digi_name = "Elizamon"`, `_alt_digi_cost = 3`); split attack effect into WhenDigivolving + OnUseAttack with shared hash; added `is_my_turn` to ESS |
| BT24-017 | Missing on-attack +2000 DP effect entirely | Added new OnUseAttack effect with once-per-turn; fixed `deck_cards` → `library_cards` |
| BT24-018 | Missing Piercing, stub WhenRemoveField, non-optional security, wrong unsuspend | Added Piercing keyword; implemented WhenRemoveField with delete-to-prevent; made security trash conditional; unsuspend targets self only |
| BT21-008 | Single reveal selection instead of two pools | Replaced with manual two-step sequential selection (Reptile/Dragonkin first, then LIBERATOR) |
| BT21-025 | No trait check on attack target change, missing ESS turn guard | Added Reptile/Dragonkin trait check on attacking perm; added `is_my_turn` to ESS |
| BT21-029 | 2 effects with NO process callbacks (stubs) | Implemented WhenDigivolving + EndOfAttack delete callbacks; added once-per-turn; fixed opponent checks on token play |
| BT21-072 | Wrong modifier arg order, static DP, invalid expiry | Fixed arg order; replaced static DP with dynamic `register_modifier` counting opponent Digimon; fixed `'persistent'` → `'permanent'` |
| BT21-081 | Missing opponent gate, no trait filter, force attack stub | Added opponent-has-Digimon gate; restricted target to Reptile/Dragonkin; added turn expiry to Piercing; replaced stub with FORCE_ATTACK modifier |
| BT24-089 | Wrong filter (names vs traits), no source selection | Fixed digi_filter to Reptile/Dragonkin+LIBERATOR traits; added source perm selection; fixed Owen Dreadnought trigger check |
| EX11-012 | Missing DP cap on delete, WhenRemoveField stub | Added DP comparison to delete target filter; implemented token-delete-to-prevent WhenRemoveField |
| EX11-054 | Missing trait check, missing +3000 DP to Progress | Split into On Play + When Digivolving effects; added Reptile/Dragonkin trait check; added Progress Digimon DP grant |
| P-103 | Spurious trash pop, no red filter, no cost reduction | Removed trash pop; added red filter to reveal; added red filter + `cost_reduction=2` to delay digivolve; added SecuritySkill |

## Blocked Cards
### BT5-008 Gaossmon (partial)
- Effect text: "[Opponent's Turn] Your opponent can't reduce digivolution costs."
- Missing mechanic: No modifier type for preventing opponent cost reductions
- Suggested engine change: Add `ModifierType.CANNOT_REDUCE_DIGIVOLVE_COST` that the cost reduction system checks
- Note: DP aura effect was also fixed (added `_dp_permanent_condition` for Gaossmon name filter)

## Known Engine Gaps Affecting This Archetype
1. **Hand-Activated Main Effects** (BT24-016): Approximated with alt-digi pattern
2. **Effect-Based Play Lock** (BT8-097): Uses over-broad `CANNOT_PUT_ON_FIELD` per-permanent
3. **Opponent Evo Cost Prevention** (BT5-008): Returns False (not implementable)

## Smoke Test
- 50/50 mirror-match games completed successfully (greedy first-valid-action policy)

## Files Modified
- digimon_gym/engine/data/scripts/bt18/bt18_087.py
- digimon_gym/engine/data/scripts/bt20/bt20_102.py
- digimon_gym/engine/data/scripts/bt21/bt21_001.py
- digimon_gym/engine/data/scripts/bt21/bt21_008.py
- digimon_gym/engine/data/scripts/bt21/bt21_025.py
- digimon_gym/engine/data/scripts/bt21/bt21_029.py
- digimon_gym/engine/data/scripts/bt21/bt21_072.py
- digimon_gym/engine/data/scripts/bt21/bt21_081.py
- digimon_gym/engine/data/scripts/bt21/bt21_093.py
- digimon_gym/engine/data/scripts/bt23/bt23_005.py
- digimon_gym/engine/data/scripts/bt24/bt24_012.py
- digimon_gym/engine/data/scripts/bt24/bt24_016.py
- digimon_gym/engine/data/scripts/bt24/bt24_017.py
- digimon_gym/engine/data/scripts/bt24/bt24_018.py
- digimon_gym/engine/data/scripts/bt24/bt24_082.py
- digimon_gym/engine/data/scripts/bt24/bt24_089.py
- digimon_gym/engine/data/scripts/bt5/bt5_008.py
- digimon_gym/engine/data/scripts/bt8/bt8_097.py
- digimon_gym/engine/data/scripts/ex10/ex10_010.py
- digimon_gym/engine/data/scripts/ex11/ex11_008.py
- digimon_gym/engine/data/scripts/ex11/ex11_012.py
- digimon_gym/engine/data/scripts/ex11/ex11_054.py
- digimon_gym/engine/data/scripts/lm/lm_027.py
- digimon_gym/engine/data/scripts/p/p_035.py
- digimon_gym/engine/data/scripts/p/p_103.py
