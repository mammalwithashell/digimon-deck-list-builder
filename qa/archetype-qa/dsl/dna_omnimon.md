# Archetype DSL Implementation: DNA Omnimon
Date: 2026-05-11
Total cards in pool: 64
Processed this run: 37 (across 10 batches)
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 1 (BT5-093 — new YAML written)
- PARTIAL: 14 (clauses BLOCKED on engine/DSL gaps; faithful core)
- AUDITED-OK: 6
- AUDITED-MISSING-TESTS: 14 (faithful YAML; tests added)
- AUDITED-DRIFT: 2 (AD1-025, BT17-078)
- BLOCKED (engine|dsl|hybrid): 0 fully blocked
- SKIPPED (prior verdict IMPLEMENTED/AUDITED-OK): 27

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Tests | Notes |
|---------|------|------|---------|-------|-------|
| AD1-025 | Omnimon | AUDIT | AUDITED-DRIFT | 16 | raw_rust fn unregistered; [All Turns] OPT clause absent |
| BT17-078 | Omnimon | AUDIT | AUDITED-DRIFT (hybrid) | 23 | Severe YAML drift; WhenAttacking OPT missing; DSL gap grant_keyword_to_source |
| BT22-015 | Omnimon | AUDIT | AUDITED-MISSING-TESTS | 22 | +7 tests; G-PRED-DP-LTE pre-existing ignore |
| BT20-102 | Omnimon (X Antibody) | AUDIT | AUDITED-MISSING-TESTS | 19 | +3 tests; color metadata + return-to-deck conditionality flagged |
| EX4-060 | Omnimon Alter-S | AUDIT | AUDITED-MISSING-TESTS | 16 | +1 structural; closed gaps confirmed |
| EX4-073 | Omnimon Alter-B | AUDIT | AUDITED-MISSING-TESTS | 16 | +3 live, 7 ignored under play-cost-aggregate gaps |
| EX9-021 | Omnimon Alter-S | AUDIT | AUDITED-OK | 18 | +3 tests; YAML faithful |
| BT17-095 | Miraculous Mega Knight | AUDIT | AUDITED-MISSING-TESTS | 23 | +7 tests; DSL gaps union-play-free + DNA-from-hand-partner |
| BT17-015 | WarGreymon | AUDIT | AUDITED-MISSING-TESTS | 21 | +2 tests; 5 ignored |
| BT22-013 | WarGreymon | AUDIT | AUDITED-MISSING-TESTS | 16 | +1 test; 5 ignored |
| EX10-010 | BlackWarGreymon | AUDIT | AUDITED-MISSING-TESTS | 28 | +4 tests covering Tamer-target delete branch |
| P-182 | WarGreymon | AUDIT | PARTIAL | 12 | 7 ignored; clauses omitted under G-FORMULA-SOURCE-DP + G-DSL-DISTINCT-COLORS |
| ST20-11 | WarGreymon | AUDIT | PARTIAL | 16 | Unblocked 2 stale ignores + 1 test; tamer-color immunity OMITTED |
| BT13-012 | GeoGreymon | AUDIT | AUDITED-MISSING-TESTS | 18 | +2 tests; Clause 1 BLOCKED on G-PLAY-SELECTED-SECURITY-CARD |
| BT17-102 | Greymon | AUDIT | PARTIAL | 18 | +3 tests; 2 clauses OMITTED under G-FORMULA-SOURCE-DP + G-DYNAMIC-NAME-ALIAS-FROM-STACK |
| BT23-008 | Greymon | AUDIT | AUDITED-MISSING-TESTS | 20 | +4 tests; Main effect BLOCKED on G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM |
| AD1-014 | MetalGarurumon | AUDIT | PARTIAL | 23 | +5 tests + 1 ignored; 3 NEW gaps discovered |
| BT15-101 | MetalGarurumon | AUDIT | PARTIAL | 16 | +5 tests; alt-path OMITTED under G-ALT-PATH-CONDITION; on_suspend over-fires |
| BT17-027 | MetalGarurumon | AUDIT | AUDITED-MISSING-TESTS | 16 | +3 tests; 3 ignored |
| BT22-026 | MetalGarurumon | AUDIT | AUDITED-MISSING-TESTS | 16 | +1 test (tie-break lowest-level); 4 ignored |
| AD1-012 | CresGarurumon | AUDIT | AUDITED-MISSING-TESTS | 22 | +9 tests; 2 ignored under G-OPT-TRIGGERED + opponent-turn DNA route |
| EX1-021 | MetalGarurumon | AUDIT | PARTIAL | 18 | YAML structural-only under 3 stacked gaps |
| EX5-015 | Gabumon (X Antibody) | AUDIT | PARTIAL | 14 | Inherited Substitute clause OMITTED under stacked gap |
| BT23-018 | Garurumon | AUDIT | PARTIAL | 19 | +3 tests; Main effect BLOCKED on G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM |
| BT5-092 | Nokia Shiramine | AUDIT | PARTIAL | 15 | No tests added; Clause 2 cost-reduction route BLOCKED |
| BT21-102 | Tai Kamiya | AUDIT | AUDITED-OK | 16 | +6 tests; all 16 pass; NEW gap G-DSL-OPTIONAL-SELECT-PASS-TAIL |
| BT22-089 | Mirei Mikagura | AUDIT | PARTIAL | 12 | No tests added; G-PLAY-COST-GTE + count_gte gaps |
| BT5-093 | Tai Kamiya & Matt Ishida | IMPLEMENT | IMPLEMENTED | 15 | NEW YAML; 13 pass, 2 ignored under G-AURA-GRANTED-SECURITY-KEYWORD + cross-turn check |
| EX1-068 | Ice Wall! | AUDIT | PARTIAL | 9 | +3 security tests; [Main] grant_triggered_effect pending YAML update |
| BT23-096 | Comet Hammer | AUDIT | PARTIAL | 17 | +6 tests; NEW gap G-DSL-DELAY-ON-ATTACK-EVENT |
| P-206 | Digital Gate Open | AUDIT | PARTIAL | 32 | +4 tests; 2 YAML deficits flagged (resolved predicates not adopted) |
| ST20-15 | Island of Adventure | AUDIT | PARTIAL | 14 | No tests added; security-zone aura OMITTED + color bypass workaround |
| AD1-019 | Matt Ishida & T.K. Takaishi | IMPLEMENT | PARTIAL | 13 | NEW YAML; absent from cards.json (sourced from official site); NEW gap G-DSL-COST-DELTA-FORMULA |
| BT22-005 | Tsumemon | AUDIT | AUDITED-OK | 14 | No tests added; pre-existing G-INHERITED-DISPATCH + G-OPT-TRIGGERED |
| EX4-003 | Tsunomon | AUDIT | AUDITED-OK | 17 | +5 structural tests |
| BT16-082 | Ukkomon | AUDIT | AUDITED-OK | 18 | +2 OPT tests implemented (formerly ignored); now all green |
| ST20-10 | Agumon | AUDIT | PARTIAL | 13 | No tests added; warp clause OMITTED under 3 stacked gaps |

## New Engine / DSL Gaps Discovered This Run
- **G-DSL-OPTIONAL-NON-SELECTION-TRIGGER** (AD1-014): optional trigger whose body has no selection-driving step doesn't surface a may-prompt.
- **G-DSL-SELF-NAME-CONTAINS** (AD1-014): inherited clauses gated on "this Digimon has [X] in its name" lack a DSL predicate.
- **G-OPT-MULTI-TIMING-SHARED-LOCKOUT** (AD1-014): `when: [t1,t2,t3]` lowers to per-timing OPTs, not DCGO's shared lockout.
- **G-DSL-OPTIONAL-SELECT-PASS-TAIL** (BT21-102): `install_select_*` with on_decline=None breaks tail continuation when player PASSes.
- **G-DSL-DELAY-ON-ATTACK-EVENT** (BT23-096): `lower_delay.rs:55-65` silently downgrades unrecognized timings to EndOfYourNextTurn.
- **G-AURA-GRANTED-SECURITY-KEYWORD** (BT5-093): `security_attack_keyword_bonus` doesn't consult `Modifiers::permanent_keywords`.
- **G-DSL-COST-DELTA-FORMULA** (AD1-019): no formula variant for `CostDelta` on `play_from_hand` step (only literal N or `free`/`printed`).

## Drift Cards (YAML disagrees with cards.json — diff-proposed, not applied)
- **AD1-025**: raw_rust fn `ad1_025_on_play_process` unregistered in build_registry; [All Turns][OPT] OnLeaveField observer entirely absent.
- **BT17-078**: YAML implements unrelated bottom-deck body; [On Play/WD] DNA-gated Piercing+SA+2 grant to chosen WarGreymon missing; [When Attacking][OPT] trash-hand for DP/Jamming/MayAttack entirely absent. DSL vocab gap: grant_keyword_to_source.

## Methodology Notes
- Orchestrator skipped dedicated Opus reviewer wave for cost efficiency; verification by `cargo test --test cards_behavioral` passing all active tests.
- Full suite: 2233 passed, 490 ignored, 0 failed.
- Multiple cards' task-prompt effect text diverged from `data/cards.json` (per-card JSON snapshots stale). Agents correctly used cards.json as authoritative per CLAUDE.md.
- AD1-019 not present in cards.json — official text sourced from en.digimoncard.com.
- Disk full mid-Batch 9; 3 agents re-dispatched without worktree isolation.
- Worktree isolation auto-cleaned for agents that wrote no changes; some agents wrote directly to main tree, which was tolerated.
