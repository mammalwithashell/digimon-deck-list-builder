# Gameplay QA Report — Galacticmon Retest

## Test Setup
- **Date**: 2026-03-22
- **Archetype**: Galacticmon
- **Decklists**: [0] (placement 8, BT11-105 variant), [3] (placement 4, EX11/Maquinamon variant), [4] (placement 1, pure Galacticmon)
- **Method**: DebugRunner in-process testing + script audit via sub-agent
- **Focus**: Re-verifying 27 outstanding issues from 2026-03-13 report + fixing engine bugs

## Summary
- **Prior Outstanding Issues**: 27
- **Fixed (this session)**: 3 engine/script bugs fixed, 22 issues verified fixed from prior work
- **Remaining Outstanding**: 5 (need gameplay verification)
- **New Issues Found**: 1

## Engine Bugs Fixed

### Bug 1: `_decode_trash_selection` callback mismatch (CRITICAL)
- **File**: `digimon_gym/engine/game/action_decoder.py:282`
- **Impact**: 25 card scripts with trash selection were completely broken
- **Root cause**: `_decode_trash_selection` passed `callback(idx)` (raw 0-based index) while all other selection decoders pass `callback(action_id)` (full action ID). Scripts compute `action_id - SEL_TRASH_START` which gave negative values, silently returning.
- **Fix**: Changed to `callback(action_id)` for consistency with `_decode_selection`
- **Affected scripts**: bt11_105, bt21_058, bt10_112, ex9_066, ex9_074, bt23_040, bt22_101, bt20_056, bt19_101, ex11_073, bt21_013, ex11_069, ex11_005, bt21_100, ex6_072, ex9_062, bt7_082, bt17_007, bt15_102, st10_15, ex7_053, bt7_107, lm_028, bt22_008, ex9_024

### Bug 2: BT11-105 Fusionize evo cost not deducted
- **File**: `digimon_gym/engine/data/scripts/bt11/bt11_105.py:139`
- **Root cause**: `getattr(digi_card, 'evo_costs', [])` on CardSource returns `[]` — evo_costs lives on `c_entity_base`, not CardSource
- **Fix**: Access via `digi_card.c_entity_base.evo_costs`

### Bug 3: BT21-058 Snatchmon wrong SEL_TRASH_START
- **File**: `digimon_gym/engine/data/scripts/bt21/bt21_058.py:113`
- **Root cause**: Hardcoded `SEL_TRASH_START = 1500` instead of importing from `game.constants` (actual value is 130)
- **Fix**: Import from `game.constants`

## Prior Issue Resolution

| # | Issue | Card | Severity | Status | Notes |
|---|-------|------|----------|--------|-------|
| 10 | Main effect untriggerable | BT11-061 | Critical | **FIXED** | Action 1002 now available on field |
| 11 | Memory not deducted for digi | BT11-105 | Critical | **FIXED** | evo_costs access fixed (this session) |
| 12 | Auto-selects trash/target | BT11-105 | Medium | **FIXED** | Uses request_selection with proper options |
| 13 | "you may" not enforced | BT11-105 | Medium | **FIXED** | is_optional=True on selections |
| 14 | Trash vs deck bottom | BT21-058 | High | **FIXED** | Correctly trashes revealed cards |
| 15 | Only places 1 Vemmon | BT21-058 | High | **FIXED** | Up-to-2 selection works (verified) |
| 16 | Only targets this Digimon | BT21-058 | Medium | **FIXED** | Targets any Digimon on field |
| 17 | Auto-places without choice | BT21-058 | Low | **FIXED** | Decline option available |
| 18 | Missing link condition | EX11-006 | High | **FIXED** | Script checks Maquinamon link |
| 19 | Missing digi cost reduction | EX11-006 | High | **FIXED** | Cost reduced by 2 in script |
| 20 | Only adds 1 card | EX11-027 | High | OUTSTANDING | Script has proper logic, needs gameplay verify |
| 21 | Missing link step | EX11-027 | High | **FIXED** | Link functionality implemented |
| 22 | Missing On Play flag | EX11-029 | Medium | **FIXED** | is_on_play = True confirmed |
| 23 | Missing tamer condition | EX11-029 | Medium | **FIXED** | Checks 1 or fewer Tamers |
| 24 | Missing Piercing inherited | EX11-029 | High | **FIXED** | Inherited Piercing present |
| 25 | On Play flag missing | EX11-033 | Medium | **FIXED** | is_on_play = True confirmed |
| 26 | Missing tamer condition | EX11-040 | Medium | **FIXED** | Checks 1 or fewer Tamers |
| — | Redirect attack broken | EX11-042 | High | **FIXED** | Redirect attack implemented |
| — | On Play flag missing | EX11-045 | Medium | **FIXED** | is_on_play = True confirmed |
| — | Trigger mechanism unreliable | EX11-062 | High | OUTSTANDING | Suspend observer in script, needs gameplay verify |
| 27 | Trashes own security | EX11-073 | Critical | **FIXED** | Correctly targets opponent.security_cards |
| — | Missing SA+1 | EX11-073 | High | **FIXED** | security_attack_modifier = 1 verified |
| — | Missing Blocker | EX11-073 | High | **FIXED** | _is_blocker = True confirmed |
| — | Missing DNA condition | EX11-073 | Medium | **FIXED** | Jogress condition marker present |
| — | DigiXros condition | BT18-065 | Medium | OUTSTANDING | Needs gameplay verification |
| — | Stub implementation | P-151 | Medium | OUTSTANDING | Trait filtering needs verification |
| — | Systemic: "by trashing" cost | Multiple | Medium | OUTSTANDING | Optional cost pattern not verified |

## New Issues

### Issue N1: DebugRunner trash selection display (Low)
- **Severity**: Low (cosmetic)
- **Description**: During SelectTrash phase, DebugRunner labels trash selection actions (130+) as "Attack target[N]" instead of proper trash card names. Engine behavior is correct — display only.

## Cards Tested Successfully
- BT11-061: [Main] effect triggers correctly from field (action 1002)
- BT11-105: Fusionize — trash placement + digivolution + cost deduction all working
- BT21-058: Snatchmon — reveal+trash+place up to 2 Vemmon all working
- EX11-073: SA+1 modifier = 1 verified in gameplay
- EX11-006: Inherited When Attacking effect correctly gated by link condition
- All 12 previously PASS cards: confirmed still working (no regression)

## Areas Not Covered
- Link mechanics gameplay (EX11-027, EX11-029, EX11-033, EX11-040, EX11-042 link steps)
- EX11-062 Shoto Kazama suspend observer trigger
- BT18-065 DigiXros from-trash condition
- P-151 LIBERATOR trait filtering
- ST13-08 "can't reduce play costs" aura effect
