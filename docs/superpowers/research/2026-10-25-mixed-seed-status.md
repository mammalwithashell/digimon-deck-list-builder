# Mixed-development seed experiment

The next seed is isolated from `english-meta-pets-light-fresh-2x1100-20261025`.
The default greedy entry point remains unchanged. No running campaign inputs
or weights are replaced.

Preparation command (new image/wheel with `teacher_decision` required):

```powershell
$env:PYTHONPATH='code'
python -m tools.cloud_search.mixed_bootstrap collect --snapshot INPUTS/train_snapshot.json --output OUTPUT/mixed --arm mixed --seed 60261025
python -m tools.cloud_search.mixed_bootstrap fit --output OUTPUT/mixed --seed 60261025
```

Repeat with separate directories and `--arm greedy` / `--arm development` for
matched controls. Default 32 rounds gives 416 games for 13 archetypes. Every
archetype has 64 appearances, 32 per teacher in the mixed arm. Eight-round
blocks independently balance opening seats and G/G, D/D, G/D, D/G assignments.
The fitting set uniformly samples 64 rows with replacement per player-game;
game IDs remain intact for critic held-out splitting. Search replay never
contains these demonstrations. Caps/crashes prevent the fitting manifest and
qualification report from being produced. Preserve the failed output and use
a new directory after investigation, rather than silently skipping a game.

Diagnostics include candidate scores/unknown metadata, per-game events,
per-action net hand changes and the existing per-card route report. Gross
draw/search/retrieval attribution remains unmeasured where native events do
not identify it; net hand changes are not called draw counts.

## Baseline failure discovered during preparation

The existing cloud campaign stopped at **2026-10-25 18:56:12 UTC** in generation
1. Chunk 268 failed twice, exit 101. A separate read-only-input reproduction
matched game 268, decision 117, `SelectSecurity`, player 1 (Rust), memory -6:

```
determinization bug: the viewer's root legal-action set must be
infoset-determined, but world 1 disagrees
left: [62]
right: [30, 62]
```

Game seed: `5659078089716629489`; search seed:
`15930221759238541125`; scheduler seed: `40361025`.
Cloud diagnostic output:
`/opt/digimon-search/mixed-seed-diagnostics-20261025/game268`.
The reproduction used the existing image digest `687fc221...` with exactly
320 simulations, four worlds, batch four. The failed campaign has not been
restarted and has not produced its first fitted checkpoint.

Search-world materialization redealt hidden physical security cards while the
owned menu still pinned their old identities. The repair rebinds hypothetical
security slots and preserves genuinely inspected private cards. The recorded
position passes 32 sampled worlds; blind paging and private-inspection tests
also pass. Root legality assertions and search budgets remain unchanged.

The first local mixed qualification stopped after 23 complete games. Its loop
guard confused successive Light Hybrid effects that requested the same target.
The guard now tracks the visible trigger choice, rather than arbitrary prompt
IDs. The captured prefix is a regression fixture. This partial corpus is not
qualified and will never be fitted. Full qualification runs in fresh directories
on the tested Linux image before any seed fitting or search comparison.


## Full-image qualification caught additional paths

Image `training-search-v2026.10.25.3` / source `266bf6b95` passed CI, but its
32-game cloud preflight reproduced game 268 at decision 117. The materializer
regression alone missed `canonicalize_hidden`, which repartitioned security
before sampling and invalidated the menu there. The regression now invokes
`determinized_search_batched` with the actual search seed; it failed before the
canonicalization repair and passes afterward. Both stages rebind blind slot
pins and preserve entitled private inspection. A recording evaluator also
checks that all four inspected root observations match the real visible root.

A separate full mixed-teacher qualification completed 123 games before rejecting
game 123 (Glowing Dawn versus TS Jupitermon). Its two-card payment collects two
Tamer picks before moving either card, so an unchanged board was legitimate.
The teacher now includes remaining payment count and selected public Tamer slots
in its progress signature, without inspecting captured bindings or hidden card
identities. Trigger provenance also distinguishes physical sources and the
remaining simultaneous choices. The recorded game now completes.

Both failed outputs remain preserved on the cloud host. Neither was fitted:
`mixed-seed-image-validation-20261025/` and
`mixed-seed-teacher-qualification-v3/`. Corrected preflight must run in a fresh
`mixed-seed-image-validation-v4-20261025/` directory before launch.


The v3 diagnostic sweep (no fitting, no corpus reuse) finished all schedules:
greedy 416/416 completed; mixed had three deferred-payment failures;
development had six deferred-payment and two repeated-target failures. The
corrected native teacher completes the two additional captured target games
(102 and 401). Existing PIMC/IS-MCTS determinism, hidden-permutation invariance,
root legality and forced-win tests all pass (five tests).
