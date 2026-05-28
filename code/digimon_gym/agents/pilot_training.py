"""Pilot Agent training with MaskablePPO / MaskableRecurrentPPO.

Trains a PPO-based pilot agent to play Digimon TCG matches.
The agent controls Player 1 while an opponent policy (greedy by default)
controls Player 2. Action masking prevents illegal moves.

Supports two modes:
- MLP (default): MaskablePPO with standard feedforward policy
- LSTM (--lstm): MaskableRecurrentPPO with LSTM memory for partial observability

Usage:
    python -m digimon_gym.agents.pilot_training
    python -m digimon_gym.agents.pilot_training --lstm --timesteps 500000
    python -m digimon_gym.agents.pilot_training --self-play --timesteps 1000000

Requires: pip install stable-baselines3 sb3-contrib tensorboard
"""

import os
import argparse
import json
import random
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable, Dict, Optional, List, Set, Union

import numpy as np
import gymnasium
import yaml

_REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_MODELS_DIR = str(_REPO_ROOT / "models")

from sb3_contrib import MaskablePPO
from sb3_contrib.common.wrappers import ActionMasker
from stable_baselines3.common.callbacks import BaseCallback
from stable_baselines3.common.vec_env import DummyVecEnv, SubprocVecEnv

from digimon_gym.digimon_gym import DigimonEnv, greedy_policy, ACTION_PASS_TURN
from digimon_gym.tensor_profiles import get_tensor_profile
from digimon_engine import (
    ACTION_SPACE_SIZE,
    REGISTRY_CAPACITY,
    EMBEDDING_DIM,
    load_implemented_card_ids,
)
from digimon_gym.agents.gauntlet import (
    GeneralistDeckPool,
    GeneralistDeckPoolWrapper,
    MetaGauntlet,
    GauntletWrapper,
    UnimplementedDeckError,
    load_generalist_deck_pool,
    validate_implemented_deck,
)
from digimon_gym.agents.features_extractor import CardEmbeddingExtractor
from digimon_gym.agents.maskable_recurrent import (
    MaskableRecurrentPPO,
    MaskableMlpLstmPolicy,
)
from digimon_gym.agents.env_utils import (
    find_match_env as _find_match_env,
    find_reward_profile_wrapper as _find_reward_profile_wrapper,
    unwrap_to_digimon_env as _unwrap_to_digimon_env,
)
from digimon_gym.agents.opponent_pool import OpponentPool
from digimon_gym.agents.training_config import TrainingConfig
from digimon_gym.agents.training_recording import (
    TrainingGameRecorder,
    TrainingRecordingWrapper,
)
from digimon_gym.agents.mulligan_log import MulliganLogWrapper, MulliganLogWriter
from digimon_gym.agents.game_log import GameLogWriter


# ─── Helpers ─────────────────────────────────────────────────────────


@dataclass
class _MulliganLogConfig:
    """Per-run config for the mulligan-log subsystem. Constructed once in
    train(); per-env-index MulliganLogWriter instances are built inside the
    env factory from this config (writers can't be shared across SubprocVecEnv
    subprocesses)."""

    output_dir: Path
    enabled: bool
    run_metadata: Dict[str, Any]


def _find_eval_recording_path(eval_env) -> Optional[Path]:
    """Walk the env wrapper chain to find a TrainingRecordingWrapper and
    return its `last_recording_path`. Returns None if no recording
    wrapper is in the stack."""
    cur = eval_env
    while cur is not None:
        if isinstance(cur, TrainingRecordingWrapper):
            return cur.last_recording_path
        cur = getattr(cur, "env", None)
    return None


def _build_game_log_row(
    *,
    step: int,
    eval_window_idx: int,
    game_idx: int,
    agent_archetype: Optional[str],
    opponent_archetype: Optional[str],
    p1_digi: int,
    p1_dna: int,
    p2_digi: int,
    p2_dna: int,
    winner_id: Optional[int],
    terminal_score: float,
    steps: int,
    recording_path: Optional[Path],
    match_format: str = "single",
    match_idx: Optional[int] = None,
    game_in_match_idx: Optional[int] = None,
) -> Dict[str, Any]:
    """Construct one row for the eval game log. See
    `openspec/changes/add-per-game-eval-log/specs/per-game-eval-log/spec.md`
    for the schema.

    In `match_format="bo3"` runs, `match_idx` / `game_in_match_idx` group
    rows into the BO3 match they came from; in `match_format="single"`
    runs both fields are null and one row = one game = one episode."""
    if winner_id == 1:
        result = "win"
    elif winner_id == 2:
        result = "loss"
    else:
        result = "draw"
    rec_str: Optional[str] = None
    if recording_path is not None:
        rec_str = str(Path(recording_path).resolve())
    return {
        "step": int(step),
        "eval_window_idx": int(eval_window_idx),
        "game_idx": int(game_idx),
        "source": "eval",
        "match_format": match_format,
        "match_idx": int(match_idx) if match_idx is not None else None,
        "game_in_match_idx": int(game_in_match_idx) if game_in_match_idx is not None else None,
        "agent_archetype": agent_archetype,
        "opponent_archetype": opponent_archetype,
        "digivolves_agent": int(p1_digi),
        "dna_digivolves_agent": int(p1_dna),
        "digivolves_opponent": int(p2_digi),
        "dna_digivolves_opponent": int(p2_dna),
        "result": result,
        "episode_length": int(steps),
        "terminal_score": float(terminal_score),
        "recording_path": rec_str,
    }


def _build_reward_profile_factory(cfg: TrainingConfig):
    """Build a wrapper-applier closure for the reward profiles capability.

    Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
    (Group 8 wiring).

    Returns a callable `apply(env) -> Env` that EITHER wraps `env` in a
    `RewardProfileWrapper` (when `cfg.reward_profiles_path` resolves to
    a readable YAML file and loads successfully) OR returns `env`
    unchanged (when the file is missing — letting the legacy reward
    path run as before).

    A loader-load failure aborts the run (vs silent fallback) — silent
    fallback would mask configuration bugs in the YAML the operator
    just edited. Missing-file (path doesn't exist) IS the silent path
    so legacy runs without a YAML keep working.

    Per `add-gameplay-reward-config` (D9): `digivolve_shaping=True` is
    now inert. The `_digivolve_shaped` profile is removed and the
    universal gameplay default already includes digivolve weights. The
    flag is accepted (no warning, no error) for backward-compat — v2
    removes it. NO mapping to a special profile is applied.

    The returned closure also captures the loader handle so callers
    that want the metadata (`profiles_path`, `gameplay_path`,
    `gameplay_hash`, `profiles_hash`, `assignments_snapshot`) can read
    them via `.loader` / `.effective_override` attributes on the
    closure.
    """
    from digimon_gym.agents.reward.profile_loader import (
        ProfileConfigError,
        ProfileLoader,
    )
    from digimon_gym.agents.reward.wrapper import RewardProfileWrapper

    profiles_path = (
        Path(cfg.reward_profiles_path) if cfg.reward_profiles_path else None
    )
    gameplay_path = (
        Path(cfg.reward_gameplay_path) if cfg.reward_gameplay_path else None
    )

    # Effective override: explicit cfg.reward_profile_override wins.
    # `digivolve_shaping=True` no longer maps to anything — see docstring.
    effective_override: Optional[str] = cfg.reward_profile_override

    if (
        profiles_path is None
        or not profiles_path.exists()
        or gameplay_path is None
        or not gameplay_path.exists()
    ):
        # Either file missing — legacy reward path stays active. Return
        # an identity factory + None loader so downstream code can
        # detect the "not active" state. The two-file architecture
        # requires both files; partial config falls back to legacy.
        def apply_identity(env):
            return env

        apply_identity.loader = None  # type: ignore[attr-defined]
        apply_identity.effective_override = None  # type: ignore[attr-defined]
        apply_identity.active = False  # type: ignore[attr-defined]
        return apply_identity

    try:
        loader = ProfileLoader(
            gameplay_path=gameplay_path,
            profiles_path=profiles_path,
        )
    except ProfileConfigError as e:
        raise RuntimeError(
            f"Failed to load reward profiles from gameplay={gameplay_path} "
            f"+ profiles={profiles_path}: {e}. Fix the YAML or set both "
            "`reward_gameplay_path: ''` and `reward_profiles_path: ''` to disable."
        ) from e

    def apply_wrap(env):
        return RewardProfileWrapper(
            env,
            loader=loader,
            override=effective_override,
            hot_reload=cfg.reward_profiles_hot_reload,
        )

    apply_wrap.loader = loader  # type: ignore[attr-defined]
    apply_wrap.effective_override = effective_override  # type: ignore[attr-defined]
    apply_wrap.active = True  # type: ignore[attr-defined]
    return apply_wrap


def _seed_everything(seed: int) -> None:
    """Seed Python, NumPy, PyTorch, and CUDA RNGs for reproducible runs."""
    import torch as th

    os.environ["PYTHONHASHSEED"] = str(seed)
    random.seed(seed)
    np.random.seed(seed)
    th.manual_seed(seed)
    if th.cuda.is_available():
        th.cuda.manual_seed_all(seed)
    th.backends.cudnn.deterministic = True
    th.backends.cudnn.benchmark = False


def _model_meta_path(model_path: str | Path) -> Path:
    """Return the adjacent metadata sidecar path for a saved model."""
    return Path(model_path).with_suffix(".meta.json")


def _validate_checkpoint_contract(
    model_path: str | Path,
    observation_layout,
) -> None:
    """Reject fine-tune checkpoints with incompatible tensor/action contracts."""
    meta_path = _model_meta_path(model_path)
    if not meta_path.exists():
        raise ValueError(f"Checkpoint metadata sidecar not found: {meta_path}")
    metadata = json.loads(meta_path.read_text(encoding="utf-8"))
    checks = {
        "observation_profile": observation_layout.id,
        "tensor_layout_hash": observation_layout.layout_hash,
        "action_space_size": ACTION_SPACE_SIZE,
    }
    for key, expected in checks.items():
        actual = metadata.get(key)
        if actual != expected:
            raise ValueError(
                f"Checkpoint incompatible: {key}={actual!r}, expected {expected!r}"
            )


def _validate_explicit_deck(
    card_ids: Optional[List[str]],
    *,
    label: str,
    implemented_card_ids: Set[str],
) -> None:
    if card_ids is not None:
        validate_implemented_deck(
            card_ids,
            implemented_card_ids,
            label=label,
        )


# ─── Opponent Policies ──────────────────────────────────────────────

def random_policy(env: DigimonEnv) -> int:
    """Select a random valid action."""
    mask = env.action_mask()
    valid = np.where(mask > 0)[0]
    if len(valid) == 0:
        return ACTION_PASS_TURN
    rng = getattr(env, "np_random", None)
    if rng is not None:
        return int(rng.choice(valid))
    return int(np.random.choice(valid))


def make_agent_opponent_fn(
    weights_path: str,
    algorithm: str = "mlp",
    lstm_hidden_size: int = 256,
) -> Callable[[DigimonEnv], int]:
    """Load a saved model and return an opponent policy function.

    Creates a closure that loads MLP or LSTM weights once and returns
    a function compatible with OpponentWrapper's ``opponent_fn`` signature.

    Args:
        weights_path: Path to the saved SB3 model (without ``.zip`` extension).
        algorithm: ``"mlp"`` for MaskablePPO, ``"lstm"`` for MaskableRecurrentPPO.
        lstm_hidden_size: LSTM hidden units (only used when algorithm is ``"lstm"``).

    Returns:
        A callable ``(DigimonEnv) -> int`` that predicts Player 2 actions.
    """
    if algorithm == "lstm":
        model = MaskableRecurrentPPO.load(weights_path)
        # LSTM state must persist across steps within a single episode.
        lstm_states: list = [None]  # mutable container for closure

        def _lstm_policy(env: DigimonEnv) -> int:
            mask = env.action_mask()
            obs = env.runner.get_board_tensor(2)
            action, lstm_states[0] = model.predict(
                obs,
                state=lstm_states[0],
                action_masks=mask,
                deterministic=True,
            )
            return int(action)

        # Attach a reset hook so callers can clear LSTM state between episodes.
        _lstm_policy.reset_state = lambda: lstm_states.__setitem__(0, None)  # type: ignore[attr-defined]
        return _lstm_policy
    else:
        model = MaskablePPO.load(weights_path)

        def _mlp_policy(env: DigimonEnv) -> int:
            mask = env.action_mask()
            obs = env.runner.get_board_tensor(2)
            action, _ = model.predict(obs, action_masks=mask, deterministic=True)
            return int(action)

        return _mlp_policy


def make_pool_opponent_fn(
    pool: OpponentPool,
    mode: str = "uniform",
) -> Callable[[DigimonEnv], int]:
    """Build an opponent policy that samples saved agents from a pool."""
    cache: Dict[str, Callable[[DigimonEnv], int]] = {}
    current: list[Optional[tuple[str, Callable[[DigimonEnv], int]]]] = [None]

    def _opponent(env: DigimonEnv) -> int:
        if current[0] is None:
            entry = pool.sample(mode=mode)
            if entry.weights_path not in cache:
                cache[entry.weights_path] = make_agent_opponent_fn(
                    entry.weights_path,
                    algorithm=entry.algorithm,
                )
            current[0] = (entry.name, cache[entry.weights_path])
        return current[0][1](env)

    def _reset_state() -> None:
        if current[0] is not None:
            _name, policy = current[0]
            reset = getattr(policy, "reset_state", None)
            if reset is not None:
                reset()
        current[0] = None

    _opponent.reset_state = _reset_state  # type: ignore[attr-defined]
    return _opponent


# ─── Opponent Wrapper ────────────────────────────────────────────────

class OpponentWrapper(gymnasium.Wrapper):
    """Wraps DigimonEnv so the RL agent only controls Player 1.

    After the agent takes an action, this wrapper auto-plays Player 2's
    turns using a configurable opponent policy until it's Player 1's turn
    again (or the game ends).

    This converts the two-player environment into a single-agent MDP
    suitable for standard RL algorithms.
    """

    def __init__(self, env: DigimonEnv,
                 opponent_fn: Callable[[DigimonEnv], int] = greedy_policy):
        super().__init__(env)
        self.opponent_fn = opponent_fn
        self._unwrapped_env: DigimonEnv = env

    def reset(self, **kwargs):
        reset = getattr(self.opponent_fn, "reset_state", None)
        if reset is not None:
            reset()
        obs, info = self.env.reset(**kwargs)
        # If Player 2 goes first, auto-play until Player 1's turn
        obs, info = self._advance_opponent(obs, info)
        return obs, info

    def step(self, action):
        obs, reward, terminated, truncated, info = self.env.step(int(action))

        if terminated or truncated:
            return obs, reward, terminated, truncated, info

        # Auto-play opponent turns. With the event-based reward shape
        # (post-2026-05-23 rework), dense signal fires ONLY on security
        # count changes — including opponent attacks that remove our
        # security. Accumulating opp-step rewards passes those losses to
        # the agent so it learns to defend; under the prior dp_delta-
        # based shape, opp-step rewards were noise (board changes the
        # agent can't directly control) and were therefore discarded.
        obs, info, opp_reward, terminated, truncated = (
            self._play_opponent(obs, info)
        )
        reward = float(reward) + float(opp_reward)

        return obs, reward, terminated, truncated, info

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

    def reset_inner_only(self, **kwargs):
        """Reset the inner env without resetting the opponent policy's
        state. Used by `MatchEnv` between games of a BO3 match so the
        recurrent policy's hidden state carries across games (per the
        bo3-match-training spec). Also runs the opponent-advance loop
        so the wrapper returns at the agent's first decision point in
        the new game.
        """
        obs, info = self.env.reset(**kwargs)
        return self._advance_opponent(obs, info)

    def advance_opponent_until_agent_acts(self):
        """Drive opponent actions until either the agent has a decision
        point or the game ends. Returns `(obs, info, accumulated_reward,
        terminated, truncated)`. Equivalent to `_play_opponent` but does
        not require a starting obs/info — reads fresh state from the
        inner env. Used by `MatchEnv` to advance the engine across the
        BO3 `SelectPlayOrder` pick when the opponent is the chooser.
        """
        obs = self._unwrapped_env.runner.get_board_tensor(1)
        info = {"action_mask": self._unwrapped_env.action_mask()}
        return self._play_opponent(obs, info)

    def _play_opponent(self, obs, info):
        """Auto-play Player 2 turns until Player 1 acts or game ends.

        Accumulates EVERY step's reward (not just terminal) so the agent
        sees dense feedback from opponent actions — primarily own-security
        losses when opp attacks succeed. Required by the event-based
        reward shape (digimon_gym.py `_compute_reward`).
        """
        accumulated_reward = 0.0

        while (
            not self._unwrapped_env.is_game_over
            and self._unwrapped_env.current_player_id != 1
        ):
            opp_action = self.opponent_fn(self._unwrapped_env)
            obs, reward, terminated, truncated, info = self.env.step(int(opp_action))
            accumulated_reward += float(reward)
            if terminated or truncated:
                return obs, info, accumulated_reward, terminated, truncated

        return obs, info, accumulated_reward, self._unwrapped_env.is_game_over, False


# ─── Callbacks ───────────────────────────────────────────────────────

class WinRateCallback(BaseCallback):
    """Tracks win rate and episode statistics via TensorBoard.

    Runs evaluation episodes periodically and logs:
    - pilot/win_rate: fraction of evaluation games won by Player 1
    - pilot/mean_reward: average episode reward
    - pilot/mean_episode_length: average steps per episode
    - pilot/games_played: total training episodes so far

    Also tracks per-archetype win rates when a GauntletWrapper is present,
    accessible via ``get_archetype_results()``.
    """

    def __init__(self, eval_env_fn: Callable,
                 eval_freq: int = 10000,
                 n_eval_episodes: int = 20,
                 eval_suite=None,
                 eval_suite_path: str | None = None,
                 eval_sidecar_path: Optional[Path] = None,
                 game_log_writer: Optional[GameLogWriter] = None,
                 verbose: int = 1):
        super().__init__(verbose)
        self._eval_env_fn = eval_env_fn
        self._eval_env: Optional[gymnasium.Env] = None
        self.eval_freq = eval_freq
        self.n_eval_episodes = n_eval_episodes
        self.eval_suite = eval_suite
        self.eval_suite_path = eval_suite_path
        self.games_played = 0
        self._last_eval_step = 0
        # Per-game eval-log writer. None means emission is disabled.
        # `_eval_window_idx` is incremented at entry to each
        # `_run_evaluation` call so all rows in one window share an idx.
        self._game_log_writer = game_log_writer
        self._eval_window_idx = 0
        self.last_win_rate: float = 0.0
        self.last_mean_reward: float = 0.0
        self.last_draw_rate: float = 0.0
        self.last_mean_eval_terminal_score: float = 0.0
        self.last_mean_eval_dense_reward: float = 0.0
        self.last_mean_eval_episode_length: float = 0.0
        self._last_eval_suite_results: dict = {}
        self._eval_sidecar_path: Optional[Path] = (
            Path(eval_sidecar_path) if eval_sidecar_path is not None else None
        )
        # Per-archetype tracking. Three axes:
        #   _archetype_*           — keyed by opponent archetype (original)
        #   _agent_archetype_*     — keyed by AGENT's archetype (generalist sampling)
        #   _matchup_*             — keyed by (agent_arch, opp_arch) tuple — full N×N grid
        # All three are populated when info contains both deck1_archetype and
        # opponent_archetype (generalist mode); the opp-axis dicts also populate
        # in gauntlet mode where only opponent_archetype is set.
        self._archetype_wins: Dict[str, int] = {}
        self._archetype_draws: Dict[str, int] = {}
        self._archetype_games: Dict[str, int] = {}
        self._agent_archetype_wins: Dict[str, int] = {}
        self._agent_archetype_draws: Dict[str, int] = {}
        self._agent_archetype_games: Dict[str, int] = {}
        self._matchup_wins: Dict[tuple, int] = {}
        self._matchup_draws: Dict[tuple, int] = {}
        self._matchup_games: Dict[tuple, int] = {}
        # Per-archetype digivolve activity counters. Cumulative across all evals
        # since callback construction, same lifecycle as the wins dicts above.
        # See openspec/changes/digivolve-eval-telemetry/ for the surface spec.
        #
        # Opponent-keyed (bucketed by opp archetype, mirrors _archetype_wins):
        #   *_agent_digivolves    — AGENT's digivolves in games vs this opp (p1)
        #   *_opponent_digivolves — OPPONENT's digivolves in those games (p2)
        self._archetype_agent_digivolves: Dict[str, int] = {}
        self._archetype_agent_dna_digivolves: Dict[str, int] = {}
        self._archetype_opponent_digivolves: Dict[str, int] = {}
        self._archetype_opponent_dna_digivolves: Dict[str, int] = {}
        # Agent-keyed (bucketed by agent archetype, generalist mode only):
        self._agent_archetype_digivolves: Dict[str, int] = {}
        self._agent_archetype_dna_digivolves: Dict[str, int] = {}
        # BO3 match-tier tracking (per-episode = per-match in bo3 mode).
        # Populated only when the eval env exposes a `MatchEnv` somewhere in
        # the wrapper chain. All counters are cumulative across eval passes.
        self._match_played: int = 0
        self._match_wins: int = 0           # P1 won 2 games
        self._match_sweeps: int = 0         # 2-0 wins
        self._match_swept: int = 0          # 0-2 losses
        self._match_draws: int = 0          # rare; 1-1-1 step-limit
        self._match_total_steps: int = 0
        self._match_total_games: int = 0    # 2 or 3 per match
        # Concede tracking:
        self._concede_total: int = 0
        self._concede_when_lead: int = 0    # agent ahead 1-0 at concede
        self._concede_when_tied: int = 0    # 0-0 at concede
        self._concede_when_down: int = 0    # 0-1 at concede
        self._concede_correct: int = 0      # match won after a concede
        # Play-order pick tracking:
        self._play_order_picks: int = 0     # total picks by the agent
        self._play_first_count: int = 0     # agent chose to play first
        self._play_first_match_wins: int = 0  # match wins where agent picked first
        # Per-matchup match-tier (parallel to the game-tier _matchup_* dicts):
        self._matchup_match_wins: Dict[tuple, int] = {}
        self._matchup_match_sweeps: Dict[tuple, int] = {}
        self._matchup_match_games: Dict[tuple, int] = {}
        # `add-reward-profiles` Group 10 telemetry accumulators.
        # All cumulative across eval cycles since callback construction
        # (same lifecycle as the wins/digivolve dicts above — no
        # cross-resume persistence). Sourced from
        # `info["reward_breakdown"]` written by `RewardProfileWrapper`.
        self._component_totals: Dict[str, float] = {}
        self._component_games: Dict[str, int] = {}
        # Per-profile (the active profile for the game).
        self._profile_reward_totals: Dict[str, float] = {}
        self._profile_component_totals: Dict[tuple, float] = {}    # (profile, component) -> sum
        self._profile_games: Dict[str, int] = {}
        self._profile_clamp_steps: Dict[str, int] = {}             # steps with clamp this profile
        self._profile_total_steps: Dict[str, int] = {}             # all eval steps with this profile
        # Per-archetype × component (axis: opponent archetype).
        self._archetype_component_totals: Dict[tuple, float] = {}  # (opp_arch, component) -> sum
        # Per-archetype × component (axis: agent archetype).
        self._agent_archetype_component_totals: Dict[tuple, float] = {}
        # Boss-arrival counters (spec D14 + capability
        # "Boss-cards set and arrival-aware sidecar columns").
        # Per opponent archetype:
        self._archetype_digivolves_into_boss: Dict[str, int] = {}
        self._archetype_digivolves_into_boss_dna: Dict[str, int] = {}
        self._archetype_hardcasts_of_boss: Dict[str, int] = {}
        self._archetype_hardcasts_of_boss_full_cost: Dict[str, int] = {}
        # Per agent archetype:
        self._agent_archetype_digivolves_into_boss: Dict[str, int] = {}
        self._agent_archetype_digivolves_into_boss_dna: Dict[str, int] = {}
        self._agent_archetype_hardcasts_of_boss: Dict[str, int] = {}
        self._agent_archetype_hardcasts_of_boss_full_cost: Dict[str, int] = {}
        # Per-eval-window aggregates (reset each window):
        self._window_total_digivolves_into_boss: int = 0
        self._window_total_hardcasts_of_boss: int = 0
        self._window_total_discipline_num: int = 0  # digivolves
        self._window_total_discipline_denom: int = 0  # digivolves + hardcasts
        # add-gameplay-reward-config telemetry. Tracked per eval window.
        # `mean_eval_winning_turn` is emitted only when there were
        # agent-side wins (winner_id == 1) in the window; the bus reads
        # `turn_count` from rl_state at terminal.
        self._window_total_winning_turn: float = 0.0
        self._window_total_winning_games: int = 0
        # `mean_eval_digivolve_driven_attacks` per game (always emitted).
        self._window_total_digivolve_driven_attacks: int = 0

    def get_archetype_results(self):
        """Return per-archetype results as ArchetypeResult list."""
        from digimon_gym.agents.training_metrics import ArchetypeResult
        results = []
        for name, games in self._archetype_games.items():
            wins = self._archetype_wins.get(name, 0)
            draws = self._archetype_draws.get(name, 0)
            losses = games - wins - draws
            results.append(ArchetypeResult(
                archetype_name=name,
                games_played=games,
                wins=wins,
                losses=losses,
                draws=draws,
                win_rate=wins / max(1, games),
            ))
        return results

    def get_eval_suite_results(self) -> dict:
        """Return the last held-out eval-suite summary for metadata."""
        if self._last_eval_suite_results:
            return self._last_eval_suite_results
        if self.eval_suite_path:
            return {
                "overall_win_rate": None,
                "suite_path": self.eval_suite_path,
                "cells": {},
            }
        return {}

    def _on_step(self) -> bool:
        # Track episode completions from training
        infos = self.locals.get("infos", [])
        for info in infos:
            if "episode" in info:
                self.games_played += 1

        # Periodic evaluation (only once per eval_freq interval)
        if self.eval_freq > 0 and self.num_timesteps - self._last_eval_step >= self.eval_freq:
            self._last_eval_step = self.num_timesteps
            self._run_evaluation()

        return True

    def close(self):
        """Clean up the reused evaluation environment."""
        if self._eval_env is not None:
            self._eval_env.close()
            self._eval_env = None

    def _run_evaluation(self):
        """Run evaluation games and log win rate."""
        if self._eval_env is None:
            self._eval_env = self._eval_env_fn()
        eval_env = self._eval_env
        if eval_env is None:
            if self.verbose:
                print("  [Eval] Failed to create evaluation environment.")
            return
        wins = 0
        draws = 0
        total_reward = 0.0
        total_terminal_score = 0.0
        total_dense_reward = 0.0
        total_steps = 0
        # Digivolve activity counters — observational, logged regardless of
        # whether the shaping reward is enabled (so unshaped runs produce
        # a baseline curve for comparison). See
        # docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md.
        total_digivolves = 0
        total_dna_digivolves = 0
        total_opponent_digivolves = 0
        total_opponent_dna_digivolves = 0
        # Per-game eval-log identity captured once per window so all rows
        # share the same (step, eval_window_idx). `_games_in_window`
        # increments per ROW (per game), not per episode — necessary in
        # BO3 mode where one episode emits 2-3 rows.
        window_step = int(self.num_timesteps)
        window_idx = self._eval_window_idx
        self._eval_window_idx += 1
        self._games_in_window = 0
        # Reset per-window boss-arrival aggregates so window means are
        # window-scoped (not cumulative across windows).
        self._window_total_digivolves_into_boss = 0
        self._window_total_hardcasts_of_boss = 0
        self._window_total_discipline_num = 0
        self._window_total_discipline_denom = 0
        # add-gameplay-reward-config — reset alongside boss-arrival.
        self._window_total_winning_turn = 0.0
        self._window_total_winning_games = 0
        self._window_total_digivolve_driven_attacks = 0

        for game_idx in range(self.n_eval_episodes):
            obs, info = eval_env.reset()
            episode_reward = 0.0
            steps = 0
            done = False
            opponent_archetype = info.get("opponent_archetype")
            # In generalist mode the wrapper also sets deck1_archetype; in
            # gauntlet mode it's absent (agent always plays the same fixed deck).
            agent_archetype = info.get("deck1_archetype")

            # `add-reward-profiles` Group 10: per-game reward-breakdown
            # accumulator. Captures the sum of each component's
            # contribution across the game. Profile name + hash are
            # captured from the first step's info (locked at reset by
            # the wrapper — stable across the game). Boss-cards set is
            # read from the wrapper to drive per-game boss-arrival counts.
            game_breakdown: Dict[str, float] = {}
            game_profile_name: Optional[str] = info.get("reward_profile")
            game_profile_hash: Optional[str] = info.get("reward_profile_hash")
            game_clamp_steps: int = 0
            game_total_steps: int = 0
            # Boss-cards set + per-game arrival counts. Resolved from
            # the wrapper instance at game start.
            _wrapper_profile = _find_reward_profile_wrapper(eval_env)
            game_boss_cards: frozenset = (
                _wrapper_profile._active_profile.boss_cards
                if _wrapper_profile is not None
                and _wrapper_profile._active_profile is not None
                else frozenset()
            )
            game_digivolves_into_boss = 0
            game_digivolves_into_boss_dna = 0
            game_hardcasts_of_boss = 0
            game_hardcasts_of_boss_full_cost = 0
            # add-gameplay-reward-config per-game accumulators.
            game_digivolve_driven_attacks = 0

            # LSTM state management (None for MLP, threaded for LSTM)
            state = None
            episode_start = np.array([True])

            while not done:
                if eval_env is None:
                    break
                # Get action masks for both MLP and LSTM models
                action_masks = _unwrap_to_digimon_env(eval_env).action_mask()
                action, state = self.model.predict(
                    obs, state=state, episode_start=episode_start,
                    deterministic=True, action_masks=action_masks,
                )
                obs, reward, terminated, truncated, info = eval_env.step(
                    int(action)
                )
                episode_reward += float(reward)
                steps += 1
                done = terminated or truncated
                episode_start = np.array([False])

                # Accumulate reward breakdown for telemetry. Wrapper
                # writes `info["reward_breakdown"]` per step when
                # active; legacy-reward runs (no wrapper) skip this
                # branch silently.
                step_breakdown = info.get("reward_breakdown")
                if step_breakdown:
                    game_total_steps += 1
                    for comp_name, contrib in step_breakdown.items():
                        game_breakdown[comp_name] = (
                            game_breakdown.get(comp_name, 0.0) + float(contrib)
                        )
                    if info.get("reward_breakdown_clamped"):
                        game_clamp_steps += 1
                # Boss-arrival counters — derived from the occurrence
                # stream so they fire regardless of which profile is
                # active (works on `_default` games where boss_cards
                # is empty → all four counts stay at 0).
                if game_boss_cards:
                    occ_list = info.get("reward_occurrences", []) or []
                    from digimon_gym.agents.reward.occurrences import (
                        Digivolved as _Digivolved,
                        PlayedCard as _PlayedCard,
                    )
                    for ev in occ_list:
                        if isinstance(ev, _Digivolved) and ev.player == 1 \
                                and ev.top_card_id in game_boss_cards:
                            game_digivolves_into_boss += 1
                            if ev.was_dna:
                                game_digivolves_into_boss_dna += 1
                        elif isinstance(ev, _PlayedCard) and ev.player == 1 \
                                and ev.card_id in game_boss_cards:
                            game_hardcasts_of_boss += 1
                            if ev.cost_paid >= ev.cost_printed:
                                game_hardcasts_of_boss_full_cost += 1

                # add-gameplay-reward-config: count agent-side
                # DigivolveDrivenAttack occurrences regardless of profile
                # (the engine counter exposes them on every run).
                occ_list_all = info.get("reward_occurrences", []) or []
                if occ_list_all:
                    from digimon_gym.agents.reward.occurrences import (
                        DigivolveDrivenAttack as _DigivolveDrivenAttack,
                    )
                    for ev in occ_list_all:
                        if isinstance(ev, _DigivolveDrivenAttack) and ev.player == 1:
                            game_digivolve_driven_attacks += 1

            total_reward += episode_reward
            total_steps += steps

            # Read final digivolve counters from agent (p1) and opponent (p2).
            # Robust to backends that don't surface the keys (defaults to 0).
            # Per-game values are also bucketed by archetype below.
            p1_digi_game = 0
            p1_dna_game = 0
            p2_digi_game = 0
            p2_dna_game = 0
            if eval_env is not None:
                try:
                    final_state = _unwrap_to_digimon_env(eval_env)._rl_state()
                    p1_digi_game = int(final_state.get("p1_digivolutions", 0))
                    p1_dna_game = int(final_state.get("p1_dna_digivolutions", 0))
                    p2_digi_game = int(final_state.get("p2_digivolutions", 0))
                    p2_dna_game = int(final_state.get("p2_dna_digivolutions", 0))
                    total_digivolves += p1_digi_game
                    total_dna_digivolves += p1_dna_game
                    total_opponent_digivolves += p2_digi_game
                    total_opponent_dna_digivolves += p2_dna_game
                except Exception:
                    pass

            # Use the actual outcome to determine win/loss. In BO3 mode the
            # episode is a MATCH; in single mode it's a game. Detect MatchEnv
            # in the wrapper chain to pick the right outcome path.
            won = False
            is_draw = False
            terminal_score = 0.0
            match_env_instance = (
                _find_match_env(eval_env) if eval_env is not None else None
            )
            if match_env_instance is not None:
                # BO3 episode = match. Winner is whoever has >=2 game wins.
                agent_game_wins = match_env_instance.match_score[0]
                opp_game_wins = match_env_instance.match_score[1]
                agent_match_won = agent_game_wins >= 2
                opp_match_won = opp_game_wins >= 2
                # Match-tier tallies
                self._match_played += 1
                self._match_total_steps += match_env_instance.match_step_count
                games_this_match = agent_game_wins + opp_game_wins
                self._match_total_games += games_this_match
                if agent_match_won:
                    wins += 1
                    won = True
                    terminal_score = 1.0
                    self._match_wins += 1
                    if opp_game_wins == 0:
                        self._match_sweeps += 1
                elif opp_match_won:
                    terminal_score = -1.0
                    if agent_game_wins == 0:
                        self._match_swept += 1
                else:
                    draws += 1
                    is_draw = True
                    self._match_draws += 1
                # Concede analytics (one slot per game in concede_history).
                concedes = match_env_instance.concede_history
                # `concede_history[i]` is True iff agent conceded game i.
                # Score-state at concede time: before this game, the score
                # was the sum of WINS up to but not including this game.
                # We don't track agent vs opp per-game outcome explicitly,
                # but we can infer: if agent didn't concede, no contribution;
                # if agent conceded game i and no concede earlier, the
                # "score state" before game i is the sum of opp wins from
                # the prior games minus agent wins (but for game i, the
                # opp got it as a free win since agent conceded). Simplify:
                # walk the history.
                agent_w_so_far = 0
                opp_w_so_far = 0
                for i, conceded in enumerate(concedes):
                    if conceded:
                        self._concede_total += 1
                        if agent_w_so_far > opp_w_so_far:
                            self._concede_when_lead += 1
                        elif agent_w_so_far == opp_w_so_far:
                            self._concede_when_tied += 1
                        else:
                            self._concede_when_down += 1
                        if agent_match_won:
                            self._concede_correct += 1
                        # Conceding game i hands it to the opponent.
                        opp_w_so_far += 1
                    else:
                        # Game i was played out. We don't know its winner
                        # from concede_history alone, but match_score tells
                        # us totals; we can't perfectly back-fill. For the
                        # score-state tallies above to be accurate, we'd
                        # need per-game outcomes. Approximate: walk forward
                        # using whatever's known. For now, increment based
                        # on remaining wins to allocate.
                        # (When concedes happen the agent's score-state
                        # tally is still correct because we already counted
                        # the concede. For non-concede games we just keep
                        # the score moving via match_score totals at end.)
                        pass
                # Play-order picks: read from match_env's history.
                for entry in match_env_instance._play_order_history:
                    if entry is None:
                        continue
                    # Only count picks made by the AGENT (chooser == 1).
                    if entry.get("chooser") != 1:
                        continue
                    self._play_order_picks += 1
                    if entry.get("picked") == "first":
                        self._play_first_count += 1
                        if agent_match_won:
                            self._play_first_match_wins += 1
            else:
                # Single-game episode: use the inner DigimonEnv winner.
                winner_id = _unwrap_to_digimon_env(eval_env).winner_id
                if winner_id == 1:
                    wins += 1
                    won = True
                    terminal_score = 1.0
                elif winner_id == 2:
                    terminal_score = -1.0
                else:
                    draws += 1
                    is_draw = True
                    terminal_score = 0.0
            total_terminal_score += terminal_score
            total_dense_reward += episode_reward - terminal_score

            # Per-game eval log emission. Observational; no-op when no
            # writer is configured (CLI flag `--eval-game-log off`).
            # In BO3 mode, iterate `match_env_instance.match_game_history`
            # to emit ONE ROW PER GAME (not per match) — the callback's
            # outer loop only sees match-level terminations, so per-game
            # stats are captured by MatchEnv at each inner-game terminal.
            if self._game_log_writer is not None:
                recording_path = _find_eval_recording_path(eval_env)
                if match_env_instance is not None:
                    for g_in_match, snap in enumerate(match_env_instance.match_game_history):
                        g_winner = snap.get("winner_id")
                        if g_winner == 1:
                            g_terminal = 1.0
                        elif g_winner == 2:
                            g_terminal = -1.0
                        else:
                            g_terminal = 0.0
                        row = _build_game_log_row(
                            step=window_step,
                            eval_window_idx=window_idx,
                            game_idx=self._games_in_window,
                            agent_archetype=agent_archetype,
                            opponent_archetype=opponent_archetype,
                            p1_digi=snap.get("digivolves_agent", 0),
                            p1_dna=snap.get("dna_digivolves_agent", 0),
                            p2_digi=snap.get("digivolves_opponent", 0),
                            p2_dna=snap.get("dna_digivolves_opponent", 0),
                            winner_id=g_winner,
                            terminal_score=g_terminal,
                            steps=snap.get("steps", 0),
                            recording_path=recording_path,
                            match_format="bo3",
                            match_idx=game_idx,
                            game_in_match_idx=g_in_match,
                        )
                        self._game_log_writer.append(row)
                        self._games_in_window += 1
                else:
                    row = _build_game_log_row(
                        step=window_step,
                        eval_window_idx=window_idx,
                        game_idx=self._games_in_window,
                        agent_archetype=agent_archetype,
                        opponent_archetype=opponent_archetype,
                        p1_digi=p1_digi_game,
                        p1_dna=p1_dna_game,
                        p2_digi=p2_digi_game,
                        p2_dna=p2_dna_game,
                        winner_id=winner_id,
                        terminal_score=terminal_score,
                        steps=steps,
                        recording_path=recording_path,
                        match_format="single",
                    )
                    self._game_log_writer.append(row)
                    self._games_in_window += 1

            # Track per-archetype win rates (when gauntlet OR generalist is active).
            if opponent_archetype:
                self._archetype_games[opponent_archetype] = (
                    self._archetype_games.get(opponent_archetype, 0) + 1
                )
                if won:
                    self._archetype_wins[opponent_archetype] = (
                        self._archetype_wins.get(opponent_archetype, 0) + 1
                    )
                elif is_draw:
                    self._archetype_draws[opponent_archetype] = (
                        self._archetype_draws.get(opponent_archetype, 0) + 1
                    )
                # Per-opponent digivolve tally. Agent (p1) AND opponent (p2)
                # counts are bucketed by opponent archetype so the sidecar
                # `by_archetype[opp]` can carry both "agent's digivolves vs
                # this opp" and "this opp's digivolves" without a separate
                # axis. See openspec/changes/digivolve-eval-telemetry/.
                self._archetype_agent_digivolves[opponent_archetype] = (
                    self._archetype_agent_digivolves.get(opponent_archetype, 0)
                    + p1_digi_game
                )
                self._archetype_agent_dna_digivolves[opponent_archetype] = (
                    self._archetype_agent_dna_digivolves.get(opponent_archetype, 0)
                    + p1_dna_game
                )
                self._archetype_opponent_digivolves[opponent_archetype] = (
                    self._archetype_opponent_digivolves.get(opponent_archetype, 0)
                    + p2_digi_game
                )
                self._archetype_opponent_dna_digivolves[opponent_archetype] = (
                    self._archetype_opponent_dna_digivolves.get(opponent_archetype, 0)
                    + p2_dna_game
                )
            # Agent-archetype + full N×N matchup grid (generalist mode only;
            # gauntlet mode has a fixed agent deck so agent_archetype is None).
            if agent_archetype:
                self._agent_archetype_games[agent_archetype] = (
                    self._agent_archetype_games.get(agent_archetype, 0) + 1
                )
                if won:
                    self._agent_archetype_wins[agent_archetype] = (
                        self._agent_archetype_wins.get(agent_archetype, 0) + 1
                    )
                elif is_draw:
                    self._agent_archetype_draws[agent_archetype] = (
                        self._agent_archetype_draws.get(agent_archetype, 0) + 1
                    )
                # Per-agent-archetype digivolve tally — only the agent side,
                # since the opponent's archetype is the natural index for
                # opponent digivolves (handled in the opp guard above).
                self._agent_archetype_digivolves[agent_archetype] = (
                    self._agent_archetype_digivolves.get(agent_archetype, 0)
                    + p1_digi_game
                )
                self._agent_archetype_dna_digivolves[agent_archetype] = (
                    self._agent_archetype_dna_digivolves.get(agent_archetype, 0)
                    + p1_dna_game
                )
                if opponent_archetype:
                    matchup = (agent_archetype, opponent_archetype)
                    self._matchup_games[matchup] = (
                        self._matchup_games.get(matchup, 0) + 1
                    )
                    if won:
                        self._matchup_wins[matchup] = (
                            self._matchup_wins.get(matchup, 0) + 1
                        )
                    elif is_draw:
                        self._matchup_draws[matchup] = (
                            self._matchup_draws.get(matchup, 0) + 1
                        )
                    # Match-tier per-matchup (BO3 mode only). In single
                    # mode `match_env_instance` is None and these tallies
                    # do not update; in bo3 mode the per-episode `won` /
                    # match-score reflects match outcome (set above).
                    if match_env_instance is not None:
                        self._matchup_match_games[matchup] = (
                            self._matchup_match_games.get(matchup, 0) + 1
                        )
                        if won:
                            self._matchup_match_wins[matchup] = (
                                self._matchup_match_wins.get(matchup, 0) + 1
                            )
                            # Sweep iff opp_game_wins == 0 for this match.
                            if match_env_instance.match_score[1] == 0:
                                self._matchup_match_sweeps[matchup] = (
                                    self._matchup_match_sweeps.get(matchup, 0) + 1
                                )

            # `add-reward-profiles` Group 10: fold this game's
            # reward-breakdown + boss-arrival stats into the per-component,
            # per-profile, per-archetype × component accumulators. All
            # cumulative across eval cycles; reset only on callback
            # reconstruction (matches the _archetype_wins lifecycle).
            if game_breakdown:
                for comp_name, total in game_breakdown.items():
                    self._component_totals[comp_name] = (
                        self._component_totals.get(comp_name, 0.0) + total
                    )
                    self._component_games[comp_name] = (
                        self._component_games.get(comp_name, 0) + 1
                    )
                    if opponent_archetype:
                        key = (opponent_archetype, comp_name)
                        self._archetype_component_totals[key] = (
                            self._archetype_component_totals.get(key, 0.0) + total
                        )
                    if agent_archetype:
                        key = (agent_archetype, comp_name)
                        self._agent_archetype_component_totals[key] = (
                            self._agent_archetype_component_totals.get(key, 0.0) + total
                        )
                if game_profile_name:
                    self._profile_games[game_profile_name] = (
                        self._profile_games.get(game_profile_name, 0) + 1
                    )
                    game_total = sum(game_breakdown.values())
                    self._profile_reward_totals[game_profile_name] = (
                        self._profile_reward_totals.get(game_profile_name, 0.0)
                        + game_total
                    )
                    for comp_name, total in game_breakdown.items():
                        key = (game_profile_name, comp_name)
                        self._profile_component_totals[key] = (
                            self._profile_component_totals.get(key, 0.0) + total
                        )
                    self._profile_clamp_steps[game_profile_name] = (
                        self._profile_clamp_steps.get(game_profile_name, 0)
                        + game_clamp_steps
                    )
                    self._profile_total_steps[game_profile_name] = (
                        self._profile_total_steps.get(game_profile_name, 0)
                        + game_total_steps
                    )

            # add-gameplay-reward-config per-game hooks:
            # 1. winning_turn — accumulate turn_count at terminal for
            #    agent-won games (winner_id == 1).
            # 2. digivolve_driven_attacks — always accumulate per-game
            #    count regardless of outcome.
            self._window_total_digivolve_driven_attacks += game_digivolve_driven_attacks
            if won and not is_draw:
                try:
                    # Read turn_count from the final rl_state. In BO3 mode
                    # the last game's terminal turn_count is the one we
                    # captured; in single mode it's the only game.
                    if eval_env is not None:
                        turn_at_terminal = int(
                            _unwrap_to_digimon_env(eval_env)
                            ._rl_state()
                            .get("turn_count", 0)
                        )
                        if turn_at_terminal > 0:
                            self._window_total_winning_turn += float(turn_at_terminal)
                            self._window_total_winning_games += 1
                except Exception:
                    pass

            # Boss-arrival per-game counts (Group 10 task 10.6). Fold
            # into both axis dicts + the window-aggregate counters.
            if game_boss_cards:
                self._window_total_digivolves_into_boss += game_digivolves_into_boss
                self._window_total_hardcasts_of_boss += game_hardcasts_of_boss
                discipline_denom_game = (
                    game_digivolves_into_boss + game_hardcasts_of_boss
                )
                self._window_total_discipline_num += game_digivolves_into_boss
                self._window_total_discipline_denom += discipline_denom_game
                if opponent_archetype:
                    self._archetype_digivolves_into_boss[opponent_archetype] = (
                        self._archetype_digivolves_into_boss.get(opponent_archetype, 0)
                        + game_digivolves_into_boss
                    )
                    self._archetype_digivolves_into_boss_dna[opponent_archetype] = (
                        self._archetype_digivolves_into_boss_dna.get(opponent_archetype, 0)
                        + game_digivolves_into_boss_dna
                    )
                    self._archetype_hardcasts_of_boss[opponent_archetype] = (
                        self._archetype_hardcasts_of_boss.get(opponent_archetype, 0)
                        + game_hardcasts_of_boss
                    )
                    self._archetype_hardcasts_of_boss_full_cost[opponent_archetype] = (
                        self._archetype_hardcasts_of_boss_full_cost.get(opponent_archetype, 0)
                        + game_hardcasts_of_boss_full_cost
                    )
                if agent_archetype:
                    self._agent_archetype_digivolves_into_boss[agent_archetype] = (
                        self._agent_archetype_digivolves_into_boss.get(agent_archetype, 0)
                        + game_digivolves_into_boss
                    )
                    self._agent_archetype_digivolves_into_boss_dna[agent_archetype] = (
                        self._agent_archetype_digivolves_into_boss_dna.get(agent_archetype, 0)
                        + game_digivolves_into_boss_dna
                    )
                    self._agent_archetype_hardcasts_of_boss[agent_archetype] = (
                        self._agent_archetype_hardcasts_of_boss.get(agent_archetype, 0)
                        + game_hardcasts_of_boss
                    )
                    self._agent_archetype_hardcasts_of_boss_full_cost[agent_archetype] = (
                        self._agent_archetype_hardcasts_of_boss_full_cost.get(agent_archetype, 0)
                        + game_hardcasts_of_boss_full_cost
                    )

        win_rate = wins / self.n_eval_episodes
        mean_reward = total_reward / self.n_eval_episodes
        mean_length = total_steps / self.n_eval_episodes

        draw_rate = draws / self.n_eval_episodes
        mean_terminal_score = total_terminal_score / self.n_eval_episodes
        mean_dense_reward = total_dense_reward / self.n_eval_episodes
        mean_digivolves = total_digivolves / self.n_eval_episodes
        mean_dna_digivolves = total_dna_digivolves / self.n_eval_episodes
        mean_opp_digivolves = total_opponent_digivolves / self.n_eval_episodes
        mean_opp_dna_digivolves = total_opponent_dna_digivolves / self.n_eval_episodes

        self.last_win_rate = win_rate
        self.last_mean_reward = mean_reward
        self.last_draw_rate = draw_rate
        self.last_mean_eval_terminal_score = mean_terminal_score
        self.last_mean_eval_dense_reward = mean_dense_reward
        self.last_mean_eval_episode_length = mean_length
        self.last_mean_eval_digivolves_per_game = mean_digivolves
        self.last_mean_eval_dna_digivolves_per_game = mean_dna_digivolves
        self.last_mean_eval_opponent_digivolves_per_game = mean_opp_digivolves
        self.last_mean_eval_opponent_dna_digivolves_per_game = mean_opp_dna_digivolves

        self.logger.record("pilot/win_rate", win_rate)
        self.logger.record("pilot/draw_rate", draw_rate)
        self.logger.record("pilot/mean_eval_reward", mean_reward)
        self.logger.record("pilot/mean_eval_terminal_score", mean_terminal_score)
        self.logger.record("pilot/mean_eval_dense_reward", mean_dense_reward)
        self.logger.record("pilot/mean_eval_episode_length", mean_length)
        self.logger.record("pilot/mean_eval_digivolves_per_game", mean_digivolves)
        self.logger.record("pilot/mean_eval_dna_digivolves_per_game", mean_dna_digivolves)
        self.logger.record(
            "pilot/mean_eval_opponent_digivolves_per_game", mean_opp_digivolves
        )
        self.logger.record(
            "pilot/mean_eval_opponent_dna_digivolves_per_game",
            mean_opp_dna_digivolves,
        )
        self.logger.record("pilot/games_played", self.games_played)

        # ─── add-reward-profiles Group 10 telemetry ──────────────────────
        # Tier 1: per-component global. One scalar per component that
        # appeared in any eval game since callback construction.
        for comp_name, total in self._component_totals.items():
            games = self._component_games.get(comp_name, 0)
            if games > 0:
                self.logger.record(
                    f"pilot/reward/{comp_name}/mean_per_game",
                    total / games,
                )
        # Tier 2: per-profile aggregate (mean reward + per-component share).
        for prof_name, prof_total in self._profile_reward_totals.items():
            games = self._profile_games.get(prof_name, 0)
            if games == 0:
                continue
            mean_prof_reward = prof_total / games
            self.logger.record(
                f"pilot/profile/{prof_name}/mean_reward", mean_prof_reward,
            )
            # Component share — fractional contribution to this profile's
            # mean reward. Emit only when the profile's denominator is
            # non-zero to avoid divide-by-zero on draws.
            if prof_total != 0:
                for (p, comp), comp_total in self._profile_component_totals.items():
                    if p != prof_name:
                        continue
                    self.logger.record(
                        f"pilot/profile/{prof_name}/share_{comp}",
                        comp_total / prof_total,
                    )
            # clamp_share — fraction of steps under this profile where
            # the per-profile budget clamped at least one component.
            prof_steps = self._profile_total_steps.get(prof_name, 0)
            if prof_steps > 0:
                self.logger.record(
                    f"pilot/profile/{prof_name}/clamp_share",
                    self._profile_clamp_steps.get(prof_name, 0) / prof_steps,
                )
        # Per-window boss-arrival aggregates.
        n_episodes = max(1, self.n_eval_episodes)
        self.logger.record(
            "pilot/mean_eval_digivolves_into_boss_per_game",
            self._window_total_digivolves_into_boss / n_episodes,
        )
        self.logger.record(
            "pilot/mean_eval_hardcasts_of_boss_per_game",
            self._window_total_hardcasts_of_boss / n_episodes,
        )
        if self._window_total_discipline_denom > 0:
            self.logger.record(
                "pilot/mean_eval_digivolve_discipline",
                self._window_total_discipline_num / self._window_total_discipline_denom,
            )

        # ─── add-gameplay-reward-config telemetry ────────────────────────
        # `mean_eval_winning_turn` is emitted only when wins occurred —
        # absent (NOT zero) when no agent-won games happened in the
        # window. Operators see this as a missing point on the curve
        # rather than a misleading 0.
        if self._window_total_winning_games > 0:
            self.logger.record(
                "pilot/mean_eval_winning_turn",
                self._window_total_winning_turn / self._window_total_winning_games,
            )
        # `mean_eval_digivolve_driven_attacks` is always emitted — zero
        # is meaningful here ("the agent never landed a Lv5+ attack").
        self.logger.record(
            "pilot/mean_eval_digivolve_driven_attacks",
            self._window_total_digivolve_driven_attacks / n_episodes,
        )

        # ─── BO3 match-tier metrics ──────────────────────────────────────
        # Only emitted when at least one eval episode came from a MatchEnv-
        # wrapped chain (i.e., `--match-format bo3`). In `single` mode these
        # counters stay at zero so the spec's "single-mode skips match
        # metrics" requirement is satisfied implicitly.
        if self._match_played > 0:
            match_played = self._match_played
            self.logger.record(
                "pilot/match_win_rate",
                self._match_wins / match_played,
            )
            self.logger.record(
                "pilot/match_sweep_rate",
                self._match_sweeps / max(1, self._match_wins),
            )
            self.logger.record(
                "pilot/match_swept_rate",
                self._match_swept / max(1, match_played - self._match_wins),
            )
            self.logger.record(
                "pilot/match_total_steps_mean",
                self._match_total_steps / match_played,
            )
            self.logger.record(
                "pilot/games_per_match_mean",
                self._match_total_games / max(1, match_played),
            )
            # Concede metrics — total fires per match (since one match
            # can produce up to 2 conceded games in the agent's slots).
            self.logger.record(
                "pilot/concede_rate",
                self._concede_total / max(1, self._match_total_games),
            )
            self.logger.record(
                "pilot/concede_lead_rate",
                self._concede_when_lead / max(1, self._concede_total),
            )
            self.logger.record(
                "pilot/concede_tied_rate",
                self._concede_when_tied / max(1, self._concede_total),
            )
            self.logger.record(
                "pilot/concede_down_rate",
                self._concede_when_down / max(1, self._concede_total),
            )
            self.logger.record(
                "pilot/concede_correct_rate",
                self._concede_correct / max(1, self._concede_total),
            )
            # Play-order picks
            if self._play_order_picks > 0:
                self.logger.record(
                    "pilot/play_first_rate",
                    self._play_first_count / self._play_order_picks,
                )
                if self._play_first_count > 0:
                    self.logger.record(
                        "pilot/play_order_first_winrate",
                        self._play_first_match_wins / self._play_first_count,
                    )

        # Per-archetype matchup tracking. Populated by GeneralistDeckPoolWrapper
        # (deck1_archetype + opponent_archetype) and PoolOpponentWrapper
        # (opponent_archetype only). Logged as TB scalars so trajectory tooling
        # can surface frontrunner / outlier matchups. Cumulative across all
        # evals so far — divide wins by games to get the running win rate.
        #
        # Three axes:
        #   pilot/archetype/<opp>/...                  — agent's win% vs this opp archetype
        #   pilot/agent_archetype/<agent>/...          — agent's win% piloting this archetype
        #   pilot/matchup/<agent>_vs_<opp>/...         — full N×N matchup grid
        def _sanitize(s: str) -> str:
            # TB tag-name escape: '/' delimits tag components in TB UI,
            # ' ' / parens look noisy in scalar plots.
            return s.replace("/", "_").replace(" ", "_").replace("(", "").replace(")", "")

        for arch_name in sorted(self._archetype_games.keys()):
            games = self._archetype_games[arch_name]
            if games <= 0:
                continue
            wins = self._archetype_wins.get(arch_name, 0)
            draws = self._archetype_draws.get(arch_name, 0)
            safe = _sanitize(arch_name)
            self.logger.record(f"pilot/archetype/{safe}/win_rate", wins / games)
            self.logger.record(f"pilot/archetype/{safe}/draw_rate", draws / games)
            self.logger.record(f"pilot/archetype/{safe}/games", float(games))
            # Opponent-side digivolve rates — how often THIS opponent archetype
            # digivolves on average in games against the agent. Surfaced under
            # the existing pilot/archetype/<opp>/... namespace so dashboards
            # group with the existing win/draw scalars.
            opp_digi = self._archetype_opponent_digivolves.get(arch_name, 0)
            opp_dna = self._archetype_opponent_dna_digivolves.get(arch_name, 0)
            self.logger.record(
                f"pilot/archetype/{safe}/opponent_digivolves_per_game",
                opp_digi / games,
            )
            self.logger.record(
                f"pilot/archetype/{safe}/opponent_dna_digivolves_per_game",
                opp_dna / games,
            )

        for arch_name in sorted(self._agent_archetype_games.keys()):
            games = self._agent_archetype_games[arch_name]
            if games <= 0:
                continue
            wins = self._agent_archetype_wins.get(arch_name, 0)
            draws = self._agent_archetype_draws.get(arch_name, 0)
            safe = _sanitize(arch_name)
            self.logger.record(f"pilot/agent_archetype/{safe}/win_rate", wins / games)
            self.logger.record(f"pilot/agent_archetype/{safe}/draw_rate", draws / games)
            self.logger.record(f"pilot/agent_archetype/{safe}/games", float(games))
            # Agent-side digivolve rates — how often the AGENT digivolves when
            # piloting this archetype. Sibling to win_rate above.
            agent_digi = self._agent_archetype_digivolves.get(arch_name, 0)
            agent_dna = self._agent_archetype_dna_digivolves.get(arch_name, 0)
            self.logger.record(
                f"pilot/agent_archetype/{safe}/digivolves_per_game",
                agent_digi / games,
            )
            self.logger.record(
                f"pilot/agent_archetype/{safe}/dna_digivolves_per_game",
                agent_dna / games,
            )

        for matchup in sorted(self._matchup_games.keys()):
            games = self._matchup_games[matchup]
            if games <= 0:
                continue
            wins = self._matchup_wins.get(matchup, 0)
            draws = self._matchup_draws.get(matchup, 0)
            agent_arch, opp_arch = matchup
            tag = f"{_sanitize(agent_arch)}_vs_{_sanitize(opp_arch)}"
            self.logger.record(f"pilot/matchup/{tag}/win_rate", wins / games)
            self.logger.record(f"pilot/matchup/{tag}/draw_rate", draws / games)
            self.logger.record(f"pilot/matchup/{tag}/games", float(games))

        # Per-matchup BO3 scalars (only populated when MatchEnv was active).
        for matchup in sorted(self._matchup_match_games.keys()):
            games = self._matchup_match_games[matchup]
            if games <= 0:
                continue
            match_wins = self._matchup_match_wins.get(matchup, 0)
            sweeps = self._matchup_match_sweeps.get(matchup, 0)
            agent_arch, opp_arch = matchup
            tag = f"{_sanitize(agent_arch)}_vs_{_sanitize(opp_arch)}"
            self.logger.record(f"pilot/matchup/{tag}/match_win_rate", match_wins / games)
            self.logger.record(
                f"pilot/matchup/{tag}/sweep_rate",
                sweeps / max(1, match_wins),
            )

        # Matchup-grid sidecar JSON for the training MCP / downstream
        # tooling. Written only in BO3 mode with at least one matchup
        # observed; the bo3-match-training spec defines this schema.
        if self._match_played > 0 and self._matchup_match_games:
            try:
                tb_log_dir = getattr(self.logger, "dir", None) or getattr(self.model, "tensorboard_log", None)
                if tb_log_dir is not None:
                    out_dir = Path(tb_log_dir).parent if Path(tb_log_dir).is_file() else Path(tb_log_dir)
                    out_dir.mkdir(parents=True, exist_ok=True)
                    grid_path = out_dir / f"matchup_grid_{int(self.num_timesteps)}.json"
                    grid: Dict[str, Dict[str, Dict[str, int]]] = {}
                    for (agent_arch, opp_arch), games in self._matchup_match_games.items():
                        bucket = grid.setdefault(agent_arch, {})
                        match_wins = self._matchup_match_wins.get((agent_arch, opp_arch), 0)
                        sweeps = self._matchup_match_sweeps.get((agent_arch, opp_arch), 0)
                        total_games_in_matchup = (
                            self._matchup_games.get((agent_arch, opp_arch), 0)
                        )
                        bucket[opp_arch] = {
                            "matches": int(games),
                            "match_wins": int(match_wins),
                            "sweeps": int(sweeps),
                            "total_games": int(total_games_in_matchup),
                        }
                    grid_path.write_text(json.dumps(grid, indent=2, sort_keys=True))
            except Exception:
                # Sidecar write is best-effort; don't break the eval pass.
                pass

        if self.eval_suite is not None:
            suite_states = {}

            def _agent_fn(env: DigimonEnv) -> int:
                mask = env.action_mask()
                obs = env.runner.get_board_tensor(1)
                key = id(env)
                episode_start = key not in suite_states or env._step_count == 0
                action, state = self.model.predict(
                    obs,
                    state=suite_states.get(key),
                    episode_start=np.array([episode_start]),
                    action_masks=mask,
                    deterministic=True,
                )
                suite_states[key] = state
                return int(action)

            suite_result = self.eval_suite.run(agent_fn=_agent_fn)
            self._last_eval_suite_results = {
                "overall_win_rate": suite_result.overall_win_rate,
                "suite_path": self.eval_suite_path,
                "cells": {
                    name: {
                        "games_played": cell.games_played,
                        "wins": cell.wins,
                        "losses": cell.losses,
                        "draws": cell.draws,
                        "win_rate": cell.win_rate,
                    }
                    for name, cell in suite_result.cell_results.items()
                },
            }
            self.logger.record(
                "eval_suite/overall_win_rate",
                suite_result.overall_win_rate,
            )
            for name, cell in suite_result.cell_results.items():
                self.logger.record(f"eval_suite/{name}/win_rate", cell.win_rate)

        if self.verbose:
            print(
                f"  [Eval @ {self.num_timesteps} steps] "
                f"Win rate: {win_rate:.1%} | "
                f"Mean reward: {mean_reward:.3f} | "
                f"Games played: {self.games_played}"
            )

        if self._eval_sidecar_path is not None:
            row: Dict[str, object] = {
                "step": int(self.num_timesteps),
                "wall_time": time.time(),
                "win_rate": win_rate,
                "mean_reward": mean_reward,
                "draw_rate": draw_rate,
                "mean_terminal_score": mean_terminal_score,
                "mean_dense_reward": mean_dense_reward,
                "mean_eval_episode_length": mean_length,
                "games_played": self.games_played,
                # Digivolve activity per eval window. Fired unconditionally
                # (observational, not gated on digivolve_shaping) so the
                # sidecar schema is stable across shaped/unshaped runs.
                "mean_eval_digivolves_per_game": float(
                    getattr(self, "last_mean_eval_digivolves_per_game", 0.0)
                ),
                "mean_eval_dna_digivolves_per_game": float(
                    getattr(self, "last_mean_eval_dna_digivolves_per_game", 0.0)
                ),
                "mean_eval_opponent_digivolves_per_game": float(
                    getattr(
                        self, "last_mean_eval_opponent_digivolves_per_game", 0.0
                    )
                ),
                "mean_eval_opponent_dna_digivolves_per_game": float(
                    getattr(
                        self,
                        "last_mean_eval_opponent_dna_digivolves_per_game",
                        0.0,
                    )
                ),
            }
            if self._archetype_games:
                # Per-opponent-archetype cumulative tallies. `digivolves` /
                # `dna_digivolves` are the AGENT's counts when facing this
                # opponent (sourced from p1_*). `opponent_*` are this opp's
                # counts in those games (sourced from p2_*). Asymmetry is
                # intentional — see openspec/changes/digivolve-eval-telemetry/
                # design.md Decision 5.
                row["by_archetype"] = {
                    name: {
                        "wins": self._archetype_wins.get(name, 0),
                        "draws": self._archetype_draws.get(name, 0),
                        "games": games,
                        "win_rate": self._archetype_wins.get(name, 0) / max(1, games),
                        "digivolves": self._archetype_agent_digivolves.get(name, 0),
                        "dna_digivolves": self._archetype_agent_dna_digivolves.get(
                            name, 0
                        ),
                        "opponent_digivolves": self._archetype_opponent_digivolves.get(
                            name, 0
                        ),
                        "opponent_dna_digivolves": self._archetype_opponent_dna_digivolves.get(
                            name, 0
                        ),
                        # `add-reward-profiles` Group 10 task 10.5:
                        # per-(opp_arch × component) mean contribution
                        # per game. Only includes components that
                        # actually contributed to games vs this opp.
                        "component_means": {
                            comp: total / max(1, games)
                            for (a, comp), total in self._archetype_component_totals.items()
                            if a == name
                        },
                        # Group 10 task 10.6: per-(opp_arch) boss-arrival
                        # counts. Always present (even when 0) so the
                        # schema is stable across legacy + profile runs.
                        "digivolves_into_boss": int(
                            self._archetype_digivolves_into_boss.get(name, 0)
                        ),
                        "digivolves_into_boss_dna": int(
                            self._archetype_digivolves_into_boss_dna.get(name, 0)
                        ),
                        "hardcasts_of_boss": int(
                            self._archetype_hardcasts_of_boss.get(name, 0)
                        ),
                        "hardcasts_of_boss_full_cost": int(
                            self._archetype_hardcasts_of_boss_full_cost.get(name, 0)
                        ),
                    }
                    for name, games in self._archetype_games.items()
                }
            # Group 10 task 10.5: top-level by_agent_archetype map.
            # Mirrors by_archetype's shape but keyed on AGENT archetype
            # (generalist mode only — absent when no agent_archetype
            # was seen).
            if self._agent_archetype_games:
                row["by_agent_archetype"] = {
                    name: {
                        "games": games,
                        "wins": self._agent_archetype_wins.get(name, 0),
                        "draws": self._agent_archetype_draws.get(name, 0),
                        "win_rate": self._agent_archetype_wins.get(name, 0) / max(1, games),
                        "digivolves": self._agent_archetype_digivolves.get(name, 0),
                        "dna_digivolves": self._agent_archetype_dna_digivolves.get(name, 0),
                        "component_means": {
                            comp: total / max(1, games)
                            for (a, comp), total in self._agent_archetype_component_totals.items()
                            if a == name
                        },
                        "digivolves_into_boss": int(
                            self._agent_archetype_digivolves_into_boss.get(name, 0)
                        ),
                        "digivolves_into_boss_dna": int(
                            self._agent_archetype_digivolves_into_boss_dna.get(name, 0)
                        ),
                        "hardcasts_of_boss": int(
                            self._agent_archetype_hardcasts_of_boss.get(name, 0)
                        ),
                        "hardcasts_of_boss_full_cost": int(
                            self._agent_archetype_hardcasts_of_boss_full_cost.get(name, 0)
                        ),
                    }
                    for name, games in self._agent_archetype_games.items()
                }
            # Group 10 task 10.6 + 10.8: top-level window-mean boss-arrival
            # columns. Always emitted (zero on legacy/_default games).
            n_eval = max(1, self.n_eval_episodes)
            row["mean_eval_digivolves_into_boss_per_game"] = (
                self._window_total_digivolves_into_boss / n_eval
            )
            row["mean_eval_hardcasts_of_boss_per_game"] = (
                self._window_total_hardcasts_of_boss / n_eval
            )
            row["mean_eval_digivolve_discipline"] = (
                self._window_total_discipline_num / self._window_total_discipline_denom
                if self._window_total_discipline_denom > 0
                else None
            )
            self._eval_sidecar_path.parent.mkdir(parents=True, exist_ok=True)
            with open(self._eval_sidecar_path, "a", encoding="utf-8") as fh:
                fh.write(json.dumps(row, separators=(",", ":")) + "\n")


class PeriodicCheckpointCallback(BaseCallback):
    """Save model checkpoints at fixed env-step intervals with rotation."""

    def __init__(
        self,
        save_freq: int,
        save_dir: Path,
        run_name: str,
        keep_last: int = 3,
        verbose: int = 0,
    ):
        super().__init__(verbose)
        self.save_freq = save_freq
        self.save_dir = Path(save_dir)
        self.run_name = run_name
        self.keep_last = keep_last
        self._last_save = 0
        self.saved_at: list[dict[str, str | int]] = []

    def _on_step(self) -> bool:
        if self.save_freq <= 0:
            return True
        if self.num_timesteps - self._last_save < self.save_freq:
            return True
        self._last_save = self.num_timesteps
        ckpt_dir = self.save_dir / self.run_name / "checkpoints"
        ckpt_dir.mkdir(parents=True, exist_ok=True)
        path = ckpt_dir / f"step_{self.num_timesteps:09d}.zip"
        self.model.save(str(path))
        self.saved_at.append(
            {"step": self.num_timesteps, "path": str(path), "saved_at": datetime.now().isoformat()}
        )
        checkpoints = sorted(ckpt_dir.glob("step_*.zip"))
        for old in checkpoints[: -self.keep_last]:
            old.unlink(missing_ok=True)
        return True


class ActionValidityCallback(BaseCallback):
    """Track how often sampled training actions are legal under the mask."""

    def __init__(self, verbose: int = 0):
        super().__init__(verbose)
        self.valid_count = 0
        self.total_count = 0

    def _on_step(self) -> bool:
        infos = self.locals.get("infos", [])
        actions = self.locals.get("actions", [])
        masks = self.locals.get("action_masks")
        if masks is not None:
            mask_iter = masks
        else:
            mask_iter = [info.get("action_mask") for info in infos]
        for mask, action in zip(mask_iter, actions):
            if mask is None:
                continue
            action_id = int(np.asarray(action).item())
            self.total_count += 1
            if mask[action_id] > 0:
                self.valid_count += 1
        if self.total_count and self.total_count % 1000 < max(1, len(actions)):
            self.logger.record(
                "action_validity/rate",
                self.valid_count / self.total_count,
            )
        return True


# ─── Training ────────────────────────────────────────────────────────

def make_env(opponent: str = "greedy",
             deck1: Optional[List[str]] = None,
             deck2: Optional[List[str]] = None,
             gauntlet: Optional[MetaGauntlet] = None,
             bounty_threshold: float = 0.15,
             bounty_bonus: float = 0.5,
             deck_pool_variants: Optional[List[List[str]]] = None,
             deck_pool_egg: Optional[List[str]] = None,
             deck_pool_mode: str = "eager",
             deck_pool_generate_fn: Optional[Callable] = None,
             deck_pool_generate_kwargs: Optional[Dict] = None,
             deck_pool_seed: Optional[int] = None,
             deck_pool_hybrid_max: int = 10,
             generalist_deck_pool: Optional[GeneralistDeckPool] = None,
             curriculum_seed: Optional[int] = None,
             tensor_profile: str = "standard_lite_v2",
             recording_writer: Optional[TrainingGameRecorder] = None,
             recording_source: str = "train",
             recording_env_index: int = 0,
             mulligan_log_cfg: Optional[_MulliganLogConfig] = None,
             digivolve_shaping: bool = False,
             digivolve_reward: float = 0.1,
             dna_digivolve_bonus: float = 3.9,
             match_format: str = "single",
             match_env_seed: Optional[int] = None,
             reward_profile_factory: Optional[Callable] = None) -> gymnasium.Env:
    """Create a wrapped DigimonEnv for single-agent RL training.

    Args:
        opponent: Opponent policy name ("greedy", "random", or "self-play").
                  "self-play" skips the opponent wrapper (agent plays both sides).
        deck1: Player 1 deck (card IDs). Defaults to ST1 starter.
        deck2: Player 2 deck (card IDs). Defaults to ST1 starter.
        gauntlet: MetaGauntlet instance for opponent sampling. When provided,
                  opponent decks are sampled per-episode from the deck library.
        bounty_threshold: Threat index above which bounty bonus applies.
        bounty_bonus: Bonus reward for beating high-TI opponents.
        deck_pool_variants: Pre-generated deck variants for DeckPoolWrapper.
                            When provided (non-empty), wraps env in DeckPoolWrapper.
        deck_pool_egg: Egg deck for DeckPoolWrapper (reserved for future use).
        deck_pool_mode: Generation mode for DeckPoolWrapper ("eager" or "hybrid").
        deck_pool_generate_fn: On-the-fly variant generation function (hybrid mode).
        deck_pool_generate_kwargs: Keyword arguments for generate_fn.
        deck_pool_seed: RNG seed for DeckPoolWrapper reproducibility.
        deck_pool_hybrid_max: Max dynamic variants in hybrid mode (default: 10).
        generalist_deck_pool: Eligible deck pool for generalist pretraining.
        curriculum_seed: RNG seed for generalist deck-pair sampling.
        tensor_profile: Observation tensor profile passed to DigimonEnv.

    Returns:
        ActionMasker-wrapped environment ready for MaskablePPO.

    Wrapper chain:
        DigimonEnv -> OpponentWrapper -> DeckPoolWrapper -> GauntletWrapper -> ActionMasker
    """
    if generalist_deck_pool is not None and gauntlet is not None:
        raise ValueError("--generalist cannot be combined with --gauntlet")
    if generalist_deck_pool is not None and deck_pool_variants:
        raise ValueError("generalist deck sampling cannot be combined with deck_pool_variants")

    record_this_source = (
        recording_writer is not None
        and recording_writer.enabled
        and not (recording_source == "train" and recording_writer.mode == "eval")
    )
    base_env = DigimonEnv(
        deck1=deck1,
        deck2=deck2,
        tensor_profile=tensor_profile,
        record_actions=record_this_source,
        record_tensors=record_this_source and recording_writer.record_tensors,
        digivolve_shaping=digivolve_shaping,
        digivolve_reward=digivolve_reward,
        dna_digivolve_bonus=dna_digivolve_bonus,
    )
    # `add-reward-profiles` Group 8: insert RewardProfileWrapper between
    # DigimonEnv and OpponentWrapper when a profile factory is supplied.
    # Identity factory no-ops; real factory flips DigimonEnv's
    # `emit_reward_occurrences=True` and `legacy_reward=False` flags.
    if reward_profile_factory is not None:
        base_env = reward_profile_factory(base_env)

    if opponent == "self-play":
        env = base_env
    else:
        opponent_policies = {
            "greedy": greedy_policy,
            "random": random_policy,
        }
        try:
            opponent_fn = opponent_policies[opponent]
        except KeyError:
            valid_opponents = list(opponent_policies.keys()) + ["self-play"]
            raise ValueError(
                f"Unknown opponent {opponent!r}. "
                f"Expected one of {valid_opponents}."
            )
        env = OpponentWrapper(base_env, opponent_fn=opponent_fn)

    # BO3 match wrapper sits between OpponentWrapper and any deck-pool
    # wrappers so each deck-pool reset corresponds to a MATCH (not a
    # single game). See `add-bo3-match-training/design.md` §D5.
    if match_format == "bo3":
        from digimon_gym.agents.match_env import MatchEnv
        env = MatchEnv(env, seed=match_env_seed)

    # Deck pool wrapper for agent deck variation
    if deck_pool_variants and len(deck_pool_variants) > 0:
        from digimon_gym.agents.deck_pool import DeckPoolWrapper
        env = DeckPoolWrapper(
            env,
            variants=deck_pool_variants,
            egg_deck=deck_pool_egg,
            generation_mode=deck_pool_mode,
            generate_fn=deck_pool_generate_fn,
            generate_kwargs=deck_pool_generate_kwargs or {},
            hybrid_max_dynamic=deck_pool_hybrid_max,
            seed=deck_pool_seed,
        )

    if generalist_deck_pool is not None:
        env = GeneralistDeckPoolWrapper(
            env,
            deck_pool=generalist_deck_pool,
            seed=curriculum_seed,
        )

    # Gauntlet wrapper for meta-weighted opponent sampling
    if gauntlet is not None and gauntlet.deck_count > 0:
        player_deck = deck1 if deck1 else base_env._deck1
        env = GauntletWrapper(
            env, gauntlet,
            player_deck=player_deck,
            bounty_threshold=bounty_threshold,
            bounty_bonus=bounty_bonus,
        )

    # Always insert TrainingRecordingWrapper — it is the panic safety
    # rail that converts PyO3 engine panics into synthetic terminal
    # steps so training survives unexpected engine crashes. Recording is
    # a separate concern handled inside the wrapper via
    # `recording_writer.should_record(...)`; when the recorder is
    # disabled (mode="off") the wrapper is a pure panic-catch with no
    # artifact writes.
    env = TrainingRecordingWrapper(
        env,
        recording_writer,
        source=recording_source,
        env_index=recording_env_index,
    )

    if mulligan_log_cfg is not None and mulligan_log_cfg.enabled:
        writer = MulliganLogWriter(
            output_dir=mulligan_log_cfg.output_dir,
            env_index=recording_env_index,
            enabled=mulligan_log_cfg.enabled,
            run_metadata=mulligan_log_cfg.run_metadata,
        )
        env = MulliganLogWrapper(
            env,
            writer=writer,
            source=recording_source,
            env_index=recording_env_index,
        )

    def mask_fn(env):
        return _unwrap_to_digimon_env(env).action_mask()

    return ActionMasker(env, mask_fn)


def make_vec_env(
    cfg: TrainingConfig,
    opponent_fn: Callable[[DigimonEnv], int],
    deck1: Optional[List[str]] = None,
    deck2: Optional[List[str]] = None,
    generalist_deck_pool: Optional[GeneralistDeckPool] = None,
    curriculum_seed: Optional[int] = None,
    recording_writer: Optional[TrainingGameRecorder] = None,
    mulligan_log_cfg: Optional[_MulliganLogConfig] = None,
    reward_profile_factory: Optional[Callable] = None,
):
    """Build ActionMasker-wrapped vector environments from TrainingConfig."""

    def _factory(rank: int):
        def _init():
            record_this_source = (
                recording_writer is not None
                and recording_writer.enabled
                and recording_writer.mode != "eval"
            )
            base_env = DigimonEnv(
                deck1=deck1,
                deck2=deck2,
                tensor_profile=cfg.tensor_profile,
                record_actions=record_this_source,
                record_tensors=record_this_source and recording_writer.record_tensors,
                digivolve_shaping=cfg.digivolve_shaping,
                digivolve_reward=cfg.digivolve_reward,
                dna_digivolve_bonus=cfg.dna_digivolve_bonus,
            )
            # `add-reward-profiles` Group 8: wrap before OpponentWrapper.
            if reward_profile_factory is not None:
                base_env = reward_profile_factory(base_env)
            wrapped = OpponentWrapper(base_env, opponent_fn=opponent_fn)
            # BO3 match wrapper between OpponentWrapper and deck-pool wrappers.
            if cfg.match_format == "bo3":
                from digimon_gym.agents.match_env import MatchEnv
                match_seed = None if cfg.seed is None else cfg.seed + rank
                wrapped = MatchEnv(wrapped, seed=match_seed)
            if generalist_deck_pool is not None:
                seed = None if curriculum_seed is None else curriculum_seed + rank
                wrapped = GeneralistDeckPoolWrapper(
                    wrapped,
                    deck_pool=generalist_deck_pool,
                    seed=seed,
                )
            # Panic safety rail — see comment in `make_env` above.
            wrapped = TrainingRecordingWrapper(
                wrapped,
                recording_writer,
                source="train",
                env_index=rank,
            )

            if mulligan_log_cfg is not None and mulligan_log_cfg.enabled:
                writer = MulliganLogWriter(
                    output_dir=mulligan_log_cfg.output_dir,
                    env_index=rank,
                    enabled=mulligan_log_cfg.enabled,
                    run_metadata=mulligan_log_cfg.run_metadata,
                )
                wrapped = MulliganLogWrapper(
                    wrapped,
                    writer=writer,
                    source="train",
                    env_index=rank,
                )

            def mask_fn(env):
                return _unwrap_to_digimon_env(env).action_mask()

            env = ActionMasker(wrapped, mask_fn)
            env.reset(seed=cfg.seed + rank)
            return env

        return _init

    factories = [_factory(rank) for rank in range(cfg.n_envs)]
    if getattr(cfg, "vec_env_backend", "dummy") == "subproc" and cfg.n_envs > 1:
        return SubprocVecEnv(factories, start_method="spawn")
    return DummyVecEnv(factories)


def save_model(model: Union[MaskablePPO, MaskableRecurrentPPO],
               models_dir: str = DEFAULT_MODELS_DIR,
               job_id: Optional[str] = None) -> str:
    """Save trained model with a unique filename.

    When ``job_id`` is provided (worker-dispatched jobs), the UUID is used
    as the filename suffix for guaranteed uniqueness across parallel runs.
    Otherwise falls back to a timestamp (CLI usage).

    Returns:
        Path to the saved model file.
    """
    os.makedirs(models_dir, exist_ok=True)
    if job_id is not None:
        suffix = job_id
    else:
        suffix = datetime.now().strftime("%Y%m%d_%H%M%S")
    path = os.path.join(models_dir, f"pilot_ppo_{suffix}")
    model.save(path)
    return path


def train(total_timesteps: int = 100_000,
          opponent: str = "greedy",
          eval_freq: int = 10_000,
          n_eval_episodes: int = 20,
          learning_rate: float = 3e-4,
          n_steps: int = 2048,
          batch_size: int = 64,
          n_epochs: int = 10,
          gamma: float = 0.99,
          tensorboard_log: str = "runs/pilot_ppo",
          verbose: int = 1,
          save_dir: str = "models",
          gauntlet: Optional[MetaGauntlet] = None,
          deck1: Optional[List[str]] = None,
          deck2: Optional[List[str]] = None,
          bounty_threshold: float = 0.15,
          bounty_bonus: float = 0.5,
          use_lstm: bool = False,
          lstm_hidden_size: int = 256,
          device: str = "auto",
          job_id: Optional[str] = None,
          deck_pool_variants: Optional[List[List[str]]] = None,
          deck_pool_mode: str = "eager",
          deck_pool_seed: Optional[int] = None,
          deck_pool_hybrid_max: int = 10,
          generalist_deck_pool: Optional[GeneralistDeckPool] = None,
          curriculum_seed: Optional[int] = None,
          eval_seed: Optional[int] = None,
          deck_pool_snapshot_path: Optional[str] = None,
          cfg: Optional[TrainingConfig] = None,
          reward_profiles_override_mismatch: bool = False,
          ) -> Union[MaskablePPO, MaskableRecurrentPPO]:
    """Train a Pilot Agent using MaskablePPO or MaskableRecurrentPPO.

    Args:
        total_timesteps: Total environment steps to train for.
        opponent: Opponent policy ("greedy", "random", "self-play").
        eval_freq: Steps between evaluation rounds.
        n_eval_episodes: Games per evaluation round.
        learning_rate: PPO learning rate.
        n_steps: Rollout buffer size (steps per update).
        batch_size: Minibatch size for PPO updates.
        n_epochs: PPO epochs per update.
        gamma: Discount factor.
        tensorboard_log: TensorBoard log directory.
        verbose: Verbosity level (0=silent, 1=info).
        save_dir: Directory for saving model checkpoints.
        gauntlet: MetaGauntlet for meta-weighted opponent sampling.
        deck1: Player 1 deck (card IDs). Defaults to ST1 starter.
        deck2: Player 2 deck (card IDs). Defaults to ST1 starter.
        bounty_threshold: TI threshold for bounty bonus.
        bounty_bonus: Bonus reward for beating high-TI opponents.
        use_lstm: Use LSTM policy (MaskableRecurrentPPO) instead of MLP.
        lstm_hidden_size: LSTM hidden units per layer (default: 256).
        device: PyTorch device for training (default: "auto" lets SB3 choose).
        job_id: Optional job UUID for unique checkpoint filenames.
        deck_pool_variants: Pre-generated deck variants for DeckPoolWrapper.
        deck_pool_mode: Generation mode ("eager" or "hybrid").
        deck_pool_seed: RNG seed for DeckPoolWrapper reproducibility.
        deck_pool_hybrid_max: Max dynamic variants in hybrid mode (default: 10).
        generalist_deck_pool: Eligible deck pool for generalist pretraining.
        curriculum_seed: RNG seed for generalist deck-pair sampling.
        eval_seed: RNG seed for evaluation deck-pair sampling.
        deck_pool_snapshot_path: Optional path to write the frozen deck-pool snapshot.

    Returns:
        Trained model (MaskablePPO or MaskableRecurrentPPO).
    """
    config_driven = cfg is not None
    if cfg is None:
        cfg = TrainingConfig(
            algorithm="lstm" if use_lstm else "mlp",
            timesteps=total_timesteps,
            seed=0,
            learning_rate=learning_rate,
            n_steps=n_steps,
            batch_size=batch_size,
            n_epochs=n_epochs,
            gamma=gamma,
            opponent=opponent,
            eval_freq=eval_freq,
            eval_episodes=n_eval_episodes,
            checkpoint_every=0,
            models_dir=save_dir,
            tensorboard_log=tensorboard_log,
            curriculum_seed=curriculum_seed,
            eval_seed=eval_seed,
        )
    else:
        total_timesteps = cfg.timesteps
        opponent = cfg.opponent
        eval_freq = cfg.eval_freq
        n_eval_episodes = cfg.eval_episodes
        learning_rate = cfg.learning_rate
        n_steps = cfg.n_steps
        batch_size = cfg.batch_size
        n_epochs = cfg.n_epochs
        gamma = cfg.gamma
        tensorboard_log = cfg.tensorboard_log
        save_dir = cfg.models_dir
        use_lstm = cfg.algorithm == "lstm"
        lstm_hidden_size = cfg.lstm_hidden_size
        curriculum_seed = cfg.curriculum_seed if curriculum_seed is None else curriculum_seed
        eval_seed = cfg.eval_seed if eval_seed is None else eval_seed

    _seed_everything(cfg.seed)
    observation_layout = get_tensor_profile(cfg.tensor_profile)
    run_name = cfg.run_name or datetime.now().strftime("pilot_ppo_%Y%m%d_%H%M%S")
    run_dir = Path(save_dir) / run_name

    # `add-reward-profiles` Group 8: build the reward-profile factory
    # once at run-start. The factory is an identity no-op when the
    # configured `reward_profiles_path` doesn't exist (legacy reward
    # path stays active). Loader failures abort the run — silent
    # fallback would mask config bugs.
    reward_profile_factory = _build_reward_profile_factory(cfg)
    if verbose and reward_profile_factory.active:
        loader = reward_profile_factory.loader
        eff = reward_profile_factory.effective_override
        print(
            f"  Reward profiles: loaded from gameplay={loader.gameplay_path} "
            f"+ profiles={loader.profiles_path} "
            f"(gameplay_hash={loader.profiles.gameplay_hash[:14]}..., "
            f"profiles_hash={loader.profiles.profiles_hash[:14]}..., "
            f"override={eff!r}, hot_reload={cfg.reward_profiles_hot_reload})"
        )

    # Resume-time reward-profile hash check + run-start sidecar write.
    # Per spec: when resuming AND the current profiles file's canonical
    # hash differs from the checkpoint's recorded hash, abort with a
    # clear error naming both hashes. The CLI flag
    # `--reward-profiles-override-mismatch` bypasses (caller takes
    # responsibility for intentional reward-shape changes mid-run).
    if reward_profile_factory.active:
        from digimon_gym.agents.reward.run_metadata import (
            check_resume_hash as _rp_check_resume,
            write_sidecar as _rp_write_sidecar,
        )
        profiles_snap = reward_profile_factory.loader.profiles
        current_profiles_hash = profiles_snap.profiles_hash
        current_gameplay_hash = profiles_snap.gameplay_hash
        if cfg.resume_from:
            # Resolve the resume-target's run_dir from the checkpoint
            # zip path. The sidecar lives in the parent dir. The single
            # `--reward-profiles-override-mismatch` flag covers BOTH
            # files; if both drifted, the gameplay-side error fires
            # first (operators usually want to know about the universal
            # shape change first).
            checkpoint_dir = Path(cfg.resume_from).parent
            _rp_check_resume(
                checkpoint_dir,
                current_profiles_hash,
                override_mismatch=reward_profiles_override_mismatch,
                current_gameplay_hash=current_gameplay_hash,
            )
        # Always write a fresh sidecar at run-start (new run OR resume
        # after override). The sidecar reflects the CURRENT hashes so
        # later resumes have an up-to-date reference point.
        _rp_write_sidecar(
            run_dir,
            reward_profiles_path=cfg.reward_profiles_path,
            reward_profiles_hash=current_profiles_hash,
            reward_gameplay_path=cfg.reward_gameplay_path,
            reward_gameplay_hash=current_gameplay_hash,
            reward_profile_override=reward_profile_factory.effective_override,
            reward_assignments_snapshot=dict(profiles_snap.assignments),
        )
    recordings_dir = Path(cfg.record_games_dir) if cfg.record_games_dir else run_dir / "recordings"
    recording_writer = TrainingGameRecorder(
        recordings_dir,
        mode=cfg.record_games,
        max_recordings=cfg.record_games_max,
        sample_rate=cfg.record_games_sample_rate,
        record_tensors=cfg.record_game_tensors,
        run_metadata={
            "run_name": run_name,
            "backend": os.environ.get("DIGIMON_BACKEND") or "auto",
            "tensor_profile": observation_layout.id,
            "tensor_layout_hash": observation_layout.layout_hash,
        },
        seed=cfg.seed,
    )

    mulligan_log_cfg = _MulliganLogConfig(
        output_dir=run_dir,
        enabled=(cfg.mulligan_log != "off"),
        run_metadata={
            "run_name": run_name,
            "started_at": datetime.now(timezone.utc).isoformat(),
            "backend": os.environ.get("DIGIMON_BACKEND") or "auto",
            "tensor_profile": observation_layout.id,
            "tensor_layout_hash": observation_layout.layout_hash,
        },
    )

    eval_game_log_writer: Optional[GameLogWriter] = None
    if cfg.eval_game_log != "off":
        run_dir.mkdir(parents=True, exist_ok=True)
        eval_game_log_writer = GameLogWriter(run_dir / "eval_game_log.jsonl")

    if cfg.resume_from and cfg.init_from:
        raise ValueError("resume_from and init_from are mutually exclusive")
    if cfg.init_from:
        _validate_checkpoint_contract(cfg.init_from, observation_layout)
    if generalist_deck_pool is not None:
        if deck_pool_snapshot_path is None:
            deck_pool_snapshot_path = cfg.curriculum_pool_out
        if not deck_pool_snapshot_path and not cfg.curriculum_pool:
            deck_pool_snapshot_path = str(run_dir / "deck_pool_snapshot.json")
        if deck_pool_snapshot_path:
            generalist_deck_pool.write_snapshot(deck_pool_snapshot_path)
    algorithm_name = "MaskableRecurrentPPO" if use_lstm else "MaskablePPO"
    if verbose:
        print("=" * 60)
        print("Digimon TCG Pilot Agent Training")
        print("=" * 60)
        print(f"  Algorithm:      {algorithm_name}")
        if use_lstm:
            print(f"  LSTM hidden:    {lstm_hidden_size}")
        print(f"  Opponent:       {opponent}")
        print(f"  Total steps:    {total_timesteps:,}")
        print(f"  Learning rate:  {learning_rate}")
        print(f"  Batch size:     {batch_size}")
        print(f"  Rollout steps:  {n_steps}")
        print(f"  Eval freq:      every {eval_freq:,} steps")
        print(f"  TensorBoard:    {tensorboard_log}")
        print(f"  Tensor profile: {observation_layout.id}")
        print(
            f"  Tensor layout:  size={observation_layout.tensor_size}, "
            f"tensor_v={observation_layout.tensor_version}, "
            f"schema={observation_layout.feature_schema_version}"
        )
        if observation_layout.layout_hash:
            print(f"  Layout hash:    {observation_layout.layout_hash}")
        if recording_writer.enabled:
            print(f"  Record games:   {cfg.record_games} -> {recordings_dir}")
        if mulligan_log_cfg.enabled:
            print(f"  Mulligan log:   on -> {mulligan_log_cfg.output_dir}/mulligan_log_env_*.jsonl")
        else:
            print(f"  Mulligan log:   off")
        if eval_game_log_writer is not None:
            print(f"  Eval game log:  on -> {eval_game_log_writer.path}")
        else:
            print(f"  Eval game log:  off")
        if gauntlet and gauntlet.deck_count > 0:
            print(f"  Gauntlet:       {gauntlet.archetype_count} archetypes, "
                  f"{gauntlet.deck_count} decks")
            print(f"  Bounty:         +{bounty_bonus} if TI > {bounty_threshold}")
        if generalist_deck_pool is not None:
            print(
                f"  Generalist:     {generalist_deck_pool.archetype_count} archetypes, "
                f"{generalist_deck_pool.deck_count} decks"
            )
            print(f"  Curriculum seed:{curriculum_seed}")
        print("=" * 60)

    pool_opponent_fn = None
    if opponent == "pool":
        pool = OpponentPool.load(Path(cfg.opponent_pool_manifest or ""))
        if pool.size == 0:
            raise ValueError(f"opponent_pool_manifest {cfg.opponent_pool_manifest} is empty")
        pool_opponent_fn = make_pool_opponent_fn(
            pool,
            mode=cfg.opponent_pool_mode,
        )

    # Create training environment
    if pool_opponent_fn is not None:
        env = make_vec_env(
            cfg,
            opponent_fn=pool_opponent_fn,
            deck1=deck1,
            deck2=deck2,
            generalist_deck_pool=generalist_deck_pool,
            curriculum_seed=curriculum_seed,
            recording_writer=recording_writer,
            mulligan_log_cfg=mulligan_log_cfg,
            reward_profile_factory=reward_profile_factory,
        )
    elif cfg.n_envs > 1:
        opponent_policies = {"greedy": greedy_policy, "random": random_policy}
        if opponent not in opponent_policies:
            raise ValueError(f"cfg.n_envs > 1 currently supports greedy/random, got {opponent}")
        env = make_vec_env(
            cfg,
            opponent_fn=opponent_policies[opponent],
            deck1=deck1,
            deck2=deck2,
            generalist_deck_pool=generalist_deck_pool,
            curriculum_seed=curriculum_seed,
            recording_writer=recording_writer,
            mulligan_log_cfg=mulligan_log_cfg,
            reward_profile_factory=reward_profile_factory,
        )
    else:
        env = make_env(
            opponent=opponent,
            deck1=deck1,
            deck2=deck2,
            gauntlet=gauntlet,
            bounty_threshold=bounty_threshold,
            bounty_bonus=bounty_bonus,
            deck_pool_variants=deck_pool_variants,
            deck_pool_mode=deck_pool_mode,
            deck_pool_seed=deck_pool_seed,
            deck_pool_hybrid_max=deck_pool_hybrid_max,
            generalist_deck_pool=generalist_deck_pool,
            curriculum_seed=curriculum_seed,
            tensor_profile=cfg.tensor_profile,
            recording_writer=recording_writer,
            recording_source="train",
            mulligan_log_cfg=mulligan_log_cfg,
            digivolve_shaping=cfg.digivolve_shaping,
            digivolve_reward=cfg.digivolve_reward,
            dna_digivolve_bonus=cfg.dna_digivolve_bonus,
            match_format=cfg.match_format,
            match_env_seed=cfg.seed,
            reward_profile_factory=reward_profile_factory,
        )

    # Load autoencoder embeddings for warm-start (if available)
    pretrained_embeddings = None
    emb_path = os.path.join(
        os.path.dirname(__file__), '..', 'engine', 'data', 'card_embeddings.npy'
    )
    if os.path.exists(emb_path):
        pretrained_embeddings = np.load(emb_path)
        if verbose:
            print(f"  Warm-start: loaded {len(pretrained_embeddings)} card embeddings")

    # Create model
    extractor_kwargs = dict(
        features_extractor_class=CardEmbeddingExtractor,
        features_extractor_kwargs=dict(
            features_dim=512,
            pretrained_embeddings=pretrained_embeddings,
            observation_layout=observation_layout,
        ),
    )

    if cfg.resume_from:
        if use_lstm:
            model = MaskableRecurrentPPO.load(cfg.resume_from, env=env, device=device)
        else:
            model = MaskablePPO.load(cfg.resume_from, env=env, device=device)
        if verbose:
            print(f"  [resume] loaded checkpoint, num_timesteps={model.num_timesteps}")
    elif cfg.init_from:
        if use_lstm:
            model = MaskableRecurrentPPO.load(cfg.init_from, env=env, device=device)
        else:
            model = MaskablePPO.load(cfg.init_from, env=env, device=device)
        if verbose:
            print(f"  [init] loaded base checkpoint from {cfg.init_from}")
    elif use_lstm:
        model = MaskableRecurrentPPO(
            MaskableMlpLstmPolicy,
            env,
            learning_rate=learning_rate,
            n_steps=n_steps,
            batch_size=n_steps,  # RecurrentPPO requires batch_size == n_steps
            n_epochs=n_epochs,
            gamma=gamma,
            gae_lambda=cfg.gae_lambda,
            clip_range=cfg.clip_range,
            ent_coef=cfg.ent_coef,
            vf_coef=cfg.vf_coef,
            max_grad_norm=cfg.max_grad_norm,
            tensorboard_log=tensorboard_log,
            verbose=0,
            device=device,
            seed=cfg.seed,
            policy_kwargs=dict(
                lstm_hidden_size=lstm_hidden_size,
                n_lstm_layers=1,
                enable_critic_lstm=True,
                net_arch=dict(pi=[64], vf=[64]),
                **extractor_kwargs,
            ),
        )
    else:
        model = MaskablePPO(
            "MlpPolicy",
            env,
            learning_rate=learning_rate,
            n_steps=n_steps,
            batch_size=batch_size,
            n_epochs=n_epochs,
            gamma=gamma,
            gae_lambda=cfg.gae_lambda,
            clip_range=cfg.clip_range,
            ent_coef=cfg.ent_coef,
            vf_coef=cfg.vf_coef,
            max_grad_norm=cfg.max_grad_norm,
            tensorboard_log=tensorboard_log,
            verbose=0,
            device=device,
            seed=cfg.seed,
            policy_kwargs=extractor_kwargs,
        )

    # Create evaluation callback
    if pool_opponent_fn is not None:
        def eval_env_fn():
            base_env = DigimonEnv(
                deck1=deck1,
                deck2=deck2,
                tensor_profile=cfg.tensor_profile,
                record_actions=recording_writer.enabled,
                record_tensors=recording_writer.enabled and recording_writer.record_tensors,
                digivolve_shaping=cfg.digivolve_shaping,
                digivolve_reward=cfg.digivolve_reward,
                dna_digivolve_bonus=cfg.dna_digivolve_bonus,
            )
            # `add-reward-profiles` Group 8: wrap before OpponentWrapper.
            if reward_profile_factory is not None:
                base_env = reward_profile_factory(base_env)
            wrapped = OpponentWrapper(base_env, opponent_fn=pool_opponent_fn)
            # BO3 match wrapper between OpponentWrapper and deck-pool wrappers.
            if cfg.match_format == "bo3":
                from digimon_gym.agents.match_env import MatchEnv
                wrapped = MatchEnv(wrapped, seed=eval_seed)
            if generalist_deck_pool is not None:
                wrapped = GeneralistDeckPoolWrapper(
                    wrapped,
                    deck_pool=generalist_deck_pool,
                    seed=eval_seed if eval_seed is not None else curriculum_seed,
                )
            # Panic safety rail — see comment in `make_env` above.
            wrapped = TrainingRecordingWrapper(
                wrapped,
                recording_writer,
                source="eval",
                env_index=0,
            )
            if mulligan_log_cfg is not None and mulligan_log_cfg.enabled:
                writer = MulliganLogWriter(
                    output_dir=mulligan_log_cfg.output_dir,
                    env_index=0,
                    enabled=mulligan_log_cfg.enabled,
                    run_metadata=mulligan_log_cfg.run_metadata,
                )
                wrapped = MulliganLogWrapper(
                    wrapped,
                    writer=writer,
                    source="eval",
                    env_index=0,
                )
            return ActionMasker(
                wrapped,
                lambda env: _unwrap_to_digimon_env(env).action_mask(),
            )
    else:
        eval_env_fn = lambda: make_env(
            opponent=opponent, deck1=deck1, deck2=deck2,
            gauntlet=gauntlet, bounty_threshold=bounty_threshold,
            bounty_bonus=bounty_bonus,
            deck_pool_variants=deck_pool_variants,
            deck_pool_mode=deck_pool_mode,
            deck_pool_seed=deck_pool_seed,
            deck_pool_hybrid_max=deck_pool_hybrid_max,
            generalist_deck_pool=generalist_deck_pool,
            curriculum_seed=eval_seed if eval_seed is not None else curriculum_seed,
            tensor_profile=cfg.tensor_profile,
            recording_writer=recording_writer,
            recording_source="eval",
            mulligan_log_cfg=mulligan_log_cfg,
            digivolve_shaping=cfg.digivolve_shaping,
            digivolve_reward=cfg.digivolve_reward,
            dna_digivolve_bonus=cfg.dna_digivolve_bonus,
            match_format=cfg.match_format,
            match_env_seed=cfg.eval_seed,
            reward_profile_factory=reward_profile_factory,
        )
    eval_suite = None
    if cfg.eval_suite:
        from digimon_gym.agents.eval_suite import HeldOutEvalSuite

        eval_suite = HeldOutEvalSuite.from_yaml(
            Path(cfg.eval_suite),
            tensor_profile=cfg.tensor_profile,
        )
    eval_sidecar_path: Optional[Path] = (
        Path(tensorboard_log) / "evals.jsonl" if tensorboard_log else None
    )
    win_rate_cb = WinRateCallback(
        eval_env_fn=eval_env_fn,
        eval_freq=eval_freq,
        n_eval_episodes=n_eval_episodes,
        eval_suite=eval_suite,
        eval_suite_path=cfg.eval_suite,
        eval_sidecar_path=eval_sidecar_path,
        game_log_writer=eval_game_log_writer,
        verbose=verbose,
    )
    action_validity_cb = ActionValidityCallback()
    callbacks: list[BaseCallback] = [win_rate_cb, action_validity_cb]
    checkpoint_cb = None
    if cfg.checkpoint_every > 0:
        checkpoint_cb = PeriodicCheckpointCallback(
            save_freq=cfg.checkpoint_every,
            save_dir=Path(save_dir),
            run_name=run_name,
            keep_last=cfg.keep_last_checkpoints,
        )
        callbacks.append(checkpoint_cb)

    # Train
    start = time.time()
    try:
        model.learn(
            total_timesteps=total_timesteps,
            callback=callbacks,
            reset_num_timesteps=cfg.resume_from is None,
        )
    finally:
        win_rate_cb.close()
    elapsed = time.time() - start

    if verbose:
        print()
        print(f"Training complete in {elapsed:.1f}s")
        print(f"  Steps/sec: {total_timesteps / elapsed:,.0f}")

    # Save model
    if config_driven:
        run_dir.mkdir(parents=True, exist_ok=True)
        final_path = run_dir / "final.zip"
        model.save(str(final_path))
        model_path = str(final_path)
    else:
        model_path = save_model(model, save_dir, job_id=job_id)
    if verbose:
        print(f"  Model saved to: {model_path}")

    # Save training run metadata as JSON sidecar
    from digimon_gym.agents.training_metrics import TrainingRunMetadata

    run_id = Path(model_path).stem
    if generalist_deck_pool is not None:
        training_mode = "generalist"
        sampling_policy = "uniform_archetype_then_deck"
        eligible_archetypes = generalist_deck_pool.archetype_names
        eligible_deck_count = generalist_deck_pool.deck_count
        snapshot_path = generalist_deck_pool.snapshot_path
        snapshot_hash = generalist_deck_pool.snapshot_hash
    elif cfg.init_from:
        training_mode = "fine_tune"
        sampling_policy = ""
        eligible_archetypes = []
        eligible_deck_count = 0
        snapshot_path = ""
        snapshot_hash = ""
    else:
        training_mode = "standard"
        sampling_policy = ""
        eligible_archetypes = []
        eligible_deck_count = 0
        snapshot_path = ""
        snapshot_hash = ""

    meta = TrainingRunMetadata(
        run_id=run_id,
        started_at=datetime.fromtimestamp(start).isoformat(),
        finished_at=datetime.now().isoformat(),
        algorithm="LSTM" if use_lstm else "MLP",
        total_timesteps=total_timesteps,
        opponent_type=opponent,
        model_path=model_path,
        tensorboard_log_dir=tensorboard_log,
        observation_profile=observation_layout.id,
        tensor_version=observation_layout.tensor_version,
        feature_schema_version=observation_layout.feature_schema_version,
        tensor_size=observation_layout.tensor_size,
        tensor_layout_hash=observation_layout.layout_hash,
        action_space_size=ACTION_SPACE_SIZE,
        card_registry_capacity=REGISTRY_CAPACITY,
        embedding_dim=EMBEDDING_DIM,
        training_mode=training_mode,
        sampling_policy=sampling_policy,
        training_seed=cfg.seed,
        curriculum_seed=curriculum_seed,
        eval_seed=eval_seed,
        deck_pool_snapshot_path=snapshot_path,
        deck_pool_snapshot_hash=snapshot_hash,
        eligible_archetypes=eligible_archetypes,
        eligible_deck_count=eligible_deck_count,
        base_checkpoint=cfg.init_from or "",
        fine_tune_deck_config={
            "deck1_card_count": len(deck1) if deck1 is not None else 0,
            "deck2_card_count": len(deck2) if deck2 is not None else 0,
            "uses_gauntlet": gauntlet is not None,
        } if cfg.init_from else {},
        final_win_rate=win_rate_cb.last_win_rate,
        final_mean_reward=win_rate_cb.last_mean_reward,
        final_draw_rate=win_rate_cb.last_draw_rate,
        final_mean_eval_terminal_score=win_rate_cb.last_mean_eval_terminal_score,
        final_mean_eval_dense_reward=win_rate_cb.last_mean_eval_dense_reward,
        final_mean_eval_episode_length=win_rate_cb.last_mean_eval_episode_length,
        total_games=win_rate_cb.games_played,
        archetype_results=win_rate_cb.get_archetype_results(),
        hyperparameters={
            **cfg.to_dict(),
            "learning_rate": learning_rate,
            "n_steps": n_steps,
            "batch_size": batch_size,
            "n_epochs": n_epochs,
            "gamma": gamma,
        },
        digivolve_shaping=cfg.digivolve_shaping,
        digivolve_reward=cfg.digivolve_reward,
        dna_digivolve_bonus=cfg.dna_digivolve_bonus,
        match_format=cfg.match_format,
        # add-gameplay-reward-config: persist gameplay-yaml path + hash
        # so downstream tooling can distinguish runs by their universal
        # shape without re-loading the YAML. Empty defaults when the
        # reward-profile factory isn't active (legacy run).
        reward_gameplay_path=(
            cfg.reward_gameplay_path
            if reward_profile_factory.active
            else ""
        ),
        reward_gameplay_hash=(
            reward_profile_factory.loader.profiles.gameplay_hash
            if reward_profile_factory.active
            else ""
        ),
    )
    if checkpoint_cb is not None:
        meta.checkpoint_timestamps = checkpoint_cb.saved_at
    if eval_suite is not None:
        meta.eval_suite_results = win_rate_cb.get_eval_suite_results()
    meta_path = Path(model_path).with_suffix(".meta.json")
    meta.save(meta_path)
    if verbose:
        print(f"  Metadata saved to: {meta_path}")

    return model


# ─── CLI ─────────────────────────────────────────────────────────────

def _build_argparser() -> argparse.ArgumentParser:
    """Build and return the CLI argument parser for pilot training."""
    parser = argparse.ArgumentParser(
        description="Train Digimon TCG Pilot Agent (MaskablePPO / MaskableRecurrentPPO)"
    )
    parser.add_argument(
        "--config", type=str, default="configs/training/default.yaml",
        help="Path to TrainingConfig YAML."
    )
    parser.add_argument(
        "--set", action="append", default=[], dest="overrides",
        help="Override one config field, e.g. --set seed=42 (repeatable)."
    )
    parser.add_argument(
        "--resume", type=str, default=None,
        help="Resume from a saved checkpoint .zip."
    )
    parser.add_argument(
        "--reward-profiles-override-mismatch", action="store_true",
        default=False,
        help=(
            "Bypass the resume-time reward-profile hash check when the "
            "current `profiles.yaml` differs from the checkpoint's snapshot. "
            "Use only when you intend to switch reward shape mid-run."
        ),
    )
    parser.add_argument(
        "--init-from", type=str, default=None,
        help="Initialize a fine-tune run from a compatible base checkpoint .zip."
    )
    parser.add_argument(
        "--timesteps", type=int, default=None,
        help="Total training timesteps."
    )
    opponent_group = parser.add_mutually_exclusive_group()
    opponent_group.add_argument(
        "--opponent", choices=["greedy", "random"],
        default=None,
        help="Opponent policy."
    )
    opponent_group.add_argument(
        "--self-play", action="store_true",
        help="Enable self-play (agent plays both sides)"
    )
    parser.add_argument(
        "--lr", type=float, default=None,
        help="Learning rate."
    )
    parser.add_argument(
        "--batch-size", type=int, default=None,
        help="Minibatch size."
    )
    parser.add_argument(
        "--n-steps", type=int, default=None,
        help="Rollout buffer size."
    )
    parser.add_argument(
        "--eval-freq", type=int, default=None,
        help="Steps between evaluations."
    )
    parser.add_argument(
        "--eval-episodes", type=int, default=None,
        help="Games per evaluation."
    )
    parser.add_argument(
        "--log-dir", type=str, default=None,
        help="TensorBoard log directory."
    )
    parser.add_argument(
        "--save-dir", type=str, default=None,
        help="Model save directory."
    )
    parser.add_argument(
        "--gauntlet", action="store_true",
        help="Enable MetaGauntlet opponent sampling from deck_library.json"
    )
    parser.add_argument(
        "--gauntlet-sampling", choices=["meta", "random"], default="meta",
        help="Gauntlet sampling mode: threat-index meta weights or uniform random decks."
    )
    parser.add_argument(
        "--generalist", action="store_true",
        help="Sample both player decks from eligible Rust DSL archetypes."
    )
    parser.add_argument(
        "--generalist-sampling", choices=["uniform-archetype"], default="uniform-archetype",
        help="Generalist sampling mode. Currently uniform archetype, then uniform deck."
    )
    parser.add_argument(
        "--curriculum-seed", type=int, default=None,
        help="Seed controlling generalist deck-pair sampling."
    )
    parser.add_argument(
        "--eval-seed", type=int, default=None,
        help="Seed controlling evaluation deck-pair sampling."
    )
    parser.add_argument(
        "--curriculum-pool", type=str, default=None,
        help="Path to a frozen generalist deck-pool snapshot to reuse."
    )
    parser.add_argument(
        "--curriculum-pool-out", type=str, default=None,
        help="Path to write the frozen generalist deck-pool snapshot."
    )
    parser.add_argument(
        "--deck1", type=str, default=None,
        help="Path to player 1 deck file (TTS/text format)"
    )
    parser.add_argument(
        "--deck-json", "--deck1-json", dest="deck_json", type=str, default=None,
        help="Path to JSON file containing a flat list of player 1 card IDs"
    )
    parser.add_argument(
        "--deck2", type=str, default=None,
        help="Path to player 2 deck file (TTS/text format)"
    )
    parser.add_argument(
        "--deck2-json", type=str, default=None,
        help="Path to JSON file containing a flat list of player 2 card IDs"
    )
    parser.add_argument(
        "--bounty-threshold", type=float, default=0.15,
        help="Threat index above which bounty bonus applies (default: 0.15)"
    )
    parser.add_argument(
        "--bounty-bonus", type=float, default=0.5,
        help="Bonus reward for beating high-TI opponents (default: 0.5)"
    )
    parser.add_argument(
        "--lstm", action="store_true",
        help="Use LSTM policy (MaskableRecurrentPPO) instead of MLP"
    )
    parser.add_argument(
        "--lstm-hidden-size", type=int, default=None,
        help="LSTM hidden units per layer."
    )
    parser.add_argument(
        "--tensor-profile", type=str, default=None,
        help="Observation tensor profile, e.g. standard_compact_v1 or standard_lite_v2."
    )
    parser.add_argument(
        "--record-games",
        choices=["off", "all", "sampled", "draws", "anomalies", "eval"],
        default=None,
        help="Persist selected training/eval game recording artifacts."
    )
    parser.add_argument(
        "--record-games-dir", type=str, default=None,
        help="Directory for game recording artifacts. Defaults to <run>/recordings."
    )
    parser.add_argument(
        "--record-game-tensors", action="store_true",
        help="Include per-step tensor and action-mask snapshots in recordings."
    )
    parser.add_argument(
        "--record-games-max", type=int, default=None,
        help="Maximum recording artifacts to save for this run."
    )
    parser.add_argument(
        "--record-games-sample-rate", type=float, default=None,
        help="Sample rate for --record-games sampled."
    )
    parser.add_argument(
        "--mulligan-log",
        choices=["on", "off"],
        default="on",
        help="Write per-game starting-hand + mulligan-choice records to "
             "models/<run>/mulligan_log_env_<NNN>.jsonl (one file per env, "
             "default: on, ~3 MB per 1M steps across all env files).",
    )
    parser.add_argument(
        "--match-format",
        choices=["bo3", "single"],
        default=None,
        help=(
            "Episode shape. `bo3` (default in `TrainingConfig`) makes one Gym "
            "episode = one best-of-three match (up to 3 games), enables the "
            "concede action (93) and SelectPlayOrder selection (94/95), and "
            "defaults `digivolve_shaping=True`. `single` retains the legacy "
            "one-game-per-episode behavior."
        ),
    )
    parser.add_argument(
        "--eval-game-log",
        choices=["on", "off"],
        default="on",
        help="Write per-game eval outcomes (winner, digivolves, archetypes, "
             "recording path) to models/<run>/eval_game_log.jsonl. One row "
             "per eval game. Default: on.",
    )
    return parser


def main():
    parser = _build_argparser()

    args = parser.parse_args()
    if args.gauntlet and (args.deck2 or args.deck2_json):
        parser.error("--deck2/--deck2-json cannot be combined with --gauntlet")
    if args.generalist and args.gauntlet:
        parser.error("--generalist cannot be combined with --gauntlet")
    if args.generalist and (args.deck1 or args.deck_json or args.deck2 or args.deck2_json):
        parser.error("--generalist samples deck1/deck2 and cannot be combined with explicit decks")
    if args.resume and args.init_from:
        parser.error("--resume and --init-from are mutually exclusive")

    overrides = {}
    for kv in args.overrides:
        if "=" not in kv:
            parser.error(f"--set expects key=value, got {kv!r}")
        key, value = kv.split("=", 1)
        overrides[key] = yaml.safe_load(value)

    legacy_overrides = {
        "timesteps": args.timesteps,
        "learning_rate": args.lr,
        "batch_size": args.batch_size,
        "n_steps": args.n_steps,
        "eval_freq": args.eval_freq,
        "eval_episodes": args.eval_episodes,
        "tensorboard_log": args.log_dir,
        "models_dir": args.save_dir,
        "lstm_hidden_size": args.lstm_hidden_size,
        "tensor_profile": args.tensor_profile,
        "curriculum_seed": args.curriculum_seed,
        "eval_seed": args.eval_seed,
        "curriculum_pool": args.curriculum_pool,
        "curriculum_pool_out": args.curriculum_pool_out,
        "init_from": args.init_from,
        "record_games": args.record_games,
        "record_games_dir": args.record_games_dir,
        "record_games_max": args.record_games_max,
        "record_games_sample_rate": args.record_games_sample_rate,
    }
    overrides.update({key: value for key, value in legacy_overrides.items() if value is not None})
    # --mulligan-log always has a value (choices default "on"); always propagate it.
    overrides["mulligan_log"] = args.mulligan_log
    # Same for --eval-game-log.
    overrides["eval_game_log"] = args.eval_game_log
    if args.record_game_tensors:
        overrides["record_game_tensors"] = True
    if args.self_play:
        overrides["opponent"] = "self-play"
    elif args.opponent is not None:
        overrides["opponent"] = args.opponent
    if args.lstm:
        overrides["algorithm"] = "lstm"
    if args.resume:
        overrides["resume_from"] = args.resume
    if args.generalist:
        overrides["generalist"] = True
    if args.match_format is not None:
        overrides["match_format"] = args.match_format

    # BO3-implies-digivolve-on default. Only applies when the user hasn't
    # explicitly set digivolve_shaping (either via --set or via YAML). The
    # check is conservative — we look at the merged overrides and the raw
    # YAML below. The cleanest approach: ALWAYS set digivolve_shaping=True
    # under bo3 unless the user explicitly opted out via --set digivolve_shaping=false.
    effective_match_format = overrides.get("match_format", "bo3")
    if effective_match_format == "bo3" and "digivolve_shaping" not in overrides:
        overrides["digivolve_shaping"] = True

    cfg = TrainingConfig.from_yaml(Path(args.config), overrides=overrides)

    # Load gauntlet if requested
    gauntlet = None
    if args.gauntlet:
        gauntlet = MetaGauntlet(sampling_mode=args.gauntlet_sampling)
        try:
            gauntlet.load()
            print(f"  MetaGauntlet: {gauntlet.archetype_count} archetypes, "
                  f"{gauntlet.deck_count} fully implemented decks loaded "
                  f"({args.gauntlet_sampling} sampling)")
        except FileNotFoundError:
            print("  WARNING: deck_library.json not found. "
                  "Run tools/meta_loader.py --build first.")
            gauntlet = None

    # Load player deck if specified
    deck1 = None
    if args.deck1:
        from digimon_engine import parse_deck
        with open(args.deck1, "r") as f:
            deck1 = parse_deck(f.read())
        print(f"  Player deck: {len(deck1)} cards from {args.deck1}")
    if args.deck_json:
        with open(args.deck_json, "r") as f:
            deck1 = json.load(f)
        print(f"  Player deck: {len(deck1)} cards from {args.deck_json}")

    deck2 = None
    if args.deck2:
        from digimon_engine import parse_deck
        with open(args.deck2, "r") as f:
            deck2 = parse_deck(f.read())
        print(f"  Opponent deck: {len(deck2)} cards from {args.deck2}")
    if args.deck2_json:
        with open(args.deck2_json, "r") as f:
            deck2 = json.load(f)
        print(f"  Opponent deck: {len(deck2)} cards from {args.deck2_json}")

    try:
        implemented_ids = load_implemented_card_ids()
        _validate_explicit_deck(deck1, label="--deck1/--deck-json", implemented_card_ids=implemented_ids)
        _validate_explicit_deck(deck2, label="--deck2/--deck2-json", implemented_card_ids=implemented_ids)
    except UnimplementedDeckError as exc:
        parser.error(str(exc))

    generalist_deck_pool = None
    if cfg.generalist:
        try:
            if cfg.curriculum_pool:
                generalist_deck_pool = GeneralistDeckPool.from_snapshot(
                    cfg.curriculum_pool,
                    implemented_card_ids=load_implemented_card_ids(),
                )
                print(
                    f"  Generalist pool: {generalist_deck_pool.archetype_count} archetypes, "
                    f"{generalist_deck_pool.deck_count} decks from {cfg.curriculum_pool}"
                )
            else:
                generalist_deck_pool = load_generalist_deck_pool()
                print(
                    f"  Generalist pool: {generalist_deck_pool.archetype_count} archetypes, "
                    f"{generalist_deck_pool.deck_count} fully implemented decks loaded"
                )
        except FileNotFoundError:
            print("  WARNING: deck_library.json not found. Run tools/meta_loader.py --build first.")
            generalist_deck_pool = None

    train(
        total_timesteps=cfg.timesteps,
        opponent=cfg.opponent,
        eval_freq=cfg.eval_freq,
        n_eval_episodes=cfg.eval_episodes,
        learning_rate=cfg.learning_rate,
        n_steps=cfg.n_steps,
        batch_size=cfg.batch_size,
        tensorboard_log=cfg.tensorboard_log,
        save_dir=cfg.models_dir,
        gauntlet=gauntlet,
        deck1=deck1,
        deck2=deck2,
        bounty_threshold=args.bounty_threshold,
        bounty_bonus=args.bounty_bonus,
        use_lstm=cfg.algorithm == "lstm",
        lstm_hidden_size=cfg.lstm_hidden_size,
        generalist_deck_pool=generalist_deck_pool,
        curriculum_seed=cfg.curriculum_seed,
        eval_seed=cfg.eval_seed,
        deck_pool_snapshot_path=cfg.curriculum_pool_out,
        cfg=cfg,
        reward_profiles_override_mismatch=args.reward_profiles_override_mismatch,
    )


if __name__ == "__main__":
    main()
