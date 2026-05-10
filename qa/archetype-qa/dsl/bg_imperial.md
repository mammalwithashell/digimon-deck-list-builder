# Archetype DSL Implementation: BG Imperial
Date: 2026-05-04
Total cards in pool: 25
Processed this run: 25
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 6
- PARTIAL: 17
- BLOCKED: 2

## Per-Card Verdicts
| Card ID | Name | Verdict | Gap kind | Notes |
|---------|------|---------|----------|-------|
| BT12-002 | DemiVeemon | IMPLEMENTED | - | Inherited When Attacking conditional Draw |
| BT12-021 | Veemon | IMPLEMENTED | - | OnPlay reveal-add + inherited EOYT DNA registration |
| BT12-022 | ExVeemon | PARTIAL | hybrid | Cost reduction + grant_keyword active_when both blocked |
| BT12-028 | Paildramon | BLOCKED | hybrid | All clauses gap-blocked: trash-top-N digi cards, DNA filter, source name |
| BT12-031 | Imperialdramon: Fighter Mode | PARTIAL | dsl | OPT suspend-all + return + +DP-per-color ship; alt-cost + 2+colors aura blocked |
| BT12-047 | Wormmon | IMPLEMENTED | - | Sister to BT12-021 — Wormmon line reveal-add |
| BT12-050 | Stingmon | PARTIAL | hybrid | Sister to BT12-022 — same gaps |
| BT16-025 | Paildramon | PARTIAL | dsl | Partition + DNA sub-clause + WhenAttacking ship; 2 clauses blocked |
| BT16-027 | Imperialdramon: Fighter Mode | PARTIAL | dsl | Blast Digivolve + ACE -4 ship; OnPlay/EndOfAttack blocked |
| BT16-028 | Imperialdramon: Dragon Mode | PARTIAL | hybrid | CannotUnsuspend ships; alt-cost branch + effect-initiated trigger blocked |
| BT16-040 | Wormmon | PARTIAL | dsl | Effect-initiated digivolve from trash chain blocked |
| BT16-085 | Davis Motomiya & Ken Ichijoji | PARTIAL | engine | Free-play ships; delayed return + DNA trash blocked (3 new gaps) |
| BT17-077 | Imperialdramon: Paladin Mode | PARTIAL | hybrid | Blast Digivolve + ACE -5 + WhenAttacking ship; Track E return-all-trash verb landed; OnPlay all-source trash / player-choice / returned-card rider still blocked |
| BT17-097 | Return to the Primogenitor | PARTIAL | dsl | Replaced fixture; Delay replacement + Security ship; effect-initiated digivolve blocked |
| BT20-020 | Imperialdramon: Fighter Mode | PARTIAL | dsl | Raid + Piercing + WhenDigivolving ship; security-loss DP-LTE-source blocked |
| BT21-037 | Lighdramon | PARTIAL | engine | G-DECLARATIVE-KEYWORD blocks runtime keyword install |
| BT3-002 | DemiVeemon | PARTIAL | dsl | carrier_has_keyword predicate gap; over-fires |
| BT3-093 | Davis Motomiya | IMPLEMENTED | - | Start-of-turn memory swing + reveal-add + Security free-play |
| BT3-103 | Hidden Potential Discovered! | PARTIAL | hybrid | Security add-to-hand ships; Main cost reduction blocked |
| EX1-014 | ExVeemon | PARTIAL | dsl | Aura over-targets; trait arm of active_when blocked |
| LM-030 | Green Scramble | PARTIAL | hybrid | Main + Security ship; Delay clause blocked (4 known gaps) |
| P-117 | Veemon | PARTIAL | hybrid | Cost reduction omitted; color-count condition omitted |
| ST9-05 | Paildramon | IMPLEMENTED | - | on_dna_digivolve return-to-deck-bottom + WhenAttacking unsuspend OPT |
| ST9-06 | Imperialdramon Dragon Mode | BLOCKED | dsl | G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES (whole card) |
| ST9-09 | Stingmon | IMPLEMENTED | - | Cost reduction on play + inherited When Attacking Draw |

## New Engine + DSL Gaps Filed in This Run
Cumulative across all batches; see `qa/dsl-vocab-gaps.md` and `qa/archetype-qa/engine-gaps.md` for details.

**DSL gaps (new):**
- G-DSL-CARRIER-HAS-KEYWORD (BT3-002)
- G-DSL-SELF-COLOR-COUNT-GTE (P-117)
- G-BEFORE-PAY-COST-GAIN-MEMORY (BT12-022, BT12-050)
- G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED (BT12-022, BT12-050)
- G-DSL-AURA-TARGET-SOURCE-PERMANENT (EX1-014)
- G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT (EX1-014)
- G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET (BT16-040, LM-030)
- G-PRED-STACK-SIZE-LTE-SOURCE (BT16-027, BT16-025)
- G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME (BT16-027, BT12-028)
- G-DSL-TRASH-TOP-N-DIGI-CARDS (BT12-028) — closed 2026-05-09 as `trash_top_n_digivolution_cards_of_each`; BT17-077 still needs all-source mass trash under G-ASL-07.
- G-DSL-IF-NO-TARGET (BT16-025, BT16-028)
- G-IS-EFFECT-INITIATED (BT16-028)
- G-FORMULA-SOURCE-DP (BT20-020)
- G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME (BT12-031)
- G-RETURN-ALL-TRASH-TO-DECK-BOTTOM (BT17-077) — partially closed 2026-05-09 as `return_all_trash_to_deck_bottom: { of }`; BT17-077 still needs the printed "your or your opponent's trash" player-choice branch.
- G-ANY-RETURNED-CARD-PREDICATE (BT17-077)
- G-COST-REDUCE-ALLY-DIGIVOLVE (BT3-103)
- G-COST-REDUCE-NEXT-SINGLE-FIRE (BT3-103)
- G-PAY-COST-SELECT-ARBITRARY-SUSPEND (BT3-103)
- G-PLAY-FROM-HAND-FREE-BIND-AS (BT16-085)
- G-EVENT-CARD-COLOR-IS (BT16-085)
- G-SELECT-OPPONENT-SOURCES (BT16-085)

**Engine gaps (new):**
- G-OPT-RESET-VIA-ATTACK-CYCLE (BT16-040)

**Pre-existing gaps cross-listed for cards in this run:**
- G-BEFORE-PAY-COST-DIGIVOLVE-TARGET (BT23-005 origin → P-117, BT12-022, BT12-050)
- G-DECLARATIVE-KEYWORD (Phase 3 → BT21-037)
- G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES (EX4-060/BT22-015 → ST9-06)
- G-DSL-IS-DNA-DIGIVOLVING (DNA Omnimon → BT12-028, BT16-085) — RESOLVED 2026-05-08 as `dna_origin: true`; BT16-085 still needs opponent-source selection for its DNA rider
- G-ASL-07 (BT17-077 all-source mass trash; bounded top-N sibling closed by Track E)
- G-DELAY-START-OF-TURN, G-PRED-DP-LTE, G-ZONE-TRASH-TO-DECK, G-OPTIONAL-SELECTION-CONTINUE-TAIL (LM-027/LM-029 → LM-030)

## Test Coverage Totals
- Total tests authored: ~250 across 25 card files
- Passing: 249
- Ignored (gap-blocked): 77
- Failing: 0
