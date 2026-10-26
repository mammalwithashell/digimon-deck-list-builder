# Mixed checkpoint continuation

Started October 26, 2026 at 03:22 Central on the existing Hetzner host after
the completed mixed-versus-development comparison. The selected mixed model
won 53.3% of the 6,760 direct evaluation games (95% block-bootstrap interval
52.3–54.3%). This campaign trains four further models; it does not promote them
automatically.

- Root: `/opt/digimon-search/mixed-continuation-20261026`.
- Service: `digimon-mixed-continuation.service`, no automatic restart, 24-hour limit.
- Controller overlay: commit `1215e9add24b8498994e97b0098adec735c5d288`.
- Immutable native image: `ghcr.io/mammalwithashell/digimon-search-cpu@sha256:76d7cf1e6fd0c25af41bbb4b0199c5da812eb0bac952d0db3dcbf57072ab8444`.
- Parent checkpoint SHA-256: `66a3c4eacc9a509777b6f4e91dda224826f59a342ab20000cb795051f778a0f8`.
- Development anchor SHA-256: `f2c78c4eda90f87c694668edb76ffddddb12fd508e35c1dd5b4490424153fee3`.
- Four generations of 1,100 search games, eight workers, 320 simulations,
  four determinized worlds, evaluation batch four, decision cap 1,200.
- Three-generation replay window, capped at 1,100 games per generation.
  The first update reuses the parent's completed search generation and its
  new generation. Heuristic demonstrations are excluded from replay.
- Two fitting epochs per update, batch 512, learning rate 0.0001, CPU.
- Collection seed 10261026; evaluation seed 12261026.

The unchanged, equally weighted training pool has 13 archetypes: Chronicle,
Chronomon, Examon, Glowing Dawn, TS Jupitermon, TS Mervamon, Three Musketeers,
Toho Braves, Light Hybrid, Medusamon, SW, ShineGreymon Data Squad and Virus
Busters. Each generation has about 169 expected appearances per archetype.

Evaluation uses a combined frozen pool of the eight held-out meta lists and
five same-list pet cohorts, allowing cross-pool matchups. It runs once on the
parent baseline and once after each new model: 200 games per candidate
archetype against each of greedy, frozen mixed-start and frozen
development-fixed. That is 7,800 games per evaluation and 39,000 overall.
These are policy-only evaluations, with balanced opening order; they do not
measure search-assisted playing strength. Pet results remain same-list
evidence, not unseen-list generalization.

Preparation verified all 275 parent chunks and their hashes, 1,100 completed
games and 96,384 replay rows, model/ONNX observation contracts, card database,
native image and deck admission. Eight full-budget native smoke games
completed with zero truncations. Twenty focused controller and generation
contract tests passed. The runtime, inputs and validation evidence are hashed
in `campaign-plan.json`; source archives are mounted read-only. Collection
fails on truncation instead of silently training on incomplete games.

At launch, both campaign and monitor services were active. The baseline
evaluation had completed its first 86 games and W&B confirmed an online
connection. Progress and per-card reports are stored under `evaluations/`;
models and replay are under `generation/`. The monitor has a durable upload
spool and publishes each completed evaluation's card reports.

- W&B: https://forge.coreweave.com/wandb/james-peters97-mammal/digimon-rl/runs/english-meta-mixed-4x1100-20261026
- Local dashboard: http://127.0.0.1:8503
- Local cache/config: `runs/mixed-continuation-dashboard`.

The initial estimate is roughly six to eight hours for the campaign, based
on the parent collection rate and previous evaluation timings. Stronger fixed
opponents may change evaluation duration; use fresh progress for later ETAs.
