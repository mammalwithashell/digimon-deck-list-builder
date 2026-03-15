# Archetype Implementation Status

**Last updated**: 2026-03-11

## Implemented Archetypes

Archetypes that have gone through the `/implement-archetype` pipeline (card script QA + fixes) and gameplay QA testing.

| Rank | Archetype | Meta Share | Lists | Cards | QA Status | Outstanding Issues |
|------|-----------|------------|-------|-------|-----------|--------------------|
| 1 | CS Hudiemon | 11.36% | 20 | 40 | Gameplay QA'd | 3 (engine limitations) |
| 2 | TS Neptune | 5.68% | 10 | 30 | Gameplay QA'd | 9 outstanding |
| 4 | Medusa | 5.11% | 9 | 30 | Gameplay QA'd | 2 (partial cards) |
| 5 | Royal Knights | 5.11% | 9 | 35 | Gameplay QA'd | 5 outstanding |
| 6 | Rocks | 4.55% | 8 | 28 | Gameplay QA'd | 1 outstanding |
| 7 | Millennium | 3.98% | 7 | 37 | Gameplay QA'd | 2 outstanding |
| 8 | Diaboromon | 3.41% | 6 | 26 | Gameplay QA'd | 2 outstanding |
| 11 | TS Olympos | 2.84% | 5 | 31 | Implemented + Gameplay QA'd | 2 outstanding |
| 19 | BG Imperial | 1.14% | 2 | 25 | Implemented (QA'd as opponent) | — |
| — | CS Mastemon | 5.11% | 9 | 65 | Gameplay QA'd | 1 outstanding |

**Notes:**
- CS Hudiemon, TS Neptune, Medusa, Royal Knights, Rocks, Millennium, Diaboromon, and CS Mastemon were QA'd through the earlier gameplay QA pipeline (reports in `qa/qa-reports/`).
- TS Olympos and BG Imperial went through the full `/implement-archetype` pipeline with per-card script review and fixes.
- "Outstanding Issues" counts come from `qa/qa-reports/INDEX.md`.

---

## Next 10: Highest Meta Share — Not Yet Implemented

These archetypes have the highest meta share among those that have NOT been through the implementation pipeline.

| Priority | Archetype | Meta Share | Lists | Unique Cards | Notes |
|----------|-----------|------------|-------|--------------|-------|
| 1 | Jesmon GX | 3.41% | 6 | 31 | Royal Knights adjacent — may share cards with RK archetype |
| 2 | Sakuya | 3.41% | 6 | 28 | |
| 3 | Zephaga | 2.84% | 5 | 40 | Large card pool |
| 4 | DarkMaster | 2.27% | 4 | 28 | |
| 5 | TS Jupiter | 2.27% | 4 | 30 | TS archetype — may share cards with TS Neptune/Olympos |
| 6 | Hudie | 1.70% | 3 | 28 | Related to CS Hudiemon — likely shares cards |
| 7 | RK | 1.70% | 3 | 35 | Royal Knights variant — likely overlaps with Royal Knights |
| 8 | Red Jesmon | 1.70% | 3 | 31 | Jesmon variant — may share cards with Jesmon GX |
| 9 | Appmon | 1.14% | 2 | 20 | Small card pool |
| 10 | Bloomlord | 1.14% | 2 | 28 | |

---

## Implementation Priority Recommendations

1. **Jesmon GX** (3.41%) — High meta share, 6 decklists. May share cards with already-implemented Royal Knights.
2. **Sakuya** (3.41%) — Equal meta share to Jesmon GX, independent archetype.
3. **Zephaga** (2.84%) — Large unique card pool (40 cards), will add significant coverage.
4. **TS Jupiter** (2.27%) — TS family archetype, may reuse TS Neptune/Olympos cards reducing new work.
5. **DarkMaster** (2.27%) — Independent archetype, moderate card pool.

---

## How to Generate This Data

```bash
python tools/rank_archetypes.py --top 20
```
