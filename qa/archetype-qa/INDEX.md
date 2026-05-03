# Priority Archetype Index

17 priority archetypes -- faithfulness reviewed 2026-03-17.

These statuses are from the legacy Python QA lane. Rust YAML DSL readiness reports live under `dsl/` and may have stricter blocked verdicts where executable Rust card specs, action masks, or pending-selection support are missing.

## Status Summary

| # | Archetype | Cards | Faithful | Fixed | Deferred | Status |
|---|-----------|-------|----------|-------|----------|--------|
| 1 | Chaos Control | 24 | 14 | 10 | 0 | Complete |
| 2 | Medusamon | 53 | 33 | 20 | 0 | Complete |
| 3 | Dark Masters | 58 | 31 | 11 | 16 | Complete |
| 4 | TS Jupitermon | 30 | 20 | 10 | 0 | Complete |
| 5 | Royal Knights | 61 | 47 | 11 | 3 | Complete |
| 6 | DNA Omnimon | 47 | 37 | 7 | 3 | Complete |
| 7 | Jesmon | 118 | 69 | 27 | 9 | Complete (13 engine gaps) |
| 8 | Puppets | 57 | 45 | 5 | 7 | Complete |
| 9 | Hudiemon | 73 | 39 | 12 | 3 | Complete (~20 unaudited generic tech) |
| 10 | TS Neptunemon | 30 | 19 | 7 | 4 | Complete |
| 11 | Millenniummon | 92 | 79 | 7 | 6 | Complete |
| 12 | ExMaquinamon | 16 | 12 | 2 | 2 | Complete |
| 13 | Galacticmon | 36 | 16 | 0 | 5 | Complete (sampled) |
| 14 | Zephagamon | 73 | 64 | 0 | 9 | Complete |
| 15 | BG Imperial | 25 | 21 | 0 | 4 | Complete |
| 16 | Rocks | 47 | 40 | 2 | 5 | Complete |
| 17 | TS Olympos | 105 | 90+ | 5 | 0 | Complete (shared with TS Jupitermon/Neptunemon) |

**Totals**: ~870 cards reviewed, ~586 faithful, ~136 fixed, ~76 deferred

## QA Documents

| Archetype | QA Doc |
|-----------|--------|
| BG Imperial | [bg-imperial.md](bg-imperial.md) |
| Chaos Control | [chaos_control.md](chaos_control.md) |
| Dark Masters | [Dark_Masters.md](Dark_Masters.md) |
| DNA Omnimon | [DNA_Omnimon.md](DNA_Omnimon.md) |
| ExMaquinamon | [ExMaquinamon.md](ExMaquinamon.md) |
| Galacticmon | [Galacticmon.md](Galacticmon.md) |
| Hudiemon | [hudiemon.md](hudiemon.md) |
| Jesmon | [Jesmon.md](Jesmon.md) |
| Medusamon | [medusa.md](medusa.md) |
| Millenniummon | [millenniummon.md](millenniummon.md) |
| Puppets | [Puppets.md](Puppets.md) |
| Rocks | [rocks.md](rocks.md) |
| Royal Knights | [royal-knights.md](royal-knights.md) |
| TS Jupitermon | [TS_Jupitermon.md](TS_Jupitermon.md) |
| TS Neptunemon | [ts_neptunemon.md](ts_neptunemon.md) |
| TS Olympos | [ts_olympos.md](ts_olympos.md) |
| Zephagamon | [Zephaga.md](Zephaga.md) |

## Other QA Artifacts

| File | Purpose |
|------|---------|
| [engine-api-reference.md](engine-api-reference.md) | Engine scripting API reference for card implementation agents |
| [engine-gaps.md](engine-gaps.md) | Accumulated engine gaps found during reviews |
| [dsl/bg-imperial.md](dsl/bg-imperial.md) | Rust YAML DSL readiness assessment for BG Imperial |
| [dsl/2026-05-03-dna-omnimon-dsl-engine-gaps.md](dsl/2026-05-03-dna-omnimon-dsl-engine-gaps.md) | DNA Omnimon Rust DSL/engine gap source document for cross-archetype spec compilation |
| [dsl/bg-imperial-cross-archetype-gaps-2026-05-03.md](dsl/bg-imperial-cross-archetype-gaps-2026-05-03.md) | BG Imperial Rust DSL/engine reusable gap inputs for cross-archetype spec compilation |
| [dsl/rocks-gap-inputs-2026-05-03.md](dsl/rocks-gap-inputs-2026-05-03.md) | Rocks Rust DSL/engine reusable gap inputs for cross-archetype spec compilation |
| [dsl/puppets-2026-05-03-engine-dsl-gaps.md](dsl/puppets-2026-05-03-engine-dsl-gaps.md) | Rust engine / DSL gap inventory for Puppets |
