# EX13 fair-belief software qualification (2026-10-29)

The software blockers identified by the original 1/21 cloud-pool smoke are fixed
for the qualification below. The broader model comparison remains pending;
software completion does not establish stronger models or justify promotion.
The PR must retain that distinction. No champion has been promoted.

## Population and completed probes

The original cloud partitions and their admission gates remain unchanged.
No card substitutions, restrictions, action masks or player choices were relaxed.

| Partition | Lists | Archetypes | Completed probes | Failed / truncated |
| --- | ---: | ---: | ---: | ---: |
| Training, three seeds | 21 distinct, 26 weighted slots | 13 | 63 | 0 / 0 |
| Source-held-out | 23 | 12 | 23 | 0 / 0 |
| Supplemental validation | 36 | 10 | 36 | 0 / 0 |
| Original pet regressions | 5 | 5 | 5 | 0 / 0 |
| Total probes | Not a distinct-list denominator | | 127 | 0 / 0 |

Each probe is one mirror game with a uniform evaluator, four total simulations,
two worlds/batch two, `standard_lite_v2` and a 1,200-decision cap. These are
software checks, not matchup-strength evidence or exhaustive state coverage.
Pet/train overlap remains explicit; Light Hybrid has no independent unseen-list
test in this cohort. Supplemental sources are not equated with source-held-out
evidence. [All probe outcomes and runtime hashes](evidence/cloud-qualification/probes.json)
are retained. [Development receipts](evidence/cloud-qualification/development-probes.json)
preserve 34 earlier probe reports, including every failed/truncated case; their
input order and seed assignments are recorded individually.

## What changed

Closed worlds audit and rebind cached continuation references, including link
host selections, cost choices, reveal/bucket choices and parked observer bodies.
Composition-changing paired tests check sampled worlds, legal choices and actual
continuation resolution. The link and security-observer regressions also check
that changing actual hidden cards leaves the policy observation unchanged.

Knowledge schema 2 separates entitled identities from anonymous movements.
Concealed field cards and unknown security/deck movements receive sampled
identities; the owner also samples unseen own deck/security assignments. Public
hand departures preserve independently remembered identities without revealing
which matching physical copy was selected. Nested public security holders remain
resolvable. A saved 326-action Chronomon prefix checks remembered positions after
every action. Ordered returns route Digi-Eggs to the Digi-Egg deck and preserve
their legitimately remembered order.

A conservative DSL analysis proves when an entire security-removal observer and
its gates do not read event-card identity. Only certified hypothetical queued or
optional-trigger contexts can omit unused concealed references. Owner/cause
scalars and entitled references remain; identity snapshots, raw Rust and unknown
constructs keep strict auditing. The source game is unchanged.

Foreign-owned concealed zones/field sources, unaudited private security transits
and transformed concealed metadata still stop explicitly. This qualification
does not certify every admitted card, all possible histories, or DCGO parity.
No observation or action contract changed. The policy's existing 45-row trash
identity window remains a limitation; search accounting reads the full public
trash rather than that window.

## Validation and operational checks

- Rust: 2,174 passed, five existing ignores, covering determinization, DSL,
  infrastructure, judge/rules/FAQ/keyword conformance, search and self-play.
- Rebuilt private Python binding: 148 passed, one existing skip, including
  tensor profiles, runner compatibility and native binding surface checks.
- Repository tier 0: all 13 checks passed, including 183 verification-tool tests.
  Both related OpenSpec changes pass strict validation.
- Runbook-budget collection/fitting/export check: 32 mixed-deck games, 2,688
  rows, zero truncation/quarantine, 2,009 cheap value-only rows, compatible SB3
  and ONNX outputs. It uses 320 total simulations/four worlds/batch four,
  eight workers, phase/cheap budgets 64 and full-search fraction 0.25.
- Full-controller startup check: eight training games, 36 evaluation games and
  two fresh 512-step exploiters matching target capacity. Every stage completed
  with zero failed/truncated evaluation games. Its pet-list panels are strictly
  functional checks; their overlap with training precludes a held-out claim.

[Runtime, contract and validation receipt](evidence/cloud-qualification/runtime-and-validation.json)
and [controller startup receipt](evidence/cloud-qualification/comparison-startup.json)
include exact hashes. Smoke-trained weights are never used as the full study's
initializer or a promoted model. Raw data, logs and immutable executables remain
under `D:/cargo-target-wt/closed-deck-belief-training/`.

## Broader comparison and merge status

The [frozen protocol](evidence/cloud-qualification/protocol.json) uses seeds
11/22/33; both regimes start from the same checkpoint-12 model
(`4836d024b87806400287c8c09c0486243df0a5dc8d458ef73e5aef76e5517f1f`).
Its legacy training information regime is untracked and is not relabelled fair.

Each regime/seed receives three 1,100-game generations at the runbook search
budgets, two fitting epochs, batch 512, learning rate 0.0001 and a three-generation
replay window. That is 19,800 training games. Four 2,600-game panels per
regime/seed plus unchanged-initializer checks total 70,200 evaluation games.
Six fresh, capacity-matched exploiters each receive 131,072 timesteps. The
checkpoint-12 anchor stays frozen. Policy-only, greedy-anchor, exploiter and
search-assisted results remain separate, with per-seed uncertainty gates.
The component Wilson intervals provide nominal pairwise coverage, not a joint
confidence certificate or a formal exploitability bound.

The experiment directory is
`D:/cargo-target-wt/closed-deck-belief-training/ex13-full-comparison-20261029/`.
Its source archive and native runtime are frozen separately from the editable
worktree. The [launch receipt](evidence/cloud-qualification/launch.json) records
the actual process and input identities. The full comparison launched locally
at 2026-10-29 18:42 UTC from source commit `26bfc3f79`, with eight workers.
All 23,020 frozen source files and the copied runtime were hash-verified before
launch. This is a multi-day run; its launch receipt is not a completion result.
Broad comparison task 7.4 remains open until the run and its evidence reporting
complete; merge qualification is held pending that evidence and CI. No held-out
win rates were used to tune policies or budgets while correcting engine behavior.

The earlier [narrow ST1 candidate](evidence-report.md) remains rejected: policy
play and exploiter resistance regressed despite search-assisted improvement.
Neither the 127 successful probes nor the operational checks override that
negative result or prove the broader candidate stronger.
