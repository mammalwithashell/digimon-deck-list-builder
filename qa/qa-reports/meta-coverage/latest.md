# Meta coverage

**Generated — do not hand-edit.** Regenerate with `PYTHONPATH=code python -m tools.meta_coverage`.

- Generated: 2026-10-03T08:35:56+00:00 (git `0bbfdc083`)
- Window: **BT26** — 2026-09-04 → 2026-09-12 · 108 decklists · 650 distinct cards · sources dcg_nexus 108
- Weighting: **digilab** — DigiLab names 67.05% of the field explicitly; the other 32.95% is spread over unnamed archetypes by list count
- ⚠️ Only 108 decklists fall in this window (< 150); card-level figures are a small sample. Refresh data/deck_library.json (meta_loader.py --scrape-dcg-nexus) and re-run.

## Headline

| Measure | Implemented | Tested | DCGO-verified |
|---|---|---|---|
| Share of meta card copies | 64.7% | 63.8% | 13.1% |
| Distinct meta cards | 414/650 (63.7%) | 407/650 (62.6%) | 50/650 (7.7%) |
| Field share of decks with every card at this level | 17.0% (19 lists) | 16.2% (17 lists) | 0.0% (0 lists) |

**Trainable today**: 1.9% of the field (2 lists) — fully playable AND admitted by the training deck pool's archetype gate (`gauntlet._load_fully_implemented_archetypes`, which matches free-form ledger labels).

Clauses on meta cards: 2458 — confirmed 242, diverged 2, unreachable 13, unavailable 0, unmeasured 2201 (9.8% confirmed; 15.7% weighted by field play rate).

Whole pool: 927/4472 cards implemented (20.7%), 19 flagged PARTIAL/BLOCKED, 100 DCGO-verified.

## Launch plan

Goal: Optimize Digimon TCG decklists for the live meta (as DigiLab and DigimonMeta measure it) by training agents on a simulator that plays that meta faithfully. (plan updated 2026-10-03; edit `qa/qa-reports/meta-coverage/launch_plan.json`)

- **M0 · Measure the gap** (done) — 2/2 gates met
  - [x] Meta coverage report + dashboard regenerate from repo data
  - [x] BT26 field shares transcribed from DigiLab (data/meta_shares.json)
- **M1 · Make the BT26 field playable** (active) — 0/3 gates met
  - [ ] Field share of fully playable decks: 16.96 (target >= 75 %)
  - [ ] BT26 cards seen in meta lists that are implemented: 0 (target >= 60 cards)
  - [ ] Deck library holds at least 300 BT26-window lists: 108 (target >= 300 lists)
- **M2 · Let training see the meta** (next) — 1/2 gates met
  - [ ] Playable decks the training pool rejects (field share): 15.02 (target <= 1 %)
  - [x] Implemented cards missing from the deck-builder allowlist: 0 (target <= 0 cards)
- **M3 · Verify what the field plays** (next) — 0/2 gates met
  - [ ] Meta card copies that are DCGO-verified: 13.13 (target >= 40 %)
  - [ ] Clauses confirmed, weighted by how often the field plays each card: 15.74 (target >= 50 %)
- **M4 · Meta training pilot** (later) — 0/3 gates met
  - [ ] Field share the training pool can use: 1.94 (target >= 60 %)
  - [ ] 2026-07-02 anchored baselines re-measured on the current engine
  - [ ] A BT26 generalist clears 55% anchored vs greedy and the frozen champions
- **M5 · Deck optimization loop** (later) — 0/2 gates met
  - [ ] Architect agent ranks list variants against a BT26 gauntlet
  - [ ] Training runs (training_run.json, anchored_evals.jsonl) charted on this dashboard

**Unlock path** (cards to implement, in `implement_next` order, until that much of the field is fully playable): 25% → 12 cards, 50% → 33 cards, 75% → 74 cards, 90% → 138 cards, 100% → 236 cards. 236 missing cards in total.

## By set

| Set | Share of meta copies | Meta cards implemented | Implemented copies | Whole set implemented |
|---|---|---|---|---|
| BT26 | 21.2% | 0/74 | 0.0% | 0/104 |
| BT25 | 17.0% | 82/85 | 98.1% | 98/104 |
| BT24 | 16.8% | 34/47 | 70.3% | 46/102 |
| EX12 | 10.4% | 54/54 | 100.0% | 54/77 |
| ST23 | 4.6% | 15/15 | 100.0% | 15/15 |
| BT21 | 3.9% | 32/45 | 77.8% | 49/102 |
| ST24 | 3.1% | 15/15 | 100.0% | 15/15 |
| P | 2.5% | 23/33 | 80.5% | 46/240 |
| AD1 | 1.8% | 10/13 | 87.1% | 17/25 |
| EX9 | 1.7% | 10/23 | 57.5% | 14/74 |
| BT20 | 1.6% | 6/19 | 25.3% | 18/102 |
| LM | 1.4% | 12/20 | 68.9% | 16/68 |
| BT19 | 1.4% | 16/19 | 85.5% | 25/102 |
| EX7 | 1.0% | 8/8 | 100.0% | 26/74 |

## Archetypes

| Archetype | Field share | Lists | Implemented | Tested | Verified | Clauses confirmed | Playable lists | Training gate | Closest list missing |
|---|---|---|---|---|---|---|---|---|---|
| TS Jupitermon | 10.5% | 12 | 71.8% | 71.0% | 4.2% | 7.1% | 0/12 | fail | 1 (BT9-083) |
| Glowing Dawn | 8.1% | 9 | 93.0% | 93.0% | 0.0% | 0.0% | 4/9 | fail | 0 |
| Toho Braves | 6.8% | 7 | 90.2% | 90.2% | 77.2% | 71.6% | 1/7 | pass | 0 |
| Titans | 6.4% | 7 | 18.8% | 14.0% | 6.6% | 9.8% | 0/7 | fail | 13 (BT24-009, BT24-013, BT24-021, BT24-026, BT24-042, BT24-045…) |
| Data Squad | 5.8% | 10 | 59.9% | 59.1% | 0.7% | 0.9% | 2/10 | fail | 0 |
| Chronomon | 5.7% | 4 | 20.8% | 20.8% | 7.4% | 13.1% | 0/4 | fail | 13 (BT26-001, BT26-009, BT26-011, BT26-015, BT26-016, BT26-021…) |
| Plutomon | 5.5% | 2 | 22.2% | 21.3% | 13.0% | 9.7% | 0/2 | fail | 14 (BT24-009, BT24-021, BT24-023, BT24-026, BT24-042, BT24-045…) |
| TS Mervamon | 4.8% | 2 | 74.1% | 74.1% | 5.6% | 9.9% | 0/2 | fail | 6 (BT26-067, BT26-073, BT26-081, BT26-087, BT26-088, BT26-092) |
| Three Musketeers | 4.2% | 3 | 98.8% | 98.8% | 94.4% | 93.2% | 2/3 | fail | 0 |
| Apps Dantemon | 3.2% | 3 | 24.1% | 24.1% | 4.3% | 5.2% | 0/3 | fail | 11 (BT24-099, BT26-007, BT26-010, BT26-019, BT26-028, BT26-037…) |
| Omni Ladder | 2.9% | 3 | 100.0% | 100.0% | 1.9% | 3.8% | 3/3 | fail | 0 |
| TS Toolbox | 2.9% | 3 | 79.0% | 74.7% | 2.5% | 4.0% | 0/3 | fail | 4 (BT26-067, BT26-073, BT26-081, BT26-088) |
| Accel | 1.9% | 2 | 38.0% | 38.0% | 0.0% | 0.0% | 0/2 | fail | 11 (BT20-004, BT20-030, BT20-031, BT20-033, BT20-038, BT20-039…) |
| Gammamon | 1.9% | 2 | 29.6% | 29.6% | 0.9% | 1.9% | 0/2 | fail | 13 (BT21-002, BT21-077, BT21-080, BT21-090, BT22-045, EX10-042…) |
| Green TyrantKabuterimon | 1.9% | 2 | 15.7% | 15.7% | 0.0% | 0.0% | 0/2 | fail | 12 (BT15-004, BT15-043, BT15-085, BT16-042, BT16-045, BT16-048…) |
| Red Hybrid (AncientGreymon) | 1.9% | 2 | 43.1% | 35.8% | 3.7% | 8.6% | 0/2 | pass | 12 (AD1-020, BT12-088, BT17-012, BT17-014, BT18-011, BT18-022…) |
| Styracomon | 1.9% | 2 | 97.2% | 97.2% | 4.6% | 15.5% | 1/2 | fail | 0 |
| TS Cosmic Area | 1.9% | 2 | 67.6% | 67.6% | 13.0% | 14.1% | 0/2 | fail | 6 (BT25-103, BT26-022, BT26-067, BT26-081, BT26-088, P-200) |
| TS Vulcanusmon | 1.9% | 2 | 91.7% | 91.7% | 15.7% | 28.1% | 0/2 | fail | 1 (P-210) |
| Virus Busters | 1.9% | 2 | 98.2% | 98.2% | 0.0% | 0.0% | 1/2 | fail | 0 |
| Xros | 1.9% | 2 | 94.4% | 94.4% | 0.9% | 1.8% | 1/2 | fail | 0 |
| Saiyu Warriors | 1.8% | 4 | 97.7% | 97.7% | 7.4% | 36.0% | 1/4 | fail | 0 |
| ShineGreymon | 1.7% | 3 | 93.2% | 93.2% | 4.3% | 5.4% | 0/3 | fail | 1 (LM-059) |
| TS Ceresmon | 1.4% | 7 | 59.5% | 48.4% | 1.6% | 2.1% | 1/7 | fail | 0 |
| Apps Reboot | 1.1% | 1 | 88.9% | 88.9% | 3.7% | 6.2% | 0/1 | fail | 3 (BT26-010, BT26-086, EX1-072) |

## Implement next

Ordered to make the most field share fully playable soonest (see `unlock_order`).

| # | Card | Name | Kind | Copies/deck | Field play | Clauses | Cumulative playable | Played in |
|---|---|---|---|---|---|---|---|---|
| 1 | BT26-104 | Kunlun | Tamer | 0.14 | 6.2% | 4 | 18.3% | Toho Braves 78%, Saiyu Warriors 22% |
| 2 | BT26-081 | Mervamon | Digimon | 0.44 | 14.9% | 6 | 18.3% | TS Jupitermon 35%, TS Mervamon 32%, TS Toolbox 20% |
| 3 | BT26-073 | Aegiochusmon: Dark | Digimon | 0.39 | 19.3% | 5 | 18.3% | TS Jupitermon 41%, Chronomon 30%, TS Mervamon 25% |
| 4 | BT26-029 | Aegiochusmon: Holy | Digimon | 0.29 | 12.2% | 6 | 18.3% | TS Jupitermon 64%, Chronomon 12%, TS Angels 8% |
| 5 | BT26-090 | Kanan Yuki | Tamer | 0.23 | 12.2% | 2 | 19.2% | TS Jupitermon 43%, Chronomon 23%, TS Toolbox 16% |
| 6 | BT26-092 | Shota Kuroi | Tamer | 0.48 | 20.6% | 3 | 19.2% | Chronomon 28%, Titans 27%, TS Mervamon 23% |
| 7 | BT26-033 | Jupitermon | DUAL | 0.21 | 10.1% | 10 | 19.2% | TS Jupitermon 60%, Chronomon 28%, TS Toolbox 10% |
| 8 | BT26-103 | Jupitermon: Wrath Mode | Digimon | 0.13 | 6.1% | 8 | 20.9% | TS Jupitermon 100% |
| 9 | BT26-083 | Junomon: Hysteric Mode | Digimon | 0.07 | 5.0% | 9 | 22.7% | TS Jupitermon 52%, Three Musketeers 28%, TS Angels 19% |
| 10 | BT26-067 | Wizardmon | Digimon | 0.21 | 8.6% | 4 | 22.7% | TS Mervamon 56%, TS Toolbox 23%, TS Cosmic Area 11% |
| 11 | BT26-088 | Hiroko Sagisaka | Tamer | 0.17 | 9.0% | 3 | 23.7% | TS Mervamon 53%, Chronomon 16%, TS Cosmic Area 11% |
| 12 | BT26-087 | Toya Kuga | Tamer | 0.23 | 12.2% | 3 | 28.5% | Chronomon 46%, TS Mervamon 39%, TS Jupitermon 14% |
| 13 | BT26-078 | Cherubimon | Digimon | 0.42 | 16.2% | 4 | 29.9% | Chronomon 35%, Plutomon 34%, Titans 11% |
| 14 | BT26-009 | Hyokomon | Digimon | 0.35 | 10.2% | 4 | 30.8% | Chronomon 56%, TS Jupitermon 34%, TS Cosmic Area 10% |
| 15 | BT26-016 | Chronomon: Holy Mode | Digimon | 0.35 | 10.2% | 6 | 32.5% | Chronomon 56%, TS Jupitermon 34%, TS Cosmic Area 10% |
| 16 | BT26-022 | Sorcermon | Digimon | 0.08 | 3.8% | 3 | 33.5% | TS Angels 26%, TS Cosmic Area 26%, TS Toolbox 26% |
| 17 | BT26-011 | Buraimon | Digimon | 0.32 | 9.3% | 4 | 34.3% | Chronomon 61%, TS Jupitermon 28%, TS Cosmic Area 10% |
| 18 | BT26-012 | Manekimon | Digimon | 0.13 | 4.9% | 3 | 34.3% | Toho Braves 100% |
| 19 | BT26-008 | Kotemon | Digimon | 0.04 | 2.9% | 4 | 36.3% | Toho Braves 100% |
| 20 | BT26-014 | Darumamon | Digimon | 0.02 | 1.9% | 5 | 38.2% | Toho Braves 100% |
| 21 | BT26-089 | Kyo Sawashiro | Tamer | 0.10 | 3.6% | 3 | 39.1% | Glowing Dawn 100% |
| 22 | LM-059 | Heat Training | Option | 0.06 | 2.7% | 0 | 40.3% | ShineGreymon 64%, Styracomon 36% |
| 23 | BT26-015 | Butenmon | Digimon | 0.32 | 8.4% | 4 | 40.3% | Chronomon 68%, TS Jupitermon 21%, TS Cosmic Area 12% |
| 24 | BT26-060 | Chronomon: Destroy Mode | Digimon | 0.28 | 8.4% | 8 | 40.3% | Chronomon 68%, TS Jupitermon 21%, TS Cosmic Area 12% |
| 25 | BT26-001 | Yokomon | Digi-Egg | 0.30 | 7.5% | 1 | 40.3% | Chronomon 76%, TS Jupitermon 24% |
| 26 | BT26-085 | Giant Slayer | Digimon | 0.23 | 7.5% | 7 | 40.3% | Chronomon 76%, TS Jupitermon 24% |
| 27 | BT26-101 | Cross Arts | Option | 0.06 | 4.1% | 3 | 42.0% | TS Jupitermon 42%, Chronomon 34%, TS Vulcanusmon 23% |
| 28 | BT26-032 | Ceresmon | DUAL | 0.05 | 3.9% | 7 | 43.5% | Chronomon 74%, TS Ceresmon 26% |
| 29 | BT26-021 | Gekomon | Digimon | 0.13 | 6.1% | 4 | 44.9% | Chronomon 70%, Titans 30% |
| 30 | BT26-096 | Kosuke Misono | Tamer | 0.12 | 3.8% | 3 | 47.7% | Chronomon 75%, TS Cosmic Area 25% |

## Exam next (implemented, clauses unmeasured)

| Card | Name | Field play | Unmeasured | Confirmed | Clauses |
|---|---|---|---|---|---|
| BT24-034 | Aegiomon | 23.6% | 4 | 0 | 4 |
| BT25-008 | Coronamon | 23.4% | 4 | 0 | 4 |
| ST23-09 | Atratusmon | 8.1% | 8 | 0 | 8 |
| BT24-101 | Jupitermon | 11.5% | 5 | 0 | 5 |
| BT25-043 | Habakirimon | 8.1% | 7 | 0 | 7 |
| BT24-041 | Minervamon | 14.0% | 4 | 0 | 4 |
| BT24-031 | Elecmon | 18.0% | 3 | 0 | 3 |
| BT24-102 | Homeros | 18.0% | 3 | 0 | 3 |
| BT24-083 | Hiroko Sagisaka | 16.6% | 3 | 0 | 3 |
| BT25-039 | Sirenmon | 9.3% | 5 | 0 | 5 |
| BT25-057 | Monarchlizamon | 7.2% | 6 | 0 | 6 |
| BT25-020 | Marsmon | 10.7% | 4 | 0 | 4 |
| BT25-022 | Lunamon | 14.0% | 3 | 0 | 3 |
| BT25-025 | Aegiochusmon: Blue | 7.0% | 6 | 0 | 6 |
| P-213 | Aegiochusmon | 7.0% | 6 | 0 | 6 |
| BT25-104 | ShineGreymon: Burst Mode | 3.5% | 12 | 0 | 12 |
| BT25-033 | Aegiomon | 9.0% | 4 | 0 | 4 |
| BT24-100 | In-Between Theater | 8.9% | 4 | 0 | 4 |
| BT24-040 | Venusmon | 8.5% | 4 | 0 | 4 |
| ST23-06 | Gekkomon | 8.1% | 4 | 0 | 4 |
| ST23-15 | e-Pulse | 8.1% | 4 | 0 | 4 |
| BT25-041 | Murasamemon | 7.2% | 4 | 0 | 4 |
| BT25-049 | Armalizamon | 7.2% | 4 | 0 | 4 |
| ST23-03 | Cougarmon | 7.2% | 4 | 0 | 4 |
| EX12-037 | Omnimon | 3.9% | 7 | 0 | 7 |

## Known gaps in the meta (PARTIAL / BLOCKED)

| Card | Name | Status | Gap kind | Field play |
|---|---|---|---|---|
| BT25-084 | Titamon | PARTIAL | engine | 9.2% |
| BT25-077 | Bacchusmon | PARTIAL | dsl | 5.1% |
| AD1-002 | Aldamon | PARTIAL | dsl | 1.9% |
| BT25-074 | Tankdramon | BLOCKED | dsl | 1.9% |
| BT25-080 | Witchmon | BLOCKED | engine | 3.7% |
| BT25-103 | GraceNovamon | BLOCKED | dsl | 1.9% |
| BT25-050 | Kiwimon | BLOCKED | engine | 1.2% |
| BT25-087 | Thomas H. Norstein | BLOCKED | engine | 1.7% |
| BT23-102 | Mastemon | PARTIAL | engine | 1.0% |

## Trend (rolling window, coverage by what was implemented at the time)

| Date | Lists | Implemented copies | Playable decks | Clauses confirmed | Pool implemented |
|---|---|---|---|---|---|
| 2026-05-27 | 170 | 40.4% | 10.6% | 0.0% | 343 |
| 2026-06-03 | 180 | 38.4% | 6.1% | 0.0% | 484 |
| 2026-06-10 | 195 | 57.9% | 5.1% | 0.0% | 580 |
| 2026-06-17 | 213 | 69.0% | 14.1% | 0.0% | 669 |
| 2026-06-24 | 225 | 69.1% | 14.7% | 0.0% | 674 |
| 2026-07-01 | 202 | 69.5% | 14.4% | 0.0% | 674 |
| 2026-07-08 | 249 | 82.6% | 47.4% | 0.0% | 820 |
| 2026-07-15 | 286 | 80.3% | 45.8% | 0.0% | 820 |
| 2026-07-22 | 369 | 79.3% | 48.0% | 0.0% | 820 |
| 2026-07-29 | 809 | 79.2% | 49.8% | 0.0% | 820 |
| 2026-08-05 | 860 | 78.8% | 49.5% | 0.0% | 820 |
| 2026-08-12 | 925 | 78.5% | 49.1% | 0.0% | 820 |
| 2026-08-19 | 905 | 78.5% | 49.4% | 0.0% | 820 |
| 2026-08-26 | 871 | 78.1% | 48.3% | 2.0% | 821 |
| 2026-09-02 | 822 | 77.3% | 45.1% | 2.1% | 821 |
| 2026-09-09 | 454 | 72.3% | 31.3% | 2.5% | 821 |

## Definitions

- **Implemented**: a YAML spec under `code/digimon-engine/cards/` declares the card (what `build.rs` compiles into the registry).
- **Tested**: implemented, referenced by a Rust test under `code/digimon-engine/tests/`, and not flagged PARTIAL/BLOCKED in `validated_cards_dsl.json`.
- **DCGO-verified**: tested, and every printed clause has an exam verdict with none `unmeasured` or `diverged` (`tools.clause_coverage` supplies the denominator).
- **Field share / copies**: each decklist is weighted so its archetype carries its DigiLab share (`data/meta_shares.json`); copies are expected copies per deck in the field.
