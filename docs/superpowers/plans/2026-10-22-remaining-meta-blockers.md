# Remaining meta admission repairs

User authorization: “Fix that too”, covering the three residual suite failures
and the two reusable evolution capabilities blocking the four remaining cards.
Starting revision: `a74fd6951105ecb298d92f9c43577e95be55af5b`.

## Scope and order

1. Repair the printed-data mismatches for EX12-026, EX12-031 and BT22-023,
   plus BT24-085's stale color assertion. Preserve card registry assignments.
2. Implement effect-driven breeding evolution with zone-pinned physical
   identity through selection, declaration, payment, replacement, clone/resume,
   commit, bonus draw and result binding. Suppress effects that cannot apply
   in breeding; keep ordinary battle evolution unchanged.
3. Implement hand-partner effect DNA declaration with pinned field material,
   partner and result, original zones, recipe and stack orientation, payer,
   original effect context, complete reducer choices and typed continuation.
   Candidate masks and commit must share legal/payable route evaluation.
4. Complete one four-card batch: BT20-018 [IMPLEMENT], BT20-056 [AUDIT/repair],
   BT20-087 [IMPLEMENT], EX6-072 [AUDIT/repair]. Two implement, two audit,
   zero skipped; all four start blocked by the prerequisites above.
5. Independent spec and quality reviews, targeted regressions, cumulative
   tier 2 and required shared suites; refresh readiness artifacts and admission
   against the same frozen library and recent pool. Update the existing PR745.

## Contracts and ownership

No action-space or tensor expansion is needed: existing breeding, hand, field,
effect-choice and route prompts represent every choice. A selection must remain
visible even with one legal target; optional declines remain explicit.

The root owns Cargo, shared registration, trackers and reports. Breeding and DNA
workers write failing standalone regressions first. Shared reducer files are
edited sequentially: breeding first, then DNA integration. DNA can prepare its
new module independently. Per-card workers own only their YAML and test files.
Root owns correlated DNA predicates and DSL wiring. No placeholder effects or
temporary material placement in the battle area are permitted.

## Acceptance evidence

- Breeding: hand/trash sources, free/reduced/alternate routes, both owners,
  stable identity across compaction and paused choices, stale/changed/moved
  hosts, clone parity, one draw, success-only result and outer continuation,
  ordinary trigger suppression and battle-area controls.
- DNA: correlated result/field/hand choices, incompatible triples and physical
  distinctness, all matching recipe orientations including equal-cost routes,
  active/inactive payer, reductions/prohibitions and interactive payments,
  original-zone revalidation, clone parity, exact stack conservation, one
  payment/draw, DNA trigger context and success-only result continuation.
- Data: actual images and official bundles, central/override/split parity,
  unchanged existing registry indices and exact existing guard assertions.

Admission is a simulator training gate, not an oracle certification. Preserve
the earlier source and verification boundaries and report the new denominator.

## Progress

- Production-data corrections and the stale color assertion are repaired and
  independently reviewed. All three guards pass in their full suites; 21
  production JSON integrity/split-pipeline tests pass.
- Both lifecycle implementations fit existing action/tensor contracts. Runtime
  RED preceded breeding commit, hand-partner order/payment, correlated masks,
  replacement, nested continuation and owner/controller provenance repairs.
- All seven correlated-mask tests and all 69 DSL library tests pass, including
  the three new predicate validation cases.
- The four-card batch is complete: two implementations, two audits/repairs,
  zero skips. All 55 card tests pass; independent actual-image spec and quality
  reviews approve the cards and shared repairs.
- Final focused verification passes 111 tests. The broader 19-target run passes
  1,164 tests, including 474 engine unit tests. Full cards pass 12,514 with 35
  ignored; full DSL passes 1,515 with one ignored. All 10 unchanged replays
  (635 actions) verify, and all five invariant-fuzz smoke tests pass.
- The cumulative tier-2 command completed all 37 stages. It retains one failed
  initial full-card stage caused by an older BT17-095 clone harness assuming an
  immediate merge. The reviewed test-only correction passes both its focused
  run and the scheduled full-card rerun. Every other stage passes; the original
  aggregate is not relabeled clean. Failure and rerun logs remain preserved.
- Final gate artifacts and evidence notes are refreshed; all 13 tier-0 checks
  pass, including 174 tool tests. Admission is 1,468/5,834 frozen-library lists
  with 48/447 majority labels; recent 503/846 with 34/106; all 21 recent
  Chronicle/Alphamon lists and their 35 unique IDs pass. No list regresses.
- Results and boundaries are recorded in
  `docs/superpowers/research/2026-10-22-construct-tests/remaining-meta-repairs.md`;
  the change belongs to the existing draft PR #745. Python training requires
  a wheel built from this branch; no deployment is claimed.
