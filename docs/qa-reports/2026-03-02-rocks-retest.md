# Rocks Re-Test Follow-Up

- **Date**: 2026-03-02
- **Source Report**: `2026-03-01-rocks.md`
- **Verification Mode**: implementation follow-up only; deterministic gameplay re-test not executed in this session

## Summary

The shared engine updates that affect play costs, passive keyword evaluation, and option cleanup are in place. A code-only script pass also fixed the current high-confidence Rocks items: reveal/trash-pop handling in `EX8-047`, `P-039`, `P-206`, and `P-107`; `EX8-048` hand-play filtering; `EX10-025` missing place-from-trash callback; `EX10-033` missing place-from-trash callbacks; keyword/target cleanup in `EX8-070`, `EX10-028`, `EX10-032`, and `EX10-069`; count/order fixes in `EX10-033`, `EX8-055`, `EX10-036`, `EX10-034`, and `BT20-055`; and self-suspend fixes in `EX10-063` and `P-169`. The EX7 / EX8 / EX10 evo-cost data path was then repaired by fixing `tools/build_registry.py` to match the newer inference logic already used by `tools/ingest_cards.py`, followed by a targeted refresh of EX7 / EX8 / EX10 card metadata. EX8 required a per-card fetch fallback because the DigimonCard.io set endpoint returned HTTP 500 for `search.php?card=EX8`.

## Cards Re-Verified

No live gameplay re-verification was executed in this session. Code-only follow-up changes were made for:

- `EX8-047`
- `P-039`
- `P-206`
- `P-107`
- `EX8-048`
- `EX10-025`
- `EX10-033`
- `EX8-070`
- `EX10-028`
- `EX10-032`
- `EX10-069`
- `EX8-055`
- `EX10-036`
- `EX10-034`
- `BT20-055`
- `EX10-063`
- `P-169`
- `cards.json` entries for `EX7`, `EX8`, and `EX10`

## Remaining Work

- Re-run the original Rocks debug-game coverage after the data fix.
- Investigate the residual cards still missing `evo_costs` after refresh (`EX7-017`, `EX8-053`, `EX10-012`, `EX10-013`, `EX10-020`, `EX10-035`, `EX10-057`, `EX10-061`), which appear to be upstream API omissions rather than the local parser bug.
