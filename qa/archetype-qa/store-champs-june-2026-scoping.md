# Store Championship Season June 2026 — Deck Coverage Scoping

Source: https://digilab.cards/blog/store-championship-season-june-2026 (fetched 2026-07-02)
Deck profiles: https://digilab.cards/deck/<slug> (BT25 format). Deck manifest JSON kept in
session scratchpad `digilab_decks_june2026.json`; regenerate coverage with `check_deck_coverage.py`.

## Summary

13 featured decks (Rocks has no profile link and is already implemented). Across the 12
profiled decks: **286 unique cards, 184 implemented (YAML DSL or raw_rust), 102 missing**.
"Implemented" = a YAML spec exists at `code/digimon-engine/cards/<set>/<ID>.yaml` or the ID
appears in `src/cards/raw_rust/`. All 286 IDs exist in `data/cards.json` (several missing
cards have `.json` card-data sidecars but no effect spec).

## Per-deck core gaps (core = ≥25% inclusion on DigiLab)

| Deck | Colors | Core missing | Tech missing |
|---|---|---|---|
| three-musketeers-beelstarmon | Purple | 10: BT25-005, EX7-051, EX7-008, BT25-083, BT21-074, BT25-085, EX7-073, P-180, EX7-071, EX7-070 | LM-056, EX7-048 |
| vulcanusmon | Black/Red | 5: BT24-058, BT24-010, BT25-075, BT25-020, BT25-102 | BT24-063, BT25-073, BT25-085, BT25-086, BT24-092, BT24-097 |
| glowing-dawn | Green/Yellow | 0 | LM-047 |
| ts-angels | Yellow | 3: BT14-033, BT23-027, P-207 | BT24-058, BT23-032, BT14-084, BT1-087, BT25-097 |
| ts-paradise-colosseum | Green/Red | 1: BT25-020 | BT25-086, BT16-004, BT24-010, BT25-039, EX8-045, BT25-075, BT11-089, BT6-102, P-106, BT19-089, BT24-092 |
| jupitermon | Yellow | 4: BT24-003, BT24-014, P-213, BT24-084 | BT25-020, BT7-032, BT14-033, BT25-086, BT13-106, BT24-093, BT25-097, BT24-092 |
| galacticmon | Black | 15: BT21-006, BT11-061, BT21-056, BT18-060, BT21-058, BT18-065, P-094, BT21-060, EX11-046, BT21-062, EX11-066, BT18-092, BT21-087, BT21-098, BT11-105 | BT15-102 |
| ts-toolbox | Green/Black | 0 | BT25-039, BT24-094 |
| millenniummon | Purple/Black | 18: BT15-006, EX9-058, BT3-077, EX10-040, BT19-066, BT18-013, BT19-069, BT19-070, BT18-015, BT18-073, BT19-065, P-220, BT18-019, BT19-101, EX1-066, EX11-055, P-193, P-205 | BT8-107, BT19-099, BT19-006, BT9-070, BT18-007, BT13-083, EX9-060, EX9-074, BT11-018, ST16-14 |
| alter-s-ladder | Black | 3: EX1-066, LM-033, LM-049 | EX4-051, ST20-14 |
| medusamon | Red | 0 | — (BT16-082, BT23-005, EX10-010, BT24-089, BT1-090 all implemented) |
| shinegreymon | Yellow/Red | 7: BT12-034, BT21-040, BT16-029, BT13-008, BT12-038, BT13-015, BT12-092 | BT9-041, BT12-043, EX1-066 |

Fully playable already (core 100%): **glowing-dawn, ts-toolbox, medusamon**.

## Implementation slices (task list #4–#9)

1. **3M Beelstarmon** (12) — user-highlighted deck; EX7 Three Musketeers package + BT25 purple line + P-180, LM-056.
2. **Galacticmon** (16) — Vemmon/Snatchmon/Destromon/Galacticmon ladder + Xeno/Zenith/Ragnarok Cannon/Fusionize + Apocalymon. Note BT11-061 Vemmon allows any number of copies in deck (deck-validation relevant).
3. **Millenniummon** (28) — Kimeramon/Machinedramon/Millenniummon line (BT18/BT19 heavy) + options.
4. **ShineGreymon** (10) — BT12/BT13/BT21 Agumon→GeoGreymon→RizeGreymon→ShineGreymon lines + Marcus Damon tamers.
5. **TS support** (~25) — BT24/BT25 Time Strangers cards spanning Vulcanusmon/Jupitermon/TS-Angels/Paradise/Toolbox + yellow Angel legacy (BT14-033, BT23-027/032, P-207).
6. **Alter-S + staples** (~11) — EX1-066 Analog Youth, LM memory boosts, misc legacy options.

## Cross-deck high-leverage cards (in 3+ decks, missing)

- EX1-066 Analog Youth (millenniummon core, alter-s core, shinegreymon tech)
- BT25-020 Marsmon (vulcanusmon core, ts-paradise core, jupitermon tech)
- BT25-086 Dan Yuki (4 decks tech)
- BT24-092 Shock Plasma (4 decks tech)
- BT25-097 Guardian Palace (3 decks tech)
- BT14-033 Patamon, EX4-074 (implemented) already shared.

## Implemented-but-unverified (no verdict in validated_cards_dsl.json — audit candidates)

BT10-042, BT19-075, BT24-030, BT24-034, BT24-035, BT24-037, BT24-041, BT24-051, BT24-062,
BT24-083, BT24-085, BT24-088, BT24-090, BT24-091, BT24-095, BT24-102, EX11-074

## Pipeline

Per rule 28 / repo skills: `/assess-archetype-rust` per slice (gap audit → docs/RUST_ENGINE_GAPS.md
+ fix plan) → address gaps → `/batch-implement-cards-rust-dsl` (TDD, DSL-first) → archetype
interaction tests via `/archetype-interaction-test-author` once per-card tests are green.

## Assessment results (2026-07-02, 21 audit agents, all 102 missing cards + BT19-075 re-audit)

| Slice | Audited | 🟢 | 🟡 | 🔴 |
|---|---|---|---|---|
| 3M BeelStarmon | 12 | 2 | 4 | 6 |
| Galacticmon | 16 | 7 | 3 | 6 |
| Millenniummon | 29 | 20 | 3 | 6 |
| ShineGreymon | 9 | 8 | 0 | 1 |
| TS support | 24 | 16 | 2 | 6 |
| Staples + Alter-S | 13 | 10 | 2 | 1 |
| **Total** | **103** | **63** | **14** | **26** |

Gap entries: three "Store-champs June-2026 audit" sections in `docs/RUST_ENGINE_GAPS.md`
(~28 consolidated entries + existing-gap driver appends + tracker-hygiene notes). Fix plans:
`.claude/plans/rust-engine-gaps-{three-musketeers-beelstarmon,galacticmon,millenniummon,shinegreymon,ts-support-staples}.md`.

High-leverage gap clusters (by cards unblocked):
1. **Option-use from non-hand origins + in-flight Option placement** — unblocks P-180, EX7-071, EX7-070, BT25-083, BT25-085, EX7-048, BT21-062 (7 cards, 2 decks).
2. **Whole-card "in its text" predicate (HasText)** — fidelity fix across 12 cards.
3. **Source-return-to-deck-bottom observer + replacement cost** — the Galacticmon engine loop (BT21-058, BT18-065, BT21-062).
4. **Cheap predicate/formula leaves** (total-security-count, memory-count formula, binding-card-color, event-target-trait-contains, distinct-named-count, source-count threshold, cost-relative-to-subject, event-target source-count) — 8 small additions unblocking 8 cards.
5. **pay_cost park-and-resume** (BeforePayCost interactive delete cost) — BT18-073, BT13-083, half of BT13-103.
6. **DNA workstream** (trash material + printed cost + recipe enforcement) — BT18-015, BT18-073 + pre-existing EX6-072/EX3-008/BT17-095 drivers.
7. **Link economy** (link-trash-as-activation-cost, own-link-count formula, link-inherited-ESS) — BT25-073, BT25-075, BT24-092 + BT25-100/093/101.
8. **Granted-effect selection bodies** — BT23-032. **Battle-winner trigger** — BT25-020. **Security-zone field aura** — BT25-102. **Deck copy-limit raise** — BT11-061. **Per-color mass delete + source-color predicate** — EX9-074. **Event-set reveal placement** — EX11-066. **Trash-count binding** — BT19-075.
