# Final policy evaluation

User authorized both follow-ups after the four-generation continuation finished:
checkpoint 4 versus itself across different decks, and checkpoint 4 versus 3.

Cloud output: `/opt/digimon-search/final-policy-eval-20261026` on the existing
Hetzner host. Services: `digimon-final-policy-eval.service` and the separate
`digimon-final-policy-report.service`. Neither service automatically restarts.
Training and model promotion remain disabled.

## Frozen inputs

- Source: `/opt/digimon-search/mixed-continuation-20261026`, completed generation 4.
- Native image: `ghcr.io/mammalwithashell/digimon-search-cpu@sha256:6a6515fb1864096f1a431e8a296ce5998dfbfa40206ea501272ca06702d00f5f`.
- Checkpoint 3: `e8bcd070297886bd30ce4446bf9dbbe1570f65039eba1d78ad7c65091bf59773`.
- Checkpoint 4: `f33e15a451b385fd8dcb9961b9351fc54d8535a8509d4cb3fc41a83dd83a86df`.
- Same 13 evaluation lists as the completed experiment: eight held-out meta
  lists and five same-list pet cohorts. No extra training or deck modifications.
- Profile `standard_lite_v2`, schema `standard_lite_v2.4`, 9434 floats, 2192 actions.
- Controller, card database, models, pool and schedules are SHA-256 bound in
  `plan.json`. Source experiment is mounted read-only.

## Schedule and interpretation

Both models use deterministic masked Torch inference, no search, with the
existing `anchored_eval.play_one` and `OpponentWrapper`. Four-game blocks
balance engine seats and opening order. Opponent observations use player 2's
perspective. Failures retain action recordings and fail the evaluation; they
are never scored as draws or silently replaced.

- **Self matrix:** seed 21261026, 78 distinct deck pairings, 40 games each,
  3,120 games total, 480 appearances per archetype. Both players use checkpoint 4.
  Same-deck mirrors are omitted. Every outcome contributes complementary entries
  to the two decks' matrix rows; these are not independent games. This measures
  deck balance under the model, not intrinsic piloting proficiency.
- **Adjacent comparison:** seed 22261026, all 169 ordered deck combinations,
  ten four-game blocks each, 6,760 total games. Models swap engine roles within
  each block. Results normalize to checkpoint 4 regardless of seat. Each model
  pilots each archetype 520 times. The 95% interval resamples complete four-game
  blocks (10,000 bootstrap replicates), not individual correlated games.

Four subprocesses per panel run concurrently (eight total), one Torch thread
each, inside an eight-CPU/24-GiB container without network. Each shard retains
both players' per-card telemetry, a live `progress.json`, and `outcomes.json`.
The controller writes `self-result.json`, `adjacent-result.json`, and `state.json`.

The separate network-enabled publisher waits for successful completion, renders
`report/index.html` and four CSV files, then publishes summary tables, the HTML,
frozen schedules and all eight per-card reports to W&B project `digimon-rl`,
run ID `final-policy-eval-20261026`. A successful `published.json` confirms the
upload; gameplay success alone does not establish W&B publication.

## Verification

`code/tests/tools/test_final_policy_eval.py` checks exact game counts, deck/seat/
opening-order balance, both-player matrix attribution, checkpoint-role
normalization, cancellation of synthetic first-player advantage, and rejection
of duplicate, missing, changed or incomplete outcomes. Six tests passed before
launch. Cloud preflight verified both checkpoint contracts and frozen inputs;
initial games completed normally in all eight workers.

The existing training dashboard remains attached to the completed training run.
Use this evaluation's progress files and standalone report for follow-up status.

## Completed results

Both evaluations completed on October 26 at 10:24 Central: 9,880 games,
no incomplete outcomes. Checkpoint 4 won 3,412 and lost 3,348 against checkpoint
3, a 50.473% score. The paired-block 95% bootstrap interval is 49.660–51.302%.
This does not establish an improvement over the immediate predecessor, despite
checkpoint 4's better results against the older fixed anchors. No promotion was
performed.

Self-match results (480 appearances each): Chronicle 65.0%, SW 60.2%, Toho
Braves 57.3%, TS Jupitermon 55.2%, Chronomon 54.0%, TS Mervamon 53.5%,
ShineGreymon Data Squad 50.0%, Medusamon 47.7%, Examon 46.9%, Virus Busters
43.5%, Light Hybrid 39.2%, Glowing Dawn 39.0%, Three Musketeers 38.5%.

Local report and raw result copies are under
`runs/final-policy-eval-20261026/`; open `report/index.html` for both complete
matchup matrices. The W&B run is
`james-peters97-mammal/digimon-rl/final-policy-eval-20261026`.

Final verification: 21 focused tests passed, one existing optional integration
test skipped; HTML/CSV render smoke passed. Native cloud evaluation completed
all requested games on the frozen image.
