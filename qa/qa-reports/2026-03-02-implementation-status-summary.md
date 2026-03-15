# March 2 Implementation Status Summary

- **Date**: 2026-03-02
- **Source Plan**: March 1, 2026 QA fix plan for the unresolved March 1 report set, followed by the high-confidence script-fix tranche and archetype-complete broader sweep
- **Scope Covered**: `2026-03-01-ts-neptune.md`, `2026-03-01-rocks.md`, `2026-03-01-royal-knights.md`, `2026-03-01-diaboromon.md`, `2026-03-01-cs-mastemon.md`, `2026-03-01-millennium.md`, and `2026-03-01-cross-archetype-matchups.md`
- **Verification Mode**: implementation follow-up; partial static validation completed, full deterministic gameplay re-test not yet completed

## Initial Plan Summary

The original plan had three layers:

1. Land the shared engine and data fixes that multiple March 1 reports depended on.
2. Prioritize Royal Knights, especially the required `BT13-007 King Drasil_7D6` implementation.
3. Sweep the remaining archetype reports, starting with high-confidence script-local fixes and then finishing each archetype end-to-end with targeted re-tests and QA documentation updates.

The intended end state was:

- card-text-faithful engine behavior for the reported issues
- updated March 2 re-test reports
- updated `INDEX.md`
- updated `validated_cards.json`
- no issue considered fully closed until it had a successful re-test

## What Was Completed

### Shared Engine and Data Work

The following cross-cutting engine/data changes were implemented:

- Centralized play-cost calculation in `game.py`, with a shared path for normal plays and effect-based plays.
- Added breeding-area opt-in support for play-cost reducers via `_allow_breeding_source`.
- Fixed non-Delay / non-Training option cleanup so normal options trash after resolution instead of lingering in battle area.
- Fixed trigger-context collisions by separating effect-source context from event context (`played_card`, `played_permanent`, `event_player`, `event_permanent`).
- Updated passive keyword checks so runtime conditions are respected instead of treating passive keyword flags as always-on.
- Added `_alt_digi_color` handling in the digivolve validator.
- Extended `effect_play_from_zone(..., free=False, manual_reduction=N)` so scripts can apply reduced-cost plays without using incorrect free-play shortcuts.
- Added Royal Knights token types to `token_registry.py`.
- Repaired the EX7 / EX8 / EX10 evo-cost build path by fixing `tools/build_registry.py` to use the same evo-cost inference rules already present in `tools/ingest_cards.py`.
- Refreshed `cards.json` data for EX7 / EX8 / EX10, including a per-card fallback refresh for EX8 after the DigimonCard.io set endpoint returned `HTTP 500`.

### Royal Knights Priority Work

The Royal Knights priority lane from the original plan was implemented:

- `BT13-007 King Drasil_7D6` was reworked into a real breeding-area `BeforePayCost` reducer tied into the shared play-cost path.
- `BT23-072 King Drasil_7D6` was corrected to grant keywords to the played Digimon instead of itself.
- Royal Knights token stubs were replaced with real token creation.
- Sistermon conditional behavior was normalized to depend on runtime conditions instead of brittle always-on or source-location checks.
- Missing innate keyword fields were added for the remaining obvious Royal Knights omissions.

### High-Confidence Archetype Script Fixes

A large script-local sweep was completed across the reported archetypes.

#### CS Mastemon

Implemented code-only fixes for the low-risk and several medium-risk script issues, including:

- alt-digivolve restriction fixes
- corrected hand/security handling
- corrected "1 or fewer Tamers" gates
- tighter filters for trash-to-play and hand-trash costs
- better DNA-only and played-card condition checks
- corrected self-suspend / effect-branch logic in several scripts

Representative cards patched include:

- `BT16-030`
- `EX8-030`
- `BT14-084`
- `BT19-067`
- `BT8-035`
- `EX5-028`
- `EX5-057`
- `BT22-093`
- `BT23-088`
- `P-187`
- `BT14-033`
- `EX6-022`
- `BT19-039`
- `BT17-025`
- `EX5-061`
- `EX5-059`

#### Millennium

Implemented code-only fixes for the documented low-risk issues, including:

- corrected Security Attack modifier field usage
- corrected wrong-zone play-from-trash behavior
- corrected prompt / trash-selection handling
- removed broken On Play suppression guards
- corrected `BT19-101` destination behavior from bounce-to-hand to deck-bottom

Representative cards patched include:

- `BT18-015`
- `BT19-065`
- `BT19-070`
- `BT19-099`
- `BT13-083`
- `BT3-006`
- `BT19-069`
- `BT18-007`
- `BT18-013`
- `BT18-019`
- `BT18-073`
- `EX2-046`
- `BT19-101`

#### Rocks

Implemented the most complete script/data sweep so far:

- removed spurious `trash_cards.pop()` reveal bugs
- fixed broad filters, target selection, wrong counts, wrong order, and self-suspend mistakes
- added missing process callbacks for place-from-trash effects
- fixed `P-206` Delay to use cost reduction instead of incorrect free play
- fixed `EX10-069` Delay targeting/filter logic
- repaired the shared EX7 / EX8 / EX10 evo-cost data path and refreshed card metadata

Representative cards patched include:

- `EX8-047`
- `P-039`
- `P-206`
- `P-107`
- `EX8-048`
- `EX10-025`
- `EX8-070`
- `EX10-028`
- `EX10-032`
- `EX10-033`
- `EX8-055`
- `EX10-036`
- `EX10-034`
- `BT20-055`
- `EX10-063`
- `P-169`
- `EX10-069`

#### Diaboromon

Implemented the low-risk Diaboromon script sweep:

- replaced token stubs with `game.effect_play_token(player, "diaboromon")`
- removed reveal/trash-pop bugs
- tightened delete filters
- added missing condition gates
- added the missing innate keyword on `BT24-064`

Representative cards patched include:

- `EX6-043`
- `BT22-064`
- `BT24-052`
- `BT22-059`
- `EX6-036`
- `EX6-039`
- `BT22-053`
- `BT22-057`
- `BT24-064`

#### TS Neptune

Implemented the remaining low-risk script-local items:

- removed broken On Play suppression guards
- fixed the trash-to-draw prompt flow
- corrected a start-of-main sequencing issue
- corrected a wrong self-suspend target

Representative cards patched include:

- `BT24-088`
- `BT3-093`
- `BT24-102`

### QA Follow-Up Reports Added or Updated

The March 2 follow-up reports were created and/or updated to document the implementation work in progress:

- `2026-03-02-royal-knights-retest.md`
- `2026-03-02-ts-neptune-retest.md`
- `2026-03-02-rocks-retest.md`
- `2026-03-02-diaboromon-retest.md`
- `2026-03-02-cs-mastemon-retest.md`
- `2026-03-02-millennium-retest.md`
- `2026-03-02-cross-archetype-retest.md`

These reports currently reflect implementation follow-up and code-only changes. They are not yet the final “all issues fixed and fully re-validated” reports.

### Validation Completed So Far

The work completed so far has been statically validated in the following ways:

- `python -m py_compile` was run on each batch of touched Python files and passed.
- `cards.json` was re-validated as parseable JSON after the data refresh.

## What Is Still Left To Do

### Full Re-Test and Status Promotion

The largest remaining gap is verification:

- run deterministic gameplay re-tests for each archetype
- confirm memory deltas, action legality, trigger prompts, and targeting are correct
- then update `INDEX.md` issue statuses from `OUTSTANDING` to `FIXED` where confirmed
- then update `validated_cards.json` to promote cards from `PARTIAL` / `FAIL` only after successful re-test

Most March 2 reports are still implementation notes, not final validation closures.

### Remaining CS Mastemon Work

Still pending:

- full lock-effect enforcement verification for the memory / cost / play restrictions
- any remaining medium-risk script rewrites not yet fully validated
- complete archetype re-test and final documentation/index promotion

### Remaining Millennium Work

Still pending:

- nuanced interaction re-test for the newly touched scripts
- any remaining medium-risk Millennium behavior that needs live confirmation rather than script inspection
- re-test of options affected by the shared option cleanup

### Remaining Rocks Work

Still pending:

- deterministic Rocks re-test after the data repair
- investigation of the residual EX7 / EX8 / EX10 cards still missing `evo_costs` after refresh:
  - `EX7-017`
  - `EX8-053`
  - `EX10-012`
  - `EX10-013`
  - `EX10-020`
  - `EX10-035`
  - `EX10-057`
  - `EX10-061`
- these appear to be upstream API omissions rather than the local parser bug, so they may require manual fallback handling or a different data source

### Remaining Diaboromon Work

Still pending:

- medium-risk behavior not yet completed, especially attack redirect / Overclock / any remaining cost-branch corrections
- full live archetype re-test and issue promotion

### Remaining TS Neptune Work

Still pending:

- persistent pending-selection deadlock fix
- clean implementation for “place from hand as bottom source” patterns
- link attachment lifecycle fixes
- full live archetype re-test

### Remaining Royal Knights Work

Still pending:

- live re-test of the already patched Royal Knights core, especially:
  - `BT13-007`
  - `BT20-017`
  - `BT23-072`
  - `BT23-057`
  - `BT8-097`
  - `BT9-103`
  - `BT6-082`
  - `ST12-12`
- any follow-up bug fixes that only show up in a real debug-game replay

### Cross-Archetype Closure

Still pending:

- final cross-archetype replay after the remaining TS Neptune and verification work
- confirmation that the original selection-phase failure no longer occurs

## Current Risks and Known Gaps

- The DigimonCard.io `search.php?card=EX8` set endpoint currently returns `HTTP 500`, so EX8 refresh required a per-card fallback path.
- A full `test_rocks_qa.py` run timed out during this implementation pass, so Rocks still needs a narrower or staged deterministic re-test.
- Several archetype reports have substantial code fixes landed but are still not safe to mark as fully resolved without live verification.

## Recommended Next Steps

1. Run deterministic gameplay re-tests for Rocks first, since the data blocker was just cleared and that archetype now has the most complete implementation pass.
2. After each successful archetype re-test, update `INDEX.md` and `validated_cards.json` immediately instead of deferring all documentation to the end.
3. Resume the broader sweep in the planned order for the remaining medium-risk items:
   - CS Mastemon
   - Millennium
   - Diaboromon
   - TS Neptune
   - Royal Knights
   - cross-archetype final pass
