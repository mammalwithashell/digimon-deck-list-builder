# Priority Archetype Index

17 priority archetypes for faithful card effect implementation.

## Archetypes

| # | Archetype | Deck Library Key | Unique Cards | Decklists | QA Doc | Priority |
|---|-----------|-----------------|--------------|-----------|--------|----------|
| 1 | BG Imperial | BG Imperial | 25 | 2 | [bg-imperial.md](bg-imperial.md) | High (19 unvalidated) |
| 2 | Chaos Control | Chaos Control | 24 | 1 | [chaos_control.md](chaos_control.md) | High (stubs to audit) |
| 3 | Dark Masters | Dark Masters | 58 | 19 | [Dark_Masters.md](Dark_Masters.md) | Medium (grant_skill stubs) |
| 4 | DNA Omnimon | DNA Omnimon | 47 | 64 | [DNA_Omnimon.md](DNA_Omnimon.md) | Medium (name aliasing) |
| 5 | ExMaquinamon | ExMaquinamon | 16 | 7 | [ExMaquinamon.md](ExMaquinamon.md) | High (redirect/force attack) |
| 6 | Galacticmon | Galacticmon | 36 | 14 | [Galacticmon.md](Galacticmon.md) | Medium (OnDigiCardReturnToDeck) |
| 7 | Hudiemon | Hudiemon | 73 | 138 | [hudiemon.md](hudiemon.md) | Medium (stubs to audit) |
| 8 | Jesmon | Jesmon | 118 | 94 | [Jesmon.md](Jesmon.md) | Medium (name aliasing) |
| 9 | Medusamon | Medusamon | 53 | 94 | [medusa.md](medusa.md) | Medium (stubs to audit) |
| 10 | Millenniummon | Millenniummon | 92 | 26 | [millenniummon.md](millenniummon.md) | Medium (stubs to audit) |
| 11 | Puppets | Puppets | 57 | 23 | [Puppets.md](Puppets.md) | Medium (selection fixes) |
| 12 | Rocks | Rocks | 47 | 118 | [rocks.md](rocks.md) | Medium (1 PARTIAL DigiXros) |
| 13 | Royal Knights | Royal Knights | 61 | 49 | [royal-knights.md](royal-knights.md) | Medium (name aliasing) |
| 14 | TS Jupitermon | TS Jupitermon | 30 | 4 | [TS_Jupitermon.md](TS_Jupitermon.md) | Medium (grant_skill stubs) |
| 15 | TS Neptunemon | TS Neptunemon | 30 | 10 | [ts_neptunemon.md](ts_neptunemon.md) | High (8 outstanding issues) |
| 16 | TS Olympos | TS Olympos | 105 | 64 | [ts_olympos.md](ts_olympos.md) | Medium (stubs to audit) |
| 17 | Zephagamon | Zephagamon | 73 | 54 | [Zephaga.md](Zephaga.md) | High (58 unvalidated) |

## Implementation Priority Order

1. **Zephagamon** — 58 unvalidated cards
2. **BG Imperial** — 19 unvalidated cards
3. **TS Neptunemon** — 8 outstanding issues
4. **Chaos Control** — purple control, stubs to audit
5. **ExMaquinamon** — redirect/force attack engine deps
6. **Rocks** — 1 PARTIAL (DigiXros)
7. **Millenniummon** — stub audit
8. **Hudiemon** — stub audit
9. **Dark Masters** — grant_skill stubs
10. **DNA Omnimon** — name aliasing verification
11. **Jesmon** — name aliasing verification
12. **Royal Knights** — name aliasing verification
13. **Puppets** — selection fixes
14. **Galacticmon** — OnDigiCardReturnToDeck
15. **TS Jupitermon** — grant_skill stubs
16. **TS Olympos** — stub audit
17. **Medusamon** — stub audit

## Engine Dependencies

These engine fixes (Phase 1) must land before archetype-level work:

| Fix | Impact | Archetypes Unblocked |
|-----|--------|---------------------|
| 1A: Security Effect Duplication | ~43 tamer scripts | All |
| 1B: Name Aliasing | ~150 scripts | DNA Omnimon, Jesmon, Royal Knights |
| 1C: Disable Effect Enforcement | ~10 scripts | Millenniummon, Chaos Control |
| 1D: DP Floor | 1 script | ExMaquinamon |
| 1E: Redirect Attack | ~5 scripts | ExMaquinamon |
| 1F: Force Attack | ~4 scripts | ExMaquinamon |
| 1G: CANNOT_PLAY_BY_EFFECT | 1 script | Misc |
| 1I: Suppress On Play | 1 script | Misc |

## Matchup Pairs for QA (Phase 6)

| Batch | Matchup | Focus |
|-------|---------|-------|
| 1 | Zephagamon vs BG Imperial | 77 cards needing work |
| 1 | TS Neptunemon vs Chaos Control | 8 outstanding + control stubs |
| 1 | ExMaquinamon vs Rocks | redirect/force attack + DigiXros |
| 1 | Millenniummon vs Hudiemon | Stub audit for both |
| 2 | Dark Masters vs DNA Omnimon | grant_skill + name aliasing |
| 2 | Jesmon vs Royal Knights | name aliasing verification |
| 2 | Puppets vs Galacticmon | selection fixes + OnDigiCardReturn |
| 2 | TS Jupitermon vs TS Olympos | grant_skill + stub audit |
| 3 | Medusamon vs (mirror or cross) | Final regression |
