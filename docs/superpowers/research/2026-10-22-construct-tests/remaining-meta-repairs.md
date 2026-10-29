# Remaining meta admission repairs

This follow-up starts at `a74fd6951105ecb298d92f9c43577e95be55af5b` and
implements the user's request to fix the remaining baseline failures and the
two evolution capabilities blocking the four remaining cards. The earlier
[meta admission report](meta-admission-followup.md) retains its historical
source snapshots and failed aggregate results. This report records the new
verification separately.

## Four-card batch

| Card | Mode | Result | Focused tests |
| --- | --- | --- | --- |
| BT20-018 Ouryumon | IMPLEMENT | Full printed YAML; new behavioral module | 12 passed |
| BT20-056 Alphamon | AUDIT / authorized repair | Restored independent attack-only hand/trash breeding evolution tail; retained existing clauses | 19 passed |
| BT20-087 Kota Domoto & Yuji Musya | IMPLEMENT | Both field zones, explicit suspend cost and reduced-cost hand evolution; aliases, memory setter and Security | 12 passed |
| EX6-072 Mega Digimon Assembly! | AUDIT / authorized repair | Correlated DNA choices, printed payment, mandatory blue Security recovery and add-self | 12 passed |

Totals: two implementations, two audits/repairs, zero skips. The focused
`remaining_meta_cards` target reuses the modules registered in the full
`cards_behavioral` suite. All **55 tests pass** in the focused side target and
rebuilt full card suite in `remaining-tier2-final.log`. Independent actual-image
spec and code quality reviews approve all four cards. These are simulator results; there is
no new live DCGO confirmation for this batch.

Runtime RED preceded implementation. Initial BT20-018 and BT20-087 tests failed
because their production YAMLs were absent. BT20-056's four new attack-tail
tests failed because the printed choices were missing. EX6-072 initially
failed correlated masks, optionality and Security scope checks. Test-fixture
repairs preserve explicit legal choices: opponent-only selections use their
zone owner rather than a mixed-field action offset; simultaneous result
effects require a visible mandatory trigger-order choice.

## Shared mechanics

Breeding evolution retains the target's zone, physical identity and original
top card through route selection and resume. The measured consumers use hand
and trash sources, free or reduced initiating costs, existing breeding target
actions, one evolution draw and success-only clause results. Ordinary
When Digivolving, evolution observers and battle-area payment effects do not
activate for a breeding evolution. The eight shared breeding regressions and
the ten ordinary effect-cost-choice controls pass.

Hand-partner DNA now carries a typed declaration with both materials and the
result pinned in their original areas, selected recipe and orientation, payer,
cost policy, source context and continuations. Hand materials remain in hand
until the merge. The `dna_hand_candidate` predicate checks complete legal and
payable triples at the result, field and partner selection stages using clone
probes. Its seven runtime tests and all 69 DSL library tests pass. The 17
DNA declaration tests pass, including inactive-player payment, optional
interactive reductions, replacement choices, clone parity and distinct
equal-cost orientations.

The completed integration also fixes mixed ordinary/DNA payment ordering,
including three-deep ancestry; original context at field-reducer acceptance;
DNA provenance in parked DSL payment tails; and failed replacement cleanup
when its original subject disappears. Scalar observer conditions and bodies
now receive the same declared evolution flag and physical target context.
Their runtime RED evidence remains in the third/fourth integration,
`evolution-observer-and-security-red.log` and
`evolution-nested-fix-review-red.log` logs. All **111 focused tests** (55 card,
44 shared mechanics and 12 ordinary evolution controls), **474 engine unit
tests**, and **69 DSL library tests** now pass. Independent shared spec and
quality re-reviews approve the final repairs.

The actual BT17-095 Delay consumer's **four** compatibility controls cover
deletion and return-to-hand, recipe and nested replacement choices, canceled
DNA, physical card preservation and clone parity. They passed before any
Delay behavior patch; the earlier source-only suspicion of a lost
cancellation was withdrawn. Only an unused private wrapper was removed.

Security-to-breeding uses a separate Security-removal commit pipeline that is
not breeding-aware. Its route-only/cost-aware candidates and direct execution
now fail closed without mutation. This does not block these hand/trash
consumers. The expanded eight-test breeding target covers that boundary.

The seven correlated DNA tests live in
`tests/dsl/effect_dna_candidate_masks.rs` and are registered in the full DSL
suite. The standalone target imports that same module. This matters because
the gate scans named mechanics directories; the initial standalone filename
was outside its scan roots. Moving the unchanged tests credited the actual
behavioral evidence without widening or weakening the scanner.

Interactive `BeforePayCostObserve` bodies are an existing unimplemented
primitive, documented in `game_actions/cost.rs`; this follow-up does not claim
that primitive. Scalar observers and interactive reducers have separate
acceptance controls. Related field-pair and trash-partner DNA consumers are
not individually recertified by this four-card batch.

Broader compatibility testing then exposed six failures in three older DNA
targets. The hand-partner wrapper incorrectly equated the effect controller
with the material owner. Both materials and the result must belong to one
player, who pays and draws; the initiating controller remains separate for
immunity and replacement provenance. Two new regressions failed before the
correction, including a parked opposing-effect replacement with accept/decline
branches. Three older synthetic identity/reset fixtures now use one legal
6+5 orientation; all original assertions remain, while dedicated tests retain
the visible choice between equal-cost orientations. A follow-up control also
expected effect-battle Piercing to activate inside its enclosing callback.
It now checks that activation waits until the effect finishes (§15-8-3-2),
while retaining receipt isolation and exactly one later security check. Both
controls pass without any combat-source change. Independent review approved
the ownership and timing corrections. Failed aggregates and tests-only RED are
retained in `remaining-shared-domains.log`, `dna-owner-compat-first.log` and
`dna-opposing-owner-red.log`; the timing rerun is
`dna-followup-timing-green.log`.

The first full card executable also exposed an older BT17-095 clone harness
that assumed selecting the hand partner immediately merged when both material
orders were permitted. The harness now resolves the mandatory two-order choice,
checks that PASS is masked, and compares independent clones at the result,
partner, order and completed states. This is a test-only adaptation to the
new visible decision; no BT17-095 YAML or engine change was needed.
Independent review approved it. The first executable's failure is retained;
the corrected clone test passes a focused run (`bt17-delay-clone-green.log`).
Its temporary wrapper was removed after verification. The ladder's subsequent
full-card target rebuilt and verified the change across the complete suite.
The first full card result is **12,513 passed, one failed, 35 ignored**; that
single failure is the old clone harness. All 55 four-card batch tests and all
three repaired baseline guards pass in their respective full suites.
The rebuilt full-card rerun now passes **12,514 tests, zero failures, 35
ignored**. The three DNA compatibility targets also pass again (8 object-reset,
2 combat-follow-up and 4 target-identity tests). The earlier failed stage
remains part of the cumulative command's history; the full rerun does not
relabel that first aggregate as clean.

## Baseline repairs

Actual printed card evidence and independent reviews support the following:

- EX12-026 and EX12-031 include the printed Aquatic rule trait.
- BT22-023 uses the printed Holy Dragon trait.
- BT24-085's stale assertion now expects its printed green/red colors.

Central data, durable overrides and split records agree; all existing card
registry assignments are preserved. EX12-031 split generation also restores
its previously missing yellow level-4 cost-4 route to match unchanged central
data. The 21 production JSON integrity/split-pipeline Python tests pass.
The final full DSL run passes all 1,515 tests (one ignored), including both
previously failing trait guards. The corrected BT24-085 color guard also
passes within the full card run, confirming all three residual baseline
guards are repaired.

## Admission and evidence boundaries

Fresh registry export uses
`digimon_engine::cards::build_registry().registered_card_ids()` from the rebuilt
engine. It reports **1,198 registered IDs**, **1,166 with production data** and
**32 without**. These are distinct from the **4,476** indexed production cards.
The regenerated artifact admits **1,036** indexed cards, up from 1,031. Newly
admissible IDs are BT17-095, BT20-018, BT20-056, BT20-087 and EX6-072.

Lowering coverage records verbs **176/183 tested, 170 behavioral**; predicates
**184/219 tested, 174 behavioral**; timings **57/65 tested, 55 behavioral**;
keywords **39/39 behavioral**; and **1,086/1,164** YAML cards lowering-covered.
The mechanics artifact remains `ready`: 715 obligations, 248 proven,
152 not-testable, zero flagged blocking gaps and 315 unflagged open gaps. This
gate does not assert that every open gap is outcome-neutral. The independent
oracle report remains unchanged: 37 ready cards and zero library lists with
every clause and required interaction oracle-ready.

The unchanged frozen pool uses the same canonical grouping, source policy and
recent window (2026-09-01 through 2026-10-17). A strict majority means more than
half the lists, excluding ties. The comparison baseline is a74:

| Pool | Lists before | Lists now | Majority groups before | Majority groups now |
| --- | ---: | ---: | ---: | ---: |
| Full frozen library | 1,407/5,834 | **1,468/5,834** | 44/447 | **48/447** |
| Recent window | 481/846 | **503/846** | 32/106 | **34/106** |
| Recent Chronicle/Alphamon | 0/21 | **21/21** | 0/2 | **2/2** |

There are no lost majority groups or regressed admitted lists. The four new
full-library majorities are Alphamon (3/4), Chronicle (18/20), Omnimon (1/1)
and Omnimon Ace (1/1); the last two are single-list groups. Recent Alphamon is
3/3 and Chronicle is 18/18. All **35 unique IDs** in those 21 lists pass.
Among full-library majority groups, 28 have at least five lists. Existing
aliases remain separate, so 48 library labels is not a claim of 48 distinct
strategies. DNA Omnimon improves from 14/121 to 39/121 and still lacks a
majority. This is simulator training admission, not full gameplay or oracle
certification.

The frozen inputs and a74 artifact/registry exports are retained beside
`remaining-meta-admission-report.json` and its script. An installed Python wheel
is not used as the registry source. Before using these additions in Python
training, rebuild the PyO3 wheel from this branch; these results do not claim
that a training wheel has been deployed.

### Full-library majority labels

These are the current 48 labels in the frozen full-library pool; aliases and
single-list groups retain the existing grouping policy.

| Library label | Admitted / lists |
| --- | ---: |
| 3M | 6/11 |
| 3Musketeers | 1/1 |
| Alphamon | 3/4 |
| BeatBreak | 17/17 |
| Beelstar | 19/37 |
| Beelstar (53 cards) | 1/1 |
| Beelstar 3Musketeers | 1/1 |
| Chronicle | 18/20 |
| Chronomon | 33/33 |
| DATS Ravemon | 2/2 |
| DATS Rose | 4/6 |
| DATS RoseRave | 4/5 |
| Data Squad | 47/72 |
| Examon | 47/60 |
| Galactic | 6/6 |
| Galacticmon | 22/33 |
| Galatic | 1/1 |
| Glowing Dawn | 161/177 |
| GlowingDawn | 2/2 |
| Machine Black | 1/1 |
| Magneticdra | 18/33 |
| Medusamon | 139/149 |
| Mineral | 6/8 |
| Omni Ladder | 20/37 |
| Omnimon | 1/1 |
| Omnimon Ace | 1/1 |
| Rocks | 81/140 |
| ST-2 Cocytus Blue | 1/1 |
| ST-3 Heaven's Yellow | 1/1 |
| ST-5 Machine Black | 1/1 |
| ST-6 Venomous Violet | 1/1 |
| Saiyu Warriors | 51/51 |
| Shambala | 1/1 |
| Starter Deck Venomous Violet | 1/1 |
| Styracomon | 54/54 |
| TB | 13/14 |
| TS Ceresmon | 26/44 |
| TS Chronomon | 5/5 |
| TS Homeros | 1/1 |
| TS Jupiter-Merva | 2/2 |
| TS Jupitermon | 163/196 |
| TS Merva | 1/1 |
| TS Mervamon | 24/26 |
| TS Security Control | 2/2 |
| TS Zoo | 8/12 |
| Toho Braves | 142/143 |
| Vemmon | 52/56 |
| Virus Busters | 19/20 |

## Final verification

The final broad run passes **1,164 tests across 19 targets**: 474 engine unit
tests and 690 shared-domain/ordinary-evolution controls. This includes all
three previously failing DNA targets. `remaining-shared-domains-final.log`
records the successful aggregate; earlier failed aggregates remain preserved.
The 17 declaration tests separately pass, including both new ownership cases.

An earlier cumulative run was deliberately interrupted after the ownership
regression was found. Its completed full DSL (1,515 passed, one ignored), judge
quiz (44 passed, one ignored), and rules conformance (361 passed, one ignored)
results remain in `remaining-tier2-interrupted-for-dna-owner.log`; this is not
an aggregate pass.

The cumulative tier-2 command against the original campaign baseline
`66762dc44b0756f1c64a692bd68a4b49d7ef6255` completed **all 37 stages**:
36 passed and the first full-card stage failed on the old BT17-095 harness.
The command returns exit 1 because it retains that failure. After the reviewed
test-only correction, its scheduled full-card rerun passes all 12,514 active
tests. Every other stage passes. This records the failed initial aggregate
and successful full rerun separately; it does not call the original aggregate
clean. Production engine/DSL code stayed unchanged during this run; the only
included-test correction was the BT17-095 clone harness described above.

| Final applicable check | Result |
| --- | --- |
| Full cards, rebuilt rerun | 12,514 passed; zero failed; 35 ignored |
| Full DSL, both thread configurations | 1,515 passed; zero failed; one ignored per run |
| Rules conformance / judge quiz | 361 / 44 passed; one ignored in each |
| FAQ / keyword matrix / determinism guard | 31 / 3 / 2 passed; determinism helper ignored, 40 cases filtered |
| Broad engine/domain targets | 1,164 passed across 19 targets |
| Four-card/shared/ordinary focused targets | 111 passed; overlaps the full/domain suites |
| DSL library / production-data Python checks | 69 / 21 passed |
| Tier 0 after final evidence refresh | All 13 checks pass, including 174 tool tests |
| Active replay goldens | All 10 recordings and their 635 actions verify |
| Invariant fuzz smoke | All five tests pass |

The vocabulary synchronization and resolved-gap citation checks also pass.
Replay recordings were verified unchanged; none was re-blessed or retired in
this follow-up. The results belong to the existing draft
[PR #745](https://github.com/mammalwithashell/digimon-deck-list-builder/pull/745).

The final evidence-note refresh preserves the same admission decisions and
passes all 13 tier-0 checks, including 174 verification-tool tests
(`remaining-tier0-final.log`). The frozen admission report was recomputed
after that refresh and retains the counts above with zero regressed lists.

`AGENTS.md`'s older ledger-only admission description was inconsistent with
the live training runbook and runtime. Its admission section now describes the
two committed artifacts, compiled/data playability requirement, evidence gate,
and strict-majority denominator without adding snapshot counts.

The optional `implementation_inventory` binary still reports 27 existing
exhaustiveness errors outside the green verification claim. Separate sim-only
exam CI at baseline a74 reports 1,096/1,171 passing and 75 failing scenarios,
none from this final four-card/Delay batch. That broader legacy corpus is not
recertified here; the failures are not presumed to share one cause.

Local evidence is under `.superpowers/sdd/2026-10-22-meta-admission/`.
