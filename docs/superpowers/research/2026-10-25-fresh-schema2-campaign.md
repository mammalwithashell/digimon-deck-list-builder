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
after validation; this preparation document alone does not assert a live run.
