# Archetype DSL Implementation: BG Imperial

> **Tracker hygiene sweep — 2026-05-10:** Cross-referenced against PRs
> #449–#458. Track E DSL verbs landed (PR #454) so `raw_rust` carve-outs
> for the ten zone-movement verbs in `qa/dsl-vocab-gaps.md` are now
> expressible in YAML. Track C deferred modifier variants landed (PR
> #455) with typed `ModifierPayload`; identity overlays / DigiXros
> aliases / Security Attack / EndTurn min memory / Link cost+max are
> wired but a structured DSL payload schema is still pending. Track G
> keyword library closed (PR #457) — Evade printed-semantics fix,
> Decoy color-filter via `Keyword::Decoy(u8)`, Progress card-shape
> backfill. `Expiry::UntilCondition` runtime controller landed (PR
> #458). For the canonical engine-side closures consult
> [docs/RUST_ENGINE_GAPS.md](../../../docs/RUST_ENGINE_GAPS.md);
> per-archetype `raw_rust` carve-out audit lives in
> [qa/dsl-vocab-gaps.md](../../dsl-vocab-gaps.md). See
> `.claude/plans/pre-scaling-cleanup-batch.md` §2 for the closure-
> index narrative.

Date: 2026-05-04
Total cards in pool: 25
Processed this run: 25
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 12
- PARTIAL: 13
- BLOCKED: 0

## Per-Card Verdicts
| Card ID | Name | Verdict | Gap kind | Notes |
|---------|------|---------|----------|-------|
| BT12-002 | DemiVeemon | IMPLEMENTED | - | Inherited When Attacking conditional Draw |
| BT12-021 | Veemon | IMPLEMENTED | - | OnPlay reveal-add + inherited EOYT DNA registration |
| BT12-022 | ExVeemon | PARTIAL | hybrid | Inherited conditional Jamming self-aura implemented; DNA pre-cost gain-memory still blocked |
| BT12-028 | Paildramon | IMPLEMENTED | - | When Digivolving source trash + DNA CannotAttack rider + inherited End of Attack memory |
| BT12-031 | Imperialdramon: Fighter Mode | PARTIAL | dsl | OPT suspend-all + return + +DP-per-color ship; source-ref return-to-hand alt-cost + 2+colors aura blocked |
| BT12-047 | Wormmon | IMPLEMENTED | - | Sister to BT12-021 — Wormmon line reveal-add |
| BT12-050 | Stingmon | PARTIAL | hybrid | Inherited conditional Piercing self-aura implemented; DNA pre-cost gain-memory still blocked |
| BT16-025 | Paildramon | PARTIAL | dsl | Partition + DNA sub-clause + WhenAttacking ship; source-relative stack comparison + suspend-result fallback blocked |
| BT16-027 | Imperialdramon: Fighter Mode | PARTIAL | dsl | Blast Digivolve + ACE -4 + End of Attack ship; source-relative stack-size OnPlay/WD target blocked |
| BT16-028 | Imperialdramon: Dragon Mode | PARTIAL | hybrid | CannotUnsuspend + suspend-cost/unsuspend branch + effect-play free digivolve ship; remaining ignored tests are test-side/effect-digivolve observer coverage |
| BT16-040 | Wormmon | PARTIAL | dsl | Minomon cost-0 path and trash-source field spelling ship; effect-initiated digivolve chain still blocked |
| BT16-085 | Davis Motomiya & Ken Ichijoji | PARTIAL | engine | Free-play + security play ship; delayed return, event-card color gate, and opponent-source DNA trash blocked |
| BT17-077 | Imperialdramon: Paladin Mode | IMPLEMENTED | - | Blast Digivolve + ACE -5 + OnPlay/WD source sweep/trash choice/memory rider + WhenAttacking ship |
| BT17-097 | Return to the Primogenitor | PARTIAL | dsl | Main Lv5+ Free digivolve + Delay replacement + Security ship; security hand/trash union auto-collapse blocked |
| BT20-020 | Imperialdramon: Fighter Mode | PARTIAL | dsl | Raid + Piercing + WhenDigivolving ship; security-loss delete blocked on source-DP formula |
| BT21-037 | Lighdramon | IMPLEMENTED | - | Piercing + Armor Purge runtime + When Digivolving suspend/+DP ship |
| BT3-002 | DemiVeemon | IMPLEMENTED | - | Inherited draw gated by carrier <Jamming> |
| BT3-093 | Davis Motomiya | IMPLEMENTED | - | Start-of-turn memory swing + reveal-add + Security free-play |
| BT3-103 | Hidden Potential Discovered! | PARTIAL | hybrid | Security add-to-hand ships; Main cost reduction blocked |
| EX1-014 | ExVeemon | IMPLEMENTED | - | Face-up Jamming + inherited carrier-only conditional Jamming for Imperialdramon name or Free trait |
| LM-030 | Green Scramble | PARTIAL | hybrid | Main + Security ship; Delay body blocked by trash-to-deck-top, DP predicate, optional-tail gaps |
| P-117 | Veemon | PARTIAL | hybrid | Inherited two-color draw implemented; cost reduction into Free with Tamer blocked |
| ST9-05 | Paildramon | IMPLEMENTED | - | on_dna_digivolve return-to-deck-bottom + WhenAttacking unsuspend OPT |
| ST9-06 | Imperialdramon Dragon Mode | IMPLEMENTED | - | When Digivolving blue/green source play via player-visible source selections |
| ST9-09 | Stingmon | IMPLEMENTED | - | Cost reduction on play + inherited When Attacking Draw |

## New Engine + DSL Gaps Filed in This Run
Cumulative across all batches; see `qa/dsl-vocab-gaps.md` and `qa/archetype-qa/engine-gaps.md` for details.

**DSL gaps (new):**
- G-DSL-CARRIER-HAS-KEYWORD (BT3-002) — closed locally by runtime `has_keyword` predicate evaluation.
- G-DSL-SELF-COLOR-COUNT-GTE (P-117) — closed locally for top-card color counts via `self_color_count_gte`; BT12-031 still needs stack-union color-count sibling support.
- G-BEFORE-PAY-COST-GAIN-MEMORY (BT12-022, BT12-050)
- G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED (BT12-022, BT12-050) — closed locally by active_when-aware grant_keyword lowering; BG Imperial now uses inherited self-aura forms for these clauses.
- G-DSL-AURA-TARGET-SOURCE-PERMANENT (EX1-014) — closed locally by inherited `target: {}` carrier self-aura.
- G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT (EX1-014) — closed locally via `source_permanent_trait_has` runtime evaluation.
- G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET (BT16-040, LM-030)
- G-PRED-STACK-SIZE-LTE-SOURCE (BT16-027, BT16-025)
- G-DSL-SOURCE-NAME-CONTAINS / G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME — closed locally for inherited carrier predicates; BT12-028 inherited clause implemented. BT16-027 still needs card-side authoring for its Dragon Mode source condition plus opponent suspended return branch.
- G-DSL-TRASH-TOP-N-DIGI-CARDS (BT12-028) — closed 2026-05-09 as `trash_top_n_digivolution_cards_of_each`; BT17-077 still needs all-source mass trash under G-ASL-07.
- G-DSL-IF-NO-TARGET (BT16-025, BT16-028)
- G-IS-EFFECT-INITIATED (BT16-028)
- G-FORMULA-SOURCE-DP (BT20-020)
- G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME (BT12-031)
- G-RETURN-ALL-TRASH-TO-DECK-BOTTOM (BT17-077) — closed locally for BT17-077 with player-choice branch.
- G-ANY-RETURNED-CARD-PREDICATE (BT17-077) — avoided for BT17-077 by faithful chosen-trash pre-check before return.
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
- G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES (EX4-060/BT22-015 → ST9-06) — closed locally for ST9-06 via `select_own_sources` + `play_selected_sources_free`.
- G-DSL-IS-DNA-DIGIVOLVING (DNA Omnimon → BT12-028, BT16-085) — RESOLVED 2026-05-08 as `dna_origin: true`; BT16-085 still needs opponent-source selection for its DNA rider
- G-ASL-07 (BT17-077 all-source mass trash; bounded top-N sibling closed by Track E)
- G-PRED-DP-LTE, G-ZONE-SELECTED-TRASH-TO-DECK-TOP, G-OPTIONAL-SELECTION-CONTINUE-TAIL (LM-027/LM-029 → LM-030)

## Test Coverage Totals
- Total tests selected by BG Imperial filters: 333 across 25 card files
- Passing: 289
- Ignored (gap-blocked/test-side): 44
- Failing: 0
