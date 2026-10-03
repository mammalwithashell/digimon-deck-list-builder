# Meta coverage

**Generated — do not hand-edit.** Regenerate with `PYTHONPATH=code python -m tools.meta_coverage`.

- Generated: 2026-10-03T19:03:13+00:00 (git `531ac39fe`)
- Window: **BT26** — 2026-09-04 → 2026-09-12 · 108 decklists · 650 distinct cards · sources dcg_nexus 108
- Weighting: **digilab** — DigiLab names 67.05% of the field explicitly; the other 32.95% is spread over unnamed archetypes by list count
- ⚠️ Only 108 decklists fall in this window (< 150); card-level figures are a small sample. Refresh data/deck_library.json (meta_loader.py --scrape-dcg-nexus) and re-run.

## Headline

| Measure | Implemented | Tested | DCGO-verified |
|---|---|---|---|
| Share of meta card copies | 86.0% | 85.0% | 13.1% |
| Distinct meta cards | 488/650 (75.1%) | 481/650 (74.0%) | 50/650 (7.7%) |
| Field share of decks with every card at this level | 57.7% (62 lists) | 52.5% (54 lists) | 0.0% (0 lists) |

**Trainable today**: 25.4% of the field (22 lists) — fully playable AND admitted by the training deck pool's archetype gate (`gauntlet._load_fully_implemented_archetypes`, which matches free-form ledger labels).

Clauses on meta cards: 2458 — confirmed 242, diverged 2, unreachable 13, unavailable 0, unmeasured 2201 (9.8% confirmed; 15.7% weighted by field play rate).

Whole pool: 1004/4472 cards implemented (22.4%), 19 flagged PARTIAL/BLOCKED, 100 DCGO-verified.
Deck-builder allowlist (`data/tested_cards.json`) lags the engine by 77 implemented card(s) — regenerate with `python code/tools/build_tested_cards.py`.

## Launch plan

Goal: Optimize Digimon TCG decklists for the live meta (as DigiLab and DigimonMeta measure it) by training agents on a simulator that plays that meta faithfully. (plan updated 2026-10-03; edit `qa/qa-reports/meta-coverage/launch_plan.json`)

- **M0 · Measure the gap** (done) — 2/2 gates met
  - [x] Meta coverage report + dashboard regenerate from repo data
  - [x] BT26 field shares transcribed from DigiLab (data/meta_shares.json)
- **M1 · Make the BT26 field playable** (active) — 1/3 gates met
  - [ ] Field share of fully playable decks: 57.72 (target >= 75 %)
  - [x] BT26 cards seen in meta lists that are implemented: 74 (target >= 60 cards)
  - [ ] Deck library holds at least 300 BT26-window lists: 108 (target >= 300 lists)
- **M2 · Let training see the meta** (next) — 0/2 gates met
  - [ ] Playable decks the training pool rejects (field share): 32.32 (target <= 1 %)
  - [ ] Implemented cards missing from the deck-builder allowlist: 77 (target <= 0 cards)
- **M3 · Verify what the field plays** (next) — 0/2 gates met
  - [ ] Meta card copies that are DCGO-verified: 13.13 (target >= 40 %)
  - [ ] Clauses confirmed, weighted by how often the field plays each card: 15.74 (target >= 50 %)
- **M4 · Meta training pilot** (later) — 0/3 gates met
  - [ ] Field share the training pool can use: 25.4 (target >= 60 %)
  - [ ] 2026-07-02 anchored baselines re-measured on the current engine
  - [ ] A BT26 generalist clears 55% anchored vs greedy and the frozen champions
- **M5 · Deck optimization loop** (later) — 0/2 gates met
  - [ ] Architect agent ranks list variants against a BT26 gauntlet
  - [ ] Training runs (training_run.json, anchored_evals.jsonl) charted on this dashboard

**Unlock path** (cards to implement, in `implement_next` order, until that much of the field is fully playable): 25% → 0 cards, 50% → 0 cards, 75% → 19 cards, 90% → 64 cards, 100% → 162 cards. 162 missing cards in total.

## By set

| Set | Share of meta copies | Meta cards implemented | Implemented copies | Whole set implemented |
|---|---|---|---|---|
| BT26 | 21.2% | 74/74 | 100.0% | 77/104 |
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
| TS Jupitermon | 10.5% | 12 | 99.8% | 99.1% | 4.2% | 7.1% | 11/12 | fail | 0 |
| Glowing Dawn | 8.1% | 9 | 100.0% | 100.0% | 0.0% | 0.0% | 9/9 | pass | 0 |
| Toho Braves | 6.8% | 7 | 99.2% | 99.2% | 77.2% | 71.6% | 6/7 | pass | 0 |
| Titans | 6.4% | 7 | 53.7% | 48.9% | 6.6% | 9.8% | 0/7 | fail | 8 (BT24-009, BT24-021, BT24-023, BT24-026, BT24-045, BT24-075…) |
| Data Squad | 5.8% | 10 | 98.9% | 98.2% | 0.7% | 0.9% | 8/10 | fail | 0 |
| Chronomon | 5.7% | 4 | 100.0% | 100.0% | 7.4% | 13.1% | 4/4 | pass | 0 |
| Plutomon | 5.5% | 2 | 60.2% | 59.3% | 13.0% | 9.7% | 0/2 | fail | 7 (BT24-009, BT24-021, BT24-023, BT24-026, BT24-042, BT24-045…) |
| TS Mervamon | 4.8% | 2 | 100.0% | 100.0% | 5.6% | 9.9% | 2/2 | pass | 0 |
| Three Musketeers | 4.2% | 3 | 100.0% | 100.0% | 94.4% | 93.2% | 3/3 | fail | 0 |
| Apps Dantemon | 3.2% | 3 | 93.8% | 93.8% | 4.3% | 5.2% | 0/3 | pass | 1 (BT24-099) |
| Omni Ladder | 2.9% | 3 | 100.0% | 100.0% | 1.9% | 3.8% | 3/3 | fail | 0 |
| TS Toolbox | 2.9% | 3 | 100.0% | 95.7% | 2.5% | 4.0% | 3/3 | fail | 0 |
| Accel | 1.9% | 2 | 38.0% | 38.0% | 0.0% | 0.0% | 0/2 | fail | 11 (BT20-004, BT20-030, BT20-031, BT20-033, BT20-038, BT20-039…) |
| Gammamon | 1.9% | 2 | 29.6% | 29.6% | 0.9% | 1.9% | 0/2 | fail | 13 (BT21-002, BT21-077, BT21-080, BT21-090, BT22-045, EX10-042…) |
| Green TyrantKabuterimon | 1.9% | 2 | 50.9% | 50.9% | 0.0% | 0.0% | 0/2 | pass | 7 (BT15-004, BT15-043, BT15-085, BT16-042, BT16-045, BT16-048…) |
| Red Hybrid (AncientGreymon) | 1.9% | 2 | 43.1% | 35.8% | 3.7% | 8.6% | 0/2 | pass | 12 (AD1-020, BT12-088, BT17-012, BT17-014, BT18-011, BT18-022…) |
| Styracomon | 1.9% | 2 | 97.2% | 97.2% | 4.6% | 15.5% | 1/2 | fail | 0 |
| TS Cosmic Area | 1.9% | 2 | 93.5% | 93.5% | 13.0% | 14.1% | 0/2 | fail | 1 (BT25-103) |
| TS Vulcanusmon | 1.9% | 2 | 98.2% | 98.2% | 15.7% | 28.1% | 1/2 | fail | 0 |
| Virus Busters | 1.9% | 2 | 98.2% | 98.2% | 0.0% | 0.0% | 1/2 | fail | 0 |
| Xros | 1.9% | 2 | 94.4% | 94.4% | 0.9% | 1.8% | 1/2 | fail | 0 |
| Saiyu Warriors | 1.8% | 4 | 100.0% | 100.0% | 7.4% | 36.0% | 4/4 | fail | 0 |
| ShineGreymon | 1.7% | 3 | 93.2% | 93.2% | 4.3% | 5.4% | 0/3 | fail | 1 (LM-059) |
| TS Ceresmon | 1.4% | 7 | 78.6% | 67.5% | 1.6% | 2.1% | 2/7 | fail | 0 |
| Apps Reboot | 1.1% | 1 | 96.3% | 96.3% | 3.7% | 6.2% | 0/1 | fail | 1 (EX1-072) |

## Implement next

Ordered to make the most field share fully playable soonest (see `unlock_order`).

| # | Card | Name | Kind | Copies/deck | Field play | Clauses | Cumulative playable | Played in |
|---|---|---|---|---|---|---|---|---|
| 1 | EX10-070 | God Grade Unleashed | Option | 0.06 | 2.1% | 5 | 59.9% | Apps Dantemon 100% |
| 2 | LM-059 | Heat Training | Option | 0.06 | 2.7% | 0 | 61.0% | ShineGreymon 64%, Styracomon 36% |
| 3 | BT24-021 | SnowGoblimon | Digimon | 0.44 | 11.9% | 3 | 61.0% | Titans 54%, Plutomon 46% |
| 4 | BT24-026 | Hyogamon | Digimon | 0.37 | 11.9% | 4 | 61.0% | Titans 54%, Plutomon 46% |
| 5 | BT24-045 | Ogremon | Digimon | 0.40 | 11.9% | 4 | 61.0% | Titans 54%, Plutomon 46% |
| 6 | BT24-009 | Shamanmon | Digimon | 0.39 | 11.0% | 3 | 61.0% | Plutomon 50%, Titans 50% |
| 7 | BT24-042 | Goblimon | Digimon | 0.30 | 11.0% | 3 | 61.0% | Plutomon 50%, Titans 50% |
| 8 | BT24-023 | Calmaramon | Digimon | 0.16 | 9.2% | 5 | 61.0% | Plutomon 60%, Titans 40% |
| 9 | BT24-072 | SkullGreymon | Digimon | 0.09 | 7.3% | 4 | 66.5% | Plutomon 75%, Titans 25% |
| 10 | BT24-075 | SkullBaluchimon | Digimon | 0.21 | 6.4% | 3 | 66.5% | Titans 100% |
| 11 | BT25-103 | GraceNovamon | Digimon | 0.04 | 1.9% | 7 | 67.5% | TS Cosmic Area 100% |
| 12 | BT24-013 | Fugamon | Digimon | 0.07 | 2.7% | 4 | 68.4% | Titans 100% |
| 13 | BT24-098 | Invasion of the Titans | Option | 0.10 | 4.6% | 4 | 68.4% | Titans 100% |
| 14 | BT25-080 | Witchmon | Digimon | 0.06 | 3.7% | 3 | 68.4% | Titans 100% |
| 15 | BT4-086 | Cerberusmon: Werewolf Mode | Digimon | 0.07 | 3.7% | 2 | 71.2% | Titans 100% |
| 16 | BT24-007 | Tsunomon | Digi-Egg | 0.11 | 2.7% | 1 | 72.1% | Titans 100% |
| 17 | BT2-041 | ShineGreymon | Digimon | 0.04 | 1.1% | 2 | 73.2% | Data Squad 50%, ShineGreymon 50% |
| 18 | P-240 | Arcturusmon | Digimon | 0.09 | 2.9% | 0 | 74.2% | Gammamon 67%, Virus Busters 33% |
| 19 | BT24-099 | Super Hacking | Option | 0.04 | 1.1% | 4 | 75.3% | Apps Dantemon 100% |
| 20 | EX1-072 | Emergency Program Shutdown! | Option | 0.02 | 1.1% | 2 | 76.3% | Apps Reboot 100% |
| 21 | P-200 | Kanan Yuki | Tamer | 0.03 | 1.2% | 3 | 77.3% | TS Cosmic Area 83%, TS Ceresmon 17% |
| 22 | BT8-108 | Mist Memory Boost! | Option | 0.03 | 1.0% | 3 | 78.3% | Toho Braves 100% |
| 23 | BT24-033 | Salamon | Digimon | 0.01 | 1.0% | 3 | 79.2% | TS Angels 100% |
| 24 | P-210 | Hiroko Sagisaka | Tamer | 0.02 | 1.0% | 3 | 80.2% | TS Vulcanusmon 100% |
| 25 | BT4-111 | Jack Raid | Option | 0.01 | 0.9% | 2 | 81.1% | Titans 100% |
| 26 | P-209 | Titamon | Digimon | 0.01 | 0.9% | 4 | 82.0% | Titans 100% |
| 27 | BT9-083 | Omnimon: Merciful Mode | Digimon | 0.01 | 0.9% | 3 | 82.9% | TS Jupitermon 100% |
| 28 | LM-060 | Shadow Training | Option | 0.02 | 1.4% | 0 | 83.5% | TS Ceresmon 58%, Data Squad 42% |
| 29 | LM-057 | Wall Training | Option | 0.01 | 1.0% | 0 | 83.5% | Styracomon 100% |
| 30 | LM-061 | Punching Training | Option | 0.01 | 1.0% | 0 | 84.5% | Styracomon 100% |

## Exam next (implemented, clauses unmeasured)

| Card | Name | Field play | Unmeasured | Confirmed | Clauses |
|---|---|---|---|---|---|
| BT26-079 | ZombiePlutomon | 11.0% | 10 | 0 | 10 |
| BT26-033 | Jupitermon | 10.1% | 10 | 0 | 10 |
| BT26-073 | Aegiochusmon: Dark | 19.3% | 5 | 0 | 5 |
| BT24-034 | Aegiomon | 23.6% | 4 | 0 | 4 |
| BT25-008 | Coronamon | 23.4% | 4 | 0 | 4 |
| BT26-081 | Mervamon | 14.9% | 6 | 0 | 6 |
| BT26-056 | Cerberusmon: Werewolf Mode | 9.2% | 9 | 0 | 9 |
| BT26-029 | Aegiochusmon: Holy | 12.2% | 6 | 0 | 6 |
| BT26-060 | Chronomon: Destroy Mode | 8.4% | 8 | 0 | 8 |
| BT26-078 | Cherubimon | 16.2% | 4 | 0 | 4 |
| ST23-09 | Atratusmon | 8.1% | 8 | 0 | 8 |
| BT26-092 | Shota Kuroi | 20.6% | 3 | 0 | 3 |
| BT26-016 | Chronomon: Holy Mode | 10.2% | 6 | 0 | 6 |
| BT26-059 | Plutomon | 11.9% | 5 | 0 | 5 |
| BT24-101 | Jupitermon | 11.5% | 5 | 0 | 5 |
| BT25-043 | Habakirimon | 8.1% | 7 | 0 | 7 |
| BT24-041 | Minervamon | 14.0% | 4 | 0 | 4 |
| BT24-031 | Elecmon | 18.0% | 3 | 0 | 3 |
| BT24-102 | Homeros | 18.0% | 3 | 0 | 3 |
| BT26-085 | Giant Slayer | 7.5% | 7 | 0 | 7 |
| BT24-083 | Hiroko Sagisaka | 16.6% | 3 | 0 | 3 |
| BT26-103 | Jupitermon: Wrath Mode | 6.1% | 8 | 0 | 8 |
| BT26-100 | Dark Field | 11.9% | 4 | 0 | 4 |
| BT25-039 | Sirenmon | 9.3% | 5 | 0 | 5 |
| BT26-083 | Junomon: Hysteric Mode | 5.0% | 9 | 0 | 9 |

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
