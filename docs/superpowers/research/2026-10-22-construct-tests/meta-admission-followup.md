# Meta admission follow-up: implementation and evidence boundary

The subsequent [remaining meta repairs](remaining-meta-repairs.md) report tracks
the four cards and three baseline suite failures left by this snapshot. The
results below remain the historical a74 verification and admission record.

This report covers the approved follow-up to the construct campaign and App Fuse
repair on PR #745. The implementation adds 11 production YAML cards, repairs
existing card flows, and extends shared selection, payment, protection and
validation behavior. Focused card/shared checks and library tests pass; the final
full card and DSL suites retain only the documented baseline failures. Five
repaired scenarios have fresh DCGO confirmation. The frozen
library now admits 1,407 of 5,834 lists, with 44 strict-majority canonical groups.
The ten reviewed replay recordings, permanent curated regression and ten replay
infrastructure tests pass. Cumulative tier 2 completed with a nonzero result;
the failed aggregates and corrected final-source results are preserved below.
These evidence scopes remain separate.

The [execution plan](../../plans/2026-10-22-meta-admission-followup.md) records
the chronology, observed RED runs and reviews. The campaign baseline is
`66762dc44b0756f1c64a692bd68a4b49d7ef6255`. The earlier
[construct completion report](completion-report.md) and
[App Fuse follow-up](app-fuse-opt-followup.md) retain their own evidence boundaries.

## Approved work and final implementation scope

| Order | Work | Implemented or assessed scope |
| --- | --- | --- |
| 1 | Glowing Dawn correctness | BT25-043 and ST23-05 expose only legal most-security payment choices, including ties and empty stacks. Their security payment and plural departure prevention are atomic. ST23-04 and ST23-08 keep mandatory DP work and optional paid play/use in one printed effect; inherited Tamer-source payment remains available before checking the host's Glowing Dawn trait. |
| 2 | P-241 production data | Reconcile the existing card with central data, overrides, split JSON and the append-only registry. The existing YAML and tests are reused. |
| 3 | Six shared constructs | Add behavior and action-mask controls for `own_memory_lte`, `place_as_top_source`, `level_sum_gte`, `event_target_color_any_of`, `grant_keyword_continuous` and `play_cost_gte`. EX13-060 is covered in this stage. |
| 4 | Chronomon mechanics | Exercise BT26-085 Giant Slayer stack-trash protection and successful, declined, rejected and stale-host evolution results. Replace its obsolete stand-in fixture with production BT26-060. Add BT25-051-shaped battle-result predicate controls. |
| 5 | Chronicle/Alphamon package | Process the 16 assigned cards below, author all 11 unblocked missing YAMLs, integrate Grademon evidence, and retain four explicit capability blockers. |

Final integration also corrects P-103 Offense Training's Main search: adding a
matching red card is mandatory, while zero matches still allow remainder
cleanup and Delay placement. This is an existing-card repair, not a twelfth
new YAML. The shared optional-reveal continuation now skips a declined pick's
scoped body while preserving independent following steps.

The existing EX6-072 YAML remains in `_examples`; it is not duplicated. BT20-056
retains its implemented effects and PARTIAL boundary. Its comments now identify
the actual missing lifecycle rather than incorrectly claiming attack predicates
or hand/trash selection are absent.

## Chronicle/Alphamon denominator

The frozen recent input contains **21 lists: 18 Chronicle and 3 Alphamon**, with
**35 unique IDs**. Initially 17 IDs failed admission: 13 lacked YAML and four had
existing YAML with evidence or behavior holds. EX13-060 belongs to shared stage 3;
the remaining **16 cards were assigned to card batches**. That is a processing
denominator, not 16 successful implementations.

The complete 35-card pool is:

BT16-082, BT19-089, BT20-010, BT20-012, BT20-015, BT20-018, BT20-037,
BT20-048, BT20-051, BT20-053, BT20-056, BT20-060, BT20-087, BT20-095,
BT4-104, BT7-090, BT8-097, BT9-103, BT9-104, EX13-049, EX13-055,
EX13-057, EX13-060, EX13-072, EX5-070, EX6-072, LM-031, LM-037,
LM-054, LM-059, LM-061, P-039, P-107, P-176 and P-204.

| Assigned cards | Final code scope | Evidence boundary |
| --- | --- | --- |
| BT20-010 Ryudamon, BT20-012 Ginryumon, BT20-048 Dorumon, BT20-051 Raptordramon | Four new YAMLs and card test modules | Included in the passing 176-test combined card target. |
| P-176 Dorimon, P-204 Release of the Sealed Knight!, BT20-015 Hisyaryumon | Three new YAMLs and card test modules | Included in the passing 176-test combined card target. |
| BT20-053 Grademon | Existing implementation audited; 17 standalone tests integrated into card discovery; inline breeding-play controls added | Included in the passing combined card and shared targets. |
| BT20-095 Fellowship of Hope's Keepers, EX5-070 X Antibody Proto Form, BT7-090 Kota Domoto, BT9-104 X Digivolution! | Four new YAMLs with separate tests and spec/quality reviews | All 70 final-batch tests pass within the 176-test combined card target. No new oracle claim for these cards. |
| BT20-018 Ouryumon, BT20-056 Alphamon, BT20-087 Kota Domoto & Yuji Musya, EX6-072 Mega Digimon Assembly! | Capability assessment and accurate blocker documentation | Four blocked cards; no full-card runtime or oracle certification. |

The 11 new YAMLs are exactly the four, three and four implementations in that
table. P-241, Grademon, Alphamon and EX6-072 are existing implementations, not
additional new YAMLs.

The final evidence-hold audit checks all 11 new cards plus existing Grademon
against the freshly compiled effect registry, production card data and generated
admissibility artifact. **All 12 are present and individually admissible, with
empty rejection reasons and no untested constructs.** The new cards have
`IMPLEMENTED` ledger status; Grademon has `AUDITED-OK`. Dedicated reusable tests
for `can_play_from_effect_without_cost` and `is_source_permanent` removed the
remaining test-evidence holds on BT20-051, BT20-095 and EX5-070. They discriminate
legal masks, physical instances, controller/destination ownership, boolean
polarity and inherited carrier identity; they are not parse-only coverage.
All six admissibility input fingerprints match the current files. The four
capability-blocked cards below remain denied; their status was not promoted.

BT20-018, BT20-056 and BT20-087 require
`G-ENGINE-EFFECT-DIGIVOLVE-BREEDING-TARGET`. The ordinary effect-evolution path
assumes battle-area storage during identity capture, parked payment choices,
replacement attribution, commit, result capture and trigger dispatch. A faithful
repair must carry zone-pinned identities through those stages and suppress the
ordinary effects that breeding evolution must not trigger. Breeding selectors,
hand/trash union selection, attack gating and route enumeration already exist.
Moving a breeder to battle and then evolving it, as BT20-095 does, does not close
this capability gap.

EX6-072 requires `G-ENGINE-EFFECT-DNA-HAND-PARTNER-DECLARATION`: correlated
result/material legality and payment masks, both physical materials and origins,
recipe orientation, payer context, interactive reducers, and DNA-specific
revalidation/resume/commit. Printed DNA-cost lookup and commit-time recipe checks
already exist. The ordinary one-host evolution declaration cannot substitute
for this lifecycle. The assessment also identifies hand-partner stacking and
inactive-player payment for that future work; it does not certify other DNA
consumers. See [engine gap acceptance controls](../../../../qa/archetype-qa/engine-gaps.md).

## Shared behavior and authoring contracts

The shared changes retain existing action IDs and observation/tensor contracts.
Legal choices remain exposed through pending selections. The latest focused
shared run passes 219 tests; the broader completion gates are listed separately.

- **Payment and prevention:** optional bottom face-down Tamer-source payment
  supports the joined Glowing Dawn effects. Atomic top-security payment can
  prevent all matching departures in the current batch. Its deferred filter
  rejects local bindings and effect-result dependencies that the batch finalizer
  cannot reconstruct. This does not close the separate ungrouped simultaneous
  departure gap.
- **Physical identity and result handling:** selected permanent bindings follow
  the same incarnation through pin-aware consuming steps, field compaction,
  evolution, cloned choices and resumed tails. The route-only
  `can_digivolve_onto` predicate now follows a selected or moved identity pin
  before installing its result prompt; a departed pinned host fails closed.
  Unpinned/source fallback and omitted-cost semantics are unchanged. Other
  legacy predicate/formula consumers of raw bindings are not claimed globally
  pin-aware. V3 verification includes selected pins while retaining the legacy
  projection.
- **Effect payment ownership:** ordinary effect evolution pays from the declared
  acting player's relative gauge (the evolving Digimon's controller for the DSL
  step), including off-turn execution; reducer and draw controls distinguish
  the payer from the active player. This does not repair
  the separately blocked DNA declaration flow.
- **Paid evolution candidates:** `can_digivolve_onto_with_cost` and optional
  `has_digivolve_candidate.cost` use the existing clone-only payment probe with
  the consuming effect's printed, free, fixed or reduced cost. Cost formulas
  retain the original effect source and bindings. Omitted cost keeps route-only
  behavior. The actual commit still exposes route and payment choices.
- **Revealed candidates:** `has_digivolve_candidate` supports `reveal` and filters
  the shared pool by each physical card's owner. BT9-104 retains independent
  remainder cleanup and source placement after declined evolution, with the
  evolution draw occurring before remainder cleanup. Optional reveal decline
  runs the independent tail in live, cloned and legacy callback continuations.
- **Breeding movement:** `can_move_from_breeding_by_effect` gates the real move;
  `move_from_breeding.bind_as` captures the moved identity before observers and
  clears failed or departed results. Battle-area compaction preserves the
  breeding sentinel in modifier targets and source attribution.
- **Security activation and source visibility:** attacker-scoped suppression
  affects Option Security activation only. It tracks the active checked card
  even when an earlier clause moves it and does not cancel a running body.
  Face-down source cards do not supply inherited effects or declaratives,
  including replacement-candidate collection.
- **Oracle identity bridge:** relative field prompts carry explicit target-owner
  metadata, including native producers and resumed count-capped choices. The
  adapter fails closed on absent ownership and accepts an existing absolute
  action band only when it agrees with that owner. The wire action encoding is
  preserved. Pending-state metadata changes can affect replay digests.

The validation audit is bounded to the two cost-aware predicates. A complete
walk of the serialized typed schema reaches top-level, declarative, nested
predicate, comparator/formula and step-body locations skipped by older visitors.
Cost payloads receive semantic and lexical binding validation; same-selector
filters cannot read their own future binding, sibling branches are isolated,
and scheduled/granted declarations do not leak into the enclosing tail.
Raw-Rust cost callbacks are rejected because mutable and read-only evaluation
do not share a contract. Both cost-aware forms are rejected throughout
`cost_reduction` and its `cost_assignment` alias: affordability itself evaluates
reducers and could otherwise recurse. Free and fixed-zero costs are included;
the older route-only form remains available. The broader legacy validator is
not claimed fully audited or repaired.

The generated DSL schema and [authoring guide](../../../RUST_DSL_AGENT_GUIDE.md)
reflect these additions. The refreshed lowering index reports **175/183 verbs,
183/218 predicates, 57/65 timings and 39/39 keywords tested**, with
**1,082/1,162 cards lowering-covered**. Behavioral subsets are 169 verbs,
173 predicates, 55 timings and all 39 keywords. These are construct coverage
counts, not oracle or admission totals (`lowering-final-refresh.log` and
`lowering-final-check.log`).

## Production data

The **13 changed split records** are BT20-010, BT20-012, BT20-015, BT20-048,
BT20-051, BT20-053, BT20-095, BT7-090, BT9-104, EX5-070, P-176, P-204 and
P-241. Central data, override fields and those split records were reconciled.
Corrections include stage/trait and secondary evolution-circle data, P-176's
inherited text, P-204's Security field, EX5-070's name-alias rule, and BT9-104's
revealed-card restriction and Security text.

BT20-095 uses the official errata permitting **any own level 3 or higher
breeding Digimon** for its move cost. The original image's Chronicle restriction
is superseded; its misplaced Security text is also corrected. Chronicle remains
relevant to the separately specified trigger/result conditions.

P-241 adds registry index **4,480**. All **4,475** baseline assignments are
preserved, giving **4,476** entries; the highest index is not the entry count.
The recorded central-data integrity run is **9/9 passed** (`data-final.log`).
The plan also records equality of all 13 split records and override fields.
These data checks do not establish full card behavior.

## Recorded evidence and completed verification

Earlier scoped results below are measured results on their respective source
snapshots. Later passing scopes do not erase a failed aggregate or replace a
separate replay, oracle or admission check.

| Earlier check | Recorded result | Local log under `.superpowers/sdd/2026-10-22-meta-admission/` |
| --- | --- | --- |
| First Glowing Dawn repair | 27 passed | `gd-green-first.log` |
| First Chronicle batch plus then-current shared/card scope | 39 passed; 157 passed | `shared-batch1-final-green.log` |
| First two Chronicle batches | 98 passed; the shared companion run still had 14 failures | `batch2-cost-and-owner-green-attempt.log` |
| Shared/card checks after the breeding-move prerequisite | 205 passed | `batch3-shared-executable-green.log` |
| Initial general cost-predicate validation regressions | 5 passed | `cost-predicate-validation-green.log` |
| Bounded validation audit before its repair | 32 passed / 13 failed; accepted invalid cases, not schema/build errors | `cost-audit-matrix-red.log` |
| Final-batch preimplementation/card regression run | 125 passed / 51 failed: 50 missing implementations and BT20-095's breeding modifier loss | `final-card-executable-red.log` |
| Earlier complete DSL-library attempt | 53 passed / 2 failed: stale example-count assumptions | `final-dsl-lib-green.log` |
| First cumulative full DSL run | 1,504 passed / 4 failed / 1 ignored: two baseline trait-data failures and two integration issues described below | `tier2-final.log` |
| First cumulative full card run | 12,464 passed / 3 failed / 35 ignored: the baseline BT24-085 assertion and two BT14-033 draw expectations | `tier2-final.log` |

The example-count failures predate this campaign: `c88e90872` moved BT7-107 out
of `_examples` while leaving two assertions expecting 13 examples. The assertions
now require nonempty fixtures and compare registered count against loaded specs,
while retaining compilation and the known-card lookup. The later DSL library
run passes all 66 tests; the old log's filename is not a GREEN result.

The first cumulative DSL run found P-176 missing from the explicit reviewed
folded-OPT inventory, and five newly authored blue Security clauses using a
redundant `scope: security`. P-176's inventory entry is now backed by its
decline/success/reset regressions. BT20-095, BT7-090, BT9-104, EX5-070 and P-204
now use the documented `when: on_security` form; their runtime lowering is
equivalent and the strict Security-icon lint is unchanged. Both corrected
checks pass in the final full DSL run, which finishes at 1,506 passed / two
failed / one ignored. The two remaining failures exactly match the baseline
trait-data failures documented below.

The first cumulative card run found two BT14-033 tests that omitted the
mandatory bonus draw after evolution from security. A direct run of that
module reproduced 22 passed / 2 failed (`bt14-033-full-suite-red.log`). The
fixture-only correction now checks the exact drawn card and deck before the
follow-up, then physical hand/security contents after accepting or declining
placement. It also verifies the evolved source stack and corrects an obsolete
comment claiming Patamon remained the top card. Independent review approved
the correction; both tests pass in the later rebuilt full card run. That run
finishes at 12,466 passed / one failed / 35 ignored. Its sole failure is
BT24-085's exact baseline color assertion: actual `[Green, Red]`, expected
`[Yellow, Red]`, reproduced in `bt24-085-current-baseline-comparison.log`.

The first combined attempt also remained RED: `final-combined-green-attempt.log`
records DSL 66/66, harness binary 19/19, harness library 681/685, engine library
468/474, card target 174/176 and shared target 217/218. The card/shared failures
exposed discarded cleanup after optional reveal decline and face-down inherited
replacement collection. The harness failures used obsolete split-effect prompts;
their updated fixture retains negative comparison coverage through explicitly
synthetic temporary sidecars, leaving historical JSONL untouched. Engine fixtures
now provide the Hybrid's printed evolution circle and assert removed-source
invalidation alongside surviving queue/context rekeying. The historical digest
constant is checked on an archival-only clone rather than replacing it blindly.

| Latest completed runtime scope | Measured result | Local evidence log |
| --- | --- | --- |
| DSL library, including bounded validation audit | 66 passed | `final-combined-repaired.log` |
| Engine library | 474 passed | `final-combined-repaired.log` |
| Oracle harness library and binary | 685 passed and 19 passed | `final-combined-repaired.log` |
| Combined Chronicle/card target, including the final four cards | 176 passed | `final-combined-repaired.log` |
| Shared integration after the moved-target route repair | 219 passed | `final-integration-green.log` |
| P-103 existing and new card regressions | 25 passed | `final-integration-green.log` |
| Final reusable free-play/source-identity predicate controls | 8 passed in the normal DSL target; 1,501 other tests filtered | `final-two-predicates.log` |
| Final rebuilt full card suite | 12,466 passed / 1 reproduced baseline failure / 35 ignored | `tier2-final.log` |
| Final rebuilt full DSL suite | 1,506 passed / 2 reproduced baseline failures / 1 ignored | `tier2-final.log` |
| Judge quiz, rules conformance and FAQ | 44 / 361 / 31 passed; one ignored in each of the first two targets | `tier2-final.log` |
| Keyword semantics matrix and filtered determinism guard | 3 / 2 passed; guard has 1 ignored and 40 filtered | `tier2-final.log` |
| Standalone Grademon and permanent curated replay | 17 / 1 passed | `tier2-final.log` |
| Replay goldens and invariant target | 10 recordings verified / 5 tests passed | `tier2-final.log` |

The earlier library and combined card measurements precede the moved-route and
P-103 repairs; the final full card and DSL runs include those repairs. The combined log
also contains a **failed replay diagnostic**, so it is not an overall successful
command: one report-extraction test passed and one replay test failed. Its 252
applied original actions, eight actor/mask failures and eight unexplained digest
differences exposed the P-103 continuation change; no illegal actions were forced.

| Final completion gate | Current report status |
| --- | --- |
| DSL/engine libraries and combined card/shared scopes | Measured passes above, with source boundaries retained |
| Oracle adapter/harness regression run | 685 library and 19 binary tests passed |
| Fresh DCGO comparisons for the five repaired scenarios | 5 confirmed, detailed below |
| Deliberate corrected replay 001 candidate | 66/66 rows verified; historical and physical-choice reviews complete |
| Reviewed replay corpus | 10/10 recordings verified across 635 actions and promoted; permanent curated regression 1/1 and replay infrastructure 10/10 passed |
| Cumulative tier 2 and required broader checks | All scheduled checks completed; exit 1, with failed aggregates retained and only three baseline defects in the final full card/DSL runs |
| Artifact freshness | Final tier 0 after the BT14-033 fixture correction: 13/13 checks, including 174 tool tests (`tier0-final.log`); schema/docs current and both diff checks clean |
| Compiled-registry/card-data extraction and final admission | Completed; full and recent denominators below |

The cumulative command ran against the staged/working diff from the campaign
baseline with `--continue-on-failure`. It recorded four failed suite executions:
the first DSL run, first full card run, final full card run and final full DSL
run. The first two exposed the integration/fixture corrections described above;
the latter two verify those corrections and retain only the three independently
reproduced baseline defects. The overall exit status remains **1**. No aggregate
failure is replaced with a focused pass, and no whole-repository GREEN is claimed.

Fresh DCGO runs confirm **all five repaired scenarios**, with **91 paired rows
from 93 simulator rows and 92 DCGO rows**. The difference consists of two
simulator-only rows with no DCGO prompt and one noncomparable DCGO intermediate
row in BT25-043 effect 3. The authored simulator checks pass **32 assertions in
six assertion blocks**. The other four scenarios are ST23-04 effect 0, effect 1,
effect 2 and inherited 0. `gd-fresh-oracle-summary.json` records zero divergence,
setup failure or unmeasured result within this five-scenario denominator and
preserved recording hashes. Six other legacy BT25-043 clauses remain unmeasured;
these results do not certify the entire card or the new Chronicle package.

Those five confirmed clause verdicts and their comparison evidence are now
recorded durably. Assertion backfill preserves the scenarios' original steps
and other fields; all five post-backfill simulator runs pass **1,002 assertions
in 97 blocks across the same 93 authored steps**. This is expanded regression
coverage from the existing recordings, not a second live oracle run or 1,002
independent oracle observations. `gd-durable-recording-audit.json` records the
changed-file hashes and confirms that the other six BT25-043 clause verdicts
were not promoted.

Replay preflight originally found differences in all ten goldens. An earlier
snapshot attributed 42 digest differences across 633 legal actions to owner
metadata, but the final continuation repair superseded that conclusion for
eight paths: old P-103 declines now encounter required cleanup, and the printed
Main search itself must not allow those declines. This is a behavior migration,
not a digest-only refresh.

For curated replay 001, an isolated archival runner first reproduced all **64
original actions/digests** (`replay-001-historical-trace.log`). It restored only
the documented historical P-103 behavior and used constrained clone-only owner
normalization. A separate corrected production runner then verified a reviewed
**66-action candidate** (`replay-001-candidate-run.log`): two illegal historical
declines become explicit mandatory red-card picks, with two additional explicit
sole-card remainder orders. All other decisions preserve their physical sources
and targets, and every action checks the actor and legal mask. The candidate
retains accept action 59 from physical source 43, exactly two security-removal
observers attributed to physical source 27, and a real pending Hand-selection
clone followed by all seven suffix actions. Their new zero-based anchors are
45, 55 and 58. Seed, decks, format and engine schema remain unchanged; the card
hash intentionally updates to the current loaded data. The candidate passed an
independent fresh `ReplaySession` verification before being written under ignored
scratch storage. Original bytes remain preserved. The reviewed full candidate
corpus now passes **10/10 replay recordings across 635 actions** and has been
promoted: two recordings retain their original actions with digest updates,
seven receive fresh legal paths at their original seeds/decks, and curated 001
uses the explicit physical migration above. `replay-candidate-audit.log` and
`replay-candidate-verify.log` retain the review and verification evidence. The
permanent replay regression passes 1/1 (`replay-final-regressions.log`); all 10 replay infrastructure tests pass (`replay-corpus-infra-final.log`).

## Admission measurement boundary

The final calculation uses the same frozen input and unchanged canonical-name
mapper as the baseline. `final-admission-report.json` and
`final-admission-report.log` record:

| Frozen scope | Lists | Admitted before → now | Canonical groups | Strict majorities before → now |
| --- | ---: | ---: | ---: | ---: |
| Full library, snapshot 2026-10-19 | 5,834 | 1,019 → 1,407 | 447 | 27 → 44 |
| Recent window, 2026-09-01 through 2026-10-17 | 846 | 256 → 481 | 106 | 14 → 32 |
| Recent Chronicle/Alphamon package | 21 | 0 → 0 | 2 | 0 → 0 |

There are **zero regressed lists and zero lost majority groups** in either the
full or recent comparison. Among the full-library majorities, **27 groups have
at least five lists**; the remaining 17 have smaller denominators. In the recent
window, 23 of the 32 majority groups have at least five lists. These counts
follow the existing mapper, not a claim of 44 distinct strategies. For example,
`Glowing Dawn` and `GlowingDawn`, and several starter-deck aliases, remain
separate labels. No alias cleanup changed the comparison denominator.

The current 44 full-library majority labels are:

| Canonical label | Admitted | Total lists |
| --- | ---: | ---: |
| 3M | 6 | 11 |
| 3Musketeers | 1 | 1 |
| BeatBreak | 17 | 17 |
| Beelstar | 19 | 37 |
| Beelstar (53 cards) | 1 | 1 |
| Beelstar 3Musketeers | 1 | 1 |
| Chronomon | 33 | 33 |
| Data Squad | 47 | 72 |
| DATS Ravemon | 2 | 2 |
| DATS Rose | 4 | 6 |
| DATS RoseRave | 4 | 5 |
| Examon | 47 | 60 |
| Galactic | 6 | 6 |
| Galacticmon | 22 | 33 |
| Galatic | 1 | 1 |
| Glowing Dawn | 161 | 177 |
| GlowingDawn | 2 | 2 |
| Machine Black | 1 | 1 |
| Magneticdra | 18 | 33 |
| Medusamon | 139 | 149 |
| Mineral | 6 | 8 |
| Omni Ladder | 19 | 37 |
| Rocks | 81 | 140 |
| Saiyu Warriors | 51 | 51 |
| Shambala | 1 | 1 |
| ST-2 Cocytus Blue | 1 | 1 |
| ST-3 Heaven's Yellow | 1 | 1 |
| ST-5 Machine Black | 1 | 1 |
| ST-6 Venomous Violet | 1 | 1 |
| Starter Deck Venomous Violet | 1 | 1 |
| Styracomon | 54 | 54 |
| TB | 13 | 14 |
| Toho Braves | 142 | 143 |
| TS Ceresmon | 26 | 44 |
| TS Chronomon | 5 | 5 |
| TS Homeros | 1 | 1 |
| TS Jupiter-Merva | 2 | 2 |
| TS Jupitermon | 163 | 196 |
| TS Merva | 1 | 1 |
| TS Mervamon | 24 | 26 |
| TS Security Control | 2 | 2 |
| TS Zoo | 8 | 12 |
| Vemmon | 52 | 56 |
| Virus Busters | 19 | 20 |

For each list, require every card to be registered in the freshly built engine,
present in the production `data/cards.json`, and admissible under the generated
per-card artifact. A majority archetype requires strictly more than half its
lists to pass; a tie does not qualify. Report the list and canonical-group
denominators and distinguish the recent window from the full frozen library.
Use `cards::build_registry().registered_card_ids()`, the API behind PyO3
`load_implemented_card_ids`, rather than an installed Python wheel that may
contain an older pack. The prepared extraction emits
`implemented-card-ids-final.json` and a separate
`registered-card-data-final.json` intersection report. The completed extraction
found **1,196 effect registrations: 1,164 with production data and 32 without**.
These are separate from the 4,476 card-index entries. The admission totals above
also apply the refreshed per-card gate, which marks 1,031 indexed cards
admissible; registration alone is insufficient.

Admission is a readiness gate, not independent end-to-end gameplay certification,
oracle confirmation or a statement that every list in an archetype works.
The four blocked package cards remain visible in the denominators. All 21
recent Chronicle/Alphamon lists contain the still-blocked BT20-018, so the newly
implemented cards do not yet admit any complete list from that package.

## Reproduction and baseline failures

From the repository root in PowerShell, set the campaign environment:

```powershell
$env:CARGO_TARGET_DIR = 'E:/cargo-target-wt/p3-constructs'
$env:RUST_MIN_STACK = '268435456'
$env:RUST_TEST_THREADS = '2'
$env:PYTHONIOENCODING = 'utf-8'
$env:PYTHONUTF8 = '1'
$env:PYTHONPATH = 'code'
```

Portable committed targets and artifact commands include the following. The
measured results above identify the exact executed scopes and source boundaries:

```powershell
cargo test -p digimon-dsl --lib
cargo test -p digimon-engine --lib
cargo test -p digimon-engine --test dsl constructs::
cargo test -p digimon-engine --test cards_behavioral
cargo test -p dcgo-harness --lib
cargo test -p dcgo-harness --bins
cargo test -p digimon-engine --test replay_empty_replacement_window
python -m pytest code/tests/test_cards_json_integrity.py
python -m tools.lowering_coverage
python -m digimon_gym.agents.card_admissibility
python scripts/verify --tier 0
```

Run cumulative tier 2 against the committed campaign head, using the
baseline above, and retain the complete result:

```powershell
python scripts/verify --tier 2 --base 66762dc44b0756f1c64a692bd68a4b49d7ef6255 --head HEAD --continue-on-failure
```

During development, `chronicle_scratch` and `meta_admission_scratch` selected the
same committed card/construct modules for faster combined runs. Those temporary
targets and the report/replay diagnostics were removed after preserving their
local evidence; they are not permanent product tests.
The oracle inputs are `qa/dcgo-exams/BT25/BT25-043-effect3.yaml` and
`qa/dcgo-exams/ST23/ST23-04-{effect0,effect1,effect2,inherited0}.yaml`.
Live oracle runs require a compatible healthy node and the harness's explicit
scenario/card-data configuration. Preserve original recordings and comparison
outputs when measuring them.

Historical aggregate failures remain separate from any new failures:

- `.superpowers/sdd/2026-10-22-construct-tests/final-core.log` records full DSL
  **1,350 passed / 2 failed / 1 ignored**. The failures are
  `official_rule_grants::production_card_data_carries_official_rule_trait_grants`
  (EX12-026/031 missing Aquatic) and
  `trait_parity::production_card_data_traits_superset_of_dsl_traits`
  (those cards plus BT22-023 missing Holy Dragon). Baseline `f24f26dfa`
  reproduced both in `baseline-dsl-known-reds.log`.
- `final-cards.log` in that directory records **12,261 passed / 4 failed /
  36 ignored**. BT24-085's option-use clause color assertion, Green/Red versus
  expected Yellow/Red, reproduces on the same baseline in
  `baseline-bt24-085.log`. Three other failures were outdated fixtures:
  BT22-008 lacked synthetic DNA requirements, BT9-109 expected partial atomic
  payment, and EX12-047 expected reversed bottom order. Their corrections passed
  all **37** affected-module tests in `final-card-fixture-focused.log`.
  That focused result does not replace the failed aggregate.
- The optional `implementation_inventory` binary has **27** pre-existing
  exhaustiveness compile errors. The sorted error-message multiset is identical
  in `.superpowers/sdd/2026-10-22-app-fuse-opt/inventory-check.log`, its
  `inventory-baseline.log` from `cf9ecbcdc`, and this campaign's
  `cost-predicate-validation-red.log`. `--lib` validation does not certify that
  binary. A matching failure count alone would not establish baseline identity;
  the messages were compared.

All `.superpowers/sdd/` logs are ignored local evidence. Committed tests, scenarios,
the plan and this report carry the portable scope and measured outcomes. All
unblocked approved work is implemented; the four lifecycle-blocked cards remain
explicitly unready.
