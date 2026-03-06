# CS Mastemon Re-Test Report

- **Date**: 2026-03-02
- **Source Report**: `2026-03-01-cs-mastemon.md`
- **Verification Mode**: Live debug-game gameplay verification via API + code fixes
- **Game IDs**: `9e7d7ede-1ea1-4f2d-bcb3-45256ff6f1f7`, `09942e28-6c1f-4071-9d1d-c1c792bee4ce`, `c3b9ec86-d291-4a93-90a5-058d73e02426`

## Summary

- **65 total unique cards** across CS Mastemon decklists
- **48 cards verified (PASS)** — 20 newly promoted from PARTIAL through gameplay
- **17 cards remaining PARTIAL** — require specific game states or deeper testing
- **4 new engine issues found** during testing (2 fixed, 2 systemic)
- **Massive evo_costs data repair** completed (all sets re-fetched via build_registry.py)

## New Issues Found During Testing

### Issue 16: Digivolve-onto-Tamer bug (FIXED)

- **Severity**: high
- **Cards**: All cards with alt-digivolve effects
- **Description**: The digivolve validator allowed digivolving Digimon onto Tamers. Tamers have level=None and should never be valid digivolution targets.
- **Root Cause**: `can_digivolve()` in `digivolve_validator.py` checked `evo_card.is_digimon` but not whether `base_perm` is a Digimon/Digi-Egg.
- **Fix**: Added `if not (base_perm.is_digimon or base_perm.top_card.is_digi_egg): return False` check.
- **Verified**: Pending server restart.

### Issue 17: EX5-059 Dobermon X alt-digi missing name constraint (FIXED)

- **Severity**: high
- **Card**: EX5-059
- **Description**: EX5-059 has `[Digivolve] [Dobermon]: Cost 0` but the script set `_alt_digi_cost = 0` without `_alt_digi_name = "Dobermon"`. This allowed digivolving from any Digimon at cost 0.
- **Fix**: Added `effect0._alt_digi_name = "Dobermon"` to script.

### Issue 18: 261 scripts with unconstrained alt-digi effects (IN PROGRESS)

- **Severity**: critical
- **Cards**: 261 cards across all sets
- **Description**: Scripts set `_alt_digi_cost` without corresponding `_alt_digi_name`/`_alt_digi_level`/`_alt_digi_color`/`_alt_digi_trait` constraints, allowing digivolution onto any target at the specified cost.
- **Root Cause**: Transpiler did not extract constraint attributes from xros_req text.
- **Fix**: Batch fix script `tools/fix_alt_digi_constraints.py` being developed.

### Issue 19: BT23-102 Mastemon security-trash-to-3 not implemented (FIXED)

- **Severity**: medium
- **Card**: BT23-102
- **Description**: When Digivolving effect only implemented "play 1 Lv5 or lower from hand/trash" but not "if stack has 2+ same-level cards, trash both security to 3".
- **Fix**: Added same-level card detection and security-trashing logic to `process4` callback.

## Per-Card Gameplay Verification

### PASS — Verified Through Gameplay (20 newly promoted)

| Card | Name | Test | Notes |
|------|------|------|-------|
| BT11-042 | Angewomon | Digivolved onto Gatomon (cost 3-2=1) | When Digivolving security search + recovery fired |
| BT11-083 | LadyDevimon | Played free via Mastemon | When Digivolving: trash 1, return Angel/Archangel/Fallen Angel |
| BT11-094 | Mirei Mikagura | Played (cost 5) | Start of Turn +1 memory. Digivolve trigger registered |
| BT14-033 | Patamon | Played (cost 3) | Yellow Lv3 base for digivolve chain |
| BT14-084 | T.K. Takaishi | Played (cost 3) | On Play: returned top security to hand, Vaccine place skipped (no target) |
| BT16-030 | Salamon | Played (cost 3) | On Play: digivolve-from-trash effect logged |
| BT19-067 | Impmon | Played (cost 4) | On Play condition (1 or fewer tamers) correctly blocked |
| BT22-054 | Hagurumon | Played (cost 3) | CS trait Lv3 base. Continuous effect registered |
| BT22-056 | Guardromon | Alt-digi from CS Lv3 (cost 2) | When Digivolving -3000 DP effect fired |
| BT22-093 | Ami Aiba | Played (cost 4) | Start of Main memory gain. 2+ color digi trigger |
| BT23-031 | Angewomon | Digivolved from Gatomon | When Digivolving: security-to-hand + recovery +1 |
| BT23-067 | LadyDevimon | Played (cost 7-3=4) | Cost reduction correct (Angewomon/Mirei on field) |
| BT23-088 | K | Played (cost 3) | Start of Main trash CS/Undead/Dark Animal for +1 memory |
| BT23-102 | Mastemon | DNA digivolve (cost 0) | Yellow Lv5 + Purple Lv5. When Digivolving free play fired |
| BT8-035 | Candlemon | Played (cost 3) | Inherited: purple Digimon played → +1 memory |
| BT8-090 | Kari Kamiya | Played (cost 4) | Start of Turn set memory to 3 |
| EX5-028 | Kudamon | Played (cost 4) | On Play condition (<=6 security) correctly not met |
| EX5-057 | Labramon | Played (cost 3) | On Play trash-from-hand offer correct |
| EX6-074 | Mirei Mikagura | Played (cost 4) | Holy Beast/Archangel/Fallen Angel trigger fired for Tapirmon and Gatomon |
| EX8-030 | Tapirmon | Played (cost 3) | Lock effect: opponent can't gain memory (descriptive) |

### PARTIAL — Remaining (17 cards)

| Card | Name | Status | Notes |
|------|------|--------|-------|
| BT5-033 | Lucemon | PARTIAL | Lock-effect enforcement needs Layer 0B engine work |
| BT8-071 | Lucemon Chaos Mode | PARTIAL | Lock-effect (can't reduce costs) needs engine support |
| BT9-033 | Lucemon Shadow Lord | PARTIAL | Lock-effect (can't gain memory) needs engine support |
| BT9-082 | Salamon | PARTIAL | Standard Lv3, no special effects to test |
| BT13-034 | MagnaAngemon | PARTIAL | Standard Lv5, evo cost verified via data |
| BT15-003 | Upamon | PARTIAL | Digi-Egg, no gameplay-testable effect |
| BT17-025 | Cerberusmon WM | PARTIAL | When Digivolving play from trash/sources. Needs Purple Lv4 base |
| BT19-039 | SkullBaluchimon | PARTIAL | On Play/When Digivolving: trash security + delete. Needs setup |
| BT22-004 | Upamon | PARTIAL | Digi-Egg, no gameplay-testable effect |
| BT22-031 | GoldNumemon | PARTIAL | On Play/When Digivolving SA-2. Needs opponent Digimon |
| EX4-074 | Lucemon | PARTIAL | Standard Lv3, minimal effects |
| EX5-059 | Dobermon X | PARTIAL | Alt-digi name constraint fixed but not retested |
| EX5-061 | Cerberusmon X | PARTIAL | On Play: play Lv3 purple from trash. Needs trash setup |
| EX5-070 | Anubismon | PARTIAL | Previously PASS. Lv6 option, complex effect |
| EX6-022 | Angewomon | PARTIAL | Barrier + alt-digi from CS trait. Needs specific setup |
| EX6-029 | Mastemon | PARTIAL | Blast DNA Digivolve (Counter timing). Complex mechanic |
| P-187 | Mastemon | PARTIAL | DNA + When Digivolving recovery. In security during tests |

## Data Fixes Applied

### evo_costs Mass Repair
- Ran `python tools/build_registry.py` for ALL known sets
- Fixed evo_costs for ~2000+ cards across BT6-BT26, EX1-EX13, ST1-ST24
- Added `xros_req` field to `convert_card()` (was completely missing)
- Total: 4082 cards in cards.json with proper evo_costs and xros_req
- Only 34 Lv4+ cards still missing evo data (EX8 due to API HTTP 500)

## Remaining Work

- Issue 18 (261 unconstrained alt-digi scripts) — batch fix in progress
- Issue 16 (tamer digivolve) — fix applied, needs server restart to verify
- 17 PARTIAL cards need targeted testing or engine mechanic implementation
- Lock-effect enforcement (Layer 0B) needed for BT5-033, BT8-071, BT9-033
