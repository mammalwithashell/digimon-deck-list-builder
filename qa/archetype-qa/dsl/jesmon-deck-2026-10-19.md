# Jesmon supplied deck implementation — 2026-10-19

Exact scope: 54 cards, comprising 4 Digi-Eggs and 50 main-deck cards; 20 unique IDs. Seven missing implementations added, three existing implementations audited and repaired, ten existing implementations retained and regression-tested. No blocked cards in this scope. No shared engine or action/tensor contract changes.

## Card results

| Card | Copies | Result | Passing tests |
|---|---:|---|---:|
| BT8-001 | 4 | Implemented | 4 |
| ST12-12 | 3 | Audited and repaired | 9 |
| EX13-009 | 4 | Retained; regression passed | 11 |
| EX13-065 | 3 | Retained; regression passed | 18 |
| BT23-006 | 4 | Implemented | 16 |
| BT23-076 | 1 | Implemented | 13 |
| BT6-084 | 2 | Audited and repaired | 7 |
| ST12-13 | 1 | Implemented | 5 |
| BT10-085 | 1 | Implemented | 8 |
| BT13-013 | 4 | Implemented | 9 |
| EX13-011 | 2 | Retained; regression passed | 12 |
| EX13-066 | 2 | Retained; regression passed | 17 |
| EX13-012 | 3 | Retained; regression passed | 9 |
| BT20-014 | 4 | Implemented | 9 |
| EX13-061 | 3 | Retained; regression passed | 12 |
| EX13-014 | 3 | Retained; regression passed | 13 |
| BT23-013 | 2 | Audited and repaired | 11 |
| EX13-075 | 3 | Retained; regression passed | 6 |
| ST12-15 | 4 | Retained; regression passed | 5 |
| LM-061 | 1 | Retained; regression passed | 10 |

## Implementation and review

- Batch 1: BT8-001, BT23-006, ST12-13, BT13-013 — four implementations; independent reviews passed.
- Batch 2: BT10-085, BT23-076, BT20-014, ST12-12 — three implementations and one audit/repair; independent reviews passed.
- Batch 3: BT23-013, BT6-084 — two audits/repairs; independent reviews passed.
- Existing EX13 cards, ST12-15, and LM-061 required no authoring changes; their 113 regression tests pass.

New tests cover mandatory/optional choices, action masks, legal targets, costs, declines, absent targets, and once-per-turn gates where applicable. Review-driven RED/GREEN repairs include ST12-13 Champion metadata, BT13-013 ordinary evolution, captured played-card observers after departure, white token triggers, and failed suspension payment.

ST12-12 now exposes optional discard payment before drawing. BT23-013 restores exact named and conditional evolution routes, prevents duplicate Atho/Rene/Por tokens, and preserves token/played-card attack triggers after departure. BT6-084 applies official Q1470's Sistermon Noir name and Virus attribute alongside Data; runtime assertions use its real card ID.

Printed images and official local bundles/Q&A supplied the card evidence. EX13-009/011/012/065/066/075 images are absent locally; official bundle text and existing implementations/tests were available. DCGO source was a reference only; no live oracle comparison was run.

## Verification

The first complete DSL pass exposed a production-data parity failure: BT6-084 and ST12-13 lacked the rule-granted Virus attribute in cards.json (1033 passed, 1 failed, 1 ignored). Corrected only those two attributes in cards.json and durable card_overrides.json, regenerated their per-card JSON files, and regenerated the impact index. Card-data integrity and split-data tool tests: **21 passed**; both scoped split-data checks pass. Final verification below uses these synchronized records. The impact planner does not map central data files, so those explicit data checks supplement its scope.

- Fresh embedded production registry: **204 passed, 0 failed, 0 ignored** across all 20 permanent card-test modules (44.78 seconds on the final synchronized-data rebuild). Includes BT6-084 real-ID runtime alias/trait assertions.
- Harness: `D:/cargo-target-wt/77fe-bt8/jesmon_integrated.rs`; temporary direct-rustc test crate imports permanent test modules unchanged, including their `dsl_card` registry calls, and EX13 support helpers. Linked against the engine rlib rebuilt by Cargo after the final YAML edits, with mimalloc and four test threads. No YAML substitution in this final harness.
- Earlier narrow RED/GREEN runs used exact production YAML against the previous engine library; final embedded tests supersede their registry limitations.
- Baseline tier 0: seven checks passed, including 82 Python tests.
- Final tier-2 ladder completed: DSL 1034 passed / 1 ignored (both passes), judge quiz 44 passed / 1 ignored, FAQ 31 passed, keyword matrix 3 passed, replay goldens 10 verified, invariant fuzz 5 passed. Explicit scope includes ten YAML files and four synchronized data files.
- Two ladder checks required follow-up: the impact index generator's default absolute cards_root differed from the verifier's relative path; regenerated with `--cards-dir code/digimon-engine/cards`, then all seven tier-0 checks passed (including 82 Python tests). The determinism helper's line-prefix parser fails with `RUST_TEST_THREADS=1` because libtest prefixes its output on the same line; rerunning the unchanged infra binary with `RUST_TEST_THREADS=2` passed both checks (one subprocess helper ignored). Thus the aggregate ladder invocation reports these two failures, and their explicit follow-up checks pass; no claim of a zero-exit aggregate run.
- Broader ignored tests are pre-existing: effect-initiated DNA extra-draw gap and unexecuted Burst end-turn primitive. None of the 204 supplied-deck tests is ignored.
- `git diff --check` passed. Final logs: `%TEMP%/jesmon-final-tier2-synced.log`, `%TEMP%/jesmon-final-tier0.log`, and `%TEMP%/jesmon-integrated-final.log`.
- Scope planner does not require the full cards_behavioral suite; it requests card batching for this large scope. The 20-card harness supplies that targeted coverage. Full repository cards_behavioral and tier 3 were not run.

The validation ledger and generated DSL impact index are updated. Simulator tests establish the implementation evidence recorded here; they do not establish DCGO confirmation, full interaction coverage, or training readiness. Unrelated pre-existing P-092 oracle exam edits were preserved.
