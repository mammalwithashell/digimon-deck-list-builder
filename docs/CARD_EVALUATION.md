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

With an active W&B run, every checkpoint publishes `card_eval/cards` and
`card_eval/routes` Tables (partitioned for very large reports), with the training
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
