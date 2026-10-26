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

## Completed results and fidelity finding

All 6,760 games finished in 331 seconds (about 1,225 games/minute). Every outcome
matched the original evaluation, with no missing or incomplete games. Checkpoint
4's win score remains 50.473%; this replay adds measurements, not statistical
strength evidence. The runtime therefore preserved sampled game behavior.

| Measurement | Checkpoint 3 | Checkpoint 4 |
| --- | ---: | ---: |
| Raising choices / legal decision opportunities | 25,415 / 33,509 (75.85%) | 25,342 / 33,477 (75.70%) |
| Virus Busters appearances | 520 | 520 |
| Nyaromon legal sequence opportunities | 20 | 16 |
| Nyaromon completed DNA evolutions | 19 | 12 |
| All resolved Virus Busters DNA evolutions | 61 | 45 |
| VB player-games with Nyaromon attacks but no DNA by that player | 44 | 54 |

Raising behavior is essentially stable by this aggregate descriptive measure.
The Nyaromon DNA samples are small and do not establish regression or a need to
reward every available evolution. Critically, the final row is an existing
rules-fidelity problem, not evidence of successful DNA follow-up play.

The diagnostics found 109 Nyaromon-attributed attack declarations across 98
player-games with no DNA evolution by that player anywhere in the game. This
is a conservative detectable subset; attacks following an unrelated earlier
DNA evolution are not flagged by this check.

Replayed game index 480, seed 44522532, reproduces the problem. At step 53,
player 2 (checkpoint 3, Virus Busters) receives EX12-001's follow-up attack
prompt while `p2_dna_digivolutions` is zero, selects action 159, and declares
an attack with EX12-032 WereGarurumon. The only DNA event in the entire game
belongs to player 1 and occurs later, at step 79. The trace is saved in
`report/nyaromon-trace-480.json`.

Printed image `EX12-001.webp` explicitly says the DNA-digivolved Digimon may
attack. DCGO's `EX12/Yellow/EX12_001.cs` puts that attack inside the DNA operation's
`SuccessProcess`. Our YAML instead places an unconditional `may_attack_now`
after `may_dna_digivolve_now`; the latter returns without a prompt when there
is no legal partner/target, allowing the following attack step to execute.
No gameplay fix was applied during this evaluation, preserving its provenance.

The analysis follow-up, commit `cc60e86ed`, adds a report warning and retains
the affected game indices. Its deployed analyzer SHA-256 is
`8cb31f4e5d82e1a72feb1b70d21aacc94613e430ce5ab665d641411590c229af`;
the original preparation overlay and plan remain unchanged. The revised HTML,
raw warning rows, and reproduction trace are published to the same W&B run as
an updated result artifact. All 19 focused tests pass, including a check that
an opponent's DNA cannot justify the affected player's Nyaromon attack.

Next priority: gate the attack on this effect's successful DNA result and test
unavailable/declined DNA plus result identity, then repeat the evaluation on the
fixed engine before choosing another training run. Do not treat the present
Virus Busters scores as rules-faithful. Training remains stopped.
