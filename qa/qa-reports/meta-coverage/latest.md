# Meta coverage

**Generated — do not hand-edit.** Regenerate with `PYTHONPATH=code python -m tools.meta_coverage`.

- Generated: 2026-10-03T23:38:17+00:00 (git `6b38095f`)
- Window: **BT26** — 2026-09-04 → 2026-09-12 · 108 decklists · 650 distinct cards · sources dcg_nexus 108
- Weighting: **digilab** — DigiLab names 67.05% of the field explicitly; the other 32.95% is spread over unnamed archetypes by list count
- ⚠️ Only 108 decklists fall in this window (< 150); card-level figures are a small sample. Refresh data/deck_library.json (meta_loader.py --scrape-dcg-nexus) and re-run.

## Headline

| Measure | Implemented | Tested | DCGO-verified |
|---|---|---|---|
| Share of meta card copies | 92.4% | 92.2% | 13.1% |
| Distinct meta cards | 524/650 (80.6%) | 521/650 (80.2%) | 50/650 (7.7%) |
| Field share of decks with every card at this level | 85.5% (89 lists) | 85.5% (89 lists) | 0.0% (0 lists) |

**Trainable today**: 85.5% of the field (89 lists) — fully playable AND no card flagged PARTIAL, BLOCKED, AUDITED-DRIFT or AUDITED-MISSING-TESTS in `validated_cards_dsl.json`: the training deck pool's per-list gate (`digimon_gym.agents.gauntlet`).

Clauses on meta cards: 2458 — confirmed 242, diverged 2, unreachable 13, unavailable 0, unmeasured 2201 (9.8% confirmed; 15.7% weighted by field play rate).

Whole pool: 1040/4472 cards implemented (23.3%), 15 flagged PARTIAL/BLOCKED, 100 DCGO-verified.
Deck-builder allowlist (`data/tested_cards.json`) lags the engine by 21 implemented card(s) — regenerate with `python code/tools/build_tested_cards.py`.

## Launch plan

Goal: Optimize Digimon TCG decklists for the live meta (as DigiLab and DigimonMeta measure it) by training agents on a simulator that plays that meta faithfully. (plan updated 2026-10-03; edit `qa/qa-reports/meta-coverage/launch_plan.json`)

- **M0 · Measure the gap** (done) — 2/2 gates met
  - [x] Meta coverage report + dashboard regenerate from repo data
  - [x] BT26 field shares transcribed from DigiLab (data/meta_shares.json)
- **M1 · Make the BT26 field playable** (active) — 2/3 gates met
  - [x] Field share of fully playable decks: 85.53 (target >= 75 %)
  - [x] BT26 cards seen in meta lists that are implemented: 74 (target >= 60 cards)
  - [ ] Deck library holds at least 300 BT26-window lists: 108 (target >= 300 lists)
- **M2 · Let training see the meta** (next) — 1/2 gates met
  - [x] Playable decks the training pool rejects (field share): 0 (target <= 1 %)
  - [ ] Implemented cards missing from the deck-builder allowlist: 21 (target <= 0 cards)
- **M3 · Verify what the field plays** (next) — 0/2 gates met
  - [ ] Meta card copies that are DCGO-verified: 13.13 (target >= 40 %)
  - [ ] Clauses confirmed, weighted by how often the field plays each card: 15.74 (target >= 50 %)
- **M4 · Meta training pilot** (later) — 1/3 gates met
  - [x] Field share the training pool can use: 85.53 (target >= 60 %)
  - [ ] 2026-07-02 anchored baselines re-measured on the current engine
  - [ ] A BT26 generalist clears 55% anchored vs greedy and the frozen champions
- **M5 · Deck optimization loop** (later) — 0/2 gates met
  - [ ] Architect agent ranks list variants against a BT26 gauntlet
  - [ ] Training runs (training_run.json, anchored_evals.jsonl) charted on this dashboard

**Unlock path** (cards to implement, in `implement_next` order, until that much of the field is fully playable): 25% → 0 cards, 50% → 0 cards, 75% → 0 cards, 90% → 28 cards, 100% → 126 cards. 126 missing cards in total.

## By set

| Set | Share of meta copies | Meta cards implemented | Implemented copies | Whole set implemented |
|---|---|---|---|---|
| BT26 | 21.2% | 74/74 | 100.0% | 77/104 |
| BT25 | 17.0% | 84/85 | 99.2% | 100/104 |
| BT24 | 16.8% | 47/47 | 100.0% | 59/102 |
| EX12 | 10.4% | 54/54 | 100.0% | 54/77 |
| ST23 | 4.6% | 15/15 | 100.0% | 15/15 |
| BT21 | 3.9% | 33/45 | 79.2% | 50/102 |
| ST24 | 3.1% | 15/15 | 100.0% | 15/15 |
| P | 2.5% | 28/33 | 93.4% | 51/240 |
| AD1 | 1.8% | 10/13 | 87.1% | 17/25 |
| EX9 | 1.7% | 10/23 | 57.5% | 14/74 |
| BT20 | 1.6% | 6/19 | 25.3% | 18/102 |
| LM | 1.4% | 17/20 | 88.4% | 21/68 |
| BT19 | 1.4% | 17/19 | 90.8% | 26/102 |
| EX7 | 1.0% | 8/8 | 100.0% | 26/74 |

## Archetypes

| Archetype | Field share | Lists | Implemented | Tested | Verified | Clauses confirmed | Playable lists | Trainable lists | Closest list missing |
|---|---|---|---|---|---|---|---|---|---|
| TS Jupitermon | 10.5% | 12 | 99.8% | 99.8% | 4.2% | 7.1% | 11/12 | 11/12 | 0 |
| Glowing Dawn | 8.1% | 9 | 100.0% | 100.0% | 0.0% | 0.0% | 9/9 | 9/9 | 0 |
| Toho Braves | 6.8% | 7 | 100.0% | 100.0% | 77.2% | 71.6% | 7/7 | 7/7 | 0 |
| Titans | 6.4% | 7 | 100.0% | 100.0% | 6.6% | 9.8% | 7/7 | 7/7 | 0 |
| Data Squad | 5.8% | 10 | 100.0% | 100.0% | 0.7% | 0.9% | 10/10 | 10/10 | 0 |
| Chronomon | 5.7% | 4 | 100.0% | 100.0% | 7.4% | 13.1% | 4/4 | 4/4 | 0 |
| Plutomon | 5.5% | 2 | 100.0% | 100.0% | 13.0% | 9.7% | 2/2 | 2/2 | 0 |
| TS Mervamon | 4.8% | 2 | 100.0% | 100.0% | 5.6% | 9.9% | 2/2 | 2/2 | 0 |
| Three Musketeers | 4.2% | 3 | 100.0% | 100.0% | 94.4% | 93.2% | 3/3 | 3/3 | 0 |
| Apps Dantemon | 3.2% | 3 | 100.0% | 100.0% | 4.3% | 5.2% | 3/3 | 3/3 | 0 |
| Omni Ladder | 2.9% | 3 | 100.0% | 100.0% | 1.9% | 3.8% | 3/3 | 3/3 | 0 |
| TS Toolbox | 2.9% | 3 | 100.0% | 100.0% | 2.5% | 4.0% | 3/3 | 3/3 | 0 |
| Accel | 1.9% | 2 | 38.0% | 38.0% | 0.0% | 0.0% | 0/2 | 0/2 | 11 (BT20-004, BT20-030, BT20-031, BT20-033, BT20-038, BT20-039…) |
| Gammamon | 1.9% | 2 | 36.1% | 36.1% | 0.9% | 1.9% | 0/2 | 0/2 | 12 (BT21-002, BT21-077, BT21-080, BT21-090, BT22-045, EX10-042…) |
| Green TyrantKabuterimon | 1.9% | 2 | 50.9% | 50.9% | 0.0% | 0.0% | 0/2 | 0/2 | 7 (BT15-004, BT15-043, BT15-085, BT16-042, BT16-045, BT16-048…) |
| Red Hybrid (AncientGreymon) | 1.9% | 2 | 43.1% | 35.8% | 3.7% | 8.6% | 0/2 | 0/2 | 12 (AD1-020, BT12-088, BT17-012, BT17-014, BT18-011, BT18-022…) |
| Styracomon | 1.9% | 2 | 100.0% | 100.0% | 4.6% | 15.5% | 2/2 | 2/2 | 0 |
| TS Cosmic Area | 1.9% | 2 | 100.0% | 100.0% | 13.0% | 14.1% | 2/2 | 2/2 | 0 |
| TS Vulcanusmon | 1.9% | 2 | 100.0% | 100.0% | 15.7% | 28.1% | 2/2 | 2/2 | 0 |
| Virus Busters | 1.9% | 2 | 100.0% | 100.0% | 0.0% | 0.0% | 2/2 | 2/2 | 0 |
| Xros | 1.9% | 2 | 100.0% | 100.0% | 0.9% | 1.8% | 2/2 | 2/2 | 0 |
| Saiyu Warriors | 1.8% | 4 | 100.0% | 100.0% | 7.4% | 36.0% | 4/4 | 4/4 | 0 |
| ShineGreymon | 1.7% | 3 | 100.0% | 100.0% | 4.3% | 5.4% | 3/3 | 3/3 | 0 |
| TS Ceresmon | 1.4% | 7 | 81.0% | 81.0% | 1.6% | 2.1% | 2/7 | 2/7 | 0 |
| Apps Reboot | 1.1% | 1 | 100.0% | 100.0% | 3.7% | 6.2% | 1/1 | 1/1 | 0 |

## Implement next

Ordered to make the most field share fully playable soonest (see `unlock_order`).

| # | Card | Name | Kind | Copies/deck | Field play | Clauses | Cumulative playable | Played in |
|---|---|---|---|---|---|---|---|---|
| 1 | BT9-083 | Omnimon: Merciful Mode | Digimon | 0.01 | 0.9% | 3 | 86.4% | TS Jupitermon 100% |
| 2 | AD1-011 | Paildramon | Digimon | 0.04 | 1.0% | 6 | 86.4% | Blue Green Imperialdramon 100% |
| 3 | BT12-030 | Imperialdramon: Dragon Mode | Digimon | 0.01 | 1.0% | 3 | 87.4% | Blue Green Imperialdramon 100% |
| 4 | BT15-004 | Motimon | Digi-Egg | 0.08 | 1.9% | 1 | 87.4% | Green TyrantKabuterimon 100% |
| 5 | BT15-043 | Tentomon | Digimon | 0.07 | 1.9% | 2 | 87.4% | Green TyrantKabuterimon 100% |
| 6 | BT15-085 | Izzy Izumi | Tamer | 0.06 | 1.9% | 3 | 87.4% | Green TyrantKabuterimon 100% |
| 7 | BT16-042 | BladeKuwagamon | Digimon | 0.07 | 1.9% | 2 | 87.4% | Green TyrantKabuterimon 100% |
| 8 | BT16-045 | MetallifeKuwagamon | Digimon | 0.07 | 1.9% | 2 | 87.4% | Green TyrantKabuterimon 100% |
| 9 | BT16-048 | TyrantKabuterimon | Digimon | 0.08 | 1.9% | 4 | 87.4% | Green TyrantKabuterimon 100% |
| 10 | EX8-039 | Tentomon | Digimon | 0.08 | 1.9% | 3 | 88.3% | Green TyrantKabuterimon 100% |
| 11 | P-140 | MegaKabuterimon | Digimon | 0.02 | 1.0% | 4 | 89.3% | Green TyrantKabuterimon 100% |
| 12 | BT9-047 | Pomumon | Digimon | 0.01 | 0.4% | 1 | 89.5% | TS Ceresmon 100% |
| 13 | BT10-048 | Sunflowmon | Digimon | 0.03 | 0.8% | 2 | 89.5% | TS Ceresmon 100% |
| 14 | BT14-046 | Togemon | Digimon | 0.03 | 0.8% | 2 | 89.5% | TS Ceresmon 100% |
| 15 | EX4-002 | Kokomon | Digi-Egg | 0.03 | 0.8% | 1 | 89.7% | TS Ceresmon 100% |
| 16 | BT10-053 | Ajatarmon | Digimon | 0.02 | 0.6% | 2 | 89.7% | TS Ceresmon 100% |
| 17 | P-038 | Green Memory Boost! | Option | 0.00 | 0.2% | 3 | 89.9% | TS Ceresmon 100% |
| 18 | BT20-004 | Pinamon | Digi-Egg | 0.08 | 1.9% | 1 | 89.9% | Accel 100% |
| 19 | BT20-030 | Liollmon | Digimon | 0.08 | 1.9% | 3 | 89.9% | Accel 100% |
| 20 | BT20-031 | Liamon | Digimon | 0.05 | 1.9% | 3 | 89.9% | Accel 100% |
| 21 | BT20-033 | LoaderLeomon | Digimon | 0.05 | 1.9% | 3 | 89.9% | Accel 100% |
| 22 | BT20-038 | Falcomon | Digimon | 0.06 | 1.9% | 3 | 89.9% | Accel 100% |
| 23 | BT20-039 | Diatrymon | Digimon | 0.05 | 1.9% | 3 | 89.9% | Accel 100% |
| 24 | BT20-041 | Crowmon | Digimon | 0.04 | 1.9% | 3 | 89.9% | Accel 100% |
| 25 | BT20-043 | Varodurumon | Digimon | 0.05 | 1.9% | 4 | 89.9% | Accel 100% |
| 26 | BT20-099 | Singularity of Chaos | Option | 0.07 | 1.9% | 4 | 89.9% | Accel 100% |
| 27 | BT25-074 | Tankdramon | Digimon | 0.08 | 1.9% | 4 | 89.9% | Accel 100% |
| 28 | P-221 | Chaosmon | Digimon | 0.03 | 1.9% | 5 | 90.9% | Accel 100% |
| 29 | BT16-036 | Chaosmon | Digimon | 0.01 | 1.0% | 6 | 90.9% | Accel 100% |
| 30 | LM-043 | Darkdramon | Digimon | 0.02 | 1.0% | 8 | 91.8% | Accel 100% |

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
| BT24-026 | Hyogamon | 11.9% | 4 | 0 | 4 |
| BT24-045 | Ogremon | 11.9% | 4 | 0 | 4 |
| BT26-100 | Dark Field | 11.9% | 4 | 0 | 4 |

## Known gaps in the meta (PARTIAL / BLOCKED)

| Card | Name | Status | Gap kind | Field play |
|---|---|---|---|---|
| AD1-002 | Aldamon | PARTIAL | dsl | 1.9% |
| BT25-074 | Tankdramon | BLOCKED | dsl | 1.9% |
| BT23-102 | Mastemon | PARTIAL | engine | 1.0% |

## Definitions

- **Implemented**: a YAML spec under `code/digimon-engine/cards/` declares the card (what `build.rs` compiles into the registry).
- **Tested**: implemented, referenced by a Rust test under `code/digimon-engine/tests/`, and not flagged PARTIAL/BLOCKED in `validated_cards_dsl.json`.
- **DCGO-verified**: tested, and every printed clause has an exam verdict with none `unmeasured` or `diverged` (`tools.clause_coverage` supplies the denominator).
- **Field share / copies**: each decklist is weighted so its archetype carries its DigiLab share (`data/meta_shares.json`); copies are expected copies per deck in the field.
