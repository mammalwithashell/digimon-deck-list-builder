# Focused campaign missing native definitions

Scope: implement the five source-reviewed no-effect cards in the existing isolated branch. Card implementation evidence does not admit the source snapshot or establish oracle or training readiness.

| Batch | Card | Mode | Verdict | Targeted checks | Review |
| --- | --- | --- | --- | --- | --- |
| 1 | BT1-009 | IMPLEMENT | IMPLEMENTED | 7 passed | Spec and quality approved |
| 1 | BT1-014 | IMPLEMENT | IMPLEMENTED | 7 passed | Spec and quality approved |
| 1 | BT3-007 | IMPLEMENT | IMPLEMENTED | 7 passed | Spec and quality approved |
| 1 | BT4-010 | IMPLEMENT | IMPLEMENTED | 7 passed | Spec and quality approved |
| 2 | BT4-014 | IMPLEMENT | IMPLEMENTED | 7 passed | Spec and quality approved |

Batch 1: 4 IMPLEMENT, 0 AUDIT, 0 SKIP, 0 BLOCKED. All 28 tests first failed because the native definitions were missing, then passed after minimal YAML. Two independent production-registry probes pass against all 4,483 original R4 rows, and the same two pass with the portable five-card fixture. Action masks, both seats, ordinary cost/draw, legal base choice, negative level/color, battle/security DP boundaries and absence of invented inherited effects are covered. The synthetic level-2 battle bases in two choice-routing probes are fixture-only, not legal gameplay setups.

Evidence: runs/card-loop/meta-six-20261010/five-vanilla-batch1-red-receipt-20261013.json, five-vanilla-batch1-spec-review-20261013T083322Z.json, and five-vanilla-batch1-quality-review-20261013T084521Z.json in the retained fe80 campaign workspace. Full-five broader verification and final packaging remain pending. BT4-018, BT6-068 and BT16-033 remain outside this five-card batch; source/capability scouting is separate.

Batch 2: 1 IMPLEMENT, 0 AUDIT, 0 SKIP, 0 BLOCKED. Seven intended missing-definition failures were retained before minimal YAML; all seven then passed. Independent production-registry probes passed 2/2 with the complete original R4 and 2/2 with the portable fixture, including both legal champion choices and both seats. Independent specification and quality reviews found no issues. Required Tier 2 and full-engine verification remain in progress.

Batch 2 evidence in the retained fe80 workspace: five-vanilla-batch2-red-receipt-20261013.json, five-vanilla-batch2-spec-review-20261013T090648Z.json, five-vanilla-batch2-quality-review-20261013.json.

Broader gate status at preservation: Tier 2 remains running; its 82 Python tool tests passed. Oracle-readiness freshness fails before Rust suites. Normal generator outputs from the clean seed and this candidate are byte-identical and differ from the committed artifact in 259 existing cards; none of these five cards belongs to that deck-library scope. Both generated views remain in campaign evidence, with no generated-file overwrite or waived gate. Other Rust suite outcomes remain pending. The prior EventTarget branch also recorded two official Rule/Aquatic DSL failures; this candidate must retain its own actual outcomes.


## Subsequent measured verification and remaining-effect batch

The five-card Tier 2 run completed: 17 checks passed and 4 failed. Readiness generation is byte-identical to the clean seed; the two Rule/Aquatic DSL failures and ten replay digest divergences were also reproduced on the clean seed. These baseline defects remain failed gates, with original evidence in five-vanilla-final-verification-review-20261013.json and the three separate baseline reviews. No goldens were blessed or generated source files hand-edited.

| Batch | Card | Mode | Verdict | Targeted checks | Review |
| --- | --- | --- | --- | --- | --- |
| 3 | BT4-018 Spinomon | IMPLEMENT | IMPLEMENTED | 9 passed after 9 missing-definition failures | Independent spec and quality approved |
| 3 | BT6-068 Impmon | IMPLEMENT | IMPLEMENTED | 8 passed after 8 missing-definition failures | Independent spec and quality approved |

Batch 3: 2 IMPLEMENT, 0 AUDIT, 0 SKIP, 0 BLOCKED within this slice. Existing DSL expresses both effects. Spinomon has a live own-turn self DP aura; Impmon exposes optional discard and mandatory own-trash trait-filtered return, with both-seat masks, negative targets and clone/resume checks. Original metadata rows are bound to the full R4 input. Separate ordinary-registry probes pass 2/2 in portable mode and 2/2 with all 4,483 original R4 rows, without manual card-definition registration. Impmon's first GREEN counted optional PASS among target IDs; the corrected test checks its mask separately. The first independent play probe used a pending-only helper; the corrected probe uses the masked normal decoder. Both invalid-fixture failures are retained and are not engine defects.

Evidence in the retained fe80 campaign workspace: bt4-018-independent-{spec,quality}-review-20261013.json, bt6-068-independent-{spec,quality}-review-20261013.json and remaining-effects-production-registry-review-20261013.json. Broader verification for this two-card candidate remains pending. Prior proof inventories are historical and must be regenerated against a new native pack before admission. BT16-033 Harpymon remains blocked on separately reviewed attacker security-check timing. Full clause and gating denominators, five oracle verdict classes, source consumers and training readiness remain open.


## Harpymon native authoring and reviewed security composition (2026-10-14)

| Batch | Card | Mode | Verdict | Targeted checks | Review |
| --- | --- | --- | --- | --- | --- |
| 4 | BT16-033 Harpymon | IMPLEMENT | IMPLEMENTED | 15 passed, 0 failed, 0 ignored | Independent SPEC and root QUALITY approved |

Batch 4: 1 IMPLEMENT, 0 AUDIT, 0 SKIP, 0 BLOCKED within this card slice. The separately reviewed Security timing fixes, including the later EX12-063 and P-193 regression repair, were composed into this isolated branch before authoring. Four missing-definition availability tests failed before the YAML existed. All 12 required matrix rows now pass in both seats: ordinary and exact-name digivolution routes, live security thresholds, distinct checks without OPT, source/turn/zone negatives, face-up/down checks, timing before security battle, Q2629 liveness and Armor Purge, sibling processing, prohibitions, and actual masked Q6813 play with optional decline and clone/resume. The final test explicitly asserts empty actor trash and absence of a derived Kaguyamon prompt.

The YAML follows printed memory-before-recovery order. The inspected DCGO reference processes recovery first; this difference remains a behavioral-oracle validation item. Native tests do not resolve that difference or establish oracle confirmation. Prior availability RED, compile/invalid-fixture failures and the first quality finding remain preserved.

Exact final evidence in the retained fe80 campaign workspace: harpymon-quality-followup-20261014/FINAL-QUALITY-FOLLOWUP.json (75f045ff), harpymon-independent-spec-20261014/FINAL-INDEPENDENT-SPEC-REVIEW.json (d59baeb9), harpymon-independent-spec-quality-delta-20261014/FINAL-SPEC-QUALITY-DELTA-ADDENDUM.json (baaf9dc9), and harpymon-root-quality-r2-20261014.json (6b2d6fe8). Final native log 7a812f79 records 15 passes and natural exit0 in 221.63 seconds; YAML b9e480df and test 230521a6 are the reviewed bytes.

Earlier two-card broader verification completed with 14 passed and four baseline gates failed; it is not still running. Subsequent three-branch broad verification retained source/DSL/readiness/replay failures. The later security repair passed 51 targeted tests, without claiming another full-suite pass. Fresh source composition, native-pack validation, all clause and gating-interaction denominators, all five oracle verdict classes, original human questions and training readiness remain open. Neither focused deck is admitted for training.
