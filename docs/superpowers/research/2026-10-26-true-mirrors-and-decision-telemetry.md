# True mirrors and decision telemetry

Training remains stopped. No model was promoted or changed.

## Completed mirror evaluation

Cloud root: `/opt/digimon-search/true-mirror-eval-20261026`.
Local results: `runs/true-mirror-eval-20261026/`.
W&B: <https://forge.coreweave.com/wandb/james-peters97-mammal/digimon-rl/runs/true-mirror-eval-20261026>.

Checkpoint 4 played itself without search, using the identical deck on both
sides: 200 games for each of the 13 frozen lists, 100 starts in each engine
seat. All 2,600 games completed; no incomplete games or draws. Each game used
a distinct seed. Two-game blocks balance opening order; these are not claims
of identical shuffled deals with the first player reversed.

The first player won 1,239 games (47.654%); the second won 1,361 (52.346%).
The first-player 95% opening-pair bootstrap interval was 45.731–49.615%.
This suggests a modest second-player advantage for this checkpoint and pool.
It is not evidence of universal second-player advantage or model improvement.

| Deck | First-player wins / 200 | First-player win rate |
| --- | ---: | ---: |
| Chronicle | 101 | 50.5% |
| Chronomon | 90 | 45.0% |
| Examon | 108 | 54.0% |
| Glowing Dawn | 104 | 52.0% |
| Light Hybrid | 111 | 55.5% |
| Medusamon | 93 | 46.5% |
| SW | 87 | 43.5% |
| ShineGreymon Data Squad | 84 | 42.0% |
| TS Jupitermon | 100 | 50.0% |
| TS Mervamon | 91 | 45.5% |
| Three Musketeers | 94 | 47.0% |
| Toho Braves | 90 | 45.0% |
| Virus Busters | 86 | 43.0% |

Individual deck differences are exploratory, with only 200 games each.
These are opening-order results within mirrors, not archetype rankings.

Frozen checkpoint SHA-256:
`f33e15a451b385fd8dcb9961b9351fc54d8535a8509d4cb3fc41a83dd83a86df`.
Native image:
`ghcr.io/mammalwithashell/digimon-search-cpu@sha256:6a6515fb1864096f1a431e8a296ce5998dfbfa40206ea501272ca06702d00f5f`.
The source experiment was mounted read-only. `plan.json` binds controller,
cards, deck pool, schedules, and model files to hashes. `published.json`
confirms publication of the HTML, CSVs, results, and per-card reports.

## New telemetry

`route_decisions` counts one legal opportunity per player/route/decision,
regardless of how many distinct cards offer it. Its raising choice rate answers
how often a player chooses to evolve in raising when that option is available.
The existing per-card exposure counters remain separate.

`effect_sequences` tracks EX12-001 Nyaromon by physical egg and global turn:
legal DNA opportunity; trigger-order selection or decline; partner and target
acceptance or decline; resolved DNA evolution; optional attack offer, choice,
and resolved declaration. Legal trigger-order opportunities additionally require
a reachable VB DNA target, using the engine's pair-legality check. Completing
a selection is not treated as proof that DNA evolution or attack occurred;
matching engine events must confirm them.

The immutable `get_eval_decision_context` native query supplies provenance from
pending resume frames. Card-evaluation event version is now 2. This diagnostic
context never enters policy tensors, action masks, rewards, or public UI state.
The collector is attached to the inner environment so both search evaluation
and PPO evaluation collect both players. W&B and both dashboards expose the
new tables. Old wheels/report windows mark effect context as unmeasured.

The completed mirror evaluation above used the previous frozen image. It does
not contain these new counters, and they cannot be backfilled from aggregate
card totals. A rebuilt image is required for future Nyaromon measurements.

## Validation

- 25 native event-emission tests passed, including three Nyaromon context tests
  covering legal prompts, separate eggs, unavailable targets, cloning, and
  observer immutability.
- 51 focused Python tests passed across collector, W&B publication, PPO
  callback, search evaluation, both dashboards, and mirror scheduling.
- PyO3 check and build passed. A real environment smoke loaded the new native
  binding and verified context schema 1, event version 2, and measured route
  coverage. All 51 Python tests also passed against that newly built binding.
- The 2,600-game frozen cloud evaluation completed and published successfully.

Native development validation used an isolated Cargo target with optimization
disabled for build speed; it did not change the cloud evaluation runtime. The
unoptimized Windows binding required a 64-MiB Python worker stack; its initial
default-stack smoke overflowed before the enlarged-stack verification passed.
