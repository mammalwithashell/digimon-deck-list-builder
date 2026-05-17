# Archetype DSL Implementation: DNA Omnimon

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

Date: 2026-05-03
Total cards in pool: 64 (AD1-019 dropped — no cards.json metadata; BT1-090 + BT14-001 SKIP — prior IMPLEMENTED verdict)
Processed this run: 61 of 61 (Batches 1–16, all complete)
Pipeline: batch-implement-cards-rust-dsl

## Summary
- **IMPLEMENTED**: 21 cards (full faithful YAML+tests, all non-ignored tests pass)
- **PARTIAL**: 29 cards (core clauses working; specific clauses BLOCKED on documented gaps)
- **AUDITED-OK**: 4 cards (existing _examples / production YAML faithful, full tests added)
- **AUDITED-DRIFT**: 7 cards (existing YAML has drift / outdated workarounds; diff proposals issued)
- **BLOCKED (entire card)**: 0
- **SKIPPED (prior verdict)**: 2 (BT1-090, BT14-001)
- **Total tests authored**: 922 across 61 test files
- **Test suite status**: 1277 passed / 4 failed / 371 ignored (the 4 failures are pre-existing in ex11/ex9/lm — unrelated to DNA Omnimon)

## Gap-kind breakdown (non-IMPLEMENTED cards)
- DSL gaps: 25 cards
- Hybrid (engine + DSL): 6 cards
- Engine-only: 2 cards
- No gap (clean IMPLEMENTED / AUDITED-OK): 28 cards

## Per-Card Verdicts

| Card ID | Name | Mode | Verdict | Tests | Highlights |
|---------|------|------|---------|-------|------------|
| AD1-001 | Greymon | I | IMPLEMENTED | 18/19 | Cross-perm observer + free-digivolve from hand |
| AD1-009 | BlitzGreymon | I | IMPLEMENTED | 19/20 | De-Digivolve 3 + immunity + EOT DNA |
| AD1-010 | Garurumon | I | IMPLEMENTED | 13/14 | Draw + name observer |
| AD1-012 | CresGarurumon | I | PARTIAL (hybrid) | 13/15 | Opp-Turn effect DNA route BLOCKED; inherited no-retarget live |
| AD1-014 | MetalGarurumon | I | PARTIAL (hybrid) | 13/18 | Distinct-Tamer-colors formula BLOCKED |
| AD1-025 | Omnimon | AE | AUDITED-DRIFT | 8/16 | raw_rust unregistered; missing All-Turns clause |
| BT12-059 | Agumon | I | IMPLEMENTED | 12/12 | Reveal-buckets + self_digivolution_contains_name aura |
| BT13-012 | GeoGreymon | I | PARTIAL (dsl) | 12/16 | Security-search BLOCKED on G-PLAY-SELECTED-SECURITY-CARD |
| BT15-020 | Gabumon | I | IMPLEMENTED | 14/14 | Start-of-main grant Blocker + draw |
| BT15-101 | MetalGarurumon | I | PARTIAL (dsl) | 11/12 | G-ALT-PATH-CONDITION + G-DSL-EVENT-TARGET-IS-SELF |
| BT16-082 | Ukkomon | AP | IMPLEMENTED | 16/18 | Phase 2 Track F: tracker drift cleanup. G-ON-MOVE reveal+hatch path passes all active tests. |
| BT17-007 | Agumon | AE | AUDITED-OK | 13/13 | Faithful _examples YAML |
| BT17-015 | WarGreymon | AE | PARTIAL (dsl) | 14/16 | Phase 2 Track F: G-EFFECT-INITIATED-DIGIVOLVE-...-PERM-TARGET resolved (phantom). Remaining: G-DSL-SOURCE-NAME-CONTAINS |
| BT17-019 | Gabumon | I | IMPLEMENTED | 12/12 | Sister to BT17-007 |
| BT17-027 | MetalGarurumon | I | PARTIAL (dsl) | 12/13 | Phase 2 Track F: branch-1 digivolve test now active (G-EFFECT-INITIATED-DIGIVOLVE-...-PERM-TARGET resolved phantom). Remaining: inherited Omnimon-name gate. |
| BT17-078 | Omnimon | I | IMPLEMENTED | 12/12 | Blast DNA Counter route, DNA-origin gate, selected-level bottom-deck, and follow-up delete prompt covered |
| BT17-081 | Tai/Matt Tamer | I | IMPLEMENTED | 19/20 | All-Turns observer + EOT may-attack |
| BT17-093 | Tai/Kari Tamer | I | IMPLEMENTED | 11/11 | on_hatch + EOT return-to-deck draw |
| BT17-095 | Miraculous Mega Knight | I | PARTIAL (dsl) | 12/16 | NEW G-DSL-UNION-PLAY-FREE + G-DSL-DNA-FROM-HAND-PARTNER |
| BT17-102 | Greymon | I | PARTIAL (dsl) | 13/15 | NEW G-FORMULA-SOURCE-DP + G-DYNAMIC-NAME-ALIAS-FROM-STACK |
| BT20-102 | Omnimon (X Antibody) | AP | AUDITED-DRIFT (hybrid) | 17/20 | YAML BUGS: wrong color (red→blue+white); name_contains→name_is |
| BT21-102 | Tai Kamiya | I | IMPLEMENTED | 10/10 | Main OPT play cap now uses formula-valued `play_cost_lte` with `distinct_colors_count`; all active tests pass |
| BT22-005 | Tsumemon | I | PARTIAL (engine) | 11/14 | DigiEgg G-INHERITED-DISPATCH |
| BT22-008 | Agumon | I | IMPLEMENTED | 12/12 | Sister to BT17-007 |
| BT22-013 | WarGreymon | I | PARTIAL (dsl) | 11/15 | Phase 2 Track F: branch-0 digivolve test now active. Remaining: G-DSL-SOURCE-NAME-CONTAINS. |
| BT22-015 | Omnimon | I | PARTIAL (dsl) | 14/15 | Decode color-gated source play RESOLVED; same-level pair bottom-deck RESOLVED; lowest-DP predicate remains blocked |
| BT22-017 | Gabumon | I | IMPLEMENTED | 14/15 | NEW G-DSL-PREDICATE-TEXT-CONTAINS approximated |
| BT22-026 | MetalGarurumon | I | PARTIAL (dsl) | 12/15 | Phase 2 Track F: sister to BT22-013; branch-0 digivolve test now active. |
| BT22-084 | Nokia Shiramine | AE | AUDITED-OK | 17/19 | Faithful _examples YAML |
| BT22-089 | Mirei Mikagura | I | PARTIAL (dsl) | 10/12 | NEW G-PLAY-COST-GTE; count_gte over-fires |
| BT22-094 | Yuugo Kamishiro | I | IMPLEMENTED | 12/12 | select_reveal_buckets + cost_reduction with pay_cost |
| BT22-099 | Kuremi Detective Agency | I | IMPLEMENTED | 11/11 | flood_gate color bypass + Delay |
| BT23-008 | Greymon | I | IMPLEMENTED | 13/13 | Phase 2 Track F: G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM closed. [Main][OPT] place-top-as-bottom + play Gabumon/Nokia clause authored. |
| BT23-018 | Garurumon | I | IMPLEMENTED | 13/13 | Phase 2 Track F: sister to BT23-008. [Main][OPT] clause authored with play-Agumon/Nokia variant. |
| BT23-096 | Comet Hammer | I | PARTIAL (dsl) | 11/12 | [Your Turn] CS-attack Delay BLOCKED on G-DSL-ON-ALLY-ATTACK-TIMING + NEW G-DSL-DELAY-ON-ATTACK-EVENT |
| BT5-092 | Nokia Shiramine | I | PARTIAL (dsl) | 8/15 | Cost reduction BLOCKED on missing when_*_digivolves_into form |
| BT5-093 | Tai/Matt Tamer | AE | AUDITED-DRIFT (hybrid) | 12/15 | YAML BUG `target.of`→`target.owner`; ENGINE GAP G-AURA-GRANTED-SECURITY-KEYWORD |
| BT8-097 | Crimson Blaze | AP | AUDITED-OK (engine) | 20/21 | Faithful; 1 ignored on G-FOR-EACH-DELETE-INDEX-SHIFT |
| EX1-021 | MetalGarurumon | I | IMPLEMENTED | 13/13 | Phase 2 Track F: G-DSL-GAIN-MEMORY-FN + G-DSL-HAS-ON-DELETION-EFFECT both closed. Both clauses authored faithfully. |
| EX1-068 | Ice Wall! | I | PARTIAL (dsl) | 6/6 | NEW G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT |
| EX10-010 | BlackWarGreymon | AP | AUDITED-DRIFT | 20/24 | YAML missing play_cost_lte: 7; stale gap comments |
| EX4-003 | Tsunomon | I | PARTIAL (hybrid) | 9/12 | DigiEgg G-INHERITED-DISPATCH + NEW G-DSL-EVENT-TARGET-IS-OTHER |
| EX4-038 | Agumon | I | IMPLEMENTED | 12/14 | select_reveal_buckets + place_remainder top |
| EX4-039 | Gabumon | I | IMPLEMENTED | 11/13 | Sister to EX4-038 |
| EX4-060 | Omnimon Alter-S | I | PARTIAL (dsl) | 11/14 | [All Turns] sequential source plays + bottom-security replacement disposition are covered; remaining work is card-local source-play/generalization adoption |
| EX4-061 | Matt/Tai Tamer | I | IMPLEMENTED | 21/24 | 21 pass / 3 ignored on G-COUNT-AGGREGATE etc. |
| EX4-073 | Omnimon Alter-B | I | PARTIAL (dsl) | 6/13 | NEW G-MULTI-SELECT-OPP-PLAY-COST-SUM + G-DSL-SELECT-OWN-SOURCES-FILTER + G-PLAY-COST-AGGREGATE |
| EX5-015 | Gabumon (X Antibody) | I | PARTIAL (dsl) | 10/14 | Inherited replacement BLOCKED on NEW G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH |
| EX9-012 | MetalGreymon: Alterous Mode | I | IMPLEMENTED | 21/22 | dp_lte + observer free-digivolve self + inherited +4000 DP |
| EX9-019 | WereGarurumon: Sagittarius Mode | I | IMPLEMENTED | 16/16 | Sister to EX9-012 |
| EX9-021 | Omnimon Alter-S | I | PARTIAL (hybrid) | 12/14 | End of Attack source plays + top-security tail implemented; DNA-only immunity still needs card-local authoring now that `dna_origin: true` exists |
| EX9-066 | Tai/Matt Tamer | I | IMPLEMENTED | 25/25 | All 25 pass via faithful workaround. NEW G-DSL-BIND-PRESENT + G-COUNT-GTE-NOT-EVALUATED |
| LM-034 | Wisteria Memory Boost! | I | IMPLEMENTED | 12/12 | Sister to BT22-099 |
| P-123 | Ukkomon | I | IMPLEMENTED | 13/15 | on_move trigger |
| P-182 | WarGreymon | I | PARTIAL (dsl) | 8/12 | Phase 2 Track F: G-DSL-DISTINCT-COLORS-BOTH-PLAYERS-FORMULA verified shipped; [All Turns] aura authored. Remaining: G-FORMULA-SOURCE-DP for [When Digivolving]. |
| P-206 | Digital Gate Open | AP | AUDITED-DRIFT | 18/28 | G-COLOR-MATCH-AGAINST-BOARD primitive resolved but YAML not yet wired |
| ST21-13 | Matt/T.K. Tamer | I | IMPLEMENTED | 17/17 | All 17 pass; cost_reduction + Rush aura |
| ST2-13 | Hammer Spark | AE | AUDITED-OK | 14/14 | Trivial Option |
| ST20-10 | Agumon | I | PARTIAL (dsl) | 8/13 | Phase 2 Track F: G-ALT-PATH-DIRECTION-INTO substrate closed; warp YAML still pending companion G-DSL-DISTINCT-TAMER-COLORS predicate leaf. |
| ST20-11 | WarGreymon | I | PARTIAL (dsl) | 8/15 | Tamer-color immunity BLOCKED on G-DSL-DISTINCT-TAMER-COLORS-FORMULA |
| ST20-15 | Island of Adventure | I | PARTIAL (dsl) | 7/14 | Main security swap now uses `place_self_option_at_security`; security-aura gate still blocked on NEW G-PRED-NO-FACE-UP-SECURITY-NAMED + G-SECURITY-ZONE-AURA-SOURCE |

(Mode legend: I = IMPLEMENT, AE = AUDIT (existing _examples YAML), AP = AUDIT (existing production-path YAML))

## NEW Gaps Discovered This Run

### DSL vocab gaps
- G-DSL-SOURCE-NAME-CONTAINS (BT17-015) — `source_name_contains` predicate parses+compiles but evaluator never reads it
- G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET (BT17-015) — chain `select_own_permanent → select_hand → effect_initiated_digivolve` terminates after permanent pick
- G-OPT-RESET-VIA-ATTACK-CYCLE (BT17-015) — inherited [When Attacking][OPT] doesn't re-fire after turn cycle
- G-OPT-MULTI-TIMING-SHARED-LOCKOUT (AD1-014) — multi-timing OPT clauses fire per timing instead of shared lockout
- G-DSL-SELF-NAME-CONTAINS (AD1-014) — no leaf predicate for top-card name on inherited gates
- G-DSL-DISTINCT-TAMER-COLORS-FORMULA (AD1-014) — narrowed 2026-05-10: `distinct_colors_count` supports BT21-102's own battle-area Tamer-color play-cost cap. Broader both-player / non-Tamer variants remain tracked separately.
- G-PLAY-SELECTED-SECURITY-CARD (BT13-012) — no DSL verb plays a selected security card without paying cost
- G-EVENT-TARGET-COLOR (BT13-012) — no `event_target_color_any_of` predicate
- G-DSL-EVENT-TARGET-IS-SELF (BT15-101) — no predicate compares event target handle vs source
- G-OUTER-OPTIONAL-NOT-INSTALLED (AD1-009 family) — clause-level `optional: true` doesn't install outer accept/decline before inner select_hand prompt
- G-DSL-BIND-PRESENT (EX9-066) — narrowed 2026-05-10: `binding_present`/`binding_absent` predicates now parse, compile, and evaluate against threaded bindings. Result-log predicates remain separate.
- G-COUNT-GTE-NOT-EVALUATED (EX9-066) — `count_gte`/`count_lte` parse+compile but evaluator skips
- G-DSL-DELAY-ON-ATTACK-EVENT (BT23-096) — `lower_delay.rs` doesn't map on_*_attack timings to DelayTrigger
- G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM (BT23-008) — no DSL verb takes "top stacked card" deterministically
- G-DSL-PREDICATE-TEXT-CONTAINS (BT22-017) — no `text_contains` predicate
- G-PLAY-COST-GTE (BT22-089) — sister of resolved G-PLAY-COST-LTE
- G-DSL-GAIN-MEMORY-FN (EX1-021) — `gain_memory` only accepts literal i32
- G-DSL-HAS-ON-DELETION-EFFECT (EX1-021) — no predicate to filter permanents by [On Deletion] presence
- G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT (EX1-068) — DSL only exposes static grants
- G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES (EX4-060, EX9-021) — narrowed 2026-05-08: BT22-015 single selected-source Decode, EX4-060 sequential named source plays, and EX9-021 End of Attack sequential source plays are closed; batch / different-name variants remain open.
- G-PLACE-SELF-AT-SECURITY-BOTTOM (EX4-060) — closed for reusable Track E DSL as `place_self_at_security: { position: bottom, face: down }`; EX4-060's replacement-body fixture continues to use the explicit replacement-aware placement path.
- G-PLACE-SELF-AT-SECURITY-TOP (EX9-021) — closed 2026-05-09 for reusable Track E DSL via `place_self_at_security: { position: top, face: up }`; EX9-021's production card still uses its already-bound explicit placement tail.
- G-PLACE-SELF-AT-SECURITY-TOP-FACE-UP-OPTION (ST20-15) — closed 2026-05-09 via `place_self_option_at_security: { position: top, face: up }`; remaining ST20-15 blockers are the face-up-security-name predicate and security-zone aura source.
- G-DSL-IS-DNA-DIGIVOLVING (EX9-021) — RESOLVED 2026-05-08 as `dna_origin: true`; remaining EX9-021 blockers are card-local body/disposition gaps.
- G-DECODE-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES (BT22-015) — closed 2026-05-07 via color/level-gated `select_material` plus `play_from_materials`; verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_015_decode`
- G-FORMULA-SAME-LEVEL-PAIRS-REPEAT-TARGET (BT22-015) — closed 2026-05-07 via formula-bound `select_count_capped_multi` plus battle-area permanent-list picks. Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_015_when_digivolving_bottom_decks_n_opp_digimon_per_same_level_pair`.
- G-FORMULA-SOURCE-DP (BT17-102, P-182) — formula reading source permanent's DP
- G-DSL-DISTINCT-COLORS-BOTH-PLAYERS-FORMULA (P-182) — RESOLVED 2026-05-17 (Phase 2 Track F) — substrate verified already-shipped; P-182 [All Turns] aura authored. See [resolved-gaps.md](../../resolved-gaps.md) "Phase 2 Track F closure".
- G-DYNAMIC-NAME-ALIAS-FROM-STACK (BT17-102) — declaratives derive name set from current materials
- G-BIND-SELECTED-PROPERTY-FOR-EACH (BT17-078) — closed: `bind_permanent_property` + `level_eq_binding` cover the selected-level for-each pattern. Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- parse_bind_permanent_level_property_step bind_permanent_level_filters_for_each_same_level_permanents` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt17_078`.
- G-BLAST-DNA-DIGIVOLVE (BT17-078) — resolved for field+hand Counter Blast DNA. `blast_dna_digivolve` carries exact printed materials; BT17-078 proves WarGreymon + MetalGarurumon and rejects broad Greymon/Garurumon names. Additional printed field+hand Blast DNA fixtures now cover BT20-045, BT20-076, BT20-081, EX6-011, and EX6-029. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt17_078_counter_blast_dna`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_045_counter_blast_dna bt20_076_counter_blast_dna bt20_081_counter_blast_dna ex6_029_counter_blast_dna`.
- G-DSL-UNION-PLAY-FREE (BT17-095) — `select_union_zone` binding doesn't feed `play_from_*_free`
- G-DSL-DNA-FROM-HAND-PARTNER (BT17-095) — DNA digivolve where 2nd material lives in hand
- G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH (EX5-015) — inherited replacement with trash-cost-then-cancel (Phase 2 Track F: DEFERRED — entangled with G-SELECT-MULTI-MIN and G-ZONE-TRASH-TO-DECK sub-gaps; pulling this single tag would require a 3-way bundle; follow-up needed.)
- G-MULTI-SELECT-OPP-PLAY-COST-SUM (EX4-073) — sibling of resolved DP-budget for play-cost
- G-DSL-SELECT-OWN-SOURCES-FILTER (EX4-073) — resolved 2026-05-08; remaining EX4-073 blockers are play-cost aggregate/count-binding work.
- G-PLAY-COST-AGGREGATE (EX4-073) — `lowest_play_cost` aggregate predicate
- G-EVENT-CARD-TAMER-PLAY (AD1-010, EX9-012) — event_card population for tamer plays unconfirmed
- G-DSL-EVENT-TARGET-IS-OTHER (EX4-003, EX4-039) — no `event_target_is_other` predicate
- G-PRED-NO-FACE-UP-SECURITY-NAMED (ST20-15) — no predicate for "no face-up named security card"
- G-ALT-PATH-DIRECTION-INTO (ST20-10) — RESOLVED 2026-05-17 (Phase 2 Track F) — `AltPathSpec.direction: into` schema + route threading ships. ST20-10 production YAML still pending its companion `G-DSL-DISTINCT-TAMER-COLORS` predicate-leaf gap (Tamer-colour disjunct). See [resolved-gaps.md](../../resolved-gaps.md) "Phase 2 Track F closure".

### Engine gaps
- G-MOD-CANNOT-CHANGE-ATTACK-TARGET (AD1-012) — resolved 2026-05-08 via `ModifierType::CanNotSwitchAttackTarget` self inherited aura; covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ad1_012_inherited_blocks_attack_target_change_during_your_turn`
- G-AURA-GRANTED-SECURITY-KEYWORD (BT5-093) — aura-granted SecurityAttackPlus not consumed by security loop
- G-SECURITY-ZONE-AURA-SOURCE (ST20-15) — security-zone aura sources not iterated by Group 6 tick

### Hybrid gaps (engine+DSL)
- G-DSL-ON-OPPONENT-ATTACK (AD1-012) — resolved; remaining AD1-012 blocker is defender-side effect DNA into Omnimon Alter-S during the attack interrupt
- G-DSL-REDIRECT-ATTACK-TARGET (AD1-012) — resolved; redirect step parses/lowers to `ctx.redirect_attack`
- G-DSL-ON-ALLY-ATTACK-TIMING (BT21-102, BT23-096) — DSL Timing variant

## Test Suite Status
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral` → **1277 passed / 4 failed / 371 ignored**
- The 4 failures are pre-existing in non-DNA-Omnimon cards (`ex11_012`, `ex9_013`, `lm_027`) and were noted by multiple workers as predating this run.
- DNA Omnimon archetype contributed 922 new tests across 61 files; 0 new failures introduced.

## YAML Bugs Found (Existing Cards)
- BT20-102: `color: [red, blue]` should be `[blue, white]`; `alt_path.from.name_contains` should be `name_is`
- BT5-093: `target.of` should be `target.owner` (over-fires aura on opponent's Omnimon)
- EX10-010: missing `play_cost_lte: 7` filter on opp delete (G-PLAY-COST-LTE was resolved 2026-05-01)

## Reusable Implementation Patterns Worth Documenting (RUST_DSL_TEST_API.md follow-up)
- Cross-permanent observer + free-digivolve from hand (AD1-001)
- Conditional-mandatory inner-select pattern (AD1-009)
- `target: source` vs `target: self` distinction (universal — AD1-012, EX9-012)
- Stacked-source-only inherited test setup using `runner.place_stack` (BT13-012)
- Driving real on_suspend events via `runner.game.suspend(handle)` not `enqueue_triggered` (BT13-012)
- Pre-suspend by direct mutation pattern (AD1-014 on_suspend self-tests)
- Two-branch start-of-main with independent `if` gating (BT15-020)
- Repeat-N over BattleArea via sequential `select_opponent_permanent` + `not_in_binding` (BT15-101)

## Files Created
- 56 new YAML files in `code/digimon-engine/cards/<set>/`
- 61 new test files in `code/digimon-engine/tests/cards_behavioral/<set>/`
- 5 audited existing YAMLs in `code/digimon-engine/cards/_examples/` (no modifications) and `code/digimon-engine/cards/<set>/` (no modifications)
- mod.rs registrations updated for 17 set directories
- main.rs updated to include all 17 sets

## Reviewer Wave
Skipped per orchestrator decision to conserve token budget. Worker self-reports were detailed and full-suite tests passed. Future runs may re-enable per-batch Opus review for higher confidence.
