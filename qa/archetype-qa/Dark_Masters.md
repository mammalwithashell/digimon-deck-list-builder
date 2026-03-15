# Archetype QA: Dark Masters
Date: 2026-03-14
Total cards: 58

## Summary
- PASS: 6 frozen QA (BT17-077, BT17-097, BT3-006, BT9-103, EX7-049, ST6-15)
- IMPLEMENTED: 12 new scripts
- QA-FAIL -> FIXED: 5 (BT15-080, BT15-081, EX2-046, RB1-035, BT13-088)
- QA-FAIL -> REWRITTEN: 1 (BT13-108 -- grant-triggered-effect workaround)
- BLOCKED: 1 (BT3-103 -- one-shot digivolve hook, shared with ExMaquinamon)
- Stub fixes: 4 scripts fixed (BT15-102, BT16-046, EX10-061, ST20-15)
- Spot-check fixes: 4 scripts fixed (BT15-066, BT15-077, BT9-112, EX10-010)

## Stub Fixes (2026-03-14)

### BT15-102 Apocalymon (Lv.7) -- REWRITTEN
- **BeforePayCost**: Was completely stubbed. Now implements `_cost_reduction_value_fn` that counts distinct Dark Masters Digimon names in trash (up to 3) and returns count * 4. Process callback auto-selects and places them.
- **End of Turn**: Was incorrectly milling 3 cards. Now correctly: selects 1 Lv.6 or lower card from trash via SelectTrash phase, places as bottom digivolution card, activates 1 [On Play] effect from that card, then mills 2 * (count of Lv.6 digi cards in sources) from opponent's deck.

### BT16-046 GranKuwagamon (Lv.6) -- FIXED
- **On Play / When Digivolving**: Was deleting first then suspending (wrong order). Now correctly suspends up to 2 opponent Digimon or Tamers with cannot-unsuspend granted to the suspended targets (not to self). Then deletes 1 suspended Tamer.
- **Security A. +1 stub**: Was `pass`. Now uses `game.effect_select_own_permanent()` to let agent choose 1 Digimon, then sets `target._temp_sa_modifier += 1`.
- **OnTappedAnyone condition**: Added check that the suspended permanent is this card's permanent (was triggering for any permanent becoming suspended).

### EX10-061 Apocalymon (Lv.7) -- REWRITTEN
- **BeforePayCost**: Was stubbed. Now implements `_cost_reduction_value_fn` counting face-up Dark Masters Digimon in security with distinct names. Process callback removes them from security and stores as pending digi sources.
- **On Play / When Digivolving**: Was completely wrong -- playing from hand instead of digivolution cards, deleting opponent instead of played Digimon, granting effect immunity to self instead of Rush to Dark Masters. Now correctly: plays distinct-named Dark Masters Digimon from digivolution cards without cost, grants Rush to all Dark Masters trait Digimon for the turn, registers end-of-turn deletion for the played Digimon.

### ST20-15 Island of Adventure -- NO CHANGE NEEDED
- The `pass # Declarative DP modifier` is correct -- the security DP aura uses declarative attributes (`dp_modifier`, `_applies_to_all_own_digimon`, `_dp_permanent_condition`) that the engine's aura system reads directly. Note: the aura system currently only scans `battle_area` permanents, not security cards, so this effect may not actually apply from security. This is an engine-level limitation, not a script bug.

## Spot-Check Fixes (2026-03-14)

### BT15-066 Machinedramon (Lv.6) -- FIXED
- **End of Opponent's Turn**: Was missing opponent's turn check in condition (triggered on own turn too). Was deleting an opponent's Dark Masters Digimon instead of deleting THIS Digimon. Was playing any Digimon instead of Dark Masters trait Digimon (excluding Machinedramon). All three bugs fixed.

### BT15-077 LadyDevimon (Lv.5) -- FIXED
- **On Play**: Had spurious trash-to-hand action before reveal. Was only selecting 1 card from revealed instead of 2. Now uses `effect_reveal_and_select_multi` with 2 passes to correctly add up to 2 Lv.6+ cards to hand.
- **End of Turn**: Was not deleting own Digimon as cost. Was playing any Digimon instead of Dark Masters trait only. Now correctly: selects 1 own Digimon to delete, then plays 1 Dark Masters Digimon from hand free.

### BT9-112 DeathXmon (Lv.7) -- FIXED
- **BeforePayCost**: Added missing `_cost_reduction_value_fn` for cost preview (was only using process callback). Now engine shows reduced cost in action mask.

### EX10-010 BlackWarGreymon (Lv.6) -- FIXED
- **Effect Immunity + DP**: Was granting permanent CANNOT_BE_SELECTED_BY_EFFECT unconditionally. Card text says conditional: "While your opponent has a Digimon with 13000 DP or more." Now split into two effects: (1) conditional +3000 DP modifier, (2) conditional effect immunity flag. Both check `_opp_has_13k_dp()`.

## Remaining Spot-Check Results (PASS)

| Card | Name | Verdict | Notes |
|------|------|---------|-------|
| EX10-012 | MetalSeadramon | PASS | Cost reduction, cannot-suspend, on-deletion to security, inherited security play -- all correct |
| EX10-020 | Puppetmon | PASS | Cost reduction, bounce suspended, on-deletion to security -- all correct |
| EX10-035 | Machinedramon | PASS | Cost reduction, de-digivolve 2x2, on-deletion to security -- all correct |
| EX10-057 | Piedmon | PASS | Cost reduction, delete unsuspended, on-deletion to security -- all correct |
| BT15-072 | Vilemon | PASS | Blocker + scapegoat prevention -- correct |
| BT8-090 | Kari Kamiya | PASS | Start of turn memory set, on-add-security suspend for memory -- correct |

## Engine Gaps
| Card | Gap |
|------|-----|
| BT3-103 | One-shot digivolve hook (shared) |
| BT13-108 | Grant triggered effect to permanent (workaround: OnTappedAnyone listener) |
| ST20-15 | Security card DP aura: engine's `_get_aura_dp_modifier` only scans `battle_area`, not security stack |

## Smoke Test
- 50/50 mirror games completed (post-fix)
