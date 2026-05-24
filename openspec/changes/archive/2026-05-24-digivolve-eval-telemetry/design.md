## Context

The Rust engine exposes four monotonic per-game digivolve counters through `RustHeadlessGame::get_rl_state()` (`p1_digivolutions`, `p1_dna_digivolutions`, `p2_digivolutions`, `p2_dna_digivolutions` — see `code/digimon-engine-py/src/lib.rs:680+`). These were added in PR #538 to feed a per-step reward shaping signal in `DigimonEnv._compute_reward` (`code/digimon_gym/digimon_gym.py:376+`), which reads the counters and pays the agent for incremental digivolves.

`WinRateCallback` (`code/digimon_gym/agents/pilot_training.py:323+`) already:

- Maintains cumulative per-archetype dicts across all evals: `_archetype_wins/draws/games` (opponent-keyed), `_agent_archetype_wins/draws/games` (agent-keyed), and `_matchup_wins/draws/games` (N×N).
- Reads `p1_digivolutions` / `p1_dna_digivolutions` from each game's terminal state and emits two top-level TB scalars (`pilot/mean_eval_digivolves_per_game`, `pilot/mean_eval_dna_digivolves_per_game`) averaged across the current eval window only.
- Writes an `evals.jsonl` row per eval containing `by_archetype: {name: {wins, draws, games, win_rate}}` — cumulative across the run.

**What is missing:** the digivolve counts are never (a) bucketed by archetype, (b) read for the opponent side, (c) accumulated cross-eval, or (d) persisted to the sidecar. A run can complete with zero DNA digivolves on a key archetype and nothing in the on-disk artifacts will reveal it.

## Goals / Non-Goals

**Goals:**

- Make per-archetype digivolve activity observable from saved artifacts alone (no checkpoint replay needed).
- Make the eval sidecar self-sufficient: a future analyst loading `evals.jsonl` can compute "DNA digivolves/game for DNA Omnimon at step 500k" directly.
- Surface the same data in TensorBoard with the existing `pilot/agent_archetype/<X>/...` and `pilot/archetype/<X>/...` naming convention so it integrates with existing dashboards.
- Keep schema stable across `digivolve_shaping=true/false` runs — fields always present, zero-valued when nothing happened.
- Zero behavior change to training: no engine code, no reward calc, no env init touched.

**Non-Goals:**

- Per-step granularity (the engine counters are already per-step-readable; this change is about per-game accumulation only).
- Retroactive backfill of older runs (the data wasn't captured; nothing to recover).
- Changes to the reward shaping itself, including magnitudes (`digivolve_reward`, `dna_digivolve_bonus`) — that is a separate concern the user may revisit after seeing the data.
- Equivalent telemetry for other engine counters (memory deltas, attack count, etc.) — out of scope; this change is digivolve-only.
- New CLI flags. Telemetry fires unconditionally whenever the counters exist (they always exist on the Rust backend).
- Migration / parity for the sunset Python engine (`code/engine_py_legacy/`) — not relevant.
- Touching the training MCP, the engine MCP, or the debug CLI. The training MCP's `run_summary` tool surfaces sidecar rows verbatim, so the new fields will appear automatically — that's a free side-benefit, not a code change.

## Decisions

### Decision 1 — Reuse the existing dict-of-dicts tally pattern, do not introduce a new structure

The callback already carries six per-archetype dicts (`_archetype_wins/draws/games`, `_agent_archetype_wins/draws/games`) plus two matchup dicts. Add four parallel dicts:

```python
self._archetype_opponent_digivolves: Dict[str, int] = {}
self._archetype_opponent_dna_digivolves: Dict[str, int] = {}
self._agent_archetype_digivolves: Dict[str, int] = {}
self._agent_archetype_dna_digivolves: Dict[str, int] = {}
```

**Alternative considered:** a single `_archetype_digivolves: Dict[str, Dict[str, int]]` with nested keys. Rejected — the existing pattern is six flat dicts, not nested; introducing a nested dict here for the same data shape is inconsistent for no benefit. Tracking dicts are private; access cost is the same either way.

### Decision 2 — Read all four counters at the terminal step, credit each to the correct archetype axis

The terminal `final_state = _unwrap_to_digimon_env(eval_env)._rl_state()` block at `pilot_training.py:487-493` already reads `p1_digivolutions` and `p1_dna_digivolutions`. Extend it to also read `p2_digivolutions` and `p2_dna_digivolutions` and credit:

- `_agent_archetype_digivolves[agent_archetype]   += p1_digi`
- `_agent_archetype_dna_digivolves[agent_archetype] += p1_dna`
- `_archetype_opponent_digivolves[opponent_archetype]   += p2_digi`
- `_archetype_opponent_dna_digivolves[opponent_archetype] += p2_dna`

**Naming asymmetry is intentional.** The existing `_archetype_*` dicts are opponent-keyed (because `_archetype_wins[opp]` means "wins vs this opponent"), so the per-opponent dict naturally counts the *opponent's* digivolves. Adding agent-side digivolves to that key would be confusing. Agent-side digivolves go to the `_agent_archetype_*` family.

**Alternative considered:** also push agent digivolves into `_archetype_*` keyed by opponent (so each opponent archetype tells you both "agent did X digivolves when facing this opponent" and "opponent did Y digivolves"). Rejected — the agent's archetype is the natural index for agent digivolves; cross-tabulating belongs in the matchup grid if anyone needs it.

### Decision 3 — Do NOT add a matchup grid for digivolves in this change

The existing `_matchup_*` dicts already produce N×N TB scalars for win_rate/draw_rate/games. Mirroring that for four digivolve fields would emit `4 × N²` new TB scalars per eval, which dominates the dashboard. We can add this later as a strictly additive change if anyone asks for it. Per-agent and per-opponent axes cover the empirical question that motivated the change ("is DNA Omnimon's DNA-digivolve count specifically zero?").

### Decision 4 — Emit telemetry unconditionally, default to zero when no data exists

The counters always exist on the Rust backend (every game has `p1_digivolutions ≥ 0`). The current top-level scalars at `pilot_training.py:580-581` already fire unconditionally — extend that policy to per-archetype scalars and sidecar fields. When a run has `digivolve_shaping=false` and no DNA Omnimon games yet, the corresponding sidecar fields will be `0` (for top-level means) or absent (for per-archetype entries that have not yet been observed) — same behavior as `wins` today for an unseen archetype.

**Why unconditional:** observational telemetry should never depend on whether a reward signal is on. A shaped-vs-unshaped A/B compare needs both legs to emit the same fields.

### Decision 5 — Sidecar `by_archetype` value shape gains four keys; older readers tolerate the addition

Today:

```json
"by_archetype": {"DNA Omnimon": {"wins": 12, "draws": 1, "games": 30, "win_rate": 0.4}}
```

After:

```json
"by_archetype": {"DNA Omnimon": {"wins": 12, "draws": 1, "games": 30, "win_rate": 0.4,
                                  "digivolves": 28, "dna_digivolves": 0,
                                  "opponent_digivolves": 22, "opponent_dna_digivolves": 1}}
```

`by_archetype` keys are the opponent archetype (matches existing wins semantic), so `digivolves` / `dna_digivolves` here are the **agent's** counts when facing this opponent — sourced from the per-game `p1_*` reading, but bucketed by opponent for consistency with the existing wins. `opponent_digivolves` / `opponent_dna_digivolves` are the same opponent's counts in those games.

**This means an additional per-eval dict is needed:** `_archetype_agent_digivolves[opp]` and `_archetype_agent_dna_digivolves[opp]`. So the full tally state grows by **six** dicts, not four:

```python
# Keyed by opponent archetype:
self._archetype_agent_digivolves: Dict[str, int] = {}       # p1 digivolves vs this opp
self._archetype_agent_dna_digivolves: Dict[str, int] = {}   # p1 DNA digivolves vs this opp
self._archetype_opponent_digivolves: Dict[str, int] = {}    # p2 digivolves (this opp)
self._archetype_opponent_dna_digivolves: Dict[str, int] = {}# p2 DNA digivolves (this opp)
# Keyed by agent archetype (generalist mode only):
self._agent_archetype_digivolves: Dict[str, int] = {}
self._agent_archetype_dna_digivolves: Dict[str, int] = {}
```

Wins-style accumulation: every terminal step credits the appropriate buckets, never decrements, persists across `_on_step` calls for the lifetime of the callback.

**Forward compatibility:** Python dict round-trip ignores unknown keys, so readers that just `json.loads` and access `wins/games/win_rate` continue working. Readers that whitelist fields (`pydantic` strict mode etc.) will need to widen — call this out in the runbook update.

### Decision 6 — TB scalar naming follows the existing per-axis convention

Today the per-archetype namespace is two flat axes:

- `pilot/archetype/<opp>/win_rate` (opponent-keyed)
- `pilot/agent_archetype/<agent>/win_rate` (agent-keyed)

Add four parallel scalar families per eval:

- `pilot/agent_archetype/<agent>/digivolves_per_game` = `_agent_archetype_digivolves[agent] / _agent_archetype_games[agent]`
- `pilot/agent_archetype/<agent>/dna_digivolves_per_game` = `_agent_archetype_dna_digivolves[agent] / _agent_archetype_games[agent]`
- `pilot/archetype/<opp>/opponent_digivolves_per_game` = `_archetype_opponent_digivolves[opp] / _archetype_games[opp]`
- `pilot/archetype/<opp>/opponent_dna_digivolves_per_game` = `_archetype_opponent_dna_digivolves[opp] / _archetype_games[opp]`

The opponent-axis scalars are prefixed `opponent_` to disambiguate them from "agent's digivolves vs this opp" (which we do not emit as a per-eval TB scalar — it's available in the sidecar `by_archetype[opp].digivolves` for tooling that wants it, and adding it as a fifth TB family would double the per-archetype scalar count without adding new information once the sidecar carries it).

**Alternative considered:** also emit `pilot/archetype/<opp>/digivolves_per_game` for the agent-side count bucketed by opponent. Rejected on the same volume-vs-value tradeoff as Decision 3.

### Decision 7 — No backfill, no migration tooling

Older `evals.jsonl` rows lack the new fields. Readers fall into two categories:

- **Lenient JSON readers** (the training MCP, ad-hoc scripts): unaffected — they tolerate missing keys.
- **Strict whitelist readers** (none today, but possible future analyzers): would need to widen.

Backfilling old runs is impossible (the counters were not recorded at eval time). Just document the cutover step in the runbook: "rows written before <commit-sha> lack the digivolve fields."

## Risks / Trade-offs

| Risk | Mitigation |
|---|---|
| Sidecar field bloat — each `by_archetype` entry grows from 4 to 8 keys, roughly doubling JSON size for that block. | The block is a tiny fraction of a row (most data is the top-level scalars); 4 extra `int` keys per archetype is negligible vs. the matchup grid which is already in TB but not the sidecar. |
| TB scalar count grows by ~`4 × N_agent_archetypes` per eval, plus `2 × N_opp_archetypes`. For a 50-archetype pool that's ~300 new scalars per eval. | TB handles this volume easily; per-eval frequency is low (every N steps); dashboard noise mitigated by keeping the four families under the existing `pilot/agent_archetype/` and `pilot/archetype/` namespaces so they group with the existing per-archetype scalars. |
| Misattribution if a game terminates with no recorded archetype info. | The existing code already guards on `if opponent_archetype:` and `if agent_archetype:` — new tally lines must live inside those same guards. The unit test covers the no-archetype case. |
| Reading `p2_*` counters when the backend doesn't expose them. | Use `final_state.get("p2_digivolutions", 0)` defensively — same pattern as the existing `p1_*` reads (`pilot_training.py:490-491`). Older Rust binaries built before PR #538 will simply contribute 0s. |
| `digivolves` field in `by_archetype` is the agent's count bucketed by opponent, while at the top level the corresponding field is the agent's count averaged across all opponents. Naming overlap could confuse readers. | The spec defines `by_archetype` keys as opponent-keyed and explicitly documents that `digivolves` there is "agent's digivolves vs this opp." The top-level mean is the same number aggregated. The runbook note should call this out. |
| Forgetting to clear the new tally dicts on callback re-init (e.g., resumed training). | The existing `_archetype_*` dicts have the same lifecycle — initialized once in `__init__`, never reset mid-run. The new dicts share this lifecycle. Resume semantics are unchanged: a resumed run starts fresh tallies, which is already documented behavior for the existing wins dicts. |
| Test fixtures that snapshot the sidecar row shape will break. | One new test (`code/tests/rl/test_training_metrics_digivolve_sidecar.py`) covers the new shape. Any existing snapshot tests of the sidecar row need to be updated in the same PR — tasks list will enumerate them. |

## Migration Plan

No phased rollout needed. The change is additive in both telemetry surfaces, and the underlying counters already exist in the engine. Land the callback patch + the new test in one PR; runs started after the merge emit the new fields.

**Rollback:** revert the patch. Sidecar rows from the brief window with the new fields will keep them; lenient readers will continue working. No data loss either direction.

## Open Questions

- Should top-level mean fields include `opponent_*` variants (`mean_eval_opponent_digivolves_per_game`) or only the agent side? **Recommendation:** include both, for symmetry with the per-archetype TB scalars; cost is two more floats per row. Locked in by the proposal's "What Changes" bullet.
- Should the runbook note mention a one-shot script that re-emits the new fields from saved recordings (where available)? **Recommendation:** no — recordings cover only `--record-games anomalies` episodes for most runs, so a backfill would be partial and misleading. Out of scope.
