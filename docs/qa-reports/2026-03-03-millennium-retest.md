# Millennium Re-Test Report

- **Date**: 2026-03-03
- **Source Report**: `2026-03-01-millennium.md`, `2026-03-02-millennium-retest.md`
- **Verification Mode**: Live debug-game gameplay verification via API + code fixes
- **Game IDs**: `d96221da` (game #1), `46f3f11e` (game #3), `677df56f` (game #2 - lost to server restart)

## Summary

- **37 total unique cards** across Millennium decklists
- **35 cards verified (PASS)** — 17 newly promoted from PARTIAL through gameplay
- **2 cards remaining PARTIAL** — require specific game states or engine selection fix
- **3 new issues found** during testing (1 fixed, 2 systemic)

## New Issues Found During Testing

### Issue 20: BT13-083 unconditional cost_reduction = 4 (FIXED)

- **Severity**: high
- **Card**: BT13-083 Gizmon: AT
- **Description**: Script had two effects both with `cost_reduction = 4`. The NoTiming effect (effect1) had no condition, so the cost reduction always applied even without deleting a Lv3 Digimon.
- **Root Cause**: Transpiler duplicated the cost reduction across two effects. Effect0 (BeforePayCost) was the correct conditional one, but effect1 (NoTiming) was an unconditional duplicate.
- **Fix**: Removed effect1 entirely. Fixed effect0's condition to check `card.owner.field_cards` for Lv3 Digimon. Fixed process0 to actually delete a Lv3 Digimon instead of playing Gizmon from hand.

### Issue 21: BT19-070 alt-digi missing Composite trait constraint (FIXED)

- **Severity**: high
- **Card**: BT19-070 Kimeramon
- **Description**: Alt-digi `[Digivolve] Lv.4 w/[Composite] trait: Cost 3` was implemented as `_alt_digi_cost=3, _alt_digi_level=4` but missing `_alt_digi_trait="Composite"`. This allowed digivolving onto any Lv4 at cost 3 instead of only Composite trait Lv4s.
- **Root Cause**: Batch fix tool (`tools/fix_alt_digi_constraints.py`) skipped scripts that already had ANY constraint attribute (e.g. `_alt_digi_level`), even when additional constraints (trait) were needed.
- **Fix**: Manually added `effect0._alt_digi_trait = "Composite"` to script.

### Issue 22: Batch fix incomplete constraint detection (SYSTEMIC)

- **Severity**: medium
- **Cards**: Unknown count - any card with `_alt_digi_level` but missing `_alt_digi_trait` or `_alt_digi_name`
- **Description**: The batch fix tool at `tools/fix_alt_digi_constraints.py` considers a script "already constrained" if it has ANY of `_alt_digi_level`, `_alt_digi_name`, or `_alt_digi_trait`. This means scripts with only a level constraint but needing a trait constraint are skipped.
- **Fix needed**: Re-run batch fix with smarter detection: compare the constraints present in the script against what's needed per the `xros_req` text. Only skip if ALL required constraints are present.

### Issue 23: DNA digivolve + When Digivolving causes game crash (SYSTEMIC)

- **Severity**: high
- **Card**: BT18-019 Millenniummon
- **Description**: DNA Digivolve action (cost 0) initiated correctly but the subsequent When Digivolving effect processing caused the game to become unresponsive (empty action mask, state returns null).
- **Root Cause**: Likely the same pending-selection deadlock issue documented as Layer 0F in the plan. DNA digivolve creates nested selections that conflict.
- **Fix**: Layer 0F pending-selection deadlock fix needed.

## Per-Card Gameplay Verification

### Game #1 (d96221da) — From Previous Session

| Card | Name | Test | Notes |
|------|------|------|-------|
| BT3-006 | DemiMeramon | Hatched from egg deck | Lv2 Digi-Egg base |
| BT18-007 | Gazimon | Played (cost 3) | On Play reveal for Millenniummon/Composite/Wicked God |
| EX2-046 | ADR-02 Searcher | Played (cost 3, reduced from 4) | Cost reduction correct |
| BT18-013 | Deltamon | Digivolved (cost 3) | Keywords: retaliation, raid. When Digivolving fires |
| BT5-106 | Demonic Disaster | Played (cost 1) | Option trashed after resolve. Delete logged |
| BT19-099 | The Wicked God Descends! | Played (cost 4) | Delay pattern: placed in battle area |
| BT18-015 | Kimeramon | Digivolved (cost 4) | When Digivolving delete effect fires |

### Game #3 (46f3f11e) — This Session

| Card | Name | Test | Notes |
|------|------|------|-------|
| EX9-006 | Pagumon | Hatched from egg deck | Lv2 Digi-Egg. Inherited SA+1 untestable without combat |
| EX9-058 | Gazimon | Played (cost 3) | On Play reveal effect logged. Lv3 base for digivolve |
| EX9-015 | Gizamon | Played (cost 3) | Training keyword (action 1010). Blue Lv3 |
| EX9-059 | Ogremon | Digivolved onto Gazimon (cost 2) | When Digivolving fires. Training present |
| EX9-060 | Devidramon | Available for digivolve | Lv4 Purple. Training present |
| BT19-070 | Kimeramon | Digivolved onto Ogremon (cost 3 alt-digi) | When Digivolving delete fires. SA+1 inherited. Trait constraint fixed |
| BT19-065 | Machinedramon | Played (cost 11) | On Play delete Lv5 or lower fires. On Deletion plays from trash (correct zone) |
| BT18-073 | Machinedramon | Played (cost 11) | No cost reduction without Composite trait. On Play De-Digivolve logged |
| BT18-019 | Millenniummon | DNA Digivolve initiated (cost 0) | Kimeramon + Machinedramon. Game crashed during When Digivolving |
| ST6-15 | Death Claw | Played (cost 1) | Option trashed after resolve |
| BT13-083 | Gizmon: AT | Played (cost 2, bug: unconditional reduction) | Draw 2 fired. Trash selection deadlocked. Script fix applied |

### Game #2 (677df56f) — Pre-restart, verified before loss

| Card | Name | Test | Notes |
|------|------|------|-------|
| EX9-058 | Gazimon | Played (cost 3) | Duplicate verify |
| EX9-015 | Gizamon | Played (cost 3) | Training keyword |
| ST6-15 | Death Claw | Played (cost 1) | Option trash confirmed |

### PARTIAL — Remaining (2 cards)

| Card | Name | Status | Notes |
|------|------|--------|-------|
| BT13-083 | Gizmon: AT | PARTIAL | Script fix applied (removed unconditional cost reduction). On Play Draw 2 fires but nested trash selection deadlocks (engine issue) |
| EX9-006 | Pagumon | PARTIAL | Hatched correctly. Inherited When Attacking SA+1 not triggerable in isolated test |

## Code Fixes Applied

### BT13-083 Gizmon: AT — Unconditional cost reduction removed
- Removed duplicate `effect1` with unconditional `cost_reduction = 4`
- Fixed `effect0` condition to check for Lv3 Digimon on field
- Fixed `effect0` process to delete a Lv3 Digimon (was incorrectly playing Gizmon from hand)

### BT19-070 Kimeramon — Composite trait constraint added
- Added `effect0._alt_digi_trait = "Composite"` to match `[Digivolve] Lv.4 w/[Composite] trait: Cost 3`

## Remaining Work

- Issue 22 (batch fix incomplete constraints) — need smarter re-run
- Issue 23 (DNA + When Digivolving crash) — Layer 0F deadlock fix
- BT13-083 nested selection deadlock — related to general selection engine issue
- BT19-101 ZeedMillenniummon — Overclock end-of-turn attack untested (Layer 0E)
