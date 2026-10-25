# Four smaller search generations

The user approved four 1,000-game generations, a three-generation replay
window, and larger checkpoint-only evaluation on 2026-10-25. The campaign
started at 10:56:47 UTC (05:56:47 Chicago), initially evaluating its parent.

## Prior collection retained

The previous continuation finished all 3,300 games in 14,485.5 seconds, with
243,442 saved rows and two capped games. Games 340 and 850 both reached 1,200
decisions without an outcome. Both involved Chronomon against Toho Braves;
that identifies a reproduction matchup, not a confirmed loop cause. The
fail-on-truncation guard stopped before fitting. No second trained model exists.

All of that collection remains at `/opt/digimon-search/experiment/generation/gen1`.
It is not used by the new campaign. The new parent is the previously validated
`gen0/model.zip`, SHA-256
`f92782e86354e85400db81bd092924c984ee691d2d8c3b3d61e1a007babbed19`.
The original gen0 replay remains available through a read-only archive mount,
with its verified game-875/game-1408 exclusions intact.

## Active protocol

- Separate root: `/opt/digimon-search/small-generations`.
- Four updates, `generation/gen1` through `gen4`, 1,000 new games each.
- Eight workers; dynamic four-game chunks; 320 total simulations, four worlds,
  evaluation batch four; evaluator cache disabled; decision cap 1,200.
- Two fitting epochs, batch 512, learning rate 0.0001; warm-start each update
  from its immediate predecessor. No automatic model promotion.
- Replay spans up to three source generations, with at most 1,000 games from
  each. Selection uses whole chunks in a fixed seeded order, without inspecting
  outcomes. All rows in each chosen healthy game are retained; game lengths
  can therefore give different decision-row contributions. Original data stays
  unchanged. Selection manifests record source and quarantine hashes.
- Equal archetype sampling slots give about 167 expected appearances for each
  of 12 archetypes per generation. These are averages, not minimum guarantees.
  Actual counts and per-seat counts are saved and published.
- Baseline once, then each newly fitted model: 200 candidate games per archetype
  per anchor. Eight meta plus four same-list pet archetypes, greedy plus the
  original frozen-bootstrap anchor: 4,800 games/checkpoint. Deck pairs play both
  opening orders; the same seeded cohort is used for every model. There are no
  repeated two-hour or duplicate final evaluations.
- Collection truncation stops fitting; evaluation failure stops the next update.
  Existing safety failures are not silently turned into successful draws.

The engine stays on CI image
`ghcr.io/mammalwithashell/digimon-search-cpu@sha256:a98d312fd98a32db0f4c594c004b8aa245321486643c1221c492dcc55dd6f413`,
native source `e1e7fff2c689e8505a8146d9012502be11dce8c1`, successful image CI
40158172185. Python overlays deployed from `0f78b9931` are individually hashed
in `campaign-plan.json`; the controller verified those hashes before starting.
No new engine build was necessary for this Python scheduling/evaluation change.

Training remains network-isolated. The independent monitor owns W&B credentials
and uploads reports from `/opt/digimon-search/small-card-evaluation`. The former
periodic evaluation service is stopped; the campaign controller owns evaluation.
Old service definitions are saved under `deployment-small/previous-units`.

[New W&B run](https://forge.coreweave.com/wandb/james-peters97-mammal/digimon-rl/runs/english-meta-pets-4x1000-20261025)
records the new protocol separately. At launch, both training and monitoring
services were active, the monitor confirmed online sync, and baseline meta
evaluation reported 1,600 games per anchor (200 for each of eight archetypes).
This is launch evidence, not a claim that baseline or any new update completed.

## Validation

52 focused tests passed across generation contracts, replay loading, bounded
selection, evaluation scheduling/cohorts, continuation validation and monitoring.
Tier 0 passed all 13 checks, including 174 verification-tool tests. Python
compilation and whitespace checks passed. Live container preflight verified all
input/runtime hashes and parent validation. The engine was not modified in this
change; no new full Rust test-suite claim is made.
