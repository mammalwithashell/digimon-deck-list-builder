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
