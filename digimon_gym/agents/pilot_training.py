"""Pilot Agent training with MaskablePPO and action masking.

Trains a PPO-based pilot agent to play Digimon TCG matches.
The agent controls Player 1 while an opponent policy (greedy by default)
controls Player 2. Action masking prevents illegal moves.

Usage:
    python -m digimon_gym.agents.pilot_training
    python -m digimon_gym.agents.pilot_training --timesteps 500000 --opponent random
    python -m digimon_gym.agents.pilot_training --self-play --timesteps 1000000

Requires: pip install stable-baselines3 sb3-contrib tensorboard
"""

import os
import argparse
import time
from datetime import datetime
from typing import Callable, Optional, List

import numpy as np
import gymnasium

from sb3_contrib import MaskablePPO
from sb3_contrib.common.wrappers import ActionMasker
from stable_baselines3.common.callbacks import BaseCallback

from digimon_gym.digimon_gym import DigimonEnv, greedy_policy, ACTION_PASS_TURN
from digimon_gym.engine.game import ACTION_SPACE_SIZE


# ─── Opponent Policies ──────────────────────────────────────────────

def random_policy(env: DigimonEnv) -> int:
    """Select a random valid action."""
    mask = env.action_mask()
    valid = np.where(mask > 0)[0]
    if len(valid) == 0:
        return ACTION_PASS_TURN
    return int(np.random.choice(valid))


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
        obs, info = self.env.reset(**kwargs)
        # If Player 2 goes first, auto-play until Player 1's turn
        obs, info = self._advance_opponent(obs, info)
        return obs, info

    def step(self, action):
        obs, reward, terminated, truncated, info = self.env.step(int(action))

        if terminated or truncated:
            return obs, reward, terminated, truncated, info

        # Auto-play opponent turns, accumulating reward
        obs, info, opp_reward, terminated, truncated = self._play_opponent(
            obs, info
        )
        # Opponent reward is from Player 1's perspective (same as env reward)
        reward += opp_reward

        return obs, reward, terminated, truncated, info

    def _advance_opponent(self, obs, info):
        """Play opponent turns after reset until Player 1 acts."""
        game = self._unwrapped_env.game
        if game is None or game.game_over:
            return obs, info

        while game.current_player_id != 1 and not game.game_over:
            opp_action = self.opponent_fn(self._unwrapped_env)
            obs, _, terminated, truncated, info = self.env.step(int(opp_action))
            if terminated or truncated:
                break

        return obs, info

    def _play_opponent(self, obs, info):
        """Auto-play Player 2 turns, returning accumulated reward."""
        game = self._unwrapped_env.game
        total_opp_reward = 0.0

        while (game is not None
               and not game.game_over
               and game.current_player_id != 1):
            opp_action = self.opponent_fn(self._unwrapped_env)
            obs, reward, terminated, truncated, info = self.env.step(
                int(opp_action)
            )
            total_opp_reward += reward
            if terminated or truncated:
                return obs, info, total_opp_reward, terminated, truncated

        terminated = game.game_over if game else True
        truncated = False
        return obs, info, total_opp_reward, terminated, truncated


# ─── Callbacks ───────────────────────────────────────────────────────

class WinRateCallback(BaseCallback):
    """Tracks win rate and episode statistics via TensorBoard.

    Runs evaluation episodes periodically and logs:
    - pilot/win_rate: fraction of evaluation games won by Player 1
    - pilot/mean_reward: average episode reward
    - pilot/mean_episode_length: average steps per episode
    - pilot/games_played: total training episodes so far
    """

    def __init__(self, eval_env_fn: Callable,
                 eval_freq: int = 10000,
                 n_eval_episodes: int = 20,
                 verbose: int = 1):
        super().__init__(verbose)
        self._eval_env_fn = eval_env_fn
        self._eval_env: Optional[gymnasium.Env] = None
        self.eval_freq = eval_freq
        self.n_eval_episodes = n_eval_episodes
        self.games_played = 0
        self._last_eval_step = 0
        self._episode_rewards: List[float] = []
        self._episode_lengths: List[int] = []

    def _on_step(self) -> bool:
        # Track episode completions from training
        infos = self.locals.get("infos", [])
        for info in infos:
            if "episode" in info:
                self.games_played += 1
                self._episode_rewards.append(info["episode"]["r"])
                self._episode_lengths.append(info["episode"]["l"])

        # Periodic evaluation (only once per eval_freq interval)
        if self.num_timesteps - self._last_eval_step >= self.eval_freq:
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
        wins = 0
        draws = 0
        total_reward = 0.0
        total_steps = 0

        for _ in range(self.n_eval_episodes):
            obs, info = eval_env.reset()
            episode_reward = 0.0
            steps = 0
            done = False

            while not done:
                mask = info["action_mask"]
                action, _ = self.model.predict(
                    obs, deterministic=True,
                    action_masks=mask
                )
                obs, reward, terminated, truncated, info = eval_env.step(
                    int(action)
                )
                episode_reward += reward
                steps += 1
                done = terminated or truncated

            total_reward += episode_reward
            total_steps += steps

            # Use the actual game outcome instead of reward as a proxy
            base_env = eval_env.unwrapped
            game = getattr(base_env, 'game', None)
            if game is not None and game.winner is not None:
                if game.winner.player_id == 1:
                    wins += 1
            else:
                draws += 1

        win_rate = wins / self.n_eval_episodes
        mean_reward = total_reward / self.n_eval_episodes
        mean_length = total_steps / self.n_eval_episodes

        draw_rate = draws / self.n_eval_episodes

        self.logger.record("pilot/win_rate", win_rate)
        self.logger.record("pilot/draw_rate", draw_rate)
        self.logger.record("pilot/mean_eval_reward", mean_reward)
        self.logger.record("pilot/mean_eval_episode_length", mean_length)
        self.logger.record("pilot/games_played", self.games_played)

        if self.verbose:
            print(
                f"  [Eval @ {self.num_timesteps} steps] "
                f"Win rate: {win_rate:.1%} | "
                f"Mean reward: {mean_reward:.3f} | "
                f"Games played: {self.games_played}"
            )


# ─── Training ────────────────────────────────────────────────────────

def make_env(opponent: str = "greedy",
             deck1: Optional[List[str]] = None,
             deck2: Optional[List[str]] = None) -> gymnasium.Env:
    """Create a wrapped DigimonEnv for single-agent RL training.

    Args:
        opponent: Opponent policy name ("greedy", "random", or "self-play").
                  "self-play" skips the opponent wrapper (agent plays both sides).
        deck1: Player 1 deck (card IDs). Defaults to ST1 starter.
        deck2: Player 2 deck (card IDs). Defaults to ST1 starter.

    Returns:
        ActionMasker-wrapped environment ready for MaskablePPO.
    """
    base_env = DigimonEnv(deck1=deck1, deck2=deck2)

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

    def mask_fn(env):
        # Unwrap to reach DigimonEnv for the action mask
        unwrapped = env
        while not isinstance(unwrapped, DigimonEnv):
            if hasattr(unwrapped, 'env'):
                unwrapped = unwrapped.env
            else:
                raise RuntimeError(
                    f"Could not find DigimonEnv in wrapper stack. "
                    f"Innermost layer is {type(unwrapped).__name__}."
                )
        return unwrapped.action_mask()

    return ActionMasker(env, mask_fn)


def save_model(model: MaskablePPO, models_dir: str = "models") -> str:
    """Save trained model with timestamp.

    Returns:
        Path to the saved model file.
    """
    os.makedirs(models_dir, exist_ok=True)
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    path = os.path.join(models_dir, f"pilot_ppo_{timestamp}")
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
          save_dir: str = "models") -> MaskablePPO:
    """Train a Pilot Agent using MaskablePPO.

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

    Returns:
        Trained MaskablePPO model.
    """
    if verbose:
        print("=" * 60)
        print("Digimon TCG Pilot Agent Training")
        print("=" * 60)
        print(f"  Algorithm:      MaskablePPO")
        print(f"  Opponent:       {opponent}")
        print(f"  Total steps:    {total_timesteps:,}")
        print(f"  Learning rate:  {learning_rate}")
        print(f"  Batch size:     {batch_size}")
        print(f"  Rollout steps:  {n_steps}")
        print(f"  Eval freq:      every {eval_freq:,} steps")
        print(f"  TensorBoard:    {tensorboard_log}")
        print("=" * 60)

    # Create training environment
    env = make_env(opponent=opponent)

    # Create model
    model = MaskablePPO(
        "MlpPolicy",
        env,
        learning_rate=learning_rate,
        n_steps=n_steps,
        batch_size=batch_size,
        n_epochs=n_epochs,
        gamma=gamma,
        tensorboard_log=tensorboard_log,
        verbose=0,
    )

    # Create evaluation callback
    eval_env_fn = lambda: make_env(opponent=opponent)
    win_rate_cb = WinRateCallback(
        eval_env_fn=eval_env_fn,
        eval_freq=eval_freq,
        n_eval_episodes=n_eval_episodes,
        verbose=verbose,
    )

    # Train
    start = time.time()
    try:
        model.learn(
            total_timesteps=total_timesteps,
            callback=win_rate_cb,
        )
    finally:
        win_rate_cb.close()
    elapsed = time.time() - start

    if verbose:
        print()
        print(f"Training complete in {elapsed:.1f}s")
        print(f"  Steps/sec: {total_timesteps / elapsed:,.0f}")

    # Save model
    model_path = save_model(model, save_dir)
    if verbose:
        print(f"  Model saved to: {model_path}")

    return model


# ─── CLI ─────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="Train Digimon TCG Pilot Agent (MaskablePPO)"
    )
    parser.add_argument(
        "--timesteps", type=int, default=100_000,
        help="Total training timesteps (default: 100000)"
    )
    opponent_group = parser.add_mutually_exclusive_group()
    opponent_group.add_argument(
        "--opponent", choices=["greedy", "random"],
        default="greedy",
        help="Opponent policy (default: greedy)"
    )
    opponent_group.add_argument(
        "--self-play", action="store_true",
        help="Enable self-play (agent plays both sides)"
    )
    parser.add_argument(
        "--lr", type=float, default=3e-4,
        help="Learning rate (default: 3e-4)"
    )
    parser.add_argument(
        "--batch-size", type=int, default=64,
        help="Minibatch size (default: 64)"
    )
    parser.add_argument(
        "--n-steps", type=int, default=2048,
        help="Rollout buffer size (default: 2048)"
    )
    parser.add_argument(
        "--eval-freq", type=int, default=10_000,
        help="Steps between evaluations (default: 10000)"
    )
    parser.add_argument(
        "--eval-episodes", type=int, default=20,
        help="Games per evaluation (default: 20)"
    )
    parser.add_argument(
        "--log-dir", type=str, default="runs/pilot_ppo",
        help="TensorBoard log directory (default: runs/pilot_ppo)"
    )
    parser.add_argument(
        "--save-dir", type=str, default="models",
        help="Model save directory (default: models)"
    )

    args = parser.parse_args()

    opponent = "self-play" if args.self_play else args.opponent

    train(
        total_timesteps=args.timesteps,
        opponent=opponent,
        eval_freq=args.eval_freq,
        n_eval_episodes=args.eval_episodes,
        learning_rate=args.lr,
        n_steps=args.n_steps,
        batch_size=args.batch_size,
        tensorboard_log=args.log_dir,
        save_dir=args.save_dir,
    )


if __name__ == "__main__":
    main()
