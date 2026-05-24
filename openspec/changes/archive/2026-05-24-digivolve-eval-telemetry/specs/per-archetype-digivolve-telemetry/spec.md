## ADDED Requirements

### Requirement: Per-archetype agent-side digivolve TB scalars

The pilot training `WinRateCallback` SHALL emit two per-agent-archetype TensorBoard scalars at every eval write, computed cumulatively from the start of the run:

- `pilot/agent_archetype/<X>/digivolves_per_game`
- `pilot/agent_archetype/<X>/dna_digivolves_per_game`

where `<X>` is the sanitized agent archetype name (same `_sanitize` transform used by the existing `pilot/agent_archetype/<X>/win_rate` scalar). The value for each archetype SHALL equal `total_p1_digivolutions_credited_to_X / total_games_credited_to_X` where both numerator and denominator are accumulated across every eval since callback construction.

The agent archetype is sourced from `info["deck1_archetype"]` populated by `GeneralistDeckPoolWrapper`. When `deck1_archetype` is absent (gauntlet mode, fixed agent deck), the scalar SHALL NOT be emitted for that game — matching existing `_agent_archetype_*` guard behavior.

#### Scenario: Cumulative accumulation across evals

- **WHEN** the callback runs three eval cycles for a generalist agent piloting "DNA Omnimon", and across all three cycles the agent plays 30 games as DNA Omnimon, performs 28 regular digivolves total and 2 DNA digivolves total
- **THEN** the final eval's `pilot/agent_archetype/DNA_Omnimon/digivolves_per_game` SHALL equal `28 / 30`
- **AND** the final eval's `pilot/agent_archetype/DNA_Omnimon/dna_digivolves_per_game` SHALL equal `2 / 30`

#### Scenario: Missing agent archetype is silently skipped

- **WHEN** the callback runs an eval game where `info["deck1_archetype"]` is absent (gauntlet mode)
- **THEN** the per-agent-archetype digivolve scalars SHALL NOT be emitted for that game's archetype bucket

#### Scenario: Telemetry fires regardless of shaping flag

- **WHEN** the eval runs with `digivolve_shaping=false`
- **THEN** the per-agent-archetype digivolve scalars SHALL still be emitted with their actual observed values (often zero)

### Requirement: Per-archetype opponent-side digivolve TB scalars

The pilot training `WinRateCallback` SHALL emit two per-opponent-archetype TensorBoard scalars at every eval write, computed cumulatively from the start of the run:

- `pilot/archetype/<X>/opponent_digivolves_per_game`
- `pilot/archetype/<X>/opponent_dna_digivolves_per_game`

where `<X>` is the sanitized opponent archetype name (same convention as the existing `pilot/archetype/<X>/win_rate` scalar). The value SHALL equal `total_p2_digivolutions_when_opponent_was_X / total_games_against_X`.

The opponent archetype is sourced from `info["opponent_archetype"]`. When absent, the scalar SHALL NOT be emitted for that game.

#### Scenario: Opponent-side counters credited from p2_*

- **WHEN** the callback runs five eval games against opponent archetype "Royal Knights" and `p2_dna_digivolutions` at terminal step is `[0, 1, 0, 2, 0]` across the five games
- **THEN** `pilot/archetype/Royal_Knights/opponent_dna_digivolves_per_game` SHALL equal `3 / 5`

### Requirement: Sidecar by_archetype block carries cumulative digivolve counts

The `evals.jsonl` row's `by_archetype` map SHALL extend each value object from the existing `{wins, draws, games, win_rate}` shape to the shape:

```json
{
  "wins": <int>,
  "draws": <int>,
  "games": <int>,
  "win_rate": <float>,
  "digivolves": <int>,
  "dna_digivolves": <int>,
  "opponent_digivolves": <int>,
  "opponent_dna_digivolves": <int>
}
```

where:

- `by_archetype` keys are opponent archetype names (matching the existing wins semantic).
- `digivolves` / `dna_digivolves` are the **agent's** cumulative digivolve counts across all games against this opponent archetype.
- `opponent_digivolves` / `opponent_dna_digivolves` are the **opponent's** cumulative counts in those same games.
- All four count fields SHALL be present even when their value is zero, so the sidecar schema is stable across shaped and unshaped runs.

#### Scenario: Sidecar row carries digivolve counts per opponent

- **WHEN** an eval row is written after 10 games against opponent "BG-Imperialdramon", with the agent performing 9 regular digivolves and 0 DNA digivolves across those games, and the opponent performing 12 regular digivolves and 1 DNA digivolve
- **THEN** the JSONL row's `by_archetype["BG-Imperialdramon"]` SHALL equal `{"wins": <w>, "draws": <d>, "games": 10, "win_rate": <wr>, "digivolves": 9, "dna_digivolves": 0, "opponent_digivolves": 12, "opponent_dna_digivolves": 1}` (with the existing wins fields preserved unchanged)

#### Scenario: Unshaped run still emits zero-valued fields

- **WHEN** `digivolve_shaping=false` and no digivolves occur in any eval game for an opponent archetype
- **THEN** that opponent's `by_archetype` value SHALL still include all four count keys with value `0`

#### Scenario: Older sidecar rows are forward-compatible

- **WHEN** an `evals.jsonl` row written by a pre-change pipeline (lacking the four new keys) is loaded by a lenient JSON reader
- **THEN** the reader SHALL successfully parse the row and treat the missing digivolve keys as absent (no schema error)

### Requirement: Top-level sidecar mean digivolve fields

Each `evals.jsonl` row SHALL include four top-level fields aggregated across all eval games in the current eval window:

- `mean_eval_digivolves_per_game` (agent side, mean across `n_eval_episodes`)
- `mean_eval_dna_digivolves_per_game` (agent side)
- `mean_eval_opponent_digivolves_per_game` (opponent side)
- `mean_eval_opponent_dna_digivolves_per_game` (opponent side)

The agent-side fields SHALL match the values emitted as the existing `pilot/mean_eval_digivolves_per_game` and `pilot/mean_eval_dna_digivolves_per_game` TB scalars. Opponent-side fields SHALL be computed analogously from `p2_*` counters.

Fields SHALL be present unconditionally and default to `0.0` when no digivolves occurred.

#### Scenario: Top-level means present on every row

- **WHEN** an eval row is written
- **THEN** the JSONL row SHALL include all four `mean_eval_..._per_game` keys regardless of shaping or game outcomes

#### Scenario: Top-level means match aggregated counter values

- **WHEN** an eval window runs 100 games, the sum of `p1_digivolutions` at terminal steps across all 100 games is 250, and the sum of `p2_dna_digivolutions` is 17
- **THEN** the row's `mean_eval_digivolves_per_game` SHALL equal `2.5`
- **AND** the row's `mean_eval_opponent_dna_digivolves_per_game` SHALL equal `0.17`

### Requirement: Tally state is private callback state, not engine state

The per-archetype digivolve accumulators SHALL live as instance attributes on `WinRateCallback` (parallel to the existing `_archetype_wins` / `_agent_archetype_wins` dicts) and SHALL be initialized to empty dicts in `__init__`. They SHALL be incremented only at the post-game terminal step inside the same archetype-presence guards (`if opponent_archetype:`, `if agent_archetype:`) that gate the existing wins increments.

The accumulators SHALL NOT be persisted across `WinRateCallback` reconstruction — a resumed training run starts fresh tallies, matching the existing wins-dict lifecycle.

#### Scenario: Resumed run starts fresh tallies

- **WHEN** a training run is checkpointed and then resumed in a new process with a fresh `WinRateCallback` instance
- **THEN** the resumed run's per-archetype digivolve accumulators SHALL start from zero, identical to the existing wins-dict resume behavior
