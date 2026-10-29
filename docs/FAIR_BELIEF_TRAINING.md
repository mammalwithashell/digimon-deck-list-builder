# Fair closed-decklist search training

Native self-play and `run_selfplay_generation.py` accept
`--information-regime closed_decklist --belief-prior <prior.json>`.
The default remains explicit compatibility mode `open_decklist`, which uses
an exact opposing composition. Existing untagged shards/models are labelled
`legacy_exact`; they cannot be relabelled or mixed into closed-list buffers.

## Belief and world boundaries

`KnowledgeLedger` is seat-local, versioned serializable entitled memory.
`BeliefEvidence` contains public opposing-owned counts, hidden zone lengths,
remembered identities and historical minimum counts. `WeightedDeckPrior`
accepts this restricted value, never a `Game` or true opponent list.

A frozen prior defines admitted IDs, format and compiled legality hash,
source/split provenance, weighted distinct candidate lists, positive unseen
mass and bounded fallback attempts. Candidate evidence conditions the list
mixture. The fallback samples legal completions from the declared population;
legality is not evidence that these unusual lists are plausible or calibrated.
The existing complete-deck validator enforces copy exceptions, restrictions,
combinations, eggs and main-deck size. No second banlist is maintained.

The closed world builder fabricates hypothetical opposing hidden cards and
original-list metadata from that hypothesis. It preserves public instances,
entitled pins, counts and the viewer's legal root mask. Both PIMC and IS-MCTS
use the same sampler boundary. Open search retains its existing sampler.

Memory starts before the first mulligan action. Public recovery follows the
card to hand; private security inspection belongs only to its entitled seat.
Positioned security-to-hand moves and known top draws preserve learned facts.
Shuffles discard order while retaining membership. Other hidden departures
conservatively forget zone membership, retaining historical lower bounds;
physical hidden handles never identify an unknown transfer. Clones preserve
memory. Scheduler resume replays whole unfinished games from frozen seeds,
without reconstructing knowledge from actual hidden cards. Mid-game legacy
scenario imports cannot enable a complete ledger. `to_scenario()` is a debug
projection, not a full continuation/knowledge checkpoint.

Unsupported continuations fail with an explicit diagnostic before writing
successful game data. Current audited forms cover standard decisions,
RunTail with entitled bindings, block/counter/raid, public permanent count and
permutation steps, and trigger ordering. Legacy callback prompts, concealed
binding references, nonempty loop history, after-selection hooks, foreign
concealed ownership, opaque or transformed concealed cards require further
audits. A legal/admitted card pool does not establish continuation coverage.

## Reproduce and resume

Run from repository root in PowerShell, using a current compatible SB3 seed
checkpoint and an optimized native CLI. All paths below are examples.

```powershell
$env:CARGO_TARGET_DIR='D:\cargo-target-wt\fair-belief'
$env:RUST_MIN_STACK='268435456'
cargo build --release -p digimon-engine-cli
python code/tools/prepare_belief_experiment.py --source cohort.json --cli D:\cargo-target-wt\fair-belief\release\digimon-engine-cli.exe --out experiment
python code/tools/run_belief_comparison.py --experiment experiment --seed-model seed.zip --engine D:\cargo-target-wt\fair-belief\release\digimon-engine-cli.exe --out comparison --workers 2
```

Preparation filters the supplied lists through existing admission and legality,
deduplicates them, freezes disjoint train/validation/test lists and commits a
protocol before gameplay. Prior candidates come only from training lists;
the fallback ID population comes from the global admitted registry, independently
of the held-out list selection. ENG/format policy is the pinned repository
snapshot, not a promise about the live tournament banlist. Neither tool
provisions resources, alters admission, or promotes a champion.

The comparison trains open and closed candidates with the same budgets and
initializer across three seeds. The initializer itself is evaluated separately.
A fixed first-generation champion and greedy anchor measure policy performance;
independent native panels separate policy-only and equal-budget search play.
Each frozen target receives a fresh equally budgeted masked-PPO exploiter,
evaluated on held-out games with balanced play/draw. This measures a
finite-budget best-response estimate, not certified exploitability.

Rerun the same command to reuse hashed completed jobs and resume native
self-play chunks. Changed engine, binding, prior, population, protocol, seed
checkpoint or training code rejects resume. Keep failed attempts and their
failure manifests. A partially written training/export stage follows the
existing generation runner's explicit recovery rules; do not mark it complete.
An optimized replacement binary requires a new comparison directory.

## Provenance and scoring

Run/shard `information` metadata records regime, knowledge schema, semantic
prior hash, legality hash and prior provenance (including split/population
hashes). Full-run `information.json` additionally binds the prior file bytes.
Scheduler identity binds executable/model/prior bytes, seeds and budgets.
SB3 checkpoints store `digimon_information`; the MLP ONNX exporter copies it
to `.meta.json`. Closed-list training here uses the existing feed-forward AZ
trainer; recurrent search/export adaptation is outside this change.
The existing observation/action contract remains required independently.

`belief_predictions.jsonl` contains predictions and mixture diagnostics.
`belief_scoring_truth.jsonl` is written only after its game ends. Actor,
prior and trainer never read it. Score an individual native closed-mode chunk:

```powershell
python -m digimon_gym.agents.belief_evaluation --score-run chunk --out scores.json
python -m digimon_gym.agents.belief_evaluation --protocol experiment/protocol.json --results comparison/results.json --out gate.json
```

Reports include per-card support coverage, log loss, Brier score, calibration
bins, unseen-world usage, sampling time and candidate elimination. Finite sampled
worlds can give zero empirical support to a possible card; these are sample
scores, not exact posterior probabilities. Resource sidecars measure process-tree
RSS at 0.5-second intervals and elapsed time. Phase timings and sampled-world
counts provide declared-budget throughput. Sampler failures remain explicit.

Promotion requires every registered per-seed policy uplift, anchor and exploiter
non-regression criterion. Conservative differences of 97.5% Wilson component
intervals provide nominal 95% pairwise coverage; no simultaneous multi-panel
coverage claim is made. Outcomes must conserve the exact requested denominator.
Failed, truncated, missing or inconclusive results block promotion while leaving
checkpoints resumable. Generation completion alone never promotes a model.

## Deployment limits and rollback

The policy tensor still exposes only 45 trash identity rows per seat. Search
uses complete public accounting and entitled memory; a feed-forward policy may
not reproduce its teacher's counting knowledge. The counting regression makes
this limitation explicit. No tensor, action, recurrent-policy or desktop
compatibility contracts change here. Existing recurrent consumers retain their
state/reset requirements; this comparison uses feed-forward policies.

The first frozen cohort is an admitted ST1-derived variant population, with
stock inadmissible cards excluded and legal counts refilled. It is not the
complete starter deck and establishes no cross-archetype robustness. The
experiment's population/protocol files are the authoritative denominators.
Independent gate failure does not justify broader readiness claims.
Rollback selects open mode with its own separate data/run directory; never
reinterpret closed-list shards or overwrite frozen provenance.

Prior-only benchmark (same release build settings):

```powershell
cargo run --release -p digimon-engine-cli --example belief_bench --config 'profile.release.package.digimon-engine.opt-level=1' -- experiment/prior.json
```

Its public-count inputs have no game or actual opponent deck. It reports
candidate conditioning, mixture sampling, all-candidates-eliminated sampling,
bounded failures and fallback request success. It does not measure per-proposal
rejection acceptance or extrapolate to larger candidate populations.


## Reusing the existing EX13 cloud cohort

Use `python code/tools/prepare_cloud_belief_pool.py --source data/training_pools/ex13-20261026 --out <fresh-directory> --cli <native-cli>` to preserve the existing cloud train/held-out/supplemental/pet partitions. It imports through shared snapshot admission, rejects content leakage, retains 26 native training slots for 21 distinct lists, and prepares the prior from training contents only. The 23 held-out lists cover 12 archetypes; same-list Light Hybrid pet results remain separate.

The [cloud-pool adoption report](../openspec/changes/add-fair-belief-selfplay/cloud-pool-adoption.md) records current Python/native admission and 21 closed-search probes. All source partitions pass admission; 20 probes stop at unsupported closed-world continuation/knowledge forms. Broad closed-search qualification requires those audited forms and a new independent protocol. No cloud training was changed by the read-only inspection or local preparation.
