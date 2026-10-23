# Expanded 640/4 versus 320/4 evaluation

## Protocol frozen before new outcomes

The user authorized more seat-balanced games, integration of main's new metrics,
and keeping training stopped. Main `f9389161a` (telemetry PR #746) is integrated
by merge `9f7f44f18`. Earlier uncommitted search work was restored without
conflicts; safety stash `65f897fa7a0b690c205d3a832ebaf6a0712451d6` is retained.

Use the same frozen terminal-value seed model and eight held-out English EX13
lists as the original budget study. Search remains exact-composition open-decklist
PIMC, four worlds, batch four, zero root noise, first-most-visited action,
unchanged depth/forced-chain caps and a 1200-decision game cap. Compare 640 total
simulations against 320, with no learning, shard generation or model promotion.

Run a fixed **64 seat pairs / 128 games**: every ordered pairing of the eight
lists once, including mirrors. For index `i*8+j`, game seed is `2026102600+index`
and search master seed `2026102700+index`. Replay the identical initial state for
the second game with budget roles reversed. Each budget controls every list in
sixteen games and goes first exactly once per pair. Queue order is shuffled with
seed 6729; worker count is four and per-pair deadline is 3600 seconds. No optional
stopping, dropped failures, mid-run sample-size changes or pooling with old-build
outcomes. Input/executable/collector/controller and output hashes are frozen.

Primary score is mean pair-average game score (win=1, loss=0, draw/cap=0.5).
Report an approximate 95% interval from bootstrap over the 36 canonical unordered
matchup blocks, retaining seat pairs and repeated directed pairings; widen with
the pair-level fractional Wilson interval as before. This broadens matchup
coverage beyond the old eight-block panel; comparing widths is descriptive,
not a same-population sequential significance test. A lower bound above 50%
supports a head-to-head advantage in this covered pool, subject to zero caps.
This experiment adds no new greedy/policy anchors or independent promotion test.

## Metric integration and validation

The native observer records only actual-game action boundaries, legal masks,
minimal card/hand/board identities, global turns and drained engine events. Its
offline snapshots are internal telemetry artifacts, never model inputs. No
search-rollout events enter counters. The Python adapter reuses main's
`CardEvalCollector` and its existing phase-aware opportunity decoding. Event
field normalization follows PyO3's `event_to_pydict`; pending selections remain
excluded. Capture direct/effect plays, hatch, raising/battle evolution, DNA,
promotion, attacks, memory paid, hand-seen, first-entry turns and legal
opportunity/choice exposures, with outcomes/denominators per card and matchup.

Produce two dashboard-compatible local reports, role-aligning each budget as
player1. These describe the same games from opposite roles, not independent
populations to add together. Budget-labelled CSVs export only each budget's own
rows. Entry routes overlap, absent opportunity routes are unmeasured, and
win-when-entered is correlation rather than causal card strength. Caps remain in
the score denominator but are incomplete for entered-card outcome correlations.
No W&B publishing or training-wheel rebuild is needed for this native/offline
evaluation path; Rust event instrumentation is built directly into the binary.

Before launching: run adapter/collector tests, replay old complete action traces
through the post-main engine with legality/outcome equality, and compare short
native search runs with collection enabled/disabled for identical actions.
Keep the earlier executable intact by using a separate Cargo target directory.

## Results

Preflight passed: native release build; five adapter/report tests plus eleven
upstream collector tests; 20 other scheduler/study/contract checks with one
POSIX-only Windows skip. A searched 12-decision-prefix control at 32/4 preserves every
action with metrics enabled/disabled. Sixteen old completed games replay through
the post-main engine with identical legal prefixes and winners. Collector counts
match actual events, including 34 hatches, three promotions and two new
`EvalDigivolve` events. These replays are validation, not new quality games.

All **128/128 new games** completed in 64 seat pairs, with **zero truncations or
native failures**. Outcomes were 125 security-attack finishes and three deck-outs.
All twelve frozen input hashes, every committed request/result hash, paired
initial snapshots, first-player balance, action traces and budget-role winners
were verified. The original executable and training stop-state are unchanged.

| Panel | Games | 640/4 score | Approximate 95% interval | Interval width |
|---|---:|---:|---|---:|
| Earlier eight-matchup panel | 32 | 65.6% | 41.5%-83.7% | 42.2 points |
| New full eight-list pairing panel | 128 | 54.7% | 42.6%-66.3% | 23.7 points |

The new interval is about **44% narrower**, while the point estimate is smaller:
640/4 won 70 games and lost 58. Its first-player score was 53.1% and second-player
score 56.2%. The interval still includes 50%, so this panel does not establish a
reliable head-to-head advantage for 640/4. Keep **320/4, batch four** as the
practical default. Meaningful improvement from 640/4 remains possible; these
results do not prove the settings equivalent. The old and new panels are not
pooled, because the build and pairing distribution differ. These lists are now
a budget-validation panel rather than untouched final promotion evidence.

640/4 performed 5,157 non-forced searches averaging 2.048 seconds per selection;
320/4 performed 5,111 averaging 1.082 seconds: **1.89x observed selection cost**.
Those are elapsed selection times over different live trajectories, excluding
mask construction, actual action resolution and telemetry output. They are not
an identical-position speed benchmark, total training throughput or Linux cloud
sizing result. The earlier fixed-root comparison measured 2.02x cost.

### New metric results

| Counter | 640/4 | 320/4 |
|---|---:|---:|
| Direct plays | 935 | 940 |
| Effect plays | 4 | 2 |
| Hatches | 138 | 130 |
| Promotions | 15 | 9 |
| Raising digivolutions | 47 | 26 |
| Battle digivolutions | 10 | 10 |
| DNA digivolutions (subset) | 0 | 1 |
| Attacks | 601 | 560 |
| Memory paid | 5,070 | 5,186 |
| Legal card/route opportunities | 12,876 | 13,109 |
| Card/route choices | 1,840 | 1,759 |

These are observed usage counts for this frozen pilot and search budget, not
causal card-strength estimates. Resolved effects can produce entries or attacks
without a corresponding tracked choice. Conversely, a choice can precede a
selection prompt or replacement without resolving an entry. All exported route
choice counts are bounded by their legal opportunity counts. Per-card rows retain
matchup, hand-seen, entry/outcome denominators and first-entry global turns; sums
of per-card game memberships must not be interpreted as additional real games.

Local-only outputs are under the ignored directory
`runs/2026-10-23-search-confirmation/`; they are not published with this report:

- `report.md` and `summary.json`.
- `cards.csv` and `routes.csv`, with explicit budget labels.
- Dashboard-compatible `eval_cards/s640-w4` and `eval_cards/s320-w4` reports; player1 is always the named budget. Both reports describe the same 128 games.
- Immutable plan, per-pair requests/results, logs and completion hashes.

Training remains `stopped_by_user`, with `auto_resume: false`; no model was fit
or promoted. Strict OpenSpec validation, scoped formatting/whitespace checks and
the native/report integrity audit pass. Linux runtime and remote CI execution
were not performed here.
