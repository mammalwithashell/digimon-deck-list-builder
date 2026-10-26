# Timing repairs and global EX13 pool refresh

The confirmed timing defects from the
[13-archetype audit](2026-10-26-archetype-strength-audit.md) are repaired locally.
The wider sweep also reproduced and repaired incorrect Digimon-play qualifiers,
start-turn simultaneous ordering and per-turn reset timing. A new exact-list
pool is prepared for the same 13 archetypes using global October EX13 evidence.
The live cloud continuation remains on its existing diagnostic image and inputs;
these changes have not been deployed or mixed into its replay.

## Repairs and evidence

| Problem | Repair | Behavioral evidence |
| --- | --- | --- |
| Printed End of Attack fired after unrelated attacks | Capture the declared attacker's stable identity and qualify printed DSL clauses against their carrier; preserve broad native observers | All eight affected frozen-pool cards reproduced, then passed; 12 real-card regression tests plus five independent identity/clone/attack controls |
| Four Digimon-play clauses observed Tamer plays | Add event-kind qualifiers to EX12-034, EX12-036, EX12-057 and BT25-013 while preserving controller/color guards | Twelve own-Digimon, opponent-Digimon and Tamer controls |
| Ordinary start-turn effects resolved before due Delay effects | Collect both in the same trigger window; preserve processing-condition rechecks, decline recurrence and native/DSL cleanup ownership | LM-027 + BT24-088 ordering, clone/decline and enabled/disabled processing controls; all 143 option-flow tests |
| Per-turn counters reset after start-turn processing | Reset before collecting start-turn triggers, once | BT25-077/BT24-088 witnesses for a suppressed new-turn use and an incorrectly erased current-turn use |
| Multiple evaluation lists rotated in lockstep | Traverse the Cartesian variant pairs within each ordered archetype matchup | Uneven and two-list cohorts, exact archetype quotas and paired first players; cohort version `archetype-paired-v2` |

The original timing defect witnesses failed before the relevant repair. The
final timing-audit binary passes 22 tests; its 12 event-kind tests and two native
cleanup identity controls are included in that count. Native counters and carrier
tracking remain internal: no model
shape, tensor profile, action encoding, search budget or hidden choice changed.
The printed End of Attack review covers all 22 such production YAML clauses;
only the explicitly tested subset has new behavioral evidence.

Rules references: CR 4.3 15-16-15-1 (printed End of Attack), 11-5-1-4 and 11-6-2
(declared attack/end timing), 15-4-3 (simultaneous ordering), 15-8-3-9-1/2
(processing conditions) and 15-14-1-5-1 (per-turn reset). Printed card images and
DCGO source were cross-checked. No new live DCGO oracle games were run.

Review also found a cleanup alias in the new native Delay adapter: the same
physical Option can return to hand and be used again during its old body's
processing. Cleanup now retains the old permanent's stack identity, including
across cloned pending selections. The initial extra fixtures did not actually
create a new permanent and are excluded as evidence. Corrected fixtures assert
successful replay before cleanup; both fail with only the two identity guards
removed and pass with the guards present. That controlled ablation is recorded
separately from the original timing defects' RED-before-repair results.

## Wider timing coverage

The original inventory covered 280 unique cards, 817 YAML clauses and 665
explicit timing declarations. The final refreshed training/held-out/pet union
covers **311 unique cards, 908 YAML clauses and 739 explicit timing declarations**,
including 44 replacements and 23 Delay declarations. It adds 45 card IDs and
removes 14. The 21 training, 23 held-out and five original-pet panel entries
overlap; they are not 49 distinct lists. Supplemental builds are excluded.

The review examined shared event/controller routing, carrier/source identity,
play-event collection, turn and Main entry, Delay scheduling and per-turn usage.
Added observer clauses received a static predicate check; no further instance
of the demonstrated Tamer-as-Digimon defect was found. The following remain
prioritized behavioral coverage gaps, rather than confirmed new defects:

| Candidate | Unmeasured interaction |
| --- | --- |
| BT21-044 | Simultaneous Tamer-deletion observers and inherited-host liveness |
| BT25-034 | Security trash timing versus ordinary security battle removal |
| LM-029 | Individual Scramble body and conditional activation |
| EX13-006 / BT9-112 | Inherited own-end-turn and opponent-end-turn clauses |
| BT25-088 / BT1-087 | Exact memory-setter combinations in the shared start window |

Neither pool contains an explicit StartOfOpponentsTurn clause or an active
scheduled end-turn Delay body; three end-turn Delay declarations are empty
persistence markers. Those general engine paths remain outside this audit's
behavioral claim. Generated keyword effects are also outside the authored-YAML
inventory denominator. Full inventories retain clause bodies and input hashes
under `runs/timing-pool-refresh-20261026/`.

## Exact list assets

The [prepared pool](../../../data/training_pools/ex13-20261026/README.md) contains:

- 21 distinct training lists across all 13 archetypes, represented by 26 native
  sampling slots so each archetype retains equal sampling probability.
- 23 unseen evaluation lists across 12 archetypes: 14 meet the declared
  competitive qualification and nine have lower source/performance evidence.
- Five unchanged original pet lists in a separate regression panel.
- A 24-list combined diagnostic panel across all 13 archetypes. Only Light
  Hybrid uses a same-list pet fallback; that row is not held-out evidence.
- A separate supplemental pool for other admitted exact builds. It does not
  silently change training or the principal evaluation panel.

Training remains two distinct lists for each of the eight original meta
archetypes, one qualified Virus Busters list, and four explicit original-pet
fallbacks. Public evidence did not justify claiming two strong fresh lists for
every pet archetype. No list was altered to bypass card admission.

Selection uses explicit EX13 October results, field size, relative placement,
recency and event/build diversity. Published records must be non-losing with at
least two wins; missing records remain unmeasured and receive a ranking penalty.
Lower-evidence evaluation variants are labeled separately. Shine includes an
explicitly reviewed global EX13-compatible build whose event-format label is
unverified; it is not represented as EX13 tournament-performance qualification.

Three Musketeers now includes the October 24 winner from a 42-player event and,
in the other partition, the October 19 winner from a 31-player event. Exact source
links, dates, counts, records, exclusions and hashes are in the pool report.
These are better supported examples, not proof of their causal win-rate advantage.

The native and Python registries agree on 1,185 cards. All five snapshots reload
through admission and standard deck legality. Held-out fingerprints overlap
neither the refreshed training lists nor the prior frozen training lists.
Public collection stopped at HTTP 403; archived records are clearly marked as
not refetched. Original card counts and evidence gaps remain visible.

## Verification and rollout boundary

On the final repaired source, all 299 combat tests pass (one pre-existing test
remains ignored), along with 102 selected DSL timing, Delay, queue, identity,
cost and replacement controls. The 22 timing regressions and 143 Option-flow
tests also pass. Earlier integration runs passed the 12 real-card audit tests,
five attack-scope controls, 18 effect tests, 25 event-emission tests and 73 phase
tests; those precede only the narrow native Delay identity follow-up, which has
its own final regression and Option-flow coverage.

The final affected-card pass adds 457 passing tests across 35 card modules,
including all 22 cards with authored EndOfAttack clauses. Four pre-existing
BT21-029 tests remain ignored (declarative keyword, lowest-DP filtering and two
card-local OPT checks); 12,312 unrelated card tests were filtered out. Together,
the final-source timing/Option/combat/scoped-DSL/card runs contain 1,023 passing
tests and five pre-existing ignored tests. The 133 earlier integration passes
and 478 interrupted broad-DSL cases are not added to that final-source total.

Pool preparation, audit tools, balanced scheduling and anchored/head-to-head
evaluation have 74 passing focused Python tests. The separate slow trained-model
discrimination experiment was not run. All Tier 0 components pass, including
183 verification-tool tests: the first run identified a stale lowering-coverage
artifact, which was regenerated alongside the impact index and passed its
targeted freshness recheck. The complete final Tier 0 rerun also passed.

An offline CLI regeneration from recorded sources and the recorded baseline
reproduces the same selected lists, partitions, coverage and fingerprints. The
two deck manifest files reproduce byte-for-byte; snapshot timestamps differ as
expected. Native extension hashing covers Windows `.pyd` and Linux `.so` files.

The full 1,597-test DSL suite was not completed. Its broad run was explicitly
interrupted after 478 passing cases and no test failures to unblock the native
Delay follow-up; its process exit is recorded as an interruption, not a pass.
Two combat witnesses that intended to observe any attack end were migrated from
printed DSL clauses to explicit broad native observers, preserving their attack
paths and assertions. These fixture corrections do not broaden printed timing.

The [tracked evidence bundle](../../../qa/archetype-qa/ex13-timing-20261026/README.md)
preserves exact test scopes, source hashes, inventories and validation receipts.
Raw RED/GREEN logs remain under `runs/timing-pool-refresh-20261026/`. The OpenSpec change is
`fix-training-pool-timing-and-refresh-ex13`; the runbook documents reproducible
preparation and the new evaluation cohort version.

Before drawing a corrected-engine strength conclusion, build a fresh immutable
training image and repeat a frozen evaluation schedule on that image. Keep the
current diagnostic results separate. A changed multi-list schedule is a new
cohort, so use its saved schedule/hash for subsequent checkpoint comparisons.
These repairs do not establish how much of any observed simulator/meta win-rate
gap they explain, and a static timing inventory is not full-pool certification.
