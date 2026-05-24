# Digivolve Reward Shaping — Design

**Status:** Approved (2026-05-23) — ready for implementation plan.
**Owner:** james
**Related:** `code/digimon_gym/digimon_gym.py::_compute_reward`, `code/digimon-engine-py/src/lib.rs::get_rl_state`, `code/digimon-engine/src/game.rs`

## Problem

The RL pilot agent under-utilizes digivolve actions, especially DNA digivolve. Inspection of recorded games shows the agent often plays into the field via raise-from-breeding or hardcasts and skips digivolve lines that would be better long-term plays. We want to nudge the policy toward digivolving via reward shaping while keeping the existing terminal-dominant shape intact — the agent must continue to optimize for winning, not for satisfying the shape.

## Constraints from existing reward shape

The current shape (digimon_gym.py:376) is **deliberately terminal-dominant** after a prior failure mode in which dense per-step rewards (DP delta × 0.0001 + security delta × 0.01) summed to ~+600 over a long game vs. +1.0 for actually winning. The agent learned to camp on a dominant board instead of closing. The current shape:

- Terminal: ±10 (win/loss), +0.001 to +5 fast-win bonus, −1 draw.
- Dense (only fires on game-state-progressing events): ±2.0 per security card moved.
- Step penalty: −0.001.

**Any new dense reward must stay well below the terminal magnitude**, both per-event and cumulatively per game.

## Decisions locked in during brainstorm

| # | Decision | Rationale |
|---|---|---|
| 1 | Reward digivolutions **directly**, not card-draws-as-proxy | Card draws fire on auto start-of-turn draw + effect draws — too noisy to attribute to digivolve behavior. |
| 2 | **Asymmetric** — agent only, never opponent | Security shape is symmetric (±2.0 own/opp) because both halves of "real progress." Digivolve nudge is a policy-shaping prior, not a state-progression signal; rewarding agent for opponent's digivolves would teach a perverse correlation. |
| 3 | **Subtle band**: +0.1 per regular digivolve, +0.3 additional for DNA (DNA total = +0.4) | Expected per-game contribution +0.3 to +1.5. Mostly under +1.5. Vs. terminal ±10, this is ~10% — visible to the gradient, cannot outweigh winning. |
| 4 | **Approach A**: Rust cumulative counters on `Game`, exposed via `get_rl_state()` PyO3 dict, Python takes per-step delta | Mirrors the existing security-delta pattern (`_prev_p1_security` / `_prev_p2_security` in digimon_gym.py:183–184, 423–436) exactly. One pattern across both shaped signals. |
| 5 | **DNA stacks on regular**: a DNA digivolve bumps both counters | Means a single `digivolve_reward` line in `_compute_reward` always fires, plus a separate `dna_digivolve_bonus` line for DNAs. Avoids branching in the reward function. |

## Engine changes (Section 1)

### New fields on `Game`

In `code/digimon-engine/src/game.rs` next to `pub turn_count: u16` (line 203):

```rust
pub n_digivolutions: [u32; 2],       // indexed by Rust 0-based PlayerId; DNA increments this too
pub n_dna_digivolutions: [u32; 2],   // DNA-only counter
```

- Indexed by Rust 0-based `PlayerId` (0 / 1).
- Initialized to `[0, 0]` in `Game::new` / `Game::default`.
- Monotonic per game — never reset, never decremented.
- `u32` will not overflow at realistic step counts; even pathological 100k-step games are 16 orders of magnitude below overflow.

### Increment sites

At the **top of each function, after legality and cost checks pass but before mutating state**, so failed/rejected digivolve attempts do not bump the counter:

| Function | File:line | Counter(s) |
|---|---|---|
| `digivolve_from_hand_inner` | `game_actions.rs:3527` | `n_digivolutions[player] += 1` |
| `digivolve_onto` | `game_actions.rs:2870` | `n_digivolutions[player] += 1` |
| `digivolve_from_hand_onto_breeding` | `game_actions.rs:4073` | `n_digivolutions[player] += 1` |
| `dna_digivolve_inner` | `game.rs:1947` | **both** `n_digivolutions[player] += 1` **and** `n_dna_digivolutions[player] += 1` |
| `dna_digivolve_hand_partner_inner` | `game.rs:2069` | **both** |

Placement is "after legality, before state mutation" because:
- It avoids crediting failed/rejected attempts.
- It groups the counter bump and the state change as a single logical unit — no opportunity for a partial-success path to mutate state without bumping.

### Engine-level test

New file `code/digimon-engine/tests/digivolve_counters.rs` using `DebugRunner`:

1. Drive a regular digivolve → assert `n_digivolutions == [1, 0]`, `n_dna_digivolutions == [0, 0]`.
2. Drive a DNA digivolve → assert `n_digivolutions == [2, 0]`, `n_dna_digivolutions == [1, 0]`.
3. Drive an illegal/rejected digivolve attempt → assert counters unchanged.

Separate file (not folded into `test_cards_behavioral.rs`) because these are engine instrumentation tests, not card-specific behavioral tests.

### Why direct mutation, not event-bus subscription

All five callsites already gate on legality + cost validation. Bumping a `u32` next to the existing state mutation is one branch-free instruction and zero allocation. Event-listener registration could be silently missed by future refactors; the illegal-attempt test pins the placement.

## PyO3 binding surface (Section 2)

### Dict additions to `RustHeadlessGame::get_rl_state`

In `code/digimon-engine-py/src/lib.rs:703` (after `p2_total_dp`):

```rust
d.set_item("p1_digivolutions", game.n_digivolutions[0])?;
d.set_item("p2_digivolutions", game.n_digivolutions[1])?;
d.set_item("p1_dna_digivolutions", game.n_dna_digivolutions[0])?;
d.set_item("p2_dna_digivolutions", game.n_dna_digivolutions[1])?;
```

- `u32` round-trips into Python as `int` via `IntoPy` — no `as` casts. Mirrors how `p1_security: usize` is already passed.
- Python 1/2 convention at the boundary (`p1` = Rust index 0, `p2` = Rust index 1), consistent with `to_python_pid` usage elsewhere in the file.

### Why expose both players (not just p1)

The agent only consumes p1 deltas for reward (asymmetric — see Section 3). Exposing both players is:
- cheap (two more `u32` writes per `get_rl_state` call),
- consistent with the symmetric `p1_security` / `p2_security` already exposed,
- useful for TensorBoard counters and replay analysis (e.g., "how often does the opponent digivolve in losses?"),
- and keeps the option to flip to symmetric shaping later without a binding change if we decide to.

### Not touched

- **Observation tensor** — counters are reward-shaping state, not policy input. Leaking them into the tensor would change trained-policy behavior. Out of scope.
- **Action mask** — unaffected.
- **`engine_py_legacy/`** — not migrating; training runs on the Rust backend (`DIGIMON_BACKEND=rust`).

### Rebuild

```
cd code/digimon-engine-py && maturin develop
```

No `pyproject.toml`, manifest, or version-bump changes — `get_rl_state` is a private dict consumed only by `DigimonEnv`.

### Binding test

Small Python smoke test alongside the Rust integration test:
- Construct a `RustHeadlessGame`, drive a digivolve via the action interface, assert the four new keys come back with expected counts. Catches binding-key typos the Rust-only test cannot.

## Python wrapper + CLI + telemetry (Section 3)

### `TrainingConfig` additions

Three flat fields in `code/digimon_gym/agents/training_config.py:63` (after `mulligan_log`):

```python
# Digivolve reward shaping (asymmetric — agent only, never opponent).
# All three default OFF/zero so existing runs are byte-identical.
digivolve_shaping: bool = False
digivolve_reward: float = 0.1       # per regular digivolve
dna_digivolve_bonus: float = 0.3    # additional on top of digivolve_reward
```

Validation in `_validate`:

```python
if self.digivolve_reward < 0:
    raise ValueError("digivolve_reward must be >= 0")
if self.dna_digivolve_bonus < 0:
    raise ValueError("dna_digivolve_bonus must be >= 0")
```

### `DigimonEnv` constructor

Three new kwargs in `DigimonEnv.__init__` (digimon_gym.py:145), all defaulting OFF so unset callers see no behavior change:

```python
digivolve_shaping: bool = False,
digivolve_reward: float = 0.1,
dna_digivolve_bonus: float = 0.3,
```

Stored on `self`. New prev-state mirror next to the security mirror (digimon_gym.py:183–184):

```python
self._prev_p1_digivolutions: Optional[int] = None
self._prev_p1_dna_digivolutions: Optional[int] = None
```

Reset to `None` in `reset()` alongside the security prevs (digimon_gym.py:312–313).

**Asymmetric is enforced at the wrapper level** by not mirroring `p2`. The binding still exposes `p2_*` for analysis, but `_compute_reward` reads only p1 prevs.

### `_compute_reward` extension

In `_compute_reward` (digimon_gym.py:376), after the security-delta block and before the `return dense_reward - 0.001`:

```python
if self.digivolve_shaping:
    p1_digi = int(state.get("p1_digivolutions", 0))
    p1_dna  = int(state.get("p1_dna_digivolutions", 0))

    if self._prev_p1_digivolutions is not None and self._prev_p1_dna_digivolutions is not None:
        # DNA already stacks on n_digivolutions in the engine, so this
        # naturally pays digivolve_reward on every digivolve and the extra
        # dna_digivolve_bonus only when it was a DNA digivolve.
        d_digi = p1_digi - self._prev_p1_digivolutions
        d_dna  = p1_dna  - self._prev_p1_dna_digivolutions
        if d_digi > 0:
            dense_reward += float(d_digi) * self.digivolve_reward
        if d_dna > 0:
            dense_reward += float(d_dna) * self.dna_digivolve_bonus

    self._prev_p1_digivolutions = p1_digi
    self._prev_p1_dna_digivolutions = p1_dna
```

First step (`_prev_*=None`) credits nothing — matches security handling exactly.

### Magnitude check

| Event | Reward |
|---|---|
| Regular digivolve | +0.1 |
| DNA digivolve | +0.4 (0.1 + 0.3 from the stacked counters) |
| Per-game total (3–15 digivolves, 0–3 DNAs) | +0.3 to +2.7, mostly +0.3 to +1.5 |
| Terminal win | +10 to +15 |
| Security ±10 band | unchanged |
| Step penalty | −0.001 unchanged |

Shaping band is ~10% of terminal — visible to the gradient, dominated by terminal.

### `pilot_training.py` wiring

Update all four `DigimonEnv(deck1=, deck2=, ...)` callsites in `code/digimon_gym/agents/pilot_training.py` (816, 919, 1196/1210/1220, 1321) to thread the three config fields:

```python
DigimonEnv(
    deck1=deck1, deck2=deck2,
    digivolve_shaping=cfg.digivolve_shaping,
    digivolve_reward=cfg.digivolve_reward,
    dna_digivolve_bonus=cfg.dna_digivolve_bonus,
)
```

**No new `argparse` flags.** Users set via the existing override idiom:

```bash
python -m digimon_gym.agents.pilot_training \
  --config configs/training/default.yaml \
  --set digivolve_shaping=true \
  --set digivolve_reward=0.1 \
  --set dna_digivolve_bonus=0.3
```

Consistent with how `tensor_profile`, `record_games`, `mulligan_log`, etc. are already configured.

### TensorBoard telemetry

Two new scalars logged in `WinRateCallback` (`code/digimon_gym/agents/pilot_training.py:323`), alongside the existing `mean_eval_terminal_score` / `mean_eval_dense_reward` / `mean_eval_episode_length` block at lines 557-559:

- `pilot/mean_eval_digivolves_per_game` — averaged across eval-suite games, read from `state["p1_digivolutions"]` at each game's terminal step.
- `pilot/mean_eval_dna_digivolves_per_game` — same, from `state["p1_dna_digivolutions"]`.

Eval-time (not per-training-episode) for symmetry with the existing eval scalars; per-training-episode would be noisy and harder to read against the win-rate curve.

**Fire regardless of `digivolve_shaping`** — observational, not reward-gated. Logging unshaped baselines lets us quantify the shaping effect post-hoc.

### Eval sidecar header

Persist the shaping config into the sidecar so runs are mechanically distinguishable downstream (replay analyzers, gauntlet selectors, retrospectives). Three new top-level fields on `TrainingRunMetadata` (`code/digimon_gym/agents/training_metrics.py:42`), parallel to `training_seed` / `eval_seed`:

```python
digivolve_shaping: bool = False
digivolve_reward: float = 0.0          # default 0 so legacy sidecars round-trip
dna_digivolve_bonus: float = 0.0
```

Populated at sidecar-write time from the same `TrainingConfig`. Top-level (not inside the `hyperparameters: dict`) because downstream tooling will need to filter/group by these without dict introspection. Defaults are zero/`False` so loading a pre-feature sidecar via `TrainingRunMetadata.load(...)` produces correct unshaped semantics.

Two runs with the same hyperparameters but different shaping must never be confusable by tooling.

### Tests

New `code/tests/rl/test_digivolve_shaping.py`:

1. Construct `DigimonEnv(digivolve_shaping=True, digivolve_reward=0.1, dna_digivolve_bonus=0.3)`. Drive one regular digivolve step → assert step reward delta is `+0.1` (after subtracting the −0.001 step penalty). Drive one DNA digivolve → assert `+0.4`. Drive a non-digivolve step → assert `−0.001` only.
2. **Byte-identical default**: construct `DigimonEnv()` with no shaping kwargs; verify a seeded game produces identical step-reward sums to the pre-change baseline. Snapshot test.
3. **First-step `_prev_*=None`**: assert no credit on the very first step even if a digivolve happens immediately.

## Engine panic interaction

`generalist_1m` runs with `--record-games anomalies` because of the open engine panic `G-PERMANENT-EMPTY-DURING-BATCH-DELETION`. Reward shaping reads counters **after** `step()` resolves — it cannot exacerbate the panic. It can, however, **mask** it in metrics if a shaped agent that digivolves more hits the panicky path more often. The eval sidecar's per-run shaping config (above) provides the bookkeeping needed to attribute new anomaly recordings to shaping vs. the latent panic.

Recommendation: both baseline and shaped runs keep `--record-games anomalies` ON so anomaly traces remain comparable.

## Run management

Out of scope for this spec. The user will kick off paired or sequential runs at their discretion after the feature lands.

## Non-goals

- Adding digivolve counters to the **observation tensor** (would change trained-policy behavior).
- Penalizing the agent for **not** digivolving (the shape is a pull-toward, not a push-away).
- Rewarding opponent digivolves (deliberately asymmetric — see Decision 2).
- Migrating counters to the legacy Python engine (sunset; training is on Rust backend).
- Differentiating regular digivolve types (from-hand vs. onto vs. from-hand-onto-breeding) — all collapse into `n_digivolutions`. If granularity is wanted later, the field becomes `[u32; 5]` and is a strictly additive change.

## Files touched

| File | Change |
|---|---|
| `code/digimon-engine/src/game.rs` | New fields on `Game`; init in `new`/`default`; increments in `dna_digivolve_inner`, `dna_digivolve_hand_partner_inner`. |
| `code/digimon-engine/src/game_actions.rs` | Increments in `digivolve_from_hand_inner`, `digivolve_onto`, `digivolve_from_hand_onto_breeding`. |
| `code/digimon-engine/tests/digivolve_counters.rs` | New integration test file. |
| `code/digimon-engine-py/src/lib.rs` | Four dict keys appended to `get_rl_state`. |
| `code/digimon_gym/digimon_gym.py` | Constructor kwargs, prev-state mirror, `_compute_reward` extension, `reset` clearing. |
| `code/digimon_gym/agents/training_config.py` | Three flat config fields + validation. |
| `code/digimon_gym/agents/pilot_training.py` | Thread config through four `DigimonEnv(...)` callsites. Telemetry callback additions. |
| `code/tests/rl/test_digivolve_shaping.py` | New behavioral tests for shaping math and default-off byte-equality. |
| `code/digimon_gym/agents/training_metrics.py` | Three new top-level fields on `TrainingRunMetadata`. |

## Implementation order

Sections must land in dependency order:
1. Engine counters + Rust integration test (Section 1) — passes on its own.
2. PyO3 binding + Python smoke test (Section 2) — depends on (1); rebuild via `maturin develop`.
3. `TrainingConfig` + `DigimonEnv` + `_compute_reward` + tests (Section 3a–3c, 3g) — depends on (2).
4. `pilot_training.py` wiring + telemetry + eval sidecar header (Section 3d–3f) — depends on (3).

Each step is independently committable.
