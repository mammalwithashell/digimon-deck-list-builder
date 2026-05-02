# Fix RL Rust Backend Failures Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `python -m pytest code/tests/rl -v` pass under the current Rust-backed PyO3 installation without hiding legal engine choices or weakening parity tests.

**Architecture:** Add an explicit Rust runner state API at the PyO3 boundary, then move Python RL code away from direct legacy `.game` object access. Preserve player-facing choices by making Rust expose mulligan decisions instead of auto-keeping them in the constructor, which should align the initial tensor phase and mask with the legacy Python runner.

**Tech Stack:** Rust `digimon-engine`, PyO3/numpy bindings in `digimon-engine-py`, Python Gymnasium/SB3 RL wrappers, pytest, cargo.

---

## Failure Summary

The full command currently fails:

```powershell
python -m pytest code/tests/rl -v --tb=short
```

Observed result:

```text
19 failed, 67 passed
```

Failures fall into these groups:

- Rust runner lacks a Python `.game` attribute, but `DigimonEnv`, `OpponentWrapper`, and `HeldOutEvalSuite` dereference `runner.game`.
- Rust `RustHeadlessGame.__new__` auto-accepts mulligans, while Python starts in `GamePhase.Mulligan`, causing initial observation and action-mask parity drift.
- `RustHeadlessGame.get_board_tensor(0)` and `get_board_tensor(3)` silently map through `saturating_sub` instead of raising.

## File Structure

- Modify `code/digimon-engine-py/src/lib.rs`
  - Validate Python player IDs before converting to Rust IDs.
  - Expose a small RL state snapshot API from `RustHeadlessGame`.
  - Expose a Rust greedy-action helper for Python fallback policies.
  - Stop auto-keeping mulligans during construction.
- Modify `code/digimon_gym/digimon_gym.py`
  - Add backend-neutral helpers for current player, winner, game-over, reward state, and greedy action.
  - Compute Rust rewards through explicit runner state instead of `runner.game`.
  - Keep legacy Python behavior for `HeadlessGame`.
- Modify `code/digimon_gym/agents/pilot_training.py`
  - Update `OpponentWrapper` and evaluation win-rate logic to use `DigimonEnv` helpers.
- Modify `code/digimon_gym/agents/eval_suite.py`
  - Update held-out evaluation to use backend-neutral helpers.
- Modify `code/tests/test_rust_bindings_surface.py`
  - Add PyO3 binding tests for player-id validation, RL state snapshot, and explicit mulligan mask.
- Create `code/tests/rl/test_rust_runner_adapter.py`
  - Add focused regression tests for reward and wrapper behavior without touching the broad training suite first.
- Existing tests to drive the work:
  - `code/tests/rl/test_player_id_translation.py`
  - `code/tests/rl/test_rust_python_parity.py`
  - `code/tests/rl/test_eval_suite.py`
  - `code/tests/rl/test_training_smoke.py`
  - `code/tests/rl/test_maskable_recurrent.py`
  - `code/tests/rl/test_onnx_roundtrip.py`
  - `code/tests/rl/test_opponent_pool.py`

---

### Task 1: PyO3 Runner State Surface

**Files:**
- Modify: `code/digimon-engine-py/src/lib.rs`
- Test: `code/tests/test_rust_bindings_surface.py`

- [ ] **Step 1: Write failing PyO3 tests**

Append these tests to `code/tests/test_rust_bindings_surface.py`:

```python
def _starter_decks():
    return ["ST1-01"] * 5 + ["ST1-03"] * 45, ["ST1-01"] * 5 + ["ST1-03"] * 45


def test_rust_headless_game_exposes_rl_state_snapshot():
    import digimon_engine

    deck1, deck2 = _starter_decks()
    runner = digimon_engine.RustHeadlessGame(deck1, deck2, seed=123)
    state = runner.get_rl_state()

    assert state["game_over"] is False
    assert state["winner_id"] is None
    assert state["current_player_id"] in (1, 2)
    assert state["phase"] == "Mulligan"
    assert state["p1_security"] == 0
    assert state["p2_security"] == 0
    assert state["p1_total_dp"] == 0
    assert state["p2_total_dp"] == 0


def test_rust_headless_game_rejects_invalid_board_tensor_player_ids():
    import pytest
    import digimon_engine

    deck1, deck2 = _starter_decks()
    runner = digimon_engine.RustHeadlessGame(deck1, deck2, seed=123)

    with pytest.raises(ValueError, match="player_id must be 1 or 2"):
        runner.get_board_tensor(0)
    with pytest.raises(ValueError, match="player_id must be 1 or 2"):
        runner.get_board_tensor(3)


def test_rust_headless_game_starts_with_explicit_mulligan_choices():
    import numpy as np
    import digimon_engine

    deck1, deck2 = _starter_decks()
    runner = digimon_engine.RustHeadlessGame(deck1, deck2, seed=123)
    mask = np.asarray(runner.get_action_mask())

    assert mask[0] == 1.0
    assert mask[1] == 1.0
    assert mask[60] == 0.0
    assert mask[62] == 0.0
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```powershell
python -m pytest code/tests/test_rust_bindings_surface.py::test_rust_headless_game_exposes_rl_state_snapshot code/tests/test_rust_bindings_surface.py::test_rust_headless_game_rejects_invalid_board_tensor_player_ids code/tests/test_rust_bindings_surface.py::test_rust_headless_game_starts_with_explicit_mulligan_choices -v
```

Expected:

```text
FAILED ... AttributeError: 'digimon_engine.RustHeadlessGame' object has no attribute 'get_rl_state'
FAILED ... DID NOT RAISE
FAILED ... assert mask[0] == 1.0
```

- [ ] **Step 3: Add checked player-id conversion**

In `code/digimon-engine-py/src/lib.rs`, add this helper near `to_python_pid`:

```rust
fn to_rust_pid(py_pid: u8) -> PyResult<u8> {
    match py_pid {
        1 | 2 => Ok(py_pid - 1),
        _ => Err(PyValueError::new_err("player_id must be 1 or 2")),
    }
}
```

Update `RustHeadlessGame.get_board_tensor` to return `PyResult<Bound<'py, PyArray1<f32>>>` and use the helper:

```rust
#[pyo3(signature = (player_id = None))]
fn get_board_tensor<'py>(
    &self,
    py: Python<'py>,
    player_id: Option<u8>,
) -> PyResult<Bound<'py, PyArray1<f32>>> {
    let rust_pid = match player_id {
        None => None,
        Some(pid) => Some(to_rust_pid(pid)?),
    };
    let tensor = self.inner.get_board_tensor(rust_pid);
    Ok(PyArray1::from_vec_bound(py, tensor))
}
```

Update `accept_mulligan` to use the same helper:

```rust
fn accept_mulligan(&mut self, pid: u8, keep: bool) -> PyResult<()> {
    let rust_pid = to_rust_pid(pid)?;
    self.inner
        .accept_mulligan(rust_pid, keep)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))
}
```

- [ ] **Step 4: Stop auto-accepting mulligans**

Remove this block from `RustHeadlessGame::new`:

```rust
let mut this = Self { inner: runner };
while let Some(p) = this.inner.mulligan_current_player() {
    let _ = this.inner.accept_mulligan(p, true);
}
Ok(this)
```

Replace it with:

```rust
Ok(Self { inner: runner })
```

Update the constructor comment above the removed block to:

```rust
// Do not auto-accept mulligans here. The Gym action mask must surface
// keep/mulligan choices to agents so the Rust backend matches the Python
// no-hidden-decisions contract.
```

- [ ] **Step 5: Expose RL state and greedy action**

Add this import near the other Rust imports:

```rust
use ::digimon_engine::policies::greedy::choose_greedy_action;
```

If `choose_greedy_action` is not public, update `code/digimon-engine/src/policies/greedy.rs` so the function used by Rust evaluation is public:

```rust
pub fn choose_greedy_action(game: &Game, pid: PlayerId, mask: &[f32]) -> u16 {
    // Keep the existing body unchanged.
}
```

Add these PyO3 methods inside `impl RustHeadlessGame`:

```rust
fn get_rl_state(&self, py: Python) -> PyResult<PyObject> {
    let game = &self.inner.game;
    let current_player = self.inner.current_decision_player() + 1;
    let p1 = game.player(0);
    let p2 = game.player(1);

    let d = PyDict::new_bound(py);
    d.set_item("game_over", game.game_over)?;
    d.set_item("winner_id", to_python_pid(game.winner.unwrap_or(u8::MAX)))?;
    d.set_item("current_player_id", current_player)?;
    d.set_item("phase", game.current_phase.py_name())?;
    d.set_item("memory", game.memory)?;
    d.set_item("p1_security", p1.security.len())?;
    d.set_item("p2_security", p2.security.len())?;
    d.set_item("p1_total_dp", p1.total_field_dp(&game.card_data))?;
    d.set_item("p2_total_dp", p2.total_field_dp(&game.card_data))?;
    Ok(d.into_py(py))
}

fn greedy_action(&self) -> u16 {
    let mask = self.inner.get_action_mask();
    choose_greedy_action(&self.inner.game, self.inner.current_decision_player(), &mask)
}

#[getter]
fn current_player_id(&self) -> u8 {
    self.inner.current_decision_player() + 1
}
```

If `HeadlessRunner::current_decision_player` is private, change it in `code/digimon-engine/src/runners/headless.rs` from:

```rust
fn current_decision_player(&self) -> PlayerId {
```

to:

```rust
pub fn current_decision_player(&self) -> PlayerId {
```

- [ ] **Step 6: Build and install the PyO3 wheel**

Run:

```powershell
python -m maturin build --manifest-path code/digimon-engine-py/Cargo.toml
python -m pip install --force-reinstall (Get-ChildItem code\digimon-engine-py\target\wheels\digimon_engine-*.whl | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
```

Expected:

```text
Successfully built digimon-engine
Successfully installed digimon-engine-...
```

- [ ] **Step 7: Run PyO3 tests to verify they pass**

Run:

```powershell
python -m pytest code/tests/test_rust_bindings_surface.py::test_rust_headless_game_exposes_rl_state_snapshot code/tests/test_rust_bindings_surface.py::test_rust_headless_game_rejects_invalid_board_tensor_player_ids code/tests/test_rust_bindings_surface.py::test_rust_headless_game_starts_with_explicit_mulligan_choices -v
```

Expected:

```text
3 passed
```

- [ ] **Step 8: Commit**

Run:

```powershell
git add code/digimon-engine-py/src/lib.rs code/digimon-engine/src/runners/headless.rs code/digimon-engine/src/policies/greedy.rs code/tests/test_rust_bindings_surface.py
git commit -m "fix: expose rust runner rl state"
```

---

### Task 2: Backend-Neutral DigimonEnv Helpers

**Files:**
- Modify: `code/digimon_gym/digimon_gym.py`
- Create: `code/tests/rl/test_rust_runner_adapter.py`

- [ ] **Step 1: Write failing adapter tests**

Create `code/tests/rl/test_rust_runner_adapter.py`:

```python
"""Rust-backend adapter behavior for DigimonEnv."""

from __future__ import annotations

import importlib
import os

import numpy as np
import pytest

pytest.importorskip("digimon_engine")


DECK = ["ST1-01"] * 5 + ["ST1-03"] * 45


def _rust_env():
    os.environ["DIGIMON_BACKEND"] = "rust"
    import digimon_gym.digimon_gym as gym_mod

    importlib.reload(gym_mod)
    return gym_mod.DigimonEnv(deck1=DECK, deck2=DECK)


def test_rust_env_reports_current_player_without_legacy_game_object():
    env = _rust_env()
    _obs, _info = env.reset(seed=7)

    assert env.game is None
    assert env.current_player_id in (1, 2)
    assert env.is_game_over is False
    assert env.winner_id is None


def test_rust_env_step_computes_reward_without_runner_game_attribute():
    env = _rust_env()
    _obs, info = env.reset(seed=7)
    valid = np.where(info["action_mask"] > 0)[0]

    obs, reward, terminated, truncated, next_info = env.step(int(valid[0]))

    assert obs.shape == env.observation_space.shape
    assert isinstance(reward, float)
    assert isinstance(terminated, bool)
    assert isinstance(truncated, bool)
    assert next_info["action_mask"].shape == (env.action_space.n,)


def test_rust_env_greedy_policy_uses_rust_policy_surface():
    env = _rust_env()
    _obs, _info = env.reset(seed=7)

    import digimon_gym.digimon_gym as gym_mod

    action = gym_mod.greedy_policy(env)
    assert env.action_mask()[action] > 0
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```powershell
python -m pytest code/tests/rl/test_rust_runner_adapter.py -v
```

Expected before implementation:

```text
FAILED ... AttributeError: 'digimon_engine.RustHeadlessGame' object has no attribute 'game'
```

- [ ] **Step 3: Add helper methods to DigimonEnv**

In `code/digimon_gym/digimon_gym.py`, add these methods inside `class DigimonEnv` after the `game` property:

```python
    def _rl_state(self) -> Dict[str, Any]:
        if self.runner is None:
            return {}
        get_state = getattr(self.runner, "get_rl_state", None)
        if get_state is not None:
            return dict(get_state())
        game = getattr(self.runner, "game", None)
        if game is None:
            return {}
        winner = getattr(game, "winner", None)
        winner_id = getattr(winner, "player_id", winner)
        return {
            "game_over": bool(getattr(game, "game_over", False)),
            "winner_id": winner_id,
            "current_player_id": int(getattr(game, "current_player_id", 1)),
            "phase": getattr(getattr(game, "current_phase", None), "name", None),
            "memory": getattr(game, "memory", 0),
            "p1_security": len(game.player1.security_cards),
            "p2_security": len(game.player2.security_cards),
            "p1_total_dp": sum((p.dp or 0) for p in game.player1.battle_area),
            "p2_total_dp": sum((p.dp or 0) for p in game.player2.battle_area),
        }

    @property
    def current_player_id(self) -> int:
        return int(self._rl_state().get("current_player_id", 1))

    @property
    def is_game_over(self) -> bool:
        if self.runner is None:
            return False
        return bool(self._rl_state().get("game_over", self.runner.is_game_over))

    @property
    def winner_id(self) -> Optional[int]:
        winner = self._rl_state().get("winner_id")
        return int(winner) if winner is not None else None

    def greedy_action(self) -> int:
        if self.runner is None:
            return ACTION_PASS_TURN
        choose = getattr(self.runner, "greedy_action", None)
        if choose is not None:
            return int(choose())
        return greedy_policy(self)
```

Change the `game` property to avoid raising when the Rust runner has no `.game`:

```python
    @property
    def game(self):
        """Back-compat accessor for the legacy Python Game instance.

        Rust runners intentionally do not expose a Python Game object. Use
        current_player_id, is_game_over, winner_id, and _rl_state() for
        backend-neutral RL code.
        """
        return getattr(self.runner, "game", None) if self.runner else None
```

- [ ] **Step 4: Update reward computation**

Replace `_compute_reward` with:

```python
    def _compute_reward(self, terminated: bool) -> float:
        """Compute reward with dense shaping and terminal bonuses."""
        state = self._rl_state()

        if terminated and bool(state.get("game_over", False)):
            winner_id = state.get("winner_id")
            if winner_id == 1:
                return 1.0
            if winner_id == 2:
                return -1.0
            return 0.0

        sec_delta = int(state.get("p1_security", 0)) - int(state.get("p2_security", 0))
        dp_delta = int(state.get("p1_total_dp", 0)) - int(state.get("p2_total_dp", 0))
        return float(sec_delta * 0.01 + dp_delta * 0.0001)
```

- [ ] **Step 5: Update step termination to use helper**

In `step`, replace:

```python
terminated = self.runner.is_game_over
```

with:

```python
terminated = self.is_game_over
```

- [ ] **Step 6: Update greedy_policy Rust path**

At the top of `greedy_policy`, after the mask is computed, add:

```python
    if isinstance(env, DigimonEnv):
        rust_greedy = getattr(env.runner, "greedy_action", None) if env.runner else None
        if rust_greedy is not None:
            return int(rust_greedy())
```

Keep the existing legacy Python logic below this branch.

- [ ] **Step 7: Run adapter tests**

Run:

```powershell
python -m pytest code/tests/rl/test_rust_runner_adapter.py -v
```

Expected:

```text
3 passed
```

- [ ] **Step 8: Commit**

Run:

```powershell
git add code/digimon_gym/digimon_gym.py code/tests/rl/test_rust_runner_adapter.py
git commit -m "fix: adapt rl env to rust runner state"
```

---

### Task 3: Remove `.game` Assumptions from RL Wrappers and Eval

**Files:**
- Modify: `code/digimon_gym/agents/pilot_training.py`
- Modify: `code/digimon_gym/agents/eval_suite.py`
- Test: `code/tests/rl/test_eval_suite.py`
- Test: `code/tests/rl/test_training_smoke.py`

- [ ] **Step 1: Write focused wrapper regression tests**

Append to `code/tests/rl/test_rust_runner_adapter.py`:

```python
def test_opponent_wrapper_reset_and_step_work_with_rust_runner():
    from digimon_gym.agents.pilot_training import OpponentWrapper
    import digimon_gym.digimon_gym as gym_mod

    env = _rust_env()
    wrapped = OpponentWrapper(env, opponent_fn=gym_mod.greedy_policy)
    obs, info = wrapped.reset(seed=11)
    valid = np.where(info["action_mask"] > 0)[0]

    obs, reward, terminated, truncated, info = wrapped.step(int(valid[0]))

    assert obs.shape == env.observation_space.shape
    assert isinstance(reward, float)
    assert isinstance(terminated, bool)
    assert isinstance(truncated, bool)
    assert info["action_mask"].shape == (env.action_space.n,)
```

- [ ] **Step 2: Run focused test to verify it fails before wrapper changes**

Run:

```powershell
python -m pytest code/tests/rl/test_rust_runner_adapter.py::test_opponent_wrapper_reset_and_step_work_with_rust_runner -v
```

Expected before implementation:

```text
FAILED ... AttributeError: 'digimon_engine.RustHeadlessGame' object has no attribute 'game'
```

- [ ] **Step 3: Update OpponentWrapper**

In `code/digimon_gym/agents/pilot_training.py`, replace `_advance_opponent` with:

```python
    def _advance_opponent(self, obs, info):
        """Play opponent turns after reset until Player 1 acts."""
        if self._unwrapped_env.is_game_over:
            return obs, info

        while self._unwrapped_env.current_player_id != 1 and not self._unwrapped_env.is_game_over:
            opp_action = self.opponent_fn(self._unwrapped_env)
            obs, _, terminated, truncated, info = self.env.step(int(opp_action))
            if terminated or truncated:
                break

        return obs, info
```

Replace `_play_opponent` with:

```python
    def _play_opponent(self, obs, info):
        """Auto-play Player 2 turns until Player 1 acts or game ends."""
        terminal_reward = 0.0

        while (
            not self._unwrapped_env.is_game_over
            and self._unwrapped_env.current_player_id != 1
        ):
            opp_action = self.opponent_fn(self._unwrapped_env)
            obs, reward, terminated, truncated, info = self.env.step(int(opp_action))
            if terminated or truncated:
                terminal_reward = reward
                return obs, info, terminal_reward, terminated, truncated

        return obs, info, terminal_reward, self._unwrapped_env.is_game_over, False
```

- [ ] **Step 4: Update WinRateCallback outcome logic**

In `WinRateCallback._run_evaluation`, replace this block:

```python
            game = None
            won = False
            is_draw = False
            if eval_env is not None:
                game = _unwrap_to_digimon_env(eval_env).game
            if game is not None and game.winner is not None:
                if game.winner.player_id == 1:
                    wins += 1
                    won = True
            else:
                draws += 1
                is_draw = True
```

with:

```python
            won = False
            is_draw = False
            winner_id = None
            if eval_env is not None:
                winner_id = _unwrap_to_digimon_env(eval_env).winner_id
            if winner_id == 1:
                wins += 1
                won = True
            elif winner_id == 2:
                pass
            else:
                draws += 1
                is_draw = True
```

- [ ] **Step 5: Update HeldOutEvalSuite**

In `code/digimon_gym/agents/eval_suite.py`, replace the action selection inside `_play_one_game`:

```python
            game = env.runner.game
            action = agent_fn(env) if game.current_player_id == 1 else opponent_fn(env)
```

with:

```python
            action = agent_fn(env) if env.current_player_id == 1 else opponent_fn(env)
```

Replace outcome extraction:

```python
        game = env.runner.game
        if not getattr(game, "game_over", False):
            return "draw"
        winner = getattr(game, "winner", None)
        winner_id = getattr(winner, "player_id", winner)
```

with:

```python
        if not env.is_game_over:
            return "draw"
        winner_id = env.winner_id
```

- [ ] **Step 6: Run focused wrapper/eval tests**

Run:

```powershell
python -m pytest code/tests/rl/test_rust_runner_adapter.py code/tests/rl/test_eval_suite.py -v
```

Expected:

```text
passed
```

- [ ] **Step 7: Run training smoke tests that previously hit `.game`**

Run:

```powershell
python -m pytest code/tests/rl/test_training_smoke.py::test_dummy_vec_env_runs code/tests/rl/test_training_smoke.py::test_same_seed_reproduces_first_action -v
```

Expected:

```text
2 passed
```

- [ ] **Step 8: Commit**

Run:

```powershell
git add code/digimon_gym/agents/pilot_training.py code/digimon_gym/agents/eval_suite.py code/tests/rl/test_rust_runner_adapter.py
git commit -m "fix: remove rl wrapper game assumptions"
```

---

### Task 4: Restore Initial Rust/Python Parity for Mulligan

**Files:**
- Modify: `code/digimon-engine-py/src/lib.rs`
- Modify: `code/digimon-engine/src/runners/headless.rs`
- Test: `code/tests/rl/test_rust_python_parity.py`
- Test: `code/tests/rl/test_player_id_translation.py`

- [ ] **Step 1: Run initial parity tests**

Run:

```powershell
python -m pytest code/tests/rl/test_rust_python_parity.py::test_initial_observation_parity code/tests/rl/test_rust_python_parity.py::test_initial_action_mask_parity code/tests/rl/test_player_id_translation.py::test_invalid_player_id_rejected -v
```

Expected after Tasks 1-3:

```text
3 passed
```

If `test_initial_observation_parity` still fails at tensor index `1`, inspect the phase values with:

```powershell
python - <<'PY'
import importlib, os, numpy as np

for backend in ("py", "rust"):
    os.environ["DIGIMON_BACKEND"] = backend
    import digimon_gym.digimon_gym as gym_mod
    importlib.reload(gym_mod)
    env = gym_mod.DigimonEnv()
    obs, info = env.reset(seed=12345)
    print(backend, "phase_slot", obs[1], "valid", np.where(info["action_mask"] > 0)[0][:10].tolist())
PY
```

The expected output shape is:

```text
py phase_slot 17.0 valid [0, 1]
rust phase_slot 17.0 valid [0, 1]
```

- [ ] **Step 2: Keep Rust construction explicit**

If Rust still starts at Breeding, confirm `RustHeadlessGame::new` contains only:

```rust
Ok(Self { inner: runner })
```

and does not call:

```rust
accept_mulligan
```

or:

```rust
while let Some(p) = this.inner.mulligan_current_player()
```

- [ ] **Step 3: Verify multi-step parity**

Run:

```powershell
python -m pytest code/tests/rl/test_rust_python_parity.py -v
```

Expected:

```text
4 passed
```

- [ ] **Step 4: Commit parity fix if Task 1 did not already commit it**

Run only if this task changed Rust/PyO3 files:

```powershell
git add code/digimon-engine-py/src/lib.rs code/digimon-engine/src/runners/headless.rs
git commit -m "fix: preserve mulligan choices in rust backend"
```

If no files changed in this task, skip the commit.

---

### Task 5: Run and Fix Remaining RL Failures

**Files:**
- Modify only files implicated by the next failing traceback.
- Likely files if failures remain:
  - `code/digimon_gym/digimon_gym.py`
  - `code/digimon_gym/agents/pilot_training.py`
  - `code/digimon_gym/agents/eval_suite.py`
  - `code/digimon-engine-py/src/lib.rs`
  - `code/digimon-engine/src/runners/headless.rs`

- [ ] **Step 1: Run the exact previously failing subset**

Run:

```powershell
python -m pytest `
  code/tests/rl/test_eval_suite.py `
  code/tests/rl/test_maskable_recurrent.py::TestMaskableRecurrentPPOIntegration `
  code/tests/rl/test_onnx_roundtrip.py `
  code/tests/rl/test_opponent_pool.py::test_pool_opponent_fn_sampling `
  code/tests/rl/test_player_id_translation.py `
  code/tests/rl/test_rust_python_parity.py `
  code/tests/rl/test_training_smoke.py `
  -v --tb=short
```

Expected:

```text
passed
```

- [ ] **Step 2: If a `.game` traceback remains, replace that access with helpers**

For any remaining traceback shaped like:

```text
AttributeError: 'digimon_engine.RustHeadlessGame' object has no attribute 'game'
```

apply this mapping:

```python
# old
game = env.runner.game
game.current_player_id
game.game_over
game.winner

# new
env.current_player_id
env.is_game_over
env.winner_id
```

Then add a focused test in `code/tests/rl/test_rust_runner_adapter.py` that exercises the failing call path with `DIGIMON_BACKEND=rust`.

- [ ] **Step 3: If action-mask parity fails, keep the failure actionable**

If parity reports mask indices, run:

```powershell
python - <<'PY'
import importlib, os, numpy as np

def make(backend):
    os.environ["DIGIMON_BACKEND"] = backend
    import digimon_gym.digimon_gym as gym_mod
    importlib.reload(gym_mod)
    env = gym_mod.DigimonEnv()
    obs, info = env.reset(seed=12345)
    return env, obs, info

py_env, py_obs, py_info = make("py")
rs_env, rs_obs, rs_info = make("rust")
diff = np.where(py_info["action_mask"] != rs_info["action_mask"])[0]
print("phase", py_obs[1], rs_obs[1])
print("diff", diff.tolist())
print("py", py_info["action_mask"][diff].tolist())
print("rs", rs_info["action_mask"][diff].tolist())
PY
```

For the known current mismatch `[0, 1, 60, 62]`, the fix is Task 1/4: Rust must start in Mulligan with keep/mulligan legal and hatch/pass illegal.

- [ ] **Step 4: Rebuild PyO3 if Rust changed**

Run this after any change under `code/digimon-engine` or `code/digimon-engine-py`:

```powershell
python -m maturin build --manifest-path code/digimon-engine-py/Cargo.toml
python -m pip install --force-reinstall (Get-ChildItem code\digimon-engine-py\target\wheels\digimon_engine-*.whl | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
```

- [ ] **Step 5: Commit remaining targeted fixes**

Run:

```powershell
git add code/digimon_gym code/digimon-engine-py/src/lib.rs code/digimon-engine/src/runners/headless.rs code/tests/rl
git commit -m "fix: pass rust backend rl tests"
```

---

### Task 6: Full Verification

**Files:**
- No source changes expected.

- [ ] **Step 1: Run binding surface tests**

Run:

```powershell
python -m pytest code/tests/test_rust_bindings_surface.py -v
```

Expected:

```text
passed
```

- [ ] **Step 2: Run full RL suite**

Run:

```powershell
python -m pytest code/tests/rl -v
```

Expected:

```text
86 passed
```

- [ ] **Step 3: Run Rust engine smoke tests touched by PyO3**

Run:

```powershell
cargo test -p digimon-engine --test mask_and_tensor -- --nocapture
```

Expected:

```text
test result: ok
```

- [ ] **Step 4: Check formatting/whitespace**

Run:

```powershell
git diff --check
```

Expected: no output.

- [ ] **Step 5: Commit verification-only docs if updated**

If no files changed, skip this step. If docs or tests were updated during verification:

```powershell
git add docs code/tests
git commit -m "test: cover rust backend rl parity"
```

---

## Self-Review

**Spec coverage:** The plan covers every current failure bucket: `.game` attribute failures, initial parity mismatch, invalid player ID validation, and broad RL training/eval smoke tests.

**Placeholder scan:** No task contains an unresolved placeholder. Task 5 includes explicit known-failure handling and commands for any remaining traceback from the same failure family.

**Type consistency:** Rust uses Python player IDs `1/2` at the PyO3 boundary and Rust player IDs `0/1` internally. Python helpers expose `current_player_id`, `is_game_over`, and `winner_id` consistently across wrappers and eval code.
