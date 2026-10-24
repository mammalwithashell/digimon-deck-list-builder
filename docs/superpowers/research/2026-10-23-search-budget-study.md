# Search budget study: preregistered protocol, 2026-10-23

The user authorized a budget sweep, tactical tests and seat-balanced evaluation
to choose a useful search-quality/compute tradeoff. Training must remain stopped.
This protocol was written before collecting budget-comparison results.

## Frozen scope and interventions

Use the v3 seed policy/value ONNX model and eight held-out English EX13 lists,
one list for each admitted archetype: Chronicle, Chronomon, Examon, Glowing Dawn,
TS Jupitermon, TS Mervamon, Three Musketeers and Toho Braves. This is the covered
subset with uniform archetype weighting, not a claim about the entire English
meta. Freeze executable, model/metadata, pool, card and admission hashes in the
machine-readable plan. No model fitting, checkpoint promotion or training-shard
generation is allowed. Keep the existing training stop-state unchanged.

The native harness calls the existing engine and PIMC search. Observation/action
contracts, depth/forced-chain caps, legal masks and frozen network are unchanged.
The information regime is **open decklist with exact deck composition** and
unknown arrangements; this does not implement closed-list beliefs. Batch size is
four. Competitive evaluation disables root noise and takes the first most-visited
action; this is an evaluation policy, not a change to self-play exploration.

Sweep total simulations 160/320/640/1280 independently against 4/8/16 worlds.
All combinations divide evenly. Two separate search seeds are paired across
settings. Reference searches are 2560/8 (more per-world exploration) and 2560/32
(broader world coverage). Reference agreement is a proxy, never ground truth.

## Position and tactical panels

Generate eight deterministic greedy-versus-frozen-policy trajectories from the
held-out lists, alternating which policy controls the target deck. Freeze two
non-forced target-player positions per game, at mid/late decision points,
preferring a pending selection for the second when available. This gives sixteen
replayable roots with initial decks/seeds, the exact legal action prefix and root
mask. Replay the same prefix and validate the mask for every budget. Report
trajectory truncations honestly; their positions have no outcome labels.

Measure action agreement, normalized-visit Jensen-Shannon divergence, seed
sensitivity, actual simulations and native seconds per searched position.
Report both references and their consensus denominator. Six synthetic engine
tactics cover immediate lethal, legal play distractions, two-attack lethal,
unnecessary board trades, consecutive same-player breeding/main backup, and
blocking an otherwise terminal attack. Run each at every grid setting, three
seeds, with both uniform guidance and the frozen model. These abstract fixtures
are not archetype/card certification; synthetic IDs use unknown embeddings under
the production registry. Tests of real meta combinations remain a limitation.

## Predeclared finalist selection and games

Among settings passing every tactical check, select a quality leader by mean
agreement with the two references, then seed action stability, lower visit JS
and lower position time. Select a cheap leader by minimum time among those within
10 percentage points of the leader's mean reference agreement. Always retain
320/4 as the baseline and select two distinct alternatives using this ordering.
If no setting passes, still investigate the highest-ranked alternatives but do
not label a failing configuration validated.

For each alternative, run 16 initial-state pairs against 320/4. Each pair uses
identical decks, shuffle seed and initial state twice; the candidate controls
seat 0 once and seat 1 once. Both settings therefore receive both decks and one
first-player game. Search seed derivation is shared across budgets and contains
no budget-dependent randomness. No training exploration noise is used.

Additionally run baseline and both alternatives against fixed Rust greedy and
the frozen policy without search, eight seat pairs per anchor/configuration.
This totals 160 complete-game attempts. The game decision cap is 1200, matching
the health check. Report capped/crashed games in denominators; do not silently
drop failures or score them as wins. A failing native job halts the study and
preserves results. No exploiter or learned-agent promotion claim is made.

## Uncertainty and choice rule

Use pair-average scores (win=1, draw/cap=0.5, loss=0), preserving seat dependence.
Report 95% block-bootstrap intervals over the eight matchup blocks. Widen score
intervals with a fractional-score Wilson interval over seed pairs so an all-tie
small panel does not imply zero uncertainty. These are approximate descriptive
intervals, not exact coverage guarantees or multiple-comparison-adjusted tests.
Report paired anchor-score differences with bootstrap intervals.

An alternative qualifies for a **pilot recommendation** only if all tactics
pass, no evaluation game truncates, both anchor differences' lower interval is
at least -0.05, and its head-to-head lower score interval is at least 0.45 when
cheaper, or strictly above 0.50 when more expensive. Choose the cheapest qualifying
alternative; otherwise retain 320/4 and explicitly report that no replacement
cleared the evidence threshold. Distinguish observed frontier candidates from a
validated optimum. Small panels can be inconclusive and miss rare failures.

All timings are from an occupied Windows development host, four workers with
single-thread native inference. They do not establish Linux cloud sizing. This
fixed-model study also cannot establish the best training budget: equal-compute
comparisons of resulting trained policies require a later authorized experiment.

## Results

**Supplement declared after the position sweep and before inspecting any full-game
outcomes:** the cheap leader under the reference-agreement rule is the existing
320/4 baseline, so the original two-alternative selection produces 640/4 and
1280/8. Add 160/4 as a separate compute challenger even though its reference
agreement misses the screening window. This prevents a proxy from eliminating
all cheaper settings before actual games test them. Preserve the original
two-alternative analysis and report the supplemental family separately. The
same outcome gates, seeds and anchor panels apply; this adds 64 games for 224
complete-game attempts overall. `supplement_plan.json` binds this amendment to
the original plan hash. No full-game outcomes were consulted for the amendment.

Execution is complete. Raw artifacts are stored separately from training under
`runs/2026-10-23-search-quality/`, with a frozen plan, per-job requests/results,
logs and resumption hashes.

### Completed position sweep and controls

All eight corpus trajectories completed naturally. The frozen panel has sixteen
roots: thirteen pending selections and three Main-phase roots. It covers six
phases (`EffectChoice`, `Main`, `SelectHand`, `SelectPermutation`, `SelectReveal`,
`SelectTarget`), but is pending-choice-heavy; do not generalize it to every
strategic Main-phase decision. Both reference configurations agree on the
aggregate preferred action at all sixteen roots.

| Simulations/worlds | Seconds/root | Reference agreement | Two-seed action agreement |
|---|---:|---:|---:|
| 160/4 | 0.649 | 87.5% | 100.0% |
| 160/8 | 0.660 | 87.5% | 100.0% |
| 160/16 | 0.666 | 87.5% | 100.0% |
| 320/4 | 1.271 | 100.0% | 93.8% |
| 320/8 | 1.301 | 87.5% | 100.0% |
| 320/16 | 1.326 | 87.5% | 100.0% |
| 640/4 | 2.571 | 100.0% | 93.8% |
| 640/8 | 2.457 | 100.0% | 93.8% |
| 640/16 | 2.590 | 87.5% | 100.0% |
| 1280/4 | 5.091 | 100.0% | 87.5% |
| 1280/8 | 5.091 | 100.0% | 100.0% |
| 1280/16 | 5.091 | 100.0% | 93.8% |

Reference agreement uses the preferred action of the mean visit distribution
over both seeds, separately against both reference configurations; seed agreement
asks whether the two individual runs chose the same action. A stable action can
still disagree with the references. The 432 tactical checks all passed (36 per
grid setting), so these basic controls do not discriminate among the budgets.
No full-game strength conclusion follows from this table alone. The completed
seat-balanced panels and compute supplement are reported below.

The two 160/4 disagreements are both Main-phase roots (TS Mervamon and Toho
Braves). Thus the cheap setting agrees on all thirteen pending selections but
only one of the three Main roots. This is a small, phase-stratified diagnostic,
not proof that those different actions are inferior. Reference searches also
have residual seed variation: both choose the same action on 15/16 roots across
their two seeds; mean inter-seed visit JS is 0.00737 bits at 2560/8 and 0.00045
at 2560/32. Aggregate reference consensus must not be mistaken for exact truth.

### Completed seat-balanced games and recommendation

All **224/224 games** completed without truncation or native failure: 160 in the
original panel and 64 in the preregistered cheaper-search supplement. No failed
attempts were dropped. Each head-to-head panel contains sixteen seat pairs
(32 games); each anchor panel contains eight pairs (16 games).

| Setting | Fixed-root cost relative to 320/4 | Score against 320/4 | Approximate 95% interval | Greedy anchor | Policy anchor |
|---|---:|---:|---|---:|---:|
| 160/4 | 0.51x | 34.4% | 16.3%-58.5% | 56.2% | 81.2% |
| 320/4 | 1.00x | Baseline | - | 56.2% | 81.2% |
| 640/4 | 2.02x | 65.6% | 41.5%-83.7% | 68.8% | 75.0% |
| 1280/8 | 4.01x | 59.4% | 35.9%-79.2% | 68.8% | 75.0% |

**Retain 320 simulations / four worlds / batch four as the provisional default.**
No alternative cleared the preset replacement gate. The 160/4 point estimate is
substantially worse head-to-head and its interval does not establish the allowed
noninferiority margin. Its identical aggregate anchor scores also hide changes
in individual policy-anchor pair outcomes. The 640/4 setting is the most useful
quality challenger: it has the best observed head-to-head score at about twice
the position cost. Its lower interval remains below 50%, and both larger-budget
settings have uncertain anchor-score differences that fail the -5-point lower
bound. The 1280/8 setting adds about four times the position cost without a better
observed score than 640/4. These are conditional measurements, not proof of
dominance or a globally optimal budget.

At fixed total simulations, eight or sixteen worlds did not improve aggregate
action agreement in this small position panel; spreading a low budget over more
worlds can leave fewer simulations per tree. Neither reference agreement nor
these game panels certify that four worlds adequately capture every hidden-state
uncertainty. Budgets below 160 and world counts below four were not tested.

Full-game cost also depends on behavior. Against the policy-only anchor, mean
worker-seconds/game were 27.66 at 320/4 and 32.92 at 160/4, even though the cheaper
setting roughly halves fixed-root time. The cheaper games contained more search
decisions (761 versus 481 across sixteen games). Thus games/minute alone does not
measure search throughput; report decision workload and time per searched
position alongside terminal-game quality. These elapsed times include changing
host contention and are not CPU-seconds or cloud benchmarks.

The 16-root panel contains only three Main decisions, the tactical controls are
abstract, and the game panels cover eight fixed matchup blocks with a frozen seed
network. The intervals are broad. A future network or broader English-meta pool
may favor a different budget; this study does not establish equal-compute
learning efficiency or justify model promotion. Repeat host calibration on the
actual Linux cloud machine before sizing unattended runs.

The local-only generated `report.md`, `quality_compute.png`, CSV and both
analysis JSON files under the ignored `runs/2026-10-23-search-quality/` directory
preserve the measured frontier and all panel denominators. They are not published
with this report.
All seven frozen input hashes and the original training stop-state were verified
unchanged. Training remains `stopped_by_user`, with `auto_resume: false`.

Validation: ordinary release native example built; native fixture and equal-budget
seat controls passed; Python study/scheduler/contract checks passed (20 passed,
one POSIX-only skip on Windows); strict OpenSpec validation passed. Linux runtime
execution and remote CI execution were not performed here.

Follow-up: the [expanded post-main comparison](2026-10-23-search-budget-confirmation.md)
adds 128 fresh seat-balanced games and PR #746's per-card metrics. On the broader
eight-list pairing matrix, 640/4 scores 54.7% (approximate 95% interval
42.6%-66.3%) at 1.89x observed live selection cost. The interval is about 44%
narrower, still includes 50%, and is reported separately from this earlier build.
The provisional default remains 320/4/batch4; training remains stopped.
