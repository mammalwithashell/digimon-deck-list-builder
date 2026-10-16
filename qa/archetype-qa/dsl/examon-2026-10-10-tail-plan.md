# Examon full-pool DCGO continuation

User request: continue until our implementation for the DCGO-implemented campaign cards matches DCGO. This extends the completed core repair to every support/tech card in the original 52-card inventory, including known interaction gaps and cards missing from the extracted denominator. No global card-catalog completion is claimed.

Starting evidence: 128 confirmed / 190 extracted clauses, 62 unmeasured; core 122/122. These counts are a baseline, not the completion target. LM-057 and LM-058 currently extract zero clauses despite printed effects; resolve that denominator defect before claiming full coverage. BT14-009 has no script at the expected per-card base DCGO path and needs an availability finding, not a false confirmation.

Worktree: `codex/examon-blocker-repairs`, existing dirty campaign preserved. Use the existing isolated EX13 player and harness; base DCGO is read-only, shared queues untouched.

## Queue

All existing production cards enter AUDIT regardless of historical authoring SKIP. Missing YAML enters IMPLEMENT after capability verification. Prior blocked findings require explicit retesting. Clauses already confirmed are preserved, but known contradictory interaction findings still require repair.

1. BT1-090 [AUDIT], BT11-022 [AUDIT], BT11-033 [AUDIT], BT3-093 [AUDIT].
2. BT3-103 [AUDIT], EX3-018 [AUDIT], EX8-002 [AUDIT], ST2-13 [AUDIT].
3. LM-034 [AUDIT], LM-036 [AUDIT], LM-051 [AUDIT], P-035 [AUDIT].
4. LM-027 [AUDIT], LM-028 [AUDIT], LM-055 [AUDIT], LM-057 [AUDIT; denominator repair].
5. LM-058 [AUDIT; denominator repair], BT13-096 [IMPLEMENT; event identity], EX3-065 [IMPLEMENT; event identity/refire], P-092 [IMPLEMENT; immutable exact event name].
6. BT9-109 [IMPLEMENT; source-card protection], EX1-068 [AUDIT; dynamic trigger grant], BT8-097 [AUDIT; effect-controller/breeding prohibition], EX1-066 [prior confirmed; regression review].
7. BT14-009 [availability verification; no oracle script found].

Totals: 20 existing YAML AUDIT cards, 4 missing YAML IMPLEMENT cards, 1 availability check. The readiness scout will refine reusable dependencies; listing a gap does not complete the requested repair.

## Gates

- Official printed evidence and card image, rules where relevant, base DCGO behavior.
- Failing Rust behavioral test before every behavior fix; visible legal choices, no approximation, unchanged action/tensor contracts.
- Per-card worker; independent spec then quality review; sequential Cargo and live oracle use.
- Legal scenario simulator preparation, then actual DCGO comparison; retain every attempt and all five verdict classes. Backfill only confirmed observations.
- Resolve missing denominator and known interaction defects; clause confirmation alone is not complete card readiness.
- Regenerate coverage/readiness/index, required verification ladder and focused shared regressions, then whitespace check. Preserve and distinguish unrelated baseline suite failures.

## Progress

- Isolated runtime preflight: GO, healthy player PID 37004, matching action-space hash.
- Full inventory per-card script check: only BT14-009 absent at expected base paths.
- LM-057/058 canonical extractor reproduced zero clauses; repair investigation pending.
- Batch 1 audits/scenario authoring started. BT1-090 printed Red versus YAML Black metadata discrepancy identified; awaiting behavior test before correction.

### Continuing evidence

- Denominator corrected to 198 clauses: eight LM-057/058 clauses restored, original 190 text hashes unchanged. Empty failed official scrapes fall back; complete vanilla records retain zero clauses. Reviewed image security cache supplies both missing security texts. Python coverage/source/readiness suite: 112 passed.
- BT14-009: one unavailable clause, per-card script/class absent from base DCGO; separate from unmeasured.
- BT11-022: all three clauses confirmed, 10/10 live steps, source backfilled.
- BT11-033: both clauses confirmed via separate bounce (9/9) and fallback (7/7) lines, source backfilled.
- BT3-093: all three clauses confirmed via single-hit security/search/start-turn line (10/10). Both-hit attempt retains documented outcome-neutral DCGO intermediate hand timing quirk (official rule 15-15-10, docs/DCGO_EXAM.md); corrected misplaced sim_only flag. Added Q4704 test, 16 card tests passed.
- BT1-090: wrong YAML color corrected after two real legality failures; 11 tests passed. Actual egg-supported scenario then exposed shared legality bug: DigiEgg kind excluded from breeding color support. Real ST1-01 hatch regression added; repair in progress, no oracle confirmation yet.
- Independent first-batch spec and quality reviews approved scoped card/denominator edits; subsequent shared fixes require their own reviews.
- Exact immutable event-card-name predicate: RED3 then GREEN3. Compiled pack version 0.6.0. Per-card P-092 restoration still pending.
- Event permanent identity: initial RED3/4; review added stale predicate/observer capture regressions RED3/7; current GREEN9 plus exact-name3. Review identified captured event-card level must remain immutable; follow-up in progress, not closed.
- BT8-097 controller/breeding effect-play regressions: both RED, shared fix candidate pending tests.
- Source-card trash protection prerequisite: implementation/TDD investigation underway.

Current coverage snapshot: 136 confirmed, 0 diverged, 0 unreachable, 1 unavailable, 61 unmeasured. Known behavior gaps still block full-card readiness even when a clause has an earlier confirmation.

### Shared interaction checks in progress

- Event identity and exact immutable-name prerequisites: 10 identity + 3 name tests passed; independent spec and quality review approved. Card restorations remain pending.
- Gravity Crush breeding-color regression: 12 tests passed after allowing actual Digi-Egg color support. Live oracle still pending rebuilt harness.
- Effect-play controller/breeding prohibition: 4 focused tests passed; independent spec and quality review approved.
- Dynamic triggered grants: initial 8 tests passed. Independent review added 3 failing regressions for distinct simultaneous carriers and verification digest duration metadata; repairs underway.
- Source-card protection: 7 initial core tests passed and 4 cost tests failed before cost fixes. Review found generic Digi-Burst payment can still award benefits after only partial trash; two real ST4-13 regressions added. This remains open until paid-source semantics are repaired and checked.
- BT3-103: 10 passed / 3 expected failures (printed cost 0 versus YAML 4, decline consumption, unpayable-cost consumption). Shared stacked-reducer, prohibition, and clone regressions added before repair. First cost-hook attempt had a test-import compile error; corrected before behavioral measurement.
- EX3-018 and EX8-002: printed/image/DCGO audits found no YAML drift; additional Rust tests and legal sim-only scenarios ready. They are not yet oracle-confirmed.

### Additional verification scope

The current interaction denominator also contains 343 official-Q&A/risk-probe entries across these 52 cards. The existing generated readiness snapshot is stale and still contains unmeasured interaction entries. Clause coverage alone must not be reported as full oracle readiness: after the card/known-gap repairs, reconcile this interaction denominator and its evidence before any full readiness claim. BT14-009 remains separately unavailable.

### Latest verified checkpoints

- BT1-090 live oracle confirmed 3/3 compared steps after red Digi-Egg legality fix; backfilled source assertions from `oracle/tail-20261011T034743Z-b8bc9aaa6e`. Clause snapshot: 137 confirmed / 0 diverged / 0 unreachable / 1 unavailable / 60 unmeasured.
- Dynamic grant final focused suite: 11/11 passed in `verification/tail-reducer-coupling-red.log`; independent spec and quality approved. EX1-068 card promotion remains pending.
- Source protection: 17/18 passed before final counted-payment fix; sole RED was actual-removal count after late protection. Final 19-test candidate has spec and quality approval, execution pending.
- Cost reduction primary fixes: 85 cost-hook tests passed. Initial coupling tests had an invalid post-build effect-cache fixture; fixtures corrected, original split call path temporarily restored with exact external backups for trustworthy RED. New ordering test reproduces the hidden choice; unified restoration and further self-gated payment check pending. Do not count the earlier cache panic as behavior evidence.
- Interaction extractor now consumes the same reviewed image cache as canonical extraction. Regression RED (missing security text) then 89 tests GREEN. Regenerated denominator adds 54 records globally, removes/changes none, and `interactions --check` passes. Current 52-card pool has 289 distinct gating interactions (all still unmeasured); this supersedes the 285 snapshot. Per-card index totals include shared/non-gating rows and are not the distinct gating denominator.

### Continuation verification

- Shared primitive checkpoint: 178 passed (cost hooks 90, dynamic grants 11, effect-play controller 4, event identity 10, exact names 3, flood gates 41, source protection 19), recorded in `verification/tail-shared-primitives-green.log`. This precedes the additional reducer identity review below.
- Batch 2 focused card tests: BT3-103 13, EX3-018 7, EX8-002 7, ST2-13 19 passed. Source-payment migration regressions continue in `verification/tail-batch2-migrations-*.log`.
- Reducer review added six actual-payment identity/context regressions. After correcting a test API compile error and two main-phase test dispatch errors, all six show behavioral failures in `verification/tail-reducer-identity-final-red.log`. They cover hand/field compaction, removal of the declared card/host, replay of the same physical host card as a new Digimon, and cost-target context. Production repair is in progress; no identity-gap closure yet.
- Official-Q&A review groundwork: 136 exact Q&A packets for 40 cards, with separate GPT-family behavioral proposals. These are not oracle confirmations or terminal exclusions. Four core probe drafts and a separate-family Q&A authoring packet are in preparation.
- Pre-wired test discovery for the four planned missing cards BT13-096, EX3-065, BT9-109 and P-092; placeholders make no behavior/readiness claim.

- Completed card/migration run: 251 tests passed across 18 filters in `verification/tail-batch2-migrations-receipt.json` (includes the 46 batch-2 card tests). Do not add overlapping shared-suite counts as distinct coverage.
- Reducer identity repair: all 98 cost-hook tests passed in `verification/tail-reducer-mandatory-identity-green.log`, including six unified-chain regressions and final synchronous host-compaction/removal/replay coverage. The mandatory compaction RED is in `tail-reducer-mandatory-identity-corrected-red.log`; initial dispatch-error attempt is retained but is not behavior evidence. Independent spec and quality review approved both paths. Rebuilt harness in `tail-harness-build-3.log` includes these repairs.
- Four gating interaction probes independently reviewed, compared and recorded confirmed: EX3-020/041 inherited granted keywords and EX13-044 effect4 / EX13-045 effect7 optional declines. Evidence `oracle/interaction-20261011T041749Z-62bd807cf7`; source assertions promoted only after recorded confirmation.
- BT14-009 Q2374/Q5209 explicitly unavailable after exact script/class search at the recorded DCGO commit; receipt `verification/bt14-009-interaction-availability.json`. Current bound interaction count: 4 confirmed, 0 diverged, 0 unreachable, 2 unavailable, 283 unmeasured (289 total).
- Batch 3 audits found alternate-color breeding access gaps: LM-034 excludes breeding entirely; LM-036/051 already include breeding but their kind filters exclude DigiEgg. Actual-action/mask regressions authored before YAML changes; RED run pending. LM-051 is Alexandrite Memory Boost!. P-035 is the fourth queued worker in this batch.

### Batch 2 complete; batch 3 verification

- All seven batch-2 scenarios confirmed in `oracle/tail-20261011T043103Z-25f63078ff`: BT3-103 Main/Security, EX3-018 alternate route/face Evade/inherited Evade, EX8-002 inherited memory, ST2-13 Main/Security. Assertions promoted to source. Clause snapshot: 145 confirmed, 0 diverged, 0 unreachable, 1 unavailable, 52 unmeasured.
- Correction to the earlier batch-3 suspicion: LM-036/051's existing `any_field_permanent` already treats breeding DigiEggs as Digimon. No production change was needed. New tests initially called a pending-selection helper for Main actions; corrected to `game.decode_action` with legal-mask assertions. Latest LM-036 15/15, LM-051 15/15 and P-035 23/23 passed.
- LM-034 has real breeding-access RED. Changing only the old flood-gate's predicates remains insufficient (latest 15 pass / 2 fail: breeding behavior plus a separate newly introduced Delay test dispatch error). Worker is repairing the supported in-hand use requirement and fixture. Do not mark this card GREEN from static review.
- All four cards received initial spec/quality review; LM-034 approval is reopened for its corrected patch. Current harness build 4 includes the intermediate predicate-only change, not the forthcoming final LM-034 correction.
- Independent Claude-family drafts authored Q2070/Q1123/Q4704/Q7360, all legal in sim with no outcome assertions yet (`oracle/interaction-20261011T043504Z-6c8f6fb1e6`). GPT encoding and blind Claude verification/classification are separate saved artifacts under `interaction-review/encode-batch1`. Initial combined-schema invocation failed due duplicate nested `$id`; no model answer was produced. Retry strips only unreferenced diagnostic IDs, retaining all validation constraints.
- Corrected ruling packet/schema wording to describe pre-decision projection rows; existing 18 packet tests passed. Observing an action's resolved result requires a later scenario row, not its own decision row.

### Batch 3 final repair and batch 4 RED preparation

- LM-034 final repair uses the supported top-level `use_requirement` over `any_field_permanent`, replacing the inert in-hand flood gate. All 17 tests passed (`verification/tail-lm034-use-requirement-green.log`), and both spec and quality approved the final patch. Combined batch-3 focused tests: 70 passed. Harness build 5 contains this final correction. Live clause comparison is running in `oracle/tail-20261011T044730Z-796939783c`; do not infer completion until its results are reconciled.
- Batch 4 LM-027/028/055 regressions authored before production edits; LM-057 fresh audit dispatched. Audits identified legal evolution target filtering and independent optional source/result choices. Parent serializes Cargo and oracle; no concurrent harness rebuild.
- Official-Q&A batch 1 Q2070/Q1123/Q4704/Q7360: independently authored GPT expected outcomes and blind Claude verification/classification agree, with `line_exercises_ruling=true`. Applied only the reviewed `expect_ruling` blocks with semantic source checks (`interaction-review/encode-batch1/claude-retry1/reconciliation.json`). These remain unmeasured until live comparison; Q1123 intermediate reveal timing needs evidence-based quirk triage if it differs.

- Batch 3 live complete: all 10 scenarios confirmed in `oracle/tail-20261011T044730Z-796939783c`. Source assertions promoted only from recorded/backfilled results. Canonical clause snapshot now 160 confirmed / 0 diverged / 0 unreachable / 1 unavailable / 37 unmeasured (198 total).
- Batch 4 RED build started after the live queue cleared. LM-027/028/055 have expected optionality/filter failures; LM-057 existing YAML appears faithful and has three additional real-action regressions. Tests frozen before build.

### Batch 4 complete; measured interaction adjudication

- Batch 4 LM-027/028/055/057 passed 31/22/16/23 tests respectively (92 total), `verification/tail-batch4-green-receipt.json`; spec and quality approved. LM-027/055 now gate compatible evolution choices and independently allow result decline; LM-028 needed only optional result selection. LM-057 required no YAML change. The initial LM-027 lifecycle NoPendingSelection was a fixture dispatch error, corrected before GREEN; four other RED failures were behavioral. LM-055 mask assertions were corrected to distinguish candidate IDs from synthesized PASS.
- Next four core decline probes and Q2070/Q4704/Q7360 confirmed live in `oracle/interaction-20261011T045846Z-6fc2c575c1`. Q1123 retained `diverged` but independently adjudicated `dcgo_quirk` by both families: only the intermediate second-pick hand snapshot differs; final state and ruling agree. Raw diff, responses, and canonical writer receipt are under `interaction-review/triage-Q1123/`. Counts: 11 confirmed / 1 adjudicated divergence / 0 unreachable / 2 unavailable / 275 unmeasured.
- Five Memory Boost Q&A encodings accepted by both families. Initial Q4066/Q4070 verifier overlooked the mandatory evolution draw; unchanged lines reverified using official PDF rule8-1-3-3 and captured legal execution. No disputed block was applied before agreement.
- Q4149 requires an additional measurement: legal Option plays at a true Main decision. Rust focused RED3/4 then GREEN11/11; full exam suite394/394 after correcting an obsolete corpus-test single-clause restriction (docs/parser already support composed covers). Spec and quality approved. New isolated DCGO player compiled successfully; source patch/build receipts under `verification/option-observation-*`. Fresh runtime is separate from the prior player and shared base. Live observation validation remains pending. Harness build6 includes batch4 repairs and observation support.
- Batch5 audit/restore queue: LM-058, BT13-096 Homer Yushima, EX3-065 Hina Kurihara, P-092 Dracomon. All four tests frozen for RED build; production YAML remains absent for the three restorations. Potential shared gaps are source-play eligibility filtering (Homer) and refired effect attribution (Hina).

### Batch 5 behavioral RED and batch 4 live continuation

- Initial batch5 baseline: LM-058 18 passed; three missing production YAML files correctly failed their tests. Restoring exact archived YAML exposed behavioral RED: Homer 9 passed / 3 failed, Hina 7 / 2, Dracomon 4 / 1 (`verification/tail-batch5-restored-red-*.log`). Hina's departed-target failure was a fixture error: full deletion parked behind the already-pending choice, leaving the original card alive. The corrected test verifies actual removal and distinct replacement identity. Attribution failure is genuine.
- Repairs awaiting GREEN: Dracomon uses immutable exact event-card name; Homer gains free-effect-play candidate legality and immutable event-card kind predicates; Hina opts into target-card/kind attribution for refiring. Existing grantor attribution stays the default. Pack format advanced to 0.8.0 for the changed compiled layout. Spec review added real blue-token, full-field, and false/missing-candidate regressions (Homer now15 tests); token semantics and aura-before-rule-check timing remain open until executed.
- Batch4 live attempt `oracle/tail-20261011T052328Z-51d8427f32`: all seven Training lines confirmed. Four Scramble lines truncated without earlier divergence because DCGO auto-bottoms the sole remaining reveal card, while our engine exposes its singleton order choice. Proven existing `sim_only` adapter added to that exact step in LM-055/057 Main/Security and LM-057 decline. Fresh live reruns are required. JSON-formatted YAML also prevented textual backfill in some confirmed lines; preserve the raw evidence and convert/replay for durable assertions without claiming a new comparison.
- Q&A batch2 has seven independent GPT encodings awaiting blind Claude verification; Q3414 remains without a safe encoding because accepting both picks does not prove decline is illegal. A request-aware selection-decline observation is being designed with actual DCGO prompt capability. Q4149 has independent negative encoding plus a separate positive legality control, with blind verification running; the new isolated recorder still needs live validation.

### Batch 5 GREEN, recorder validation, and corrected denominator

- Batch5 finished 49 focused tests: Homer15, Hina9, Dracomon7, LM05818. Homer token-kind tests exposed a real aura/rule-boundary timing gap; refresh declarative effects at the existing state-based rules boundary before effective-DP checks. Judge quiz43 pass/1 pre-existing ignored; two new boundary tests pass;19 refire-consumer tests pass. Hina target attribution is opt-in; existing refire attribution stays default. Harness build7 includes these fixes.
- All four Scramble Main/Security reruns confirmed in `oracle/tail-20261011T054927Z-fde178597f`; exact singleton-order adapters retained and assertions promoted. Some JSON-formatted Training scenarios still need offline sidecar backfill after formatting conversion.
- Selection-decline observations passed13 focused tests and full exam408/408; malformed pairings fail before mutation. C# patch tests18/18. New isolated player build/patch receipts are `verification/selection-observation-*`; artifact655dd338207aaba13526e3a04f56847c9cffc9710dd8ddb4b157e272e12f8e40. Base source/catalog and action-space identity checked.
- Q4149 negative live confirmed; separate positive control compared9/9 rows and reports legal ST1-16. Control made no ledger writes. Q3414 compared6/6 rows, both selection-decline observations explicitly false, exactly one intermediate hand difference, later states and publisher ruling agree. GPT and blind Claude independently adjudicated `dcgo_quirk`; raw diverged verdict retained. Evidence `oracle/interaction-20261011T060033Z-2c3b280584`, `interaction-review/triage-Q3414`, and `verification/option-observation-positive-live.log`.
- All seven batch2 Q&A encodings plus Q3414 and Q4149 received both-family agreement and guarded application. Other batch2 and Memory Boost Q&A live runs remain queued, not confirmed by encoding alone.
- Reviewed correction consumption now shared by clause/interaction extraction. TDD reproduced wrong BT9-109 seven-clause extraction, then five correct clauses preserving inherited Option sections. Independent review found Security-label alias handling; two additional RED tests and79 GREEN tests fixed it. Earlier broader223 Python tests passed. Generated denominators refreshed without changing historical verdicts:196 campaign clauses and287 gating interactions. Reviewed LM057/058 wording changes invalidate old clause hashes. Current clauses171 confirmed/0 diverged/0 unreachable/1 unavailable/24 unmeasured; interactions12 confirmed/2 adjudicated diverged/0 unreachable/2 unavailable/271 unmeasured.
- Batch6: BT8-09731, EX1-06816, EX1-06621 focused tests pass. Ice Wall opts into dynamic grants, Analog Youth excludes unpayable suspension. BT9-109 initial missing-YAML RED then actual token-stacking and initial/late CannotDigivolve RED. Token placement repaired and13/15 passed; both prohibition guards now applied, final GREEN build running. No batch6 completion claim until verification/review.
- Batch5 scenarios simulated: all legal after repairing Homer DCGO-only host selector; Hina draft reordered native prompts before its activation,17 legal steps. Their live runs remain pending. New batch6 scenarios and eight additional core probes are being prepared. No aggregate required-suite pass or full fidelity claim.

- Terminology correction to earlier progress notes: LM-027/028 are Red/Blue Scramble; LM-055/057/058 are Training cards. The actual YAML, clause identities, observations and QAs were consistent; earlier shorthand reversed those family names. A subsequently reported source discrepancy was retracted after checking its own matching evidence.
- Final batch6 focused run83/83 and shared dynamic-grant/effect-evolution regressions21/21 pass; final spec and quality approved. Harness build8 contains token attachment and initial/late effect-evolution prohibition guards. Generated DSL vocabulary now178 steps/210 predicates/64 timings/14 kinds, structural drift check passes.
- Exact portable reviewed inputs imported:370 correction rows and443 referenced observations, preserving source bytes.249 Python tests pass; disabling both base-repo and DCGO access reproduces all196 canonical clause records and287 gating interactions. No generated compatibility data was edited by this import. Evidence `verification/portable-source-import*`.
- Five LM028 JSON-format backfill failures repaired offline from existing recorded sidecars:55/55 steps agree;601 assertion checks pass. Original source steps/rulings/assertions retained. Evidence `oracle/lm028-offline-backfill-20261011T061454Z/receipt.json`.
- Live batch5 attempt `tail-20261011T061331Z-406210e7c0`: LM058 needs the same exact singleton-bottom adapter as the other Training cards; Homer needs an explicit DCGO-only OptionalSkill row before its DCGO-only host selection. Both draft fixes are ready for retry. Homer/Hina security and P092 route confirmed so far. Hina Main exposed a genuine shared simultaneous-evolution observer ordering gap; it must receive TDD repair rather than a presentation-only adapter. See G-ENGINE-DIGIVOLVE-SIMULTANEOUS-OBSERVERS.
