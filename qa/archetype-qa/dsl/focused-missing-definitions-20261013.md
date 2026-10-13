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
