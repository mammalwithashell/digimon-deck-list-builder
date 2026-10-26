# Checkpoint 3/4 decision telemetry replay

User approved rebuilding the merged telemetry image and evaluating checkpoints
3 and 4 with raising and EX12-001 Nyaromon opportunity/choice counters. Training
and search remain disabled; no checkpoint promotion is automatic.

## Frozen deployment

- Image source: merged PR #750, `a4debbabc0ff1669ba97cc77f003cb6f1a326923`.
- CI: <https://github.com/mammalwithashell/digimon-deck-list-builder/actions/runs/40364669882>, successful image build and native/Python smoke.
- Release tag: `training-search-v20261026-eval-decisions`.
- Immutable image: `ghcr.io/mammalwithashell/digimon-search-cpu@sha256:7c72aaf4b1b00bae85a5d5f7852a5855eefb0d51087162baada8a952f4012433`.
- Read-only preparation/analysis overlay: `decision_telemetry_eval.py`, commit
  `baf1693222de2178fc6f87eab2297300ed9f55aa`, deployed SHA-256
  `f79bcc9f0b4f31141257f6c87bbc91f09b3641b07f2c1698afb30e9b498fd224`.
- Deployment helper: commit `14af5760daa10758c9ed29c4ebd082debd2aa4df`.
- Source root: `/opt/digimon-search/final-policy-eval-20261026`, mounted read-only.
- New root: `/opt/digimon-search/decision-telemetry-eval-20261026/evaluation`.
- Services: `digimon-decision-telemetry-eval.service` and
  `digimon-decision-telemetry-report.service`, no restart, bounded to six hours.
- Evaluator: four CPU workers, 16 GiB, network disabled. Publisher: one CPU,
  4 GiB, existing W&B credentials passed by env-file without exposing contents.

Preparation checked source completion, all frozen file hashes, both model
contracts, unchanged card data, and admission of all 13 single-list cohorts.
The plan records old/new images, source outcome hashes, overlay/controller
hashes, and the unchanged checkpoints and schedule. Existing directories cannot
be silently overwritten or restarted. The deployment helper refuses to start
while the known training services are active.

## Design

Replay the exact 6,760-game adjacent-checkpoint schedule, including model roles
and opening order. Each model has 520 appearances per deck. This is a telemetry
replay, **not additional independent strength evidence**. Compare every complete
outcome to the original run; any difference stops normal publication and retains
a difference report for investigation.

The shared evaluator is unchanged. Native diagnostics are observer-only. The
analysis reads per-game context to attribute both players to their actual model
and deck, even when model roles swap. It rejects unmeasured, incomplete,
duplicate, mismatched or missing telemetry. Whole-deck raising choices and
physical-egg/global-turn Nyaromon sequences remain separate denominators.
Conditional choice rates describe the states each policy reaches, not causal
piloting skill or proof that taking every evolution opportunity is correct.

W&B target: `james-peters97-mammal/digimon-rl/decision-telemetry-eval-20261026`.
After successful evaluation and equivalence checks, the publisher renders an
HTML comparison, per-deck decision CSVs and raw diagnostic JSON, and uploads
them with the existing per-card reports. `published.json` confirms completion;
the run is not considered published merely because games completed.

## Validation and initial status

18 focused tests passed for scheduling, both-player/model attribution,
frozen-input guards, complete source outcomes and telemetry coverage. The CI
image smoke verified native event version 2 and the new decision-context API.
The deployed image passed preparation and all four workers began completing
games. At launch, no training container or training controller was active.
