# Periodic per-card evaluation

Pilot training automatically collects card telemetry during the periodic
`WinRateCallback` evaluation window. Training rollouts do not collect it.
There is no change to action masks, model inputs, rewards, or recurrent state.

Rebuild the Rust Python wheel after updating this code. The evaluation callback
checks `digimon_engine.CARD_EVAL_EVENT_VERSION` so an older wheel cannot silently
report zero promotions or misclassify effect plays. For an existing environment,
build and install a wheel with `maturin build --release --manifest-path
code/digimon-engine-py/Cargo.toml` followed by `python -m pip install --force-reinstall
--no-deps <the-wheel-path-printed-by-maturin>`. Use a worktree-specific
`CARGO_TARGET_DIR` when building from a worktree.

Run training with the existing periodic evaluation flags and optionally `--wandb`:

```powershell
python -m digimon_gym.agents.pilot_training --eval-freq 10000 --eval-episodes 20 --log-dir runs/my-pilot --wandb
```

Each evaluation writes an atomic, versioned JSON file under
`<log-dir>/eval_cards/checkpoint_<step>_<window-id>.json`. The unique window ID
prevents overwrites when resuming at the same step. The report contains:

- Checkpoint step, evaluation window index, schema version, unique window ID.
- Observation-profile/layout metadata and action-space size for provenance.
- Each inner game's seed, deck card counts, model/opponent archetype labels,
  outcome, terminal reason, steps, per-card counters and opportunities.
- Per-card and per-route aggregates, grouped by player and matchup.
- Explicit measurement coverage, including exclusions.

Both players are recorded independently. Player 1 is the learner. Collection
happens inside `DigimonEnv`, before the opponent wrapper can discard intermediate
steps. Every inner BO3 game gets its own record; it is not assigned the match's
winner. An interrupted/reset game is marked incomplete rather than a draw.

## Metric definitions

New reports also include `route_decisions`: one opportunity per player, route,
and decision boundary when **any** legal card offers that route. Choosing one
of three distinct raising evolution cards counts as one choice out of one
opportunity, while the older per-card `routes` table correctly counts three
card opportunities. Use `route_decisions` to answer how often the model evolves
in raising when it can. Both search-policy evaluation and PPO callbacks collect
these fields through the same inner `DigimonEnv` collector.

With a rebuilt native wheel exposing `get_eval_decision_context` (card-evaluation
capability version 2), `effect_sequences` tracks Nyaromon EX12-001's inherited
DNA process. Native diagnostics identify concrete resume stages and the physical
egg; English prompt text is never parsed. Legal opportunities include usable
Nyaromon choices in trigger ordering (including decline-all), partner selection,
and DNA result selection, deduplicated per physical egg/global turn. Ordering a
trigger is not counted as accepting DNA: partner and result accept/decline counts
remain distinct. `dna_completed` requires an actual DNA event for the selected
result in the resolving step. Follow-up attack offers, accept/decline choices,
and actual attack declarations are tracked separately from the DNA count.

These observer-only diagnostics never enter tensors, masks, public UI state,
reward events, or policy decisions. Older wheels retain raising route metrics
but mark effect-sequence coverage **unmeasured**. Old reports cannot be backfilled
from aggregate card counts. W&B publishes `card_eval/route_decisions` and
`card_eval/effect_sequences`; both dashboards expose these tables and coverage.

- **Plays:** resolved Play events, split by the engine's `effect_initiated`
  flag. A discounted play is still direct unless initiated by an effect.
  Using an Option from hand appears in the play opportunity/choice table;
  it is not a permanent's resolved Play entry. This report does not count
  individual Option effect resolutions.
- **Entry routes:** hatch, direct play, effect play, digivolution in raising,
  digivolution in battle, DNA digivolution, and promotion from raising. DNA is
  a subset of digivolutions. Evolving in raising and then promoting produces
  two events; these are not mutually exclusive ways to count unique cards.
  Promotions attribute the event to the stack's top card.
- **Games:** games where the card was in the supplied deck or was observed
  (for example, a generated token). Deck copies are stored per game.
- **Seen in hand:** observed at an inner action boundary, including reset.
  This is not a draw counter; a card drawn and removed inside one effect
  resolution might never be observed in hand.
- **Opportunity:** the legal mask offered at least one action for a card ID
  and route at a decision boundary. Multiple copies/targets count once.
  Repeated decisions count again. These are decision exposures, not games
  with access to the card or distinct physical copies.
- **Choice:** a selected legal action for that card and route. This measures
  the decision, independently of whether the engine eventually resolves an
  entry (for example, after a material prompt or replacement).
- **Supported opportunity routes:** main-phase play, battle/raising
  digivolve, DNA initiation, attacks; counter digivolve; end-of-turn DNA
  initiation and attacks; breeding promotion. Pending selections are
  excluded because their action IDs reuse main-action ranges. Effect-driven
  play/target selections and effect activations have no opportunity rate in
  this schema. An omitted route is unmeasured, not an observed zero.
- **First entry turn:** global engine turn count at the end of the step
  containing the first arrival; not the player's personal turn number.
- **Win when entered:** correlation with the game's outcome, not an estimate
  of causal card strength. The denominator is `completed_games_entered`;
  incomplete games are shown separately and excluded from this rate.

Telemetry-only `EvalDigivolve`, `Hatch`, and `BreedingMove` events cover additional
arrival paths without adding reward occurrences to the existing reward bus.

## Weights & Biases

With an active W&B run, every checkpoint publishes the nonempty `card_eval/cards`,
`card_eval/routes`, `card_eval/route_decisions`, and `card_eval/effect_sequences`
Tables (partitioned for very large reports), with the training
step in every row. `card_eval/checkpoint_step` is the custom chart axis; the
publisher does not set W&B's internal step, which TensorBoard sync also uses.

A versioned `card-evaluation` artifact named `card-eval-<run-id>` contains the
complete `report.json` and the Tables. The dashboard reads all logged artifact
versions, not only the latest summary Table. Upload failures leave local results
available, including for offline runs that are synchronized later.

## Dashboard

Install the optional dashboard dependencies and run locally:

```powershell
python -m pip install -e ".[dashboard]"
python -m streamlit run code/tools/card_eval_dashboard.py
```

Choose **Local reports** and enter an `eval_cards` directory, or choose
**Weights & Biases** and enter `entity/project/run_id`. The W&B SDK uses the
existing login or `WANDB_API_KEY` environment variable on the machine running
the dashboard; no key is placed in a browser URL. Press **Load / refresh** to
fetch newly completed checkpoint artifacts.

Filter by player, model deck, opponent, cards and checkpoints. The dashboard
provides per-card comparisons, entry-route charts, opportunity/choice tables,
choice-rate trends and CSV downloads. Window IDs remain in the tables for
resumed runs at the same step. Charts pool windows at the same training step;
filter and inspect the tables when those windows differ in evaluation setup.

For model comparisons, keep the evaluation opponents, deck pool and sampling
configuration consistent. Periodic evaluation uses its configured opponent
pool; telemetry alone does not make that pool fixed across checkpoints.

## Search/self-play training

Search runs can use the same collector through `tools.anchored_eval_cli`:

```powershell
python -m tools.anchored_eval_cli --candidate model.zip --champions anchors.json --deck-pool-snapshot validation.json --n 24 --device cpu --card-eval-dir eval_cards --checkpoint-step 1 --eval-panel meta_validation
```

This evaluates the **policy without search** against fixed anchors. It does not
measure hypothetical search-tree visits or training-game card usage. Both seats
are recorded; player 1 is the candidate. Reports and W&B table rows separate
panel, anchor policy and evaluation mode. The report also records checkpoint,
anchor and deck-snapshot hashes, engine event version and completion/error state.

`code/tools/cloud_search/card_eval_worker.py` runs these evaluations independently
of a cloud-search experiment. It reads the seed, generation checkpoints, frozen
anchors and panels from the experiment directory and writes a separate output
directory. Defaults are **startup, every two hours, each new stable checkpoint,
and a final window**, with **24 games per anchor per panel**. The current two
anchors and two panels mean 96 games per window. Use `--interval-seconds`,
`--games`, and repeated `--panel LABEL=SNAPSHOT_FILENAME` to adjust sampling;
`--timeout` bounds each panel. This small sample is diagnostic, not a promotion
test. Inspect the per-card denominators before interpreting a change.

Weights stay frozen during a search generation's game collection. Periodic
windows in that phase intentionally carry the same model hash, fixed seeds and
deck snapshots. They are repeated measurements, not new learning or independent
evidence. The checkpoint chart axis is **0 for the seed, generation index + 1
for fitted models**, not PPO timesteps. Training games collected and trigger
are separate provenance fields. Model changes become visible after fitting;
the worker does not alter fitting cadence or create mid-epoch checkpoints.

The dashboard defaults to the most recent window per checkpoint and scope;
select earlier windows explicitly. Panel/anchor/mode/hash/status filters retain
their identity, and failed reports preserve completed games plus incomplete
attempts. PPO reports remain supported, with PPO's existing `--eval-freq` cadence.

In the cloud, the evaluator has no credentials or network, reads training data
through a read-only mount, and is limited to one CPU / 2 GiB. One window runs at
a time. Failures and timeouts are recorded outside training; restart and retry
state are persistent. The separate W&B monitor uploads atomic reports as Tables
and `card-evaluation` artifacts, retaining failed uploads for retry. Training
search quality is unchanged, although concurrent evaluation consumes some CPU.

## Live experiment dashboard

`code/tools/training_dashboard.py` combines experiment progress with the existing
card reports. It is a read-only local view: search throughput and collection ETA,
saved checkpoints, fixed-opponent results, paired seed intervals, per-archetype
outcomes, and candidate card/entry-route metrics. It does not control training.
The older card-report dashboard remains available for inspecting individual
local or W&B artifacts.

Install the existing optional `dashboard` dependencies and put `code` on
`PYTHONPATH`. Create a local, gitignored configuration such as
`runs/training-dashboard/config.json`:

```json
{
  "host": "USER@TRAINING_HOST",
  "identity_file": "/path/to/ssh/key",
  "root": "/opt/digimon-search/EXPERIMENT",
  "cards_json": "/path/to/checkout/data/cards.json"
}
```

Omit `host` and `identity_file` for a local experiment directory. The collector
supports seed-comparison roots and the standalone experiment layout. Start these
as separate processes from the checkout:

```text
python -m tools.cloud_search.sync_dashboard --config runs/training-dashboard/config.json --out runs/training-dashboard
python -m streamlit run code/tools/training_dashboard.py --server.address 127.0.0.1 --server.port 8501 --server.headless true
```

Open `http://127.0.0.1:8501`. Set `DIGIMON_DASHBOARD_CACHE` if the cache lives
elsewhere. The sync process reads through SSH every 60 seconds; the UI rereads
the local cache every 30 seconds. `--once` performs one refresh when a continuous
sync worker is not running; a lock prevents competing writers. These processes
must remain running and do not automatically restart after a machine reboot.
No W&B token is needed: run links come from the experiment's monitor metadata.

Reports are reduced remotely once per file signature, then cached locally.
Deleted/archived reports are removed, unreadable reports are flagged, and failed
refreshes preserve the last snapshot with a stale-data warning. Completed windows
are selected once per seed/checkpoint/panel; resume archives never contribute.
Win rates use actual game outcomes. DNA denominators use exact lists with
printed DNA targets; held-out eligibility is unmeasured unless the local card
catalog matches the frozen experiment's hash. Effect-only DNA is not inferred.
Search throughput counts commits in the current scheduler invocation, excluding
games retained from an earlier invocation; unavailable event logs suppress ETA.

New `anchored_eval_cli` processes write a five-second evaluation heartbeat next
to their card-report directory (`<panel>-progress.json`), or to `--progress-path`.
It records completed games, current matchup and a bounded panel ETA without
changing game seeds, masks, policies or search settings. Output failures are
logged without failing evaluation. Existing images need rebuilding to emit it;
older runs show completed-panel counts only. A saved model is not evidence that
its evaluation is complete, and collection ETA excludes fitting and evaluation.

For a compact status response without opening the UI:

```text
python -m tools.cloud_search.dashboard_snapshot --cache runs/training-dashboard --summary
```

Read `sync_status.json` too, to distinguish a recent snapshot from a failed
refresh. The personal `digimon-training-monitor` Codex skill uses this interface
and the evidence rules above; install it separately from the repository.
