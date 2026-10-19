# Examon implementation PR status — 2026-10-15

This change packages the integrated Examon authoring and verification campaign. It includes new older-set Examon/support YAML cards, EX13 corrections, the shared engine/DSL primitives required by their interactions, action/model contract consumers, regression tests, official observation/correction inputs, and DCGO exam tooling and scenarios.

The nine EX13 campaign cards are EX13-005, EX13-008, EX13-018, EX13-021, EX13-024, EX13-039, EX13-041, EX13-044, and EX13-045. These EX13 YAMLs were already present at the branch baseline; this PR adds their follow-up fixes and the missing older support cards rather than claiming every EX13 file is newly added. Additional support cards serve reproducible shared-engine regression scenarios.

## Captured validation

PR integration merged upstream `main` at `3aa457f77`, retaining its additional source-trash adjudication in the only documentation conflict. Engine and DSL implementation files were unchanged by that merge. Fresh post-merge checks passed: `cargo test -p dcgo-harness --lib` (628 tests) and the two card-loop exam/harness pytest suites (95 tests). Logs are included in `examon-2026-10-10-evidence/verification/pr-review/`. The full engine suite was not rerun during PR packaging. The packaging whitespace check reports existing trailing whitespace and extra EOF blank lines; those bytes were preserved rather than reformatted across the captured source snapshot.

The final build22 acceptance review records 1,081 passing EX13 tests, 198 focused receipt tests, 24 successful pipeline gates, 11 separate ladder/wheel gates, and all ten replay goldens passing after the reviewed attribution correction. The original failed golden run remains a historical failure; it was resolved by a separately captured successful run. Copies of the compact reviews are under `examon-2026-10-10-evidence/verification/pr-review/`.

On PR preparation, all 3,874 non-QA working-tree file pins in the build22 comparison closure still matched. This is a source-to-evidence check, not a fresh full test run. Git line-ending normalization during staging may change repository blob bytes without changing the tested working-tree files.

The 126-case EX13 retained-recording comparison contains 125 matches and one official-ruling-supported DCGO discrepancy, EX13-041 / Q7339. Across 155 retained comparisons, 151 match and four preserve known diagnostic differences. These are comparisons against captured DCGO runs, not 155 freshly executed live games. Raw run captures, build binaries, and temporary reviewer workspaces remain local; committed compact reviews retain their original provenance paths and hashes.

## Decisions and remaining work

- Preserve EX13-041's Q7339-compliant mutual-deletion behavior. The explicit user decision is in `examon-2026-10-10-evidence/verification/decision-ex13-041-q7339-20261015.md`; upstream fix: https://github.com/DCGO2/DCGO/pull/2303. Its full patched Unity replay remains pending.
- The original 52-card coverage ledger and newer validation evidence still require reconciliation before claiming a current full-archetype completion percentage. Do not reinterpret raw disagreements as DCGO matches.
- Retained diagnostics Q1123 (BT3-093), Q1923 (BT9-109), and Q3414 (EX3-037) require final disposition/current evidence as appropriate. Existing diagnostic comparisons are not terminal-outcome approval.
- BT14-009's effect clause and two interactions retain unavailable status in the recorded DCGO inventory.
- The battle-area capacity contract gap is separately planned in `docs/BATTLE_AREA_CAPACITY_CONTRACT_PLAN.md`; this PR does not claim unrestricted DCGO parity or complete archetype certification.

This is a draft implementation PR for review of a substantial integrated campaign, not a declaration that all 52 cards or all possible interactions are fully verified. Historical reports and the checkpoint preserve their dates and scope; use this status note for the PR's current acceptance boundaries.
