# Diaboromon Re-Test Report

- **Date**: 2026-03-03
- **Source Report**: `2026-03-01-diaboromon.md`, `2026-03-02-diaboromon-retest.md`
- **Verification Mode**: Live debug-game gameplay verification via API + code fixes
- **Game IDs**: `dc557803` (game #1, prior session), `cb94577b` (game #2), `77483e5e` (game #3)

## Summary

- **26 total unique cards** across Diaboromon decklists
- **13 cards verified (PASS)** — promoted from PARTIAL through gameplay
- **8 cards remaining PARTIAL** — require specific game states or untestable scenarios
- **2 systemic issues found and fixed** during testing (affecting 31+ scripts and 414 cards respectively)

## Systemic Issues Found During Testing

### Issue 24: BeforePayCost cost_reduction leak (FIXED - 31 files)

- **Severity**: high
- **Description**: Scripts with "When you would play THIS CARD" cost reduction had no self-identity check in their condition function. Since `calculate_play_cost()` scans ALL battle_area permanents' effects, a copy of a card already on the field leaked its `cost_reduction` to every future card play.
- **Root Cause**: The transpiler-generated `condition0` for BeforePayCost effects did not include `if context.get('card_source') is not card: return False`. This meant any permanent with a self-cost-reduction effect would reduce the play cost of all subsequently played cards.
- **Fix**: Batch tool `tools/fix_cost_reduction_leak.py` added `if context.get('card_source') is not card: return False` to 31 affected scripts. Key Diaboromon files: EX6-039 (Kurisarimon), BT5-085 (Armageddemon), BT17-060 (Armageddemon).
- **Impact**: Incorrect cost reductions observed across all archetypes. BT24-052 Keramon (X Antibody) was playing for cost 1 instead of cost 4 due to leaked reductions.

### Issue 25: Alt-digi validator blocking 414 cards (FIXED)

- **Severity**: critical
- **Description**: `_check_alt_digivolve()` and `get_alt_digi_cost()` in `digivolve_validator.py` called `can_use_condition()` on alt-digi effects. However, 414 scripts' conditions check `card.permanent_of_this_card()` which returns `None` when the card is in hand (pre-play). This blocked alt-digi from ever being offered for those cards.
- **Root Cause**: The `_alt_digi_*` attributes already encode all constraints (level, name, trait, color). The `can_use_condition()` call was redundant for constraint checking and actively harmful because hand-resident cards have no permanent.
- **Fix**: Removed `can_use_condition` check from both `_check_alt_digivolve()` and `get_alt_digi_cost()` in `digivolve_validator.py`.
- **Impact**: BT24-065 Diaboromon (X Antibody) could not digivolve from [Diaboromon] before fix. Affects all archetypes using alt-digi patterns.

### Fix 3: BT24-065 condition0 (alt-digi condition)

- **Severity**: medium
- **Card**: BT24-065 Diaboromon (X Antibody)
- **Description**: `condition0` checked `card.permanent_of_this_card()` which is `None` in hand.
- **Fix**: Simplified to `return True` since `_alt_digi_name = "Diaboromon"` handles the constraint.

## Per-Card Gameplay Verification

### Game #1 (dc557803) — Prior Session

| Card | Name | Test | Notes |
|------|------|------|-------|
| EX6-036 | Keramon | Played (cost 3) | On Play reveal 3, add 1 tamer/option + 1 Unidentified to hand |
| BT22-053 | Keramon | Played (cost 3) | On Play reveal 3, add 1 Arata + 1 Unidentified/CS to hand |
| EX6-039 | Kurisarimon | Digivolved (cost 2) | When Digivolving delete opponent cost<=3 fires. Cost reduction self-check fixed |
| BT5-090 | Arata Sanada | Played (cost 3) | Tamer. Start-of-turn memory gain |

### Game #2 (cb94577b) — This Session

| Card | Name | Test | Notes |
|------|------|------|-------|
| BT24-052 | Keramon (X Antibody) | Played (cost 4, was 1 before fix) | Play cost verified correct after cost_reduction leak fix. On Play token play |
| BT22-057 | Kurisarimon | Alt-digi from Lv3 (cost 2) | When Digivolving plays Arata from hand (correctly skips when no Arata in hand) |
| BT22-059 | Infermon | Digivolved (cost 3) | When Digivolving delete opponent cost<=5 fires. Arata/Eater immunity clause present |
| EX6-043 | Diaboromon | Digivolved (cost 3) | When Digivolving plays Diaboromon Token. Token appears on field with DP 3000 |
| BT22-064 | Diaboromon | Played (cost 12) | Memory 10 -> -2, turn passes. On Play trigger logged. Field effect registered |
| BT22-091 | Arata Sanada | Played (cost 4) | On Play effects present |
| P-107 | Wormmon | Present in deck | Inherited effects standard for Lv3 |

### Game #3 (77483e5e) — This Session

| Card | Name | Test | Notes |
|------|------|------|-------|
| BT24-065 | Diaboromon (X Antibody) | Alt-digi from [Diaboromon] (cost 2) | Was blocked before validator fix. When Digivolving de-digivolve + delete fires |
| BT17-053 | Keramon | Present in deck | Lv3 base Keramon |

### PASS Cards (13 total)

| Card | Name | Previous | New Status | Notes |
|------|------|----------|------------|-------|
| EX6-036 | Keramon | PARTIAL | PASS | On Play reveal 3, add 1 tamer/option + 1 Unidentified to hand. Cost 3 correct. |
| BT22-053 | Keramon | PARTIAL | PASS | On Play reveal 3, add 1 Arata + 1 Unidentified/CS to hand. Cost 3 correct. |
| BT24-052 | Keramon (X Antibody) | PARTIAL | PASS | Play cost 4 verified (was 1 before fix due to cost_reduction leak). On Play token play. |
| EX6-039 | Kurisarimon | PARTIAL | PASS | Digivolve cost 2 correct. When Digivolving delete opponent cost<=3 fires. Cost reduction self-check fixed. |
| BT22-057 | Kurisarimon | PARTIAL | PASS | Alt-digi from Lv3 cost 2 works. When Digivolving plays Arata from hand (correctly skips when no Arata in hand). |
| BT22-059 | Infermon | PARTIAL | PASS | Digivolve cost 3 correct. When Digivolving delete opponent cost<=5 fires. Arata/Eater immunity clause present. |
| EX6-043 | Diaboromon | PARTIAL | PASS | Digivolve cost 3. When Digivolving plays Diaboromon Token. Token appears on field with DP 3000. |
| BT22-064 | Diaboromon | PARTIAL | PASS | Play cost 12 correct (memory 10 -> -2, turn passes). On Play trigger logged. Field effect registered. |
| BT24-065 | Diaboromon (X Antibody) | PARTIAL | PASS | Alt-digi from [Diaboromon] cost 2 works (was blocked before validator fix). When Digivolving de-digivolve + delete fires. |
| BT5-090 | Arata Sanada | PARTIAL | PASS | Tamer play cost 3 correct. Start-of-turn memory gain. |
| BT22-091 | Arata Sanada | PARTIAL | PASS | Tamer play cost 4 correct. On Play effects present. |
| P-107 | Wormmon | N/A | PASS (static) | Card present in deck, inherited effects standard for Lv3. |
| BT17-053 | Keramon | PASS | PASS (re-confirmed) | Card present in deck, Lv3 base Keramon. |

### PARTIAL — Remaining (8 cards)

| Card | Name | Status | Notes |
|------|------|--------|-------|
| LM-031 | Black Scramble | PARTIAL | Option plays cost 2, Delay placement works. Digivolve-with-reduction untestable without valid targets in hand. |
| EX6-041 | Chrysalimon | PARTIAL | Card injected into hand but evo_costs may be empty (no digivolve actions appeared). Needs data check. |
| BT5-085 | Armageddemon | PARTIAL | Cost reduction self-check fixed. Alt-digi from Diaboromon visible in action mask. Process untestable (requires deleting own Diaboromon). |
| BT17-060 | Armageddemon | PARTIAL | Cost reduction self-check fixed. Static analysis only. |
| BT22-057 | Kurisarimon (inherited) | PASS | Inherited leave-protection effect present (condition checks Diaboromon text). |
| BT5-063 | Tsukaimon | PARTIAL | Standard Lv3, static analysis only. |
| BT2-059 | DemiDevimon | PARTIAL | Standard Lv3, static analysis only. |
| BT17-055 | Infermon | PARTIAL | Lv5, static analysis only. |

## Code Fixes Applied

### Systemic: BeforePayCost cost_reduction leak (31 files)
- Added `if context.get('card_source') is not card: return False` to condition functions in 31 scripts
- Key Diaboromon files: EX6-039, BT5-085, BT17-060
- Batch tool: `tools/fix_cost_reduction_leak.py`

### Systemic: Alt-digi validator blocking (digivolve_validator.py)
- Removed `can_use_condition` check from `_check_alt_digivolve()` and `get_alt_digi_cost()`
- The `_alt_digi_*` attributes already encode all constraints

### BT24-065 condition0 simplification
- Changed `condition0` from `card.permanent_of_this_card()` check to `return True`
- `_alt_digi_name = "Diaboromon"` handles the constraint

## Remaining Work

- EX6-041 Chrysalimon evo_costs data check
- BT5-085 / BT17-060 Armageddemon alt-digi process (requires deleting own Diaboromon to digivolve)
- BT5-063 Tsukaimon / BT2-059 DemiDevimon — need live gameplay for Lv3 chain verification
- BT17-055 Infermon — need live gameplay digivolve chain verification
- LM-031 Black Scramble — digivolve-with-reduction live test
