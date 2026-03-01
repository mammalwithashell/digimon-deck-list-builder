# Gameplay QA Report — PARTIAL Script Fixes (Medusa + CS Hudiemon)

## Test Setup
- **Date**: 2026-03-01
- **Archetypes**: Medusa, CS Hudiemon (cross-archetype coverage)
- **Game IDs**: b85b2429-c59d-4379-9f23-13661e7a56ea
- **Total Cards Fixed**: 28 (PARTIAL → PASS)
- **Focus Areas**: Trait filters, target selection, DP targeting, keyword grants, complex rewrites, WhenDigivolving callbacks

## Summary
- **Total Issues Fixed**: 28 scripts rewritten/patched
- **Starting State**: 33 PASS / 38 PARTIAL (47% pass rate)
- **Final State**: 61 PASS / 10 PARTIAL (86% pass rate)
- **Engine Changes**: 1 (game.py action mask for `_is_cannot_attack_digimon`)

## Fix Categories

### Step 1: Trivial Fixes (8 scripts)
Simple condition/filter additions and flag cleanup.

| Card | Fix |
|------|-----|
| BT23-002 Yokomon | Added CS trait check to inherited WhenAttacking draw condition |
| BT24-001 Gigimon | Removed is_my_turn guard from OnLoseSecurity condition |
| BT23-095 Crescent Leaf | Removed duplicate is_security_effect flag |
| BT23-096 Comet Hammer | Removed duplicate is_security_effect flag |
| BT22-100 Cyberspace EDEN | Added CS trait filter to DP modifier; fixed security play filter |
| EX11-054 Owen Dreadnought | Fixed self-suspend (targets own permanent instead of opponent) |
| P-225 DigiLab | Fixed Delay condition; process3 targets own CS Lv.4+ Digimon; added security effect |
| BT23-041 Kabuterimon | Moved DP buff into on_grant callback to target selected ally |

### Step 2: Wrong Target Selection (4 scripts)

| Card | Fix |
|------|-----|
| BT23-084 Erika Mishima | Rewritten: suspend self-tamer, bounce own Hudie, play Lv.3 CS. Added +1 memory |
| BT23-085 Ryuji Mishima | Added Hudie trait filter to keyword grant. Fixed self-suspend. Added CS Option play |
| EX11-008 Elizamon | DP +3000 moved into on_grant callback. Added Reptile/Dragonkin trait filter on Raid |
| BT21-072 Arresterdramon:SM | Added Piercing. CAN_ATTACK_UNSUSPENDED modifier. Dynamic DP per opp count |

### Step 3: On Play / Inherited Logic Bugs (4 scripts)

| Card | Fix |
|------|-----|
| BT23-017 Betamon | On Play rewritten: trash ANY hand card, recover CS non-Digi-Egg from trash. **QA verified** |
| BT23-037 Tentomon | CS trait scope added to cost reduction. Hash collision fixed |
| BT23-040 Wormmon | Erika placement implemented. Hudiemon filter. cost_reduction=2. DP scoped to Hudie |
| BT22-094 Yuugo | Spurious trash pop removed. CS trait filter added. Self-removal cost implemented |

### Step 4: DP Targeting & Attack Restrictions (4 scripts + engine)

| Card | Fix |
|------|-----|
| BT23-091 Wolkenapalm | Lowest-DP filter on main+security delete. Delay rewritten to delete lowest DP. **QA verified** |
| BT21-093 Raging Serpentine | Security<=3 check. Highest-DP filter. Delay for own Reptile/Dragonkin digivolve |
| BT23-092 Ice Archery | Tamer selection as second step. Delay rewritten for cant-suspend with CS check |
| BT23-051 Golemon | cant_attack_digimon restriction added. **Engine change**: game.py action mask check |

### Step 5: Complex Rewrites (5 scripts)

| Card | Fix |
|------|-----|
| BT20-102 Omnimon X | Piercing added. Board wipe rewritten (keep self+1 opp). Bounce to deck bottom |
| BT8-097 Crimson Blaze | Dynamic cost reduction. Batch delete all <=6000 DP. Play restriction modifier |
| BT16-077 Dinobeemon | Play source changed from hand to trash. Rush grant via selection |
| BT23-059 Justimon: Blitz Arm | Option trash as cost. Lowest play cost filter. Unique hash strings. **QA verified** |
| BT23-100 Hudie Net Café | Delay filter fixed (CS Tamer). Security filter (CS+Lv.3). **QA verified** |

### Step 6: WhenDigivolving Callback Rewrites (3 scripts)

| Card | Fix |
|------|-----|
| BT8-084 Kimeramon | Process callback: Lv.5- from trash to bottom evo card. Dynamic DP per color count |
| BT10-042 Venusmon | Process callback: Security Attack -1 on all opponent Digimon |
| EX10-010 BlackWarGreymon | play_cost<=7 filter. Tamer targets. Duplicate effects merged. **QA verified** |

## Cards Remaining PARTIAL (10 — Engine Gaps / Game Rules)

| Card | Reason |
|------|--------|
| BT21-029 Medusamon | Token creation not in engine |
| BT24-017 Medusamon | Token creation not in engine |
| EX11-012 Medusamon | Token creation not in engine |
| BT24-018 Styracomon | Armor Purge not in engine |
| BT3-103 Hidden Potential | "Next digivolve this turn" cost tracking not modelable |
| EX1-071 Win Rate: 60%! | Same as BT3-103 |
| EX1-068 Ice Wall! | Granting WhenAttacking to all opp Digimon not modelable |
| BT1-090 Gravity Crush | Options trash after resolve; end-of-turn -2 can't fire |
| BT5-008 Gaossmon | Opponent cost blocking not modelable |
| BT22-099 Kuremi Detective Agency | Cosmetic action description issue |

## Engine Changes

### game.py — `_is_cannot_attack_digimon` action mask check
Added check at ~line 1681 in the Digimon-vs-Digimon attack loop:
```python
if attacker.has_keyword('_is_cannot_attack_digimon'):
    continue
```
This prevents Digimon with the keyword from targeting other Digimon in attacks (mirrors existing `_is_cannot_attack_player`).
