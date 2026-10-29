# Meta admission follow-up — 2026-10-22

User approved the full recommended order. Worktree `D:/wt/p3-constructs`, branch
`claude/p3-construct-tests`, baseline `66762dc44b0756f1c64a692bd68a4b49d7ef6255`,
existing draft PR #745. Preserve action/tensor contracts and admission semantics.

Final status: all unblocked approved work is implemented. Eleven new YAML cards
and existing Grademon pass individual admission; four lifecycle-blocked cards
remain explicitly unready. Full-library admission is 1,407/5,834 lists and
44/447 strict-majority groups. Cumulative tier 2 completed with exit 1; final
full card/DSL runs retain only three independently reproduced baseline defects.
The [completion report](../research/2026-10-22-construct-tests/meta-admission-followup.md)
records exact results and the simulator/oracle/admission boundaries.

## Ordered work

1. **Glowing Dawn correctness.** Audit/fix BT25-043, ST23-04 and
   companion patterns ST23-05/ST23-08. Add strict behavioral regressions before
   changing YAML. Recompare existing oracle recordings where available; simulator
   tests alone cannot clear an oracle divergence.
2. **P-241 production data.** Reconcile central, split and registry data with the
   existing YAML/tests and printed image.
3. **Six shared constructs.** Add discriminating behavior/mask coverage for
   `predicate:own_memory_lte`, `verb:place_as_top_source`,
   `predicate:level_sum_gte`, `predicate:event_target_color_any_of`,
   `verb:grant_keyword_continuous`, and `predicate:play_cost_gte`.
4. **Chronomon mechanics.** Cover Giant Slayer (BT26-085) stack-trash immunity
   and successful/declined/rejected digivolution result tracking. Cover the
   singleton BT25-051 battle-result predicate if its existing capability permits.
5. **Chronicle/Alphamon package.** Complete all unblocked cards in the batches
   below. Resolve reusable prerequisites faithfully or record concrete blockers;
   do not substitute placeholders or omit rare tech silently.

## Chronicle pool and batches

Frozen recent input: 21 lists (18 Chronicle, 3 Alphamon), 35 unique IDs, 17 blocked
IDs. Thirteen have no YAML; four already have YAML but need evidence/behavior work.
EX13-060 is covered by stage 3. EX6-072 exists in `_examples` and must not be duplicated.

Full pool: BT16-082, BT19-089, BT20-010, BT20-012, BT20-015, BT20-018, BT20-037,
BT20-048, BT20-051, BT20-053, BT20-056, BT20-060, BT20-087, BT20-095, BT4-104,
BT7-090, BT8-097, BT9-103, BT9-104, EX13-049, EX13-055, EX13-057, EX13-060,
EX13-072, EX5-070, EX6-072, LM-031, LM-037, LM-054, LM-059, LM-061, P-039,
P-107, P-176, P-204.

| Batch | Cards | Initial mode |
| --- | --- | --- |
| 1 | BT20-010, BT20-012, BT20-048, BT20-051 | 4 IMPLEMENT |
| 2 | P-176, P-204, BT20-015, BT20-053 | 3 IMPLEMENT, 1 AUDIT |
| 3 | BT20-018, BT20-056, BT20-087, BT20-095 | 3 IMPLEMENT, 1 AUDIT; breeding prerequisite |
| 4 | EX5-070, EX6-072, BT7-090, BT9-104 | 3 IMPLEMENT, 1 AUDIT |

Verified prerequisites: ordinary effect digivolution currently rejects breeding
targets; BT20-018/056 explicitly need breeding and BT20-087 says on the field.
BT20-095's official errata permits any own level 3+ breeder; the local original
image's Chronicle restriction is superseded. BT20-053's 17 standalone tests are
now integrated. EX6-072's printed cost/recipe checks exist, but its effect DNA
declaration lifecycle remains blocked. BT7-090's image is resolved.

After those audits, the four blocked cards are recorded explicitly. The final
active batch combines BT20-095, EX5-070, BT7-090, and BT9-104; each retains its
own tests-first implementation and spec/quality reviews.

## Evidence and completion

Each card: printed image/data cross-check, observed failing behavioral test before
production changes, masks and negative/decline controls, spec then quality review.
Root coordinates Cargo in `E:/cargo-target-wt/p3-constructs`; workers do not launch
competing Cargo builds. Track simulator, oracle and admission evidence separately.

Run cumulative tier 2 verification, required broader checks, data consistency,
regenerate admission artifacts, and `git diff --check`. Preserve baseline suite
failures as such and do not replace aggregate failures with selected passes.

Baseline library snapshot (2026-10-19): 5834 lists / 447 canonical groups;
1019 admitted lists, 27 strict-majority groups. Recent frozen window has 846 lists
(2026-09-01 through 2026-10-17), 256 admitted. These are admission figures, not
independent end-to-end gameplay certification.

## Chronological evidence

Entries below retain the state at the time of each run. Later entries supersede
earlier pending/interim statuses; the companion completion report gives the
current evidence boundary.

- Pool resolved. First Glowing Dawn RED: 16 passed / 11 failed; first fixes GREEN:
  27/27. Companion RED: ST23-05 8/13 and ST23-08 4/9. Subsequent combined run
  65 passed / 8 failed, isolating plural prevention and most-security menu masks.
- Added reusable optional `all_matching` filter on atomic security prevention;
  verification pending. Existing ungrouped simultaneous tuck gap is not claimed closed.
- Review also found inherited Tamer-cost activation improperly gated on a Glowing
  Dawn host in ST23-04/08; regressions prepared before correcting the gate.
- P-241 data integrated at append-only index 4480; all prior assignments preserved.
  Registry and selected split-JSON checks pass; 9 integrity tests and 16 P-241
  card tests pass. Admission artifacts will be regenerated after the full campaign.
- All six shared construct test files prepared; runtime validation pending.
- Chronomon Giant Slayer construct tests prepared and per-card tests switched from
  obsolete stand-in to production BT26-060; runtime validation pending.
- Five corrected oracle scenarios pass 32 simulator assertions. Real node is
  healthy (`scripted-v20`); fresh oracle runs exposed a field-identity adapter
  compatibility defect. Harness regressions/fix in progress, no verdicts changed.
- Chronicle batch 1 test discovery wired. BT20-010 authoring underway; image audit
  found central/split trait `Beast Dragon` must be `Beast` (root-owned data fix).
- Shared run `shared-green-third.log`: 131 passed / 2 failed. Plural protection,
  both inherited non-Glowing-Dawn cost regressions, and face-down inherited-source
  visibility now pass. One failure was the battle-result fixture's unbound `self`
  target (corrected to `source`); the other exposed stale selected-permanent
  identity after field compaction, with a shared binding repair in progress.
- New plural-filter review rejects future claims beyond subject/global/source
  predicates: declared local bindings and effect-result predicates are evaluated
  without their original context by the batch finalizer. Regression tests precede
  a validator restriction. Current Habakirimon filters are unaffected.
- Oracle bridge review found controller/decider ownership ambiguity; four more
  regressions precede explicit target-owner metadata and fail-closed resolution.
- Chronicle batch 1 RED: 0/37, all missing production implementations. Four YAML
  files authored; BT20-012's first typed parse failed on scalar `zone`, corrected
  to a list. BT20-051 review requires playable-Kota filtering under effect-play
  prohibitions and field capacity; two new regressions await measured RED.
- Batch 1 metadata reconciled in central/override/split records: BT20-010 Beast
  and Rookie, BT20-012/051 Champion, BT20-048 Rookie, and secondary evolution circles.
- Breeding assessment confirms BT20-018/056/087 remain engine-blocked: the effect
  evolution lifecycle assumes battle-area targets at identity capture, parked
  choices, replacement zone, commit, result capture, and trigger dispatch. Hand/
  trash union selection, attack gating, breeding selectors and route enumeration
  already work. Do not implement battle-only approximations for these clauses.
- Follow-up shared run `shared-and-batch1-fourth.log`: 137/139 passed. All 13
  selected-host/result regressions, eight battle-result checks, and six requested
  construct shapes pass. Two validator fixtures failed schema parsing and have
  been corrected to canonical comparator syntax; rerun pending.
- Batch 1 now 35/39: printed-circle hydration is repaired; two strict free-play
  legality tests exposed BT20-051's missing candidate gate (fixed after RED).
  Two off-turn tests independently expose effect-digivolution charging the active
  player instead of the effect owner. Nine shared payment regressions are added
  before the three-site payment repair.
- Selected-host pins also need explicit V3 verification-digest coverage while
  retaining the legacy Debug projection. Five regression tests precede that fix.
- Explicit field-selection owner metadata is now emitted for generic field,
  Alliance, and blocker choices. Adapter refuses missing ownership; four new
  ownership tests observed RED. Static review approved; 12-test GREEN and fresh
  oracle attempts pending.
- BT20-095 printed Chronicle restriction on the breeding-move cost is reconciled
  in central, override, and split data. Current nine central-data integrity tests
  pass. BT7-090 image resolved via lookup CDN fallback for batch 4.
- Batch 2 read-only scouts: P-204 needs conditional Delay placement after paid
  discard (official Q5199), and its single event-gated attack Delay handles both
  owners. BT20-053 has no YAML drift; integrate 17 existing standalone tests and
  add discoverable fixed-inline breeding-play construct evidence.

- Combined `shared-batch1-final-green.log` is GREEN: 39/39 first-batch card
  tests and 157/157 shared/card checks. Both reviews approve all four new cards.
  This includes 10 payer/security-draw tests, eight selected-pin digest tests,
  all 14 plural-filter checks, and the earlier Glowing Dawn/Chronomon constructs.
  Ledger updated for the four implementations and six revalidated cards.
- Batch 2 is active: P-176/P-204 tests first; BT20-053 evidence integration.
  BT20-015 exposes a bounded prerequisite: attacker-scoped Option-only Security
  suppression. Ten inline tests precede modifier wiring and activation guards.
  Root corrected P-176 In-Training/inherited split metadata and BT20-015
  Ultimate/secondary-circle metadata.
- Batch 2 observed missing-card RED: 56/98 pass, with all 42 failures confined
  to the three not-yet-authored cards. Those YAML files are now authored. All 17
  registered Grademon tests and 11 inline breeding-play tests pass.
- Shared run `batch2-red-and-security-green.log`: 177/178 pass. The one failure
  exposes face-down inherited declaratives; the source scan now skips face-down
  sources after that measured RED. A separate checked-Option return-to-hand
  regression precedes any adjustment to activation-time suppression.
- Paid evolution needs an explicit cost-aware candidate predicate. Thirteen
  inline regressions are prepared; the existing route-only predicate retains its
  semantics for free/reduced callers. P-176's cost-19 mask test is unchanged.
- Oracle ownership audit identifies ten remaining native relative-field prompt
  assignments and one existing per-color absolute-band encoding. Sixteen harness
  tests precede the remaining fixes; no new oracle verdicts are claimed.
- Metadata follow-up: P-204 Security text moved from inherited to Security in
  central/override/split records; Grademon's Ultimate form corrected. EX5-070's
  printed name-alias rule restored in the same three data surfaces.
- Authored batch 2 run `batch2-authored-cost-red-retry.log`: 95/98 pass.
  P-176 has two real affordability failures; Hisyaryumon's sole failure was a
  fixture that expected an empty breeding phase not to auto-skip, now corrected.
  Shared checks: 179/202 pass. The 23 failures are 12 new cost-aware vocabulary
  checks, ten new breeding-move result checks, and the checked-Option movement
  regression (second queued clause incorrectly draws four extra cards).
- Security activation guard now relies on the active resolution's exact card,
  defender, kind and phase, even after an earlier clause moves pending_security.
  It continues to check the actual live attacker and leaves running bodies alone.
- Oracle expanded RED: 12/16 pass; three owner failures plus a count-capped
  fixture parse error. After adding the required prompt, its separate rerun
  also fails on missing owner. All four behavioral REDs are now established.
- Both batch 2 reviews approve the card shapes; P-176 remains conditional on
  the cost-aware repair and all new YAML remains pending final runtime evidence.
- BT20-095 prerequisite has 13 tests prepared: effect-move eligibility and an
  exact result binding captured before OnMove/Training observers. Ten observed
  schema REDs; three additional semantic-validator/subject controls await RED.
- BT9-104 read-only audit identifies reveal-zone evolution eligibility as a
  bounded extension of has_digivolve_candidate. Its printed revealed-card
  restriction and Security text are reconciled in central/override/split data.
- Batch 2 card GREEN: `batch2-cost-and-owner-green-attempt.log` records 98/98
  across both batches (P-176 10, P-204 14, Hisyaryumon 18, Grademon 17). Final
  card quality review approves all four. Shared result is 191/205: all 13
  breeding-move prerequisite tests are observed schema RED; the only other
  failure is a cost-test gauge expectation after automatic turn change, now
  corrected to the payer's relative gauge. All 11 Security-suppression tests pass.
- Cost-predicate review found a validation hole outside process binding scope:
  trigger/declarative filters bypass raw-callback rejection and nested formula
  validation. Five new DSL tests precede a general-predicate validation repair.
- EX6-072 receives two independent source audits and remains BLOCKED on
  `G-ENGINE-EFFECT-DNA-HAND-PARTNER-DECLARATION`. Printed cost and recipe backstop
  already exist, but correlated masks, both-material payment context and typed
  DNA resume/commit are missing. Existing one-host ordinary-evolution state is
  not a valid substitute. No new per-card runtime or oracle pass is claimed.
- Board-owner metadata changes preserve action IDs/wire formats but affect
  native pending-state verification digests (both LegacyV2 and V3). The active
  ten-golden corpus must be measured; any reviewed digest-only refresh must
  retain actions and original evidence. Do not hide semantic/action failures.
- Independent BT20-095 spec review found that the local image is pre-errata:
  Bandai's errata list page 2 permits moving any own level 3+ Digimon, without
  the Chronicle restriction. Central/override/split text is corrected to the
  post-errata version, and misplaced Security text moved to the Security field.
  Nineteen per-card tests are prepared, including non-Chronicle movement.
- Cost predicate validation and breeding-move result/eligibility primitives
  pass independent static review. Runtime checks remain pending. Removed the
  empty EX6-072 test placeholder and corrected stale example comments: printed
  DNA cost and recipe commit checks already exist; declaration/resume remains
  the documented blocker.
- `cost-predicate-validation-green.log`: all five general predicate validation
  regressions pass. `batch3-card-executable-red.log`: 106 passed, 20 failed solely
  because BT20-095 was not yet authored. Its YAML is now authored and spec
  approved after correcting Security scope. Alphamon's eight existing tests
  pass; its ledger remains PARTIAL for the absent breeding evolution tail.
- `batch3-shared-executable-green.log`: all 205 shared/card checks pass, including
  all 13 breeding-move result tests and all 13 effect-affordability tests.
  The final batch adds 14 Kota, 20 Proto Form, and 16 X Digivolution tests;
  all await missing-implementation RED. Twelve shared tests precede optional
  cost-aware host-candidate filtering and reveal-zone candidate support.
- Full-cost evolution remains legal under a reduction prohibition if payable.
  Final-batch fixtures were corrected before RED to distinguish affordable
  printed cost 3, unaffordable printed cost 19 at memory 8, and a true evolution
  prohibition. The earlier off-turn boundary tests already used the correct
  payer-relative gauge.
- Final-batch RED is measured: `final-card-executable-red.log` has 125 passed /
  51 failed (50 missing final-three implementations; one real BT20-095 failure).
  `final-batch-red-and-fellowship-green.log` independently records the same card
  result and 210/217 shared checks: four missing optional-cost schema failures
  and three revealed-candidate behavior failures, with five negative controls
  already passing. Candidate validation RED is 2/8.
- BT20-095's remaining failure exposed battle-area compaction decrementing the
  breeding sentinel in modifier targets and source attribution. The narrow guard
  now preserves that sentinel; a 14th breeding construct test checks both owners,
  repeated deletions, normal battle-index shifts, and unchanged breeding sources.
  Independent code review approves the fix; runtime GREEN pending.
- All four final YAMLs are authored and pass spec/quality review. Shared optional
  candidate cost and reveal-zone support are authored; full DSL lib and combined
  runtime checks are next. Corrected BT9's shared fixture to explicitly continue
  after host decline so its independent cleanup remains testable.
- Existing gap G-DSL-DIGIVOLVE-CANDIDATE-AFFORDABLE can be closed for the reusable
  capability after GREEN. P-250 was not migrated or tested by this campaign;
  keep that distinction explicit in the gap notes.

- Final DSL lib attempt (`final-dsl-lib-green.log`) is 53 passed / 2 failed.
  Both failures are example-count expectations (13 expected, 12 loaded), under
  independent baseline investigation; no full-lib GREEN is claimed.
- Quality review found two new-feature validation gaps: legacy traversal can
  skip nested cost-aware predicates, and reducer evaluation can recursively
  invoke its own affordability probe. New negative matrices precede a bounded
  global audit of the two cost-aware leaves and rejection in reducer evaluation
  contexts. Existing route-only predicates retain their contract.
- Final central-data integrity is 9/9 GREEN; all 13 changed split records match
  central data and every override field agrees. All 4,475 baseline registry
  assignments are preserved; P-241 adds index 4,480 (4,476 current entries).
- `cost-audit-matrix-red.log`: 32 passed / 13 failed. All new failures are
  accepted invalid semantic cases, not parse/build failures. The matrices cover
  41 entry contexts per leaf, nested semantic/binding filters, selector/branch
  scopes and reducer recursion (printed/free/zero/reduced costs). Bounded guard
  implementation follows this measured RED.
- The example-count failures predate this work: c88e90872 promoted BT7-107
  out of `_examples` but retained both 13-count assertions. Tests now require
  nonempty fixtures and compare registry count with loaded specs, retaining
  compilation of every example and the BT17-015 lookup.
- Prior aggregate baseline evidence remains visible: full DSL 1,350 passed /
  2 failed / 1 ignored (EX12-026/031 Aquatic and BT22-023 Holy Dragon data);
  full cards 12,261 passed / 4 failed / 36 ignored. Three card fixture failures
  were subsequently repaired and 37 affected-module checks passed; BT24-085's
  color expectation remains baseline. Fresh aggregate verification is required.
  The inventory helper's 27 compile errors match the baseline error multiset;
  focused DSL-lib checks do not claim that entire crate inventory is green.
- Replay preflight with the existing executable finds digest differences in
  all ten goldens. Originals remain byte-preserved. A constrained diagnostic
  will replay every original action with actor/mask checks and attribute any
  differences before an approved digest-only refresh is considered.

- `final-combined-green-attempt.log`: DSL lib 66/66, harness binary 19/19,
  harness lib 681/685, engine lib 468/474, Chronicle target 174/176 and shared
  target 217/218. This aggregate was RED; its filename is not a pass claim.
  The card/construct failures exposed two bounded defects: optional revealed
  result decline discarded independent cleanup, and replacement collection
  read a face-down inherited source. Both now have narrow repairs against
  those failing tests and independent static review. Reveal decline tests
  exercise live resume, cloned resume and the legacy callback.
- The four harness setup failures came from obsolete prompts in the historical
  ST23-04 scenario fixture. The fixture now follows the joined printed effect;
  divergence tests use an explicitly synthetic memory mismatch in temporary
  sidecars. Historical JSONL recordings and production comparison semantics
  remain unchanged. Engine unit fixtures now assert removed-source invalidation
  and explicitly provide Hybrid's printed evolution circle. The legacy hash
  constant is preserved on a documented archival clone, not replaced blindly.
- `replay-owner-attribution.log`: all 633 original actions applied legally,
  zero actor/mask failures; all 42 digest differences across ten games exactly
  match a clone-only removal of the newly populated owner metadata, constrained
  by typed prompt-producer provenance. Zero unexplained differences. Original
  replay bytes and live replay states were preserved. Final fresh verification
  and digest-only candidate review remain required.
- `final-combined-repaired.log`: DSL lib 66/66, engine lib 474/474, harness
  lib/binary 685/685 and 19/19, Chronicle 176/176, shared 218/218. The four
  final card records were added only after their 70 tests passed. Registry
  extraction succeeded: 1,196 effect registrations, 1,164 with production data,
  32 without; these are not the 4,476 card-index entries.
- The same fresh run supersedes the prior replay-only attribution for the final
  implementation: eight old paths now encounter mandatory remainder ordering
  after a P-103 reveal decline. That behavior change is caused by the repaired
  independent reveal tail. All eight also expose a pre-existing P-103 YAML
  error: its printed mandatory red-card addition was optional. A card regression
  and narrow Main-mask repair precede final replay migration. No old recording
  has been overwritten; these eight cannot be labeled digest-only refreshes.
- Static integration review found the route-only `can_digivolve_onto` consumer
  bypassing a moved/selected identity pin before installing a result prompt.
  `moved-route-predicate-red.log` measures the new post-observer-compaction
  regression failing at the missing legal choice (218 other tests filtered).
  The repair is bounded to pinned targets; no affordability behavior changes.
- Interim freshly compiled admission calculation after ledger regeneration:
  full frozen pool 1,225/5,834 lists and 40/447 strict-majority groups, versus
  1,019 lists and 27 groups at baseline; recent 412/846 lists. Chronicle/Alphamon
  remains 0/21 recent lists. Final metrics must be rerun after oracle and
  remaining verification; these values are not a completed training certificate.

- `p103-mandatory-red.log`: the new red-card choice test fails because PASS is
  legal; the no-match control passes. P-103's printed Main addition is mandatory
  (CR 4.3 §15-1-5); its existing YAML now uses `optional: false`. Delay and
  Security are unchanged. `final-integration-green.log` passes all 25 P-103
  tests and all 219 shared tests, including the moved-route identity regression.
- `final-dsl-schema.json` was exported by the freshly built schema executable;
  the generated guide and its freshness check pass. Final lowering coverage is
  175/183 verbs (169 behavioural), 181/218 predicates (171 behavioural), 57/65
  timings (55 behavioural), 39/39 keywords, and 1,073/1,162 cards covered.
- The fresh Glowing Dawn run confirms all five requested scenarios against
  DCGO: 91 paired rows across 93 simulator and 92 DCGO rows, with two declared
  simulator-only rows and one DCGO intermediate row. Simulator assertions pass
  32/32 across six blocks. No divergence, setup failure or player restart.
  `gd-fresh-oracle-summary.json` records the ten preserved raw files and hashes.
- The final replay migration is behavioural, not merely a digest refresh.
  Eight old paths decline P-103's mandatory red-card addition and are retired;
  seven ordinary paths regenerate with the same decks, seeds and original
  action budgets. Curated 001 is migrated by physical identity: two mandatory
  red additions and their sole-card remainder choices replace two illegal
  declines, retaining BT24-082's accept and the exact BT24-016 source context.
  Its independently reviewed candidate passes all 66 actions, including the
  real pending clone and seven-action suffix. Two untouched action paths
  (008/009) change only digests. Final corpus verification remains required.

- Fresh complete candidate verification passes 10/10 replays and 635 actions.
  The audited candidates and eight original retirement records are promoted;
  `qa/replay-goldens/NOTES.md` documents exact migrations and original budgets.
  The permanent curated regression passes 1/1 and infrastructure replay tests
  pass 10/10 (`replay-final-regressions.log`, `replay-corpus-infra-final.log`).
- Fresh oracle verdict recording clears only the five measured clauses. The
  backfilled scenarios pass 1,002 assertions in 97 blocks over unchanged 93
  steps; six unrelated legacy BT25-043 clauses remain semantically unmeasured.
  Admission refresh is 1,401/5,834 full lists and 43/447 majority groups;
  recent 480/846 lists and 31/106 majority groups, with zero regressions.
- Final package audit found two additional reusable test-evidence holds:
  `can_play_from_effect_without_cost` (BT20-051/095) and `is_source_permanent`
  (EX5-070). Dedicated behavioral construct controls are being added; the
  four genuine breeding/DNA lifecycle blockers remain separate.

- The two residual predicate families now have eight independently reviewed,
  real behavioral tests in `constructs::meta_play_and_source_predicates`; all
  eight pass in the rebuilt main DSL target. No production changes were needed.
  Fresh artifacts yield 183/218 predicates tested (173 behavioural),
  1,082/1,162 lowering-covered cards and 1,031 admissible indexed cards.
  The final frozen-pool calculation is 1,407/5,834 lists with 44/447 strict
  majority groups; recent 481/846 lists with 32/106 majority groups. No baseline
  list regressed. Counts preserve the existing aliases/canonical mapping.
- Cumulative tier 2 is running against the complete staged/working diff from
  `66762dc44b0756f1c64a692bd68a4b49d7ef6255`, with continue-on-failure enabled;
  its complete result will be retained in `tier2-final.log`.

- The first tier-2 attempt passed tier-0 checks and 174 tool tests, then was
  deliberately interrupted during the full DSL suite after default concurrency
  launched 16 workers. No pass/failure is assigned to the interrupted suite.
  `tier2-first-attempt-interrupted.log` preserves it. The complete ladder was
  restarted with `RUST_TEST_THREADS=2`; explicitly serial tier-2 suites retain
  their prescribed one-worker setting. No source changed between attempts.

- The first full DSL pass in `tier2-final.log` completed with 1,504 passed,
  four failed and one ignored. Two failures reproduce the documented baseline
  trait-data defects. The other two found bounded integration issues: P-176's
  reviewed whole-effect optional OPT was absent from the folded-OPT inventory,
  and five new blue Security clauses redundantly authored `scope: security`.
  P-176 was added to the explicit inventory after review of its decline,
  success and reset tests. BT20-095, BT7-090, BT9-104, EX5-070 and P-204 now
  retain `when: on_security` without the redundant scope annotation, matching
  the documented blue-icon syntax. The strict security lint is unchanged;
  both spellings lower to the same Security effect. The required later full
  card and DSL passes will measure these corrections. Refreshed artifacts
  preserve the final 1,407/5,834 and 44/447 admission results.
- `tier0-post-normalization.log` passes all 13 checks, including 174 tool
  tests, after the final five Security syntax and P-176 inventory corrections.
  Fresh independent packaging review matches all 44 table rows to the report
  JSON, confirms the 11 new YAMLs and four blockers, and finds no accidental
  scratch/build/log artifacts or machine-specific source/data paths among the
  161 changed paths. Working and staged whitespace checks are clean.
- The cumulative card run exposed two BT14-033 fixture expectations that
  omitted the now-correct mandatory bonus draw after evolution from security.
  `bt14-033-full-suite-red.log` reproduces 22 passing and two failing tests
  directly in the currently compiled full card binary. Independent review
  approved a fixture-only correction: assert the exact drawn card and depleted
  deck before the follow-up, then the exact hand/security identities after
  accept or decline. The accepted path also checks the evolved source stack;
  its old comment incorrectly saying Patamon remained the top card is removed.
  No production behavior changed for these failures. The later rebuilt full
  card run will measure the correction.
- The first complete cumulative card run finishes at 12,464 passed / three
  failed / 35 ignored. Only the two reviewed BT14-033 fixtures and exact
  BT24-085 baseline color assertion fail. The latter independently reproduces
  `[Green, Red]` versus `[Yellow, Red]` at the original line 78. The full
  ladder continues through its required rebuilt card and DSL targets.
- `tier0-final.log` passes 13/13 checks, including 174 tool tests, after the
  BT14-033 fixture correction and final artifact regeneration. Admission stays
  1,407/5,834 lists and 44/447 strict-majority groups; recent 481/846 and 32/106.
- In the rebuilt full card run, both corrected BT14-033 tests now pass with
  their exact physical draw/placement assertions. The remainder of that full
  run and the following DSL/replay/invariant targets are still pending.
- The final rebuilt full card run completes at 12,466 passed / one failed /
  35 ignored. The sole failure is the exact BT24-085 baseline color assertion;
  all 11 new cards and both corrected BT14-033 tests pass. The full DSL target
  is rebuilding next; no further source correction was needed.
- The final full DSL run completes at 1,506 passed / two failed / one ignored.
  P-176's folded-OPT inventory and the unchanged strict Security-icon lint both
  pass. Only the two exact baseline trait-data defects remain. The permanent
  curated replay regression also passes 1/1 in the cumulative ladder.
- Cumulative tier 2 completes with exit 1 and all scheduled checks executed.
  The final replay-golden verification passes 10/10 and invariant target passes
  5/5. Rules conformance passes 361 (one ignored), judge quiz 44 (one ignored),
  FAQ 31, keyword matrix three, and the selected determinism guard two (one
  ignored, 40 filtered). The four failed suite executions retain the first
  DSL/card failures and the final DSL/card baseline-only failures; the complete
  ladder is not labeled GREEN. Final report and PR wording were independently
  reviewed, including exact counts, all 44 admission rows and evidence limits.
