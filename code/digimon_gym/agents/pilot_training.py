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
from datetime import datetime
from pathlib import Path
from typing import Callable, Dict, Optional, List, Set, Union

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
from digimon_gym.agents.opponent_pool import OpponentPool
from digimon_gym.agents.training_config import TrainingConfig
from digimon_gym.agents.training_recording import (
    TrainingGameRecorder,
    TrainingRecordingWrapper,
)


# ─── Helpers ─────────────────────────────────────────────────────────

def _unwrap_to_digimon_env(env: gymnasium.Env) -> DigimonEnv:
    """Walk the wrapper stack until we reach a DigimonEnv.

    Raises RuntimeError if no DigimonEnv is found.
    """
    unwrapped = env
    while not isinstance(unwrapped, DigimonEnv):
        if isinstance(unwrapped, gymnasium.Wrapper):
            unwrapped = unwrapped.env
        else:
            raise RuntimeError(
                f"Could not find DigimonEnv in wrapper stack. "
                f"Innermost layer is {type(unwrapped).__name__}."
            )
    return unwrapped


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

        # Auto-play opponent turns. Only terminal outcomes (win/loss)
        # are attributed to this timestep — intermediate dense shaping
        # from opponent moves reflects board changes the agent can't
        # control, so we exclude those to keep reward signal clean.
        obs, info, terminal_reward, terminated, truncated = (
            self._play_opponent(obs, info)
        )
        reward = float(reward) + float(terminal_reward)

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
        self.last_win_rate: float = 0.0
        self.last_mean_reward: float = 0.0
        self.last_draw_rate: float = 0.0
        self.last_mean_eval_terminal_score: float = 0.0
        self.last_mean_eval_dense_reward: float = 0.0
        self.last_mean_eval_episode_length: float = 0.0
        self._last_eval_suite_results: dict = {}
        # Per-archetype tracking
        self._archetype_wins: Dict[str, int] = {}
        self._archetype_draws: Dict[str, int] = {}
        self._archetype_games: Dict[str, int] = {}

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

        for _ in range(self.n_eval_episodes):
            obs, info = eval_env.reset()
            episode_reward = 0.0
            steps = 0
            done = False
            opponent_archetype = info.get("opponent_archetype")

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
                obs, reward, terminated, truncated, _ = eval_env.step(
                    int(action)
                )
                episode_reward += float(reward)
                steps += 1
                done = terminated or truncated
                episode_start = np.array([False])

            total_reward += episode_reward
            total_steps += steps

            # Use the actual game outcome instead of reward as a proxy
            won = False
            is_draw = False
            winner_id = None
            if eval_env is not None:
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

            # Track per-archetype win rates (when gauntlet is active)
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

        win_rate = wins / self.n_eval_episodes
        mean_reward = total_reward / self.n_eval_episodes
        mean_length = total_steps / self.n_eval_episodes

        draw_rate = draws / self.n_eval_episodes
        mean_terminal_score = total_terminal_score / self.n_eval_episodes
        mean_dense_reward = total_dense_reward / self.n_eval_episodes

        self.last_win_rate = win_rate
        self.last_mean_reward = mean_reward
        self.last_draw_rate = draw_rate
        self.last_mean_eval_terminal_score = mean_terminal_score
        self.last_mean_eval_dense_reward = mean_dense_reward
        self.last_mean_eval_episode_length = mean_length

        self.logger.record("pilot/win_rate", win_rate)
        self.logger.record("pilot/draw_rate", draw_rate)
        self.logger.record("pilot/mean_eval_reward", mean_reward)
        self.logger.record("pilot/mean_eval_terminal_score", mean_terminal_score)
        self.logger.record("pilot/mean_eval_dense_reward", mean_dense_reward)
        self.logger.record("pilot/mean_eval_episode_length", mean_length)
        self.logger.record("pilot/games_played", self.games_played)

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
             recording_env_index: int = 0) -> gymnasium.Env:
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
    )

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

    if record_this_source:
        env = TrainingRecordingWrapper(
            env,
            recording_writer,
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
            )
            wrapped = OpponentWrapper(base_env, opponent_fn=opponent_fn)
            if generalist_deck_pool is not None:
                seed = None if curriculum_seed is None else curriculum_seed + rank
                wrapped = GeneralistDeckPoolWrapper(
                    wrapped,
                    deck_pool=generalist_deck_pool,
                    seed=seed,
                )
            if record_this_source:
                wrapped = TrainingRecordingWrapper(
                    wrapped,
                    recording_writer,
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
            )
            wrapped = OpponentWrapper(base_env, opponent_fn=pool_opponent_fn)
            if generalist_deck_pool is not None:
                wrapped = GeneralistDeckPoolWrapper(
                    wrapped,
                    deck_pool=generalist_deck_pool,
                    seed=eval_seed if eval_seed is not None else curriculum_seed,
                )
            if recording_writer.enabled:
                wrapped = TrainingRecordingWrapper(
                    wrapped,
                    recording_writer,
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
        )
    eval_suite = None
    if cfg.eval_suite:
        from digimon_gym.agents.eval_suite import HeldOutEvalSuite

        eval_suite = HeldOutEvalSuite.from_yaml(
            Path(cfg.eval_suite),
            tensor_profile=cfg.tensor_profile,
        )
    win_rate_cb = WinRateCallback(
        eval_env_fn=eval_env_fn,
        eval_freq=eval_freq,
        n_eval_episodes=n_eval_episodes,
        eval_suite=eval_suite,
        eval_suite_path=cfg.eval_suite,
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

def main():
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
    )


if __name__ == "__main__":
    main()
