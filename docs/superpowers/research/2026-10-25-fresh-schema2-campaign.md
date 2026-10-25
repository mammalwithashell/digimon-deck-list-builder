# Fresh schema-2 Light Hybrid search campaign

The user approved restarting after main changed the model contract. The old
campaign, checkpoint 2, replay shards, and W&B history remain preserved. This
campaign inherits no weights or replay from them.

## Frozen scope

- 13 equally weighted archetypes, 21 distinct lists, 26 native sampling slots.
- Eight English EX13 meta archetypes plus Medusamon, SW, ShineGreymon Data
  Squad, Virus Busters, and the exact saved Light Hybrid list.
- Two 1,100-game search generations, approximately 169 appearances per
  archetype per generation. Replay can pool up to three generations; this
  bounded run will have one then two generations available.
- Search: 320 simulations, four determinized worlds, batch four, eight workers,
  1,200-decision cap. Capped search games stop the campaign before fitting.
- Same initialization recipe as before: a newly initialized model, 400 greedy
  demonstration games, policy imitation, and terminal-outcome value fitting.
  These demonstrations remain separate from search replay. Fresh initialization
  refuses any capped bootstrap games before fitting.
- Policy-only, seat-balanced evaluation of the fresh seed and every new model:
  200 candidate games per archetype per anchor, 5,200 games per checkpoint.
  Anchors are the heuristic and the newly frozen bootstrap. Meta lists are held
  out; pet evaluations use the exact training lists. No automatic promotion.
- Current `standard_lite_v2`: 9,434 values, feature schema v2.4, action schema 2.

## Provenance and launch gate

Image tag `training-search-v2026.10.25.2` builds commit
`8f945e484c1e4263f8da8c3dba816e25fe697c98` in CI run `40216231750`.
The preparation helpers are separately versioned and hash-bound. No runtime
source overlays replace the image's engine, controller, trainer, or evaluator.

The host preparation directory is
`/opt/digimon-search/fresh-schema2-preflight-20261025`; the campaign directory
is `/opt/digimon-search/fresh-schema2-generations-20261025`. Diagnostics never
enter search replay. Deployment requires matching image revision, checkpoint
hash, current contract, all 26 native deck slots admitted, and at least 31
complete preflight games including four Light Hybrid mirrors.

The run ID is `english-meta-pets-light-fresh-2x1100-20261025`. Systemd retains
the 48-hour workload limit and disabled automatic training restart. Monitoring
publishes scalar progress, checkpoint artifacts, and per-card evaluation
reports to `digimon-rl`. Image digest and actual launch evidence are recorded
after validation.

## Launch evidence

CI run `40216231750` passed. The deployed image is
`ghcr.io/mammalwithashell/digimon-search-cpu@sha256:687fc2213cc1bca2fcb98d3ee04a210462834792a4455535c5b14db6be0ed891`.
The host verified its OCI source revision against the build commit.

All 400 bootstrap games completed without truncation, yielding 37,087 decision
rows. Fresh seed SHA-256:
`24b542620ca4795f55111a6ceb90f242ebfcc9abf088d95fbe53fdd215039111`.
ONNX/PyTorch maximum differences were 1.43e-6 for logits and 9.54e-7 for value.
This validates export parity, not playing strength. The bootstrap critic's
last held-out outcome MSE was 0.8392, measured on its own demonstration split.

Linux preflight passed 31 complete native-search games with no crashes or caps,
including four Light Hybrid mirrors, plus the two recorded Python prefix
regressions. All 26 training sampling slots passed current admission and legal
deck checks. Batch one versus batch four produced identical saved arrays on
both 40-decision Light Hybrid diagnostic prefixes; these intentionally capped
diagnostics are excluded from replay and are not counted as complete games.

The fresh campaign launched at Unix time `1792950069.9434183` (see the live
`started.json` for the authoritative timestamp). Training, monitor, and log
mirror services were active at verification, with initial checkpoint evaluation
running. The W&B publisher had uploaded two progress records. Self-play begins
after the initial 5,200-game evaluation passes.

W&B: https://forge.coreweave.com/wandb/james-peters97-mammal/digimon-rl/runs/english-meta-pets-light-fresh-2x1100-20261025

Detailed initialization, image, preflight, and contract evidence is recorded in
`2026-10-25-light-hybrid-pool.json`. The launch does not assert completion of
either training generation or evaluation.
