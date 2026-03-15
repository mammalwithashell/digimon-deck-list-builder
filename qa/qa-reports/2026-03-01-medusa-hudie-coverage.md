# Gameplay QA Report — Medusa + CS Hudiemon Full Coverage

## Test Setup
- **Date**: 2026-03-01
- **Archetypes**: Medusa, CS Hudiemon
- **Goal**: 100% card coverage — every unique card across both archetypes gets a `validated_cards.json` entry
- **Method**: Script creation (7 new), static analysis (17 cards), gameplay testing (14 cards)
- **Prior Coverage**: 35/67 cards validated (31 PASS, 4 PARTIAL)

## Summary
- **New Cards Validated**: 32
- **Total Coverage**: 67/67 (100%)
- **Issues Found**: 5 bugs fixed in-session, ~30 transpiler-quality notes documented as PARTIAL
- Critical: 0 | High: 0 | Medium: 4 (fixed) | Low: 1 (fixed)

## Phase 1: New Script Creation (7 cards)

Created scripts for 7 cards that had no implementations:

| Card | Name | Type | Status | Notes |
|------|------|------|--------|-------|
| BT4-104 | Blinding Ray | Option | PASS | Trash security + gain 2 memory — fully implemented |
| ST9-05 | Paildramon | Digimon Lv.5 | PASS | DNA WhenDigivolving + once/turn unsuspend — fully implemented |
| BT1-090 | Gravity Crush | Option | PARTIAL | Gain 2 memory works; end-of-turn -2 can't fire (Options trash after resolve) |
| BT3-103 | Hidden Potential Discovered! | Option | PARTIAL | Conditional cost reduction stubbed (cost_reduction=5); security add-to-hand works |
| BT5-008 | Gaossmon | Digimon Lv.3 | PARTIAL | DP modifier for other Gaossmon works; opponent cost block not modelable |
| EX1-068 | Ice Wall! | Option | PARTIAL | Security gain 2 memory works; main effect (grant WhenAttacking memory loss) descriptive-tagged |
| EX1-071 | Win Rate: 60%! | Option | PARTIAL | Conditional cost reduction stubbed (cost_reduction=4); security add-to-hand works |

## Phase 2: Static Analysis (17 cards)

### Systematic Bug Fixed: Delay Draw 2
All 4 CS-themed Options (BT23-091, 092, 095, 096) had their Delay effect incorrectly templated from the main effect instead of the correct "draw 2". Fixed all 4 in-session.

### BT23-095 Crescent Leaf — Main/Security Fix
- **Bug**: Used `bounce_permanent_to_hand` instead of `return_permanent_to_deck_bottom`; no `is_suspended` filter
- **Fix**: Corrected method + added `p.is_digimon and p.is_suspended` filter on both main and security effects

### Static Analysis Results

| Card | Name | Status | Notes |
|------|------|--------|-------|
| BT23-002 | Yokomon | PARTIAL | CS trait check missing from inherited WhenAttacking condition |
| BT23-017 | Betamon | PARTIAL | On Play filter on wrong zone; inherited play uses cost instead of level filter |
| BT23-037 | Tentomon | PARTIAL | Cost reduction not scoped to CS trait; same inherited bugs as BT23-017 |
| BT23-040 | Wormmon | PARTIAL | Erika placement not implemented; missing Hudiemon name filter; DP not scoped to Hudie |
| BT23-041 | Kabuterimon | PARTIAL | Alliance works; DP buff targets self instead of suspended ally; no "other" exclusion |
| BT23-051 | Golemon | PARTIAL | Alliance+Blocker work; missing "can't attack Digimon" restriction |
| BT23-091 | Wolkenapalm | PARTIAL | Delay fixed to draw 2; main delete has no lowest-DP auto-targeting |
| BT23-092 | Ice Archery | PARTIAL | Delay fixed to draw 2; Tamer target missing from suspend-lock effect |
| BT23-095 | Crescent Leaf | PARTIAL | Main+security+Delay all fixed; was FAIL, now improved |
| BT23-096 | Comet Hammer | PARTIAL | Delay fixed to draw 2; De-Digivolve 4 main effect works |
| BT23-100 | Hudie Net Cafe | PARTIAL | Delay effect completely wrong (wrong timing+body); security filter missing CS |
| BT24-001 | Gigimon | PARTIAL | OnLoseSecurity delete mostly correct; context ambiguity on whose security |
| P-225 | DigiLab | PARTIAL | Wrong Delay sub-effect; missing security effect; placement incomplete |
| BT22-100 | Cyberspace EDEN | PARTIAL | Main security-swap effect missing; no CS trait filter on DP modifier |

## Phase 3: Gameplay Testing (14 cards)

### Hudie Tamers

| Card | Name | Status | Notes |
|------|------|--------|-------|
| BT23-084 | Erika Mishima | PARTIAL | Missing Start of Main +1 memory; End of Turn wrong target; self-suspend issues |
| BT23-085 | Ryuji Mishima | PARTIAL | Missing CS condition check; self-keyword bleed; wrong suspend target; missing Option play |
| BT22-094 | Yuugo Kamishiro | PARTIAL | Spurious trash pop; wrong action labels; missing self-removal in cost reduction |

### Medusa Core

| Card | Name | Status | Notes |
|------|------|--------|-------|
| EX11-008 | Elizamon | PARTIAL | DP boost on self instead of target; missing Reptile/Dragonkin filter |
| EX11-054 | Owen Dreadnought | PARTIAL | set_memory_3 works (PASS); On Play wrong suspend target |
| BT21-072 | Arresterdramon: SM | PARTIAL | Piercing missing; attack-without-suspend is no-op; DP flat not dynamic |
| EX11-012 | Medusamon | PARTIAL | Static analysis only — Rush/Progress keywords present; token play is engine stub |

### Medusa Tech + Options

| Card | Name | Status | Notes |
|------|------|--------|-------|
| BT21-093 | Raging Serpentine | PARTIAL | Cost reduction condition always true (no security check); wrong Delay sub-effect |
| BT8-097 | Crimson Blaze | PARTIAL | Variable cost reduction not implemented; deletes 1 not all; play restriction missing |
| EX10-010 | BlackWarGreymon | PARTIAL | Keywords (Blocker/Reboot/Raid) work; delete filter missing cost/Tamer targets |

### Boss Cards + Remaining

| Card | Name | Status | Notes |
|------|------|--------|-------|
| BT10-042 | Venusmon | PARTIAL | WhenDigivolving has no process callback; opponent restriction not implemented |
| BT20-102 | Omnimon (X Antibody) | PARTIAL | Piercing missing; board wipe wrong mechanic; bounce to hand not deck |
| BT16-077 | Dinobeemon | PARTIAL | DNA requirements missing; plays from hand not trash; Raid works |
| BT8-084 | Kimeramon | PARTIAL | WhenDigivolving no callback; DP hardcoded; opponent DP reduction missing |
| BT23-059 | Justimon: Blitz Arm | PARTIAL | Delete doesn't require Option trash; no lowest-cost filter; Blocker works |

## Phase 4: PARTIAL Re-examination

4 previously PARTIAL cards re-examined — no changes warranted:
- **BT21-029 Medusamon**: Token stubs remain → PARTIAL
- **BT24-017 Medusamon**: Token stubs remain → PARTIAL
- **BT24-018 Styracomon**: Armor Purge unimplemented → PARTIAL
- **BT22-099 Kuremi Detective Agency**: Cosmetic WONTFIX → PARTIAL

## Coverage Summary

| Category | Count | Status |
|----------|-------|--------|
| Previously PASS | 31 | PASS |
| Previously PARTIAL | 4 | PARTIAL (unchanged) |
| New scripts (simple) | 2 | PASS |
| New scripts (complex) | 5 | PARTIAL |
| Static analysis | 14 | PARTIAL |
| Gameplay tested | 11 | PARTIAL |
| **Total** | **67** | **100% coverage** |

**Final PASS rate**: 33/67 (49%)
**Final PARTIAL rate**: 34/67 (51%)

## Common Transpiler Issues Identified

These patterns recur across many scripts and represent systematic transpiler bugs:

1. **Self-keyword bleed**: Effect-level keyword flags (e.g., `_is_blocker`) set on ICardEffect leak to card display
2. **Wrong suspend targets**: `effect_select_opponent_permanent` used when self-suspend is intended
3. **Missing trait filters**: CS/Hudie/Reptile trait conditions not checked in conditions
4. **No-op process callbacks**: `pass` body where actual game actions needed
5. **Delay effects repeating main effect**: Transpiler templated Delay body from main effect instead of card text
6. **All Turns effects gated to Your Turn**: `is_my_turn` checks added to effects that should fire on both turns
