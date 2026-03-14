# Archetype QA: Medusa
Date: 2026-03-14 (updated from 2026-03-12)
Total cards: 53

## Summary (Round 2 — Opus 4.6 review)
- PASS: 39 (verified correct against card text + C# reference)
- QA-FAIL found and fixed (round 2): 7
  - Stub fixes: 3 (EX8-074, EX9-013, P-206)
  - Spot-check fixes: 4 (BT24-016, BT21-072, BT24-082, BT24-089)
- Previously fixed (round 1): 24
- BLOCKED: 1 (partial — BT5-008 evo cost prevention)

## Round 2 Fixes

### Stub Cards Fixed (3)

| Card ID | Card Name | Problem | Resolution |
|---------|-----------|---------|------------|
| EX8-074 | MedievalGallantmon | [When Digivolving] "suspend 1 Digimon" only selected own Digimon; C# `IsPermanentExistsOnBattleAreaDigimon` allows ANY Digimon on either field. Also fragile `on_decline` wrapping. | Rewrote suspend step to offer opponent Digimon first, then own, with proper decline chaining. |
| EX9-013 | BlitzGreymon | [End of Your Turn] DNA digivolve + attack was `pass` stub. Missing alt-digi requirements (Greymon name OR DM trait). | Implemented DNA digivolve via `effect_dna_digivolve_from_hand` for Omnimon Alter-S + FORCE_ATTACK grant. Added two alt-digi effects (one for Greymon name, one for DM trait) to handle OR condition. |
| P-206 | Digital Gate Open | "Ignore color requirements" was `pass` stub. Security "add to hand" didn't check card location. | Set `card._match_color_requirement = False` at init. Fixed security add-to-hand to remove from trash first. |

### Spot-Check Fixes (4)

| Card ID | Card Name | Problem | Resolution |
|---------|-----------|---------|------------|
| BT24-016 | Lamiamon | ESS play filter missing 5000 DP limit — card text says "5000 DP or lower [Reptile]/[Dragonkin]" | Added `card_dp > 5000` check to `play_filter` |
| BT21-072 | Arresterdramon: Superior Mode | DP bonus was "+1000 per opponent Digimon" — card text says "+1000 DP for each of **its digivolution cards**" | Changed `dp_value_fn` to count `len(card_sources) - 1` (digivolution cards under top) |
| BT24-082 | Owen Dreadnought | "may attack" was implemented as unsuspend — doesn't actually grant an attack | Changed to `FORCE_ATTACK` modifier registration |
| BT24-089 | Unique Emblem: Blazing Conductor | Delay condition checked Reptile/Dragonkin traits on Owen Dreadnought (a Tamer with no such traits) | Removed incorrect trait check from delay condition |

## Spot-Check Results (10 clean scripts verified)

| Card ID | Card Name | Verdict | Notes |
|---------|-----------|---------|-------|
| BT24-016 | Lamiamon | **FIXED** | ESS DP limit was missing |
| BT24-018 | Styracomon | PASS | Keywords, effects, deletion prevention correct |
| BT21-029 | Medusamon | PASS | Delete lowest DP, token play, security removal trigger all correct |
| BT21-093 | Raging Serpentine | PASS | Cost reduction, delay digivolve, security all correct |
| BT23-014 | Gallantmon | PASS | Trash play block + DP-scaled delete correct |
| BT20-102 | Omnimon (X Antibody) | PASS | Board wipe, Rush + unsuspend attack correct |
| EX11-012 | Medusamon | PASS | DP-capped delete, token play, token-death-prevention all correct |
| BT21-072 | Arresterdramon SM | **FIXED** | DP bonus source was wrong |
| BT24-082 | Owen Dreadnought | **FIXED** | "may attack" mechanism was wrong |
| BT24-089 | Unique Emblem | **FIXED** | Delay trigger condition was wrong |

## Round 1 Summary (from 2026-03-12)
- PASS: 5 (existing frozen, QA-verified: BT24-011, BT24-001, BT24-008, BT21-017, P-189)
- QA-FAIL found and fixed: 24 (11 simple, 12 complex, 1 partial blocked)

### Previously Fixed Cards (Round 1)

<details>
<summary>Click to expand round 1 details</summary>

#### SIMPLE-FIX Applied (11)
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

#### COMPLEX-FIX Revised (12)
| Card ID | Problem | Resolution |
|---------|---------|------------|
| BT24-012 | WhenRemoveField missing opponent-effect check, wrong self-bounce | Rewrote condition with `is_opponent_effect` + trait check; process returns self to hand + registers CANNOT_BE_REMOVED |
| BT24-016 | Broken [Hand][Main] condition, extra attack gate, missing ESS turn guard | Converted to alt-digi pattern; split attack effect into WhenDigivolving + OnUseAttack with shared hash; added `is_my_turn` to ESS |
| BT24-017 | Missing on-attack +2000 DP effect entirely | Added new OnUseAttack effect with once-per-turn; fixed `deck_cards` → `library_cards` |
| BT24-018 | Missing Piercing, stub WhenRemoveField, non-optional security, wrong unsuspend | Added Piercing keyword; implemented WhenRemoveField with delete-to-prevent; made security trash conditional; unsuspend targets self only |
| BT21-008 | Single reveal selection instead of two pools | Replaced with manual two-step sequential selection (Reptile/Dragonkin first, then LIBERATOR) |
| BT21-025 | No trait check on attack target change, missing ESS turn guard | Added Reptile/Dragonkin trait check on attacking perm; added `is_my_turn` to ESS |
| BT21-029 | 2 effects with NO process callbacks (stubs) | Implemented WhenDigivolving + EndOfAttack delete callbacks; added once-per-turn; fixed opponent checks on token play |
| BT21-072 | Wrong modifier arg order, static DP, invalid expiry | Fixed arg order; replaced static DP with dynamic `register_modifier` counting digivolution cards; fixed `'persistent'` → `'permanent'` |
| BT21-081 | Missing opponent gate, no trait filter, force attack stub | Added opponent-has-Digimon gate; restricted target to Reptile/Dragonkin; added turn expiry to Piercing; replaced stub with FORCE_ATTACK modifier |
| BT24-089 | Wrong filter (names vs traits), no source selection | Fixed digi_filter to Reptile/Dragonkin+LIBERATOR traits; added source perm selection; fixed Owen Dreadnought trigger check |
| EX11-012 | Missing DP cap on delete, WhenRemoveField stub | Added DP comparison to delete target filter; implemented token-delete-to-prevent WhenRemoveField |
| EX11-054 | Missing trait check, missing +3000 DP to Progress | Split into On Play + When Digivolving effects; added Reptile/Dragonkin trait check; added Progress Digimon DP grant |
| P-103 | Spurious trash pop, no red filter, no cost reduction | Removed trash pop; added red filter to reveal; added red filter + `cost_reduction=2` to delay digivolve; added SecuritySkill |

</details>

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
4. **Any-field Digimon selection** (EX8-074): Engine only has `effect_select_own_permanent` and `effect_select_opponent_permanent`; "suspend 1 Digimon" (either side) requires sequential selection pattern

## Files Modified (Round 2)
- digimon_gym/engine/data/scripts/ex8/EX8_074.py
- digimon_gym/engine/data/scripts/ex9/EX9_013.py
- digimon_gym/engine/data/scripts/p/P_206.py
- digimon_gym/engine/data/scripts/bt24/BT24_016.py
- digimon_gym/engine/data/scripts/bt21/BT21_072.py
- digimon_gym/engine/data/scripts/bt24/BT24_082.py
- digimon_gym/engine/data/scripts/bt24/BT24_089.py
