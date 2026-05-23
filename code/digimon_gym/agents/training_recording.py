"""Training game recording artifact helpers."""

from __future__ import annotations

import json
import random
import re
import sys
import traceback
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, Optional

import gymnasium
import numpy as np

from digimon_engine import ACTION_SPACE_SIZE
from digimon_gym.digimon_gym import DigimonEnv


RECORDING_MODES = {"off", "all", "sampled", "draws", "anomalies", "eval"}


@dataclass
class RecordingDecision:
    record: bool
    priority: bool = False


class TrainingGameRecorder:
    """Writes one JSON artifact per selected completed game."""

    def __init__(
        self,
        output_dir: str | Path,
        mode: str = "off",
        max_recordings: int = 25,
        sample_rate: float = 0.01,
        record_tensors: bool = False,
        run_metadata: Optional[Dict[str, Any]] = None,
        seed: Optional[int] = None,
    ) -> None:
        if mode not in RECORDING_MODES:
            raise ValueError(f"Unknown recording mode {mode!r}")
        self.output_dir = Path(output_dir)
        self.mode = mode
        self.max_recordings = int(max_recordings)
        self.sample_rate = float(sample_rate)
        self.record_tensors = bool(record_tensors)
        self.run_metadata = dict(run_metadata or {})
        self._rng = random.Random(seed)
        self.saved_count = 0
        self.seen_count = 0

    @property
    def enabled(self) -> bool:
        return self.mode != "off" and self.max_recordings != 0

    def should_record(self, source: str, outcome: Dict[str, Any]) -> RecordingDecision:
        if not self.enabled:
            return RecordingDecision(False)
        priority = bool(outcome.get("draw_reason") or outcome.get("error") or outcome.get("invalid_action"))
        if self.saved_count >= self.max_recordings and not priority:
            return RecordingDecision(False)
        if self.mode == "all":
            return RecordingDecision(True, priority)
        if self.mode == "eval":
            return RecordingDecision(source == "eval", priority)
        if self.mode == "draws":
            return RecordingDecision(outcome.get("result") == "draw", priority)
        if self.mode == "anomalies":
            return RecordingDecision(priority, priority)
        if self.mode == "sampled":
            return RecordingDecision(self._rng.random() < self.sample_rate or priority, priority)
        return RecordingDecision(False)

    def write(
        self,
        env: DigimonEnv,
        *,
        source: str,
        env_index: int = 0,
        game_index: int = 0,
        step_count: int = 0,
        terminated: bool = False,
        truncated: bool = False,
        invalid_action: bool = False,
        error: Optional[BaseException] = None,
    ) -> Optional[Path]:
        self.seen_count += 1
        recording = env.get_recording()
        outcome = build_outcome_metadata(
            env,
            step_count=step_count,
            terminated=terminated,
            truncated=truncated,
            invalid_action=invalid_action,
            error=error,
        )
        decision = self.should_record(source, outcome)
        if not decision.record:
            return None
        if recording is None:
            recording = {"initial_state": None, "actions": [], "total_actions": 0}

        artifact = {
            "schema_version": 1,
            "created_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
            "run": {
                **self.run_metadata,
                "source": source,
                "env_index": env_index,
                "game_index": game_index,
                "recording_mode": self.mode,
                "record_tensors": self.record_tensors,
                "action_space_size": ACTION_SPACE_SIZE,
            },
            "outcome": outcome,
            "recording": recording,
        }
        assert_minimal_training_recording_artifact(artifact)

        self.output_dir.mkdir(parents=True, exist_ok=True)
        path = self.output_dir / self._filename(source, env_index, game_index, outcome)
        path.write_text(json.dumps(artifact, indent=2, sort_keys=True), encoding="utf-8")
        self.saved_count += 1
        return path

    @staticmethod
    def _filename(source: str, env_index: int, game_index: int, outcome: Dict[str, Any]) -> str:
        result = _safe_slug(str(outcome.get("result") or "unknown"))
        reason = _safe_slug(str(outcome.get("win_reason") or outcome.get("draw_reason") or "unknown"))
        return f"{_safe_slug(source)}_env_{env_index:03d}_game_{game_index:06d}_{result}_{reason}.json"


class TrainingRecordingWrapper(gymnasium.Wrapper):
    """Captures completed episode recordings before vector env auto-reset.

    Also converts engine panics into terminal steps so training survives
    `pyo3_runtime.PanicException` (which derives from `BaseException`).
    The crash is recorded as a draw artifact and the episode terminates;
    SB3's VecEnv auto-resets and training continues with a fresh game.
    """

    # Class-level tally so multiple env factories share visibility.
    crash_count: int = 0

    def __init__(
        self,
        env: gymnasium.Env,
        recorder: Optional[TrainingGameRecorder],
        *,
        source: str,
        env_index: int = 0,
    ) -> None:
        super().__init__(env)
        self.recorder = recorder
        self.source = source
        self.env_index = env_index
        self.game_index = 0
        self.episode_steps = 0
        # Cached last-good observation, used as the terminal observation
        # when the engine panics mid-step. The post-crash Rust state may
        # be poisoned, so we cannot safely query the engine for a fresh
        # tensor; an in-distribution snapshot from the prior step is the
        # least-bad terminal obs for value bootstrapping.
        self._last_obs: Optional[np.ndarray] = None
        self._last_info: Dict[str, Any] = {}

    def reset(self, **kwargs):
        self.episode_steps = 0
        obs, info = self.env.reset(**kwargs)
        self._last_obs = obs
        self._last_info = dict(info)
        return obs, info

    def step(self, action):
        digimon_env = _unwrap_to_digimon_env(self.env)
        mask = digimon_env.action_mask()
        action_id = int(action)
        invalid_action = bool(action_id < 0 or action_id >= len(mask) or mask[action_id] <= 0)
        try:
            obs, reward, terminated, truncated, info = self.env.step(action)
        except (KeyboardInterrupt, SystemExit):
            # User-initiated; never swallow.
            raise
        except BaseException as exc:
            # Engine panic (PyO3 PanicException derives from
            # BaseException in <0.20, Exception in >=0.20 — BaseException
            # covers both). Record the crash, log loudly, and synthesize
            # a terminal step so SB3 auto-resets and training continues.
            TrainingRecordingWrapper.crash_count += 1
            exc_type = type(exc).__name__
            exc_msg = str(exc)
            print(
                f"[recorder env={self.env_index} game={self.game_index} "
                f"step={self.episode_steps}] ENGINE PANIC "
                f"#{TrainingRecordingWrapper.crash_count}: {exc_type}: "
                f"{exc_msg[:200]}",
                file=sys.stderr,
                flush=True,
            )
            if self.recorder is not None:
                try:
                    self.recorder.write(
                        digimon_env,
                        source=self.source,
                        env_index=self.env_index,
                        game_index=self.game_index,
                        step_count=self.episode_steps,
                        terminated=False,
                        truncated=False,
                        invalid_action=invalid_action,
                        error=exc,
                    )
                except BaseException:
                    # A poisoned Rust state after a panic may make
                    # get_recording / terminal_outcome themselves
                    # panic. Suppress so we still synthesize the
                    # terminal step.
                    pass
            self.game_index += 1
            # Synthesize terminal step. terminated=True signals natural
            # episode end (SB3 will not bootstrap V(s) — appropriate
            # because the engine state is invalid). Reward 0 — the crash
            # is not a learning signal.
            term_obs = (
                self._last_obs
                if self._last_obs is not None
                else np.zeros_like(digimon_env.observation_space.sample())
            )
            term_info = dict(self._last_info)
            term_info["engine_crash"] = True
            term_info["engine_crash_type"] = exc_type
            term_info["engine_crash_message"] = exc_msg
            # Zero out the action mask on the terminal step — no valid
            # actions in a crashed game state. The reset-side mask is
            # what the policy will see after auto-reset.
            term_info["action_mask"] = np.zeros(ACTION_SPACE_SIZE, dtype=np.float32)
            return term_obs, 0.0, True, False, term_info
        self.episode_steps += 1
        self._last_obs = obs
        self._last_info = dict(info)
        if terminated or truncated:
            if self.recorder is not None:
                self.recorder.write(
                    digimon_env,
                    source=self.source,
                    env_index=self.env_index,
                    game_index=self.game_index,
                    step_count=self.episode_steps,
                    terminated=bool(terminated),
                    truncated=bool(truncated),
                    invalid_action=invalid_action,
                )
            self.game_index += 1
        return obs, reward, terminated, truncated, info


def build_outcome_metadata(
    env: DigimonEnv,
    *,
    step_count: int,
    terminated: bool,
    truncated: bool,
    invalid_action: bool = False,
    error: Optional[BaseException] = None,
) -> Dict[str, Any]:
    if error is not None:
        return {
            "result": "draw",
            "winner_id": None,
            "win_reason": None,
            "draw_reason": "crash",
            "terminated": False,
            "truncated": False,
            "step_count": step_count,
            "invalid_action": invalid_action,
            "error": {
                "type": type(error).__name__,
                "message": str(error),
                "traceback": traceback.format_exception_only(type(error), error),
            },
        }

    terminal = env.terminal_outcome()
    winner = terminal.get("winner_id")
    reason = terminal.get("reason")
    result = terminal.get("result")
    if truncated and winner is None:
        result = "draw"
        draw_reason = "step_limit"
        win_reason = None
    elif result == "win" and winner is not None:
        win_reason = reason or "unknown_win"
        draw_reason = None
    elif terminated or truncated or terminal.get("game_over"):
        result = "draw"
        win_reason = None
        draw_reason = reason if reason and reason.endswith("draw") else "unknown_draw"
    else:
        result = None
        win_reason = None
        draw_reason = None

    return {
        "result": result,
        "winner_id": winner,
        "win_reason": win_reason,
        "draw_reason": draw_reason,
        "terminated": bool(terminated),
        "truncated": bool(truncated),
        "step_count": step_count,
        "invalid_action": invalid_action,
        "error": None,
    }


def assert_minimal_training_recording_artifact(artifact: Dict[str, Any]) -> None:
    for key in ("schema_version", "run", "outcome", "recording"):
        if key not in artifact:
            raise AssertionError(f"recording artifact missing {key!r}")
    recording = artifact["recording"]
    if "initial_state" not in recording:
        raise AssertionError("recording missing initial_state")
    if "actions" not in recording:
        raise AssertionError("recording missing actions")
    if recording.get("total_actions", len(recording["actions"])) != len(recording["actions"]):
        raise AssertionError("recording total_actions does not match action count")
    outcome = artifact["outcome"]
    if outcome.get("result") == "win":
        if outcome.get("winner_id") is None or not outcome.get("win_reason"):
            raise AssertionError("win outcome requires winner_id and win_reason")
    if outcome.get("result") == "draw" and not outcome.get("draw_reason"):
        raise AssertionError("draw outcome requires draw_reason")


def _unwrap_to_digimon_env(env: gymnasium.Env) -> DigimonEnv:
    unwrapped = env
    while not isinstance(unwrapped, DigimonEnv):
        if isinstance(unwrapped, gymnasium.Wrapper):
            unwrapped = unwrapped.env
        else:
            raise RuntimeError(f"Could not find DigimonEnv inside {type(env).__name__}")
    return unwrapped


def _safe_slug(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "_", value).strip("_") or "unknown"
