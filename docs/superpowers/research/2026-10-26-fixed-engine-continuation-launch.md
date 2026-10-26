# Fixed-engine fresh-replay continuation

Launched October 26, 2026 at approximately 13:13 Central on the existing
Hetzner CPU host, following approval to continue after the Nyaromon repair.
PR #751 was merged; preparation code starts from main merge `bd41f0983`.

The corrected 6,760-game evaluation found no Nyaromon attacks in games without
DNA. Checkpoint 4 scored 50.503% against checkpoint 3, with a paired 95% interval
of 49.689–51.346%. This does not establish improvement. Checkpoint 4 is an
experimental warm start, not a promoted champion. Its weights may still reflect
old-engine experience; this continuation tests improvement on corrected rules.

## Frozen setup

- Root: `/opt/digimon-search/fixed-continuation-20261026`.
- Services: `digimon-fixed-continuation.service` and
  `digimon-fixed-continuation-monitor.service`, no automatic restart, 24-hour limit.
- Native image:
  `ghcr.io/mammalwithashell/digimon-search-cpu@sha256:705f2c4b86c5458a2df7f09ded1e1d9cc49160f65bca442cd187b0b1480f0711`.
- Image source: `6627540bab607109b9eb8d5216ff647b8e4b9ae8`, successful
  [CI build 40370197866](https://github.com/mammalwithashell/digimon-deck-list-builder/actions/runs/40370197866).
- Preparation/launcher source: `480b6dcad`; the preparation script hash is
  recorded in `campaign-plan.json`. Runtime Python comes from the pinned image.
- Parent: prior continuation checkpoint 4, SHA-256
  `f33e15a451b385fd8dcb9961b9351fc54d8535a8509d4cb3fc41a83dd83a86df`.
- Four generations of 1,100 games; eight workers; 320 total simulations,
  four determinized worlds, evaluation batch four, decision cap 1,200.
- Three-generation replay window, capped at 1,100 games per generation.
  **No old replay is copied or mounted during training.** Generation 0 contains
  weights only. Updates use 1,100, 2,200, 3,300 and 3,300 new games respectively.
- Two fitting epochs, batch 512, learning rate 0.0001, CPU. No reward, tensor,
  action, search-budget or evaluator-cache changes.
- Collection seed 20261026; evaluation seed 22261026; separate smoke seed 30261026.

The unchanged pool has 13 equally weighted archetypes and 26 training deck slots:
Chronicle, Chronomon, Examon, Glowing Dawn, TS Jupitermon, TS Mervamon, Three
Musketeers, Toho Braves, Light Hybrid, Medusamon, SW, ShineGreymon Data Squad,
and Virus Busters. Slots are not a claim of 26 distinct lists. Expected coverage
is about 169 appearances per archetype per generation, about 508 per full
three-generation replay window. The first update has only its fresh generation.

## Evaluation and monitoring

The parent gets one baseline evaluation on the fixed engine. Each new model
then gets the same fixed evaluation schedule: 200 candidate games per archetype
against each of greedy, frozen parent checkpoint 4, and frozen development.
That is 7,800 policy-only games per evaluation, 39,000 overall. The parent
anchor replaces the older mixed-start anchor so improvement over this run's
starting model is measured directly. Older results must not be pooled with this
new schedule as the same comparison.

Evaluation uses the frozen eight held-out meta cohorts and five same-list pet
cohorts, combined to allow cross-pool opponents. Pet results remain same-list
evidence. Raising, battle evolution, DNA, Nyaromon sequences and per-card
opportunity/choice metrics are collected by the version-2 native telemetry.
Training stops on collection truncation or evaluation failure; promotion and
restarts are manual.

- [W&B run](https://forge.coreweave.com/wandb/james-peters97-mammal/digimon-rl/runs/english-meta-fixed-4x1100-20261026).
- Local dashboard: http://127.0.0.1:8504; cache `runs/fixed-continuation-dashboard`.
- Cloud reports: `evaluations/`; model/replay artifacts: `generation/`.
- W&B sidecar has read-only experiment access, durable upload spool and separate
  credentials. Gameplay has no network access. Earlier run artifacts and services
  are preserved.

## Launch validation

All 26 training slots passed native and readiness admission. The parent and
development anchor match the active 9,434-float `standard_lite_v2.4` contract
and 2,192-action layout. Frozen inputs, runtime files, native image revision,
parent model and card database were checked. Eight full-budget native smoke
games completed (664 recorded positions), with zero truncations. These smoke
games are outside training replay. The 23 controller/recovery tests passed.

At launch, both containers were running, the baseline evaluation was executing,
W&B had connected and uploaded its initial metrics/checkpoint artifact, and the
new local dashboard had a fresh successful snapshot. No new model had yet been
fit. Use live progress for later status and ETA.
