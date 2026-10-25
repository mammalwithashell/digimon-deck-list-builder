# Generation 1 recovery and continuation

## Preserved experiment

The 3,300-game collection completed. Generation 1 (directory `generation/gen0`)
trained on 248,060 rows after excluding all 1,821 rows from verified bad games
875 and 1408. Its checkpoint SHA-256 is
`f92782e86354e85400db81bd092924c984ee691d2d8c3b3d61e1a007babbed19`.
The checkpoint is backed up locally and in W&B. No collection or fitting was
restarted to recover the failed final pet evaluation.

## Failure and corrected replay

Pet evaluation seed 54522251 is game index 101 at base seed 27261024:
candidate Medusamon versus heuristic ShineGreymon Data Squad. The original
image reaches its 1,000-step limit. From step 67, it repeats Lamiamon's hand
activation (action 30) and Elizamon selection (105), with no Dimetromon in
trash. This is the same BT24-016 unpayable-cost bug fixed in `eaedce7ce`.

The corrected image finishes this exact game in 80 steps; the candidate wins.
The existing cost gate agrees with the printed card and retains all legal
choices. No additional card behavior change was necessary in this recovery.

CI run [40158172185](https://github.com/mammalwithashell/digimon-deck-list-builder/actions/runs/40158172185)
builds and smoke-tests engine revision `e1e7fff2c689e8505a8146d9012502be11dce8c1`.
The new immutable image is:

```
ghcr.io/mammalwithashell/digimon-search-cpu@sha256:a98d312fd98a32db0f4c594c004b8aa245321486643c1221c492dcc55dd6f413
```

## Learning signal

The original fixed 96-game card panel, before changing engine images, compares
the bootstrap checkpoint with generation 1 on identical seeds and lists.
Each table cell is wins out of 24 games; these small panels have substantial
sampling uncertainty.

| Panel and opponent | Bootstrap | Generation 1 |
|---|---:|---:|
| Meta / heuristic | 8/24 | 15/24 |
| Meta / frozen bootstrap | 11/24 | 19/24 |
| Pets / heuristic | 7/24 | 13/24 |
| Pets / frozen bootstrap | 11/24 | 16/24 |

Candidate attacks rose from 251 to 439. The bootstrap made no digivolutions;
generation 1 made four raising digivolutions, but no battle/DNA digivolutions
or promotions. These results support improvement against the fixed references,
not mastery of evolution chains or promotion to a production champion.

The complete non-quarantined search data does contain evolution supervision:

| Normal action route | Decisions with a legal option | Decisions with positive visit mass | Decisions whose highest-visit action uses route |
|---|---:|---:|---:|
| Raising digivolution | 60,299 | 55,170 | 2,020 |
| Battle digivolution | 50,418 | 45,828 | 779 |
| DNA digivolution | 1,480 | 1,019 | 36 |
| Promotion | 3,438 | 3,438 | 1,479 |

These are saved decision rows, not completed card events, and exclude
effect-driven evolution routes. They also differ from the card-level
opportunity denominator in card-evaluation reports. Search supplies examples,
but the deterministic policy has learned very little evolution behavior yet.
One further bounded generation is justified as a learning experiment, with
the evolution metrics retained as an explicit evaluation concern.

## Linux throughput and exactness

On the actual CCX33 server, two seeds with two reversed-order repeats compare
the previous production image with the new image, using the generation-1
ONNX model. Every case uses one CPU, 320 total simulations, four worlds, batch
four, no evaluator cache, and a 40-decision prefix.

Aggregate native throughput improves **1.9278x**. All eight output arrays
match byte-for-byte in all four paired cases (142 saved positions per variant).
This is a bounded prefix measurement, not a full-game speed forecast or a
playing-strength test. The images also differ by the Lamiamon fix; these
particular prefixes retain exact targets despite that rules change. The
earlier Windows benchmark isolates the observation change itself.

## Memory-safe replay and monitoring

Two generations of dense observations would approach or exceed the server's
RAM while the old loader retains every shard and concatenates a second copy.
The loader now streams validated observations into a private temporary file
and uses a memory map for unchanged shuffled minibatch indexing. It keeps all
rows, sparse targets, exclusions, and the existing replay-window policy.

A separate full 248,060-row, two-epoch fitting diagnostic, limited to 4 GiB,
reproduces **all 27 saved model tensors exactly**. Peak process RSS is
2,838,228 KiB. Its 217-second duration is not a fitting speed comparison:
the diagnostic's memory limit differs from the production run.

The continuation controller binds the previous checkpoint, ONNX export,
inputs, validation report, image digest, and deployed Python files to hashes.
It runs exactly one additional 3,300-game generation, refuses partial-work
restarts, keeps 320/4/batch-4 search and the existing three-generation replay
window, and never promotes a checkpoint automatically. `--fail-on-truncation`
stops before fitting if the new collection contains any capped games; originals
remain available for investigation instead of silently supplying policy targets.

Monitoring now follows the active generation instead of hardcoding gen0.
It separates generation loss epochs, ignores stale previous stdout, labels
checkpoint artifacts by generation, and refreshes the card-evaluation schedule
when a continuation changes the engine. Original logs and reports remain.

Validation: 38 focused trainer/generation/monitor/continuation/evaluation tests;
all 13 tier-0 checks, including 174 verification-tool tests; the full fitting
parity diagnostic; CI image smoke; and the exact failed-seed replay.

Operational evidence is under `/opt/digimon-search/generation1-validation` on
the server and `runs/cloud-search-20261024/generation1-validation` locally.
The completed-panel and next-generation launch results are recorded below.

## Completed evaluation on the corrected engine

All 1,200 games finish normally: two panels, two fixed anchors, 300 games per
anchor, alternating who starts. The meta panel repeats base seed 26261024;
the pet panel repeats base seed 27261024, including the originally failed game.

| Panel | Heuristic | Frozen bootstrap |
|---|---:|---:|
| Meta validation | 165/300 (55.0%) | 222/300 (74.0%) |
| Same-list pets | 171/300 (57.0%) | 230/300 (76.7%) |

The candidate completes 81 raising digivolutions and 11 promotions. It still
completes no battle or DNA digivolutions. Telemetry records 16,504 card-level
raising opportunities, 18,791 battle opportunities, 413 DNA opportunities and
105 promotion opportunities. Repeated states/cards can contribute repeated
opportunities; these are not unique games or the decision-row counts above.

The scores justify another learning experiment against the fixed references.
They do not justify calling this a strategically mature meta pilot. The pet
panel uses the same saved lists as training, not unseen-list generalization.

## Completed search check and generation 2 launch

The corrected image and generation-1 policy completed 64 diagnostic search
games with zero truncations: 4,888 saved positions and 5,311 decisions in
451.34 seconds (510.48 games/hour, 10.83 positions/second). One slow Three
Musketeers/TS Mervamon game finished normally in 315 decisions. A duplicate
trace was stopped after the original game finished; its partial diagnostic
output is preserved and is never included in training.

This small complete-game sample includes scheduler startup and a long final
game. Its roughly 6.5-hour collection projection is provisional, not a bound.
The paired prefix benchmark is the controlled optimization comparison;
comparing this sample directly with generation 1 also changes model and games.

Generation 2 (`generation/gen1`) started at **2026-10-24 22:50:39 Chicago**
using eight workers, 3,300 new games, unchanged 320/4/batch-4 search, disabled
cache, and the existing replay window with both verified bad games excluded.
The new native image is pinned above. Python operational overlays come from
`5c711eec0` and are individually SHA-256-bound in `continuation-gen1.json`.
All three training, W&B monitor, and card-evaluation services are active.

W&B confirmed publication of the two corrected 600-game reports (16 total
card reports at launch). The periodic worker continues with unique window
IDs and engine-image provenance. The existing run remains:
[english-meta-pets-20261024](https://wandb.ai/james-peters97-mammal/digimon-rl/runs/english-meta-pets-20261024).
