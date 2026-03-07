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
import time
from datetime import datetime
from typing import Callable, Dict, Optional, List, Union

import numpy as np
import gymnasium

from sb3_contrib import MaskablePPO
from sb3_contrib.common.wrappers import ActionMasker
from stable_baselines3.common.callbacks import BaseCallback

from digimon_gym.digimon_gym import DigimonEnv, greedy_policy, ACTION_PASS_TURN
from digimon_gym.engine.game import ACTION_SPACE_SIZE
from digimon_gym.agents.gauntlet import MetaGauntlet, GauntletWrapper
from digimon_gym.agents.features_extractor import CardEmbeddingExtractor
from digimon_gym.agents.maskable_recurrent import (
    MaskableRecurrentPPO,
    MaskableMlpLstmPolicy,
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


# ─── Opponent Policies ──────────────────────────────────────────────

def random_policy(env: DigimonEnv) -> int:
    """Select a random valid action."""
    mask = env.action_mask()
    valid = np.where(mask > 0)[0]
    if len(valid) == 0:
        return ACTION_PASS_TURN
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
        """Auto-play Player 2 turns until Player 1 acts or game ends.

        Returns only terminal reward (win/loss) from the opponent
        sequence. Dense shaping rewards from individual opponent steps
        are discarded — they reflect board changes the agent cannot
        influence and would add noise to the learning signal.
        """
        game = self._unwrapped_env.game
        terminal_reward = 0.0

        while (game is not None
               and not game.game_over
               and game.current_player_id != 1):
            opp_action = self.opponent_fn(self._unwrapped_env)
            obs, reward, terminated, truncated, info = self.env.step(
                int(opp_action)
            )
            if terminated or truncated:
                # Only the terminal reward (±1.0 win/loss) is meaningful
                terminal_reward = reward
                return obs, info, terminal_reward, terminated, truncated

        terminated = game.game_over if game else True
        truncated = False
        return obs, info, terminal_reward, terminated, truncated


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
                 verbose: int = 1):
        super().__init__(verbose)
        self._eval_env_fn = eval_env_fn
        self._eval_env: Optional[gymnasium.Env] = None
        self.eval_freq = eval_freq
        self.n_eval_episodes = n_eval_episodes
        self.games_played = 0
        self._last_eval_step = 0
        self.last_win_rate: float = 0.0
        self.last_mean_reward: float = 0.0
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

    def _on_step(self) -> bool:
        # Track episode completions from training
        infos = self.locals.get("infos", [])
        for info in infos:
            if "episode" in info:
                self.games_played += 1

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
        if eval_env is None:
            if self.verbose:
                print("  [Eval] Failed to create evaluation environment.")
            return
        wins = 0
        draws = 0
        total_reward = 0.0
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

        self.last_win_rate = win_rate
        self.last_mean_reward = mean_reward

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
             deck_pool_hybrid_max: int = 10) -> gymnasium.Env:
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

    Returns:
        ActionMasker-wrapped environment ready for MaskablePPO.

    Wrapper chain:
        DigimonEnv -> OpponentWrapper -> DeckPoolWrapper -> GauntletWrapper -> ActionMasker
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

    # Gauntlet wrapper for meta-weighted opponent sampling
    if gauntlet is not None and gauntlet.deck_count > 0:
        player_deck = deck1 if deck1 else base_env._deck1
        env = GauntletWrapper(
            env, gauntlet,
            player_deck=player_deck,
            bounty_threshold=bounty_threshold,
            bounty_bonus=bounty_bonus,
        )

    def mask_fn(env):
        return _unwrap_to_digimon_env(env).action_mask()

    return ActionMasker(env, mask_fn)


def save_model(model: Union[MaskablePPO, MaskableRecurrentPPO],
               models_dir: str = "models",
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

    Returns:
        Trained model (MaskablePPO or MaskableRecurrentPPO).
    """
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
        if gauntlet and gauntlet.deck_count > 0:
            print(f"  Gauntlet:       {gauntlet.archetype_count} archetypes, "
                  f"{gauntlet.deck_count} decks")
            print(f"  Bounty:         +{bounty_bonus} if TI > {bounty_threshold}")
        print("=" * 60)

    # Create training environment
    env = make_env(
        opponent=opponent,
        deck1=deck1,
        gauntlet=gauntlet,
        bounty_threshold=bounty_threshold,
        bounty_bonus=bounty_bonus,
        deck_pool_variants=deck_pool_variants,
        deck_pool_mode=deck_pool_mode,
        deck_pool_seed=deck_pool_seed,
        deck_pool_hybrid_max=deck_pool_hybrid_max,
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
        ),
    )

    if use_lstm:
        model = MaskableRecurrentPPO(
            MaskableMlpLstmPolicy,
            env,
            learning_rate=learning_rate,
            n_steps=n_steps,
            batch_size=n_steps,  # RecurrentPPO requires batch_size == n_steps
            n_epochs=n_epochs,
            gamma=gamma,
            tensorboard_log=tensorboard_log,
            verbose=0,
            device=device,
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
            tensorboard_log=tensorboard_log,
            verbose=0,
            device=device,
            policy_kwargs=extractor_kwargs,
        )

    # Create evaluation callback
    eval_env_fn = lambda: make_env(
        opponent=opponent, deck1=deck1,
        gauntlet=gauntlet, bounty_threshold=bounty_threshold,
        bounty_bonus=bounty_bonus,
        deck_pool_variants=deck_pool_variants,
        deck_pool_mode=deck_pool_mode,
        deck_pool_seed=deck_pool_seed,
        deck_pool_hybrid_max=deck_pool_hybrid_max,
    )
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
    model_path = save_model(model, save_dir, job_id=job_id)
    if verbose:
        print(f"  Model saved to: {model_path}")

    # Save training run metadata as JSON sidecar
    from pathlib import Path
    from digimon_gym.agents.training_metrics import TrainingRunMetadata

    run_id = Path(model_path).stem
    meta = TrainingRunMetadata(
        run_id=run_id,
        started_at=datetime.fromtimestamp(start).isoformat(),
        finished_at=datetime.now().isoformat(),
        algorithm="LSTM" if use_lstm else "MLP",
        total_timesteps=total_timesteps,
        opponent_type=opponent,
        model_path=model_path,
        tensorboard_log_dir=tensorboard_log,
        final_win_rate=win_rate_cb.last_win_rate,
        final_mean_reward=win_rate_cb.last_mean_reward,
        total_games=win_rate_cb.games_played,
        archetype_results=win_rate_cb.get_archetype_results(),
        hyperparameters={
            "learning_rate": learning_rate,
            "n_steps": n_steps,
            "batch_size": batch_size,
            "n_epochs": n_epochs,
            "gamma": gamma,
        },
    )
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
    parser.add_argument(
        "--gauntlet", action="store_true",
        help="Enable MetaGauntlet opponent sampling from deck_library.json"
    )
    parser.add_argument(
        "--deck1", type=str, default=None,
        help="Path to player 1 deck file (TTS/text format)"
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
        "--lstm-hidden-size", type=int, default=256,
        help="LSTM hidden units per layer (default: 256)"
    )

    args = parser.parse_args()

    opponent = "self-play" if args.self_play else args.opponent

    # Load gauntlet if requested
    gauntlet = None
    if args.gauntlet:
        gauntlet = MetaGauntlet()
        try:
            gauntlet.load()
            print(f"  MetaGauntlet: {gauntlet.archetype_count} archetypes, "
                  f"{gauntlet.deck_count} decks loaded")
        except FileNotFoundError:
            print("  WARNING: deck_library.json not found. "
                  "Run tools/meta_loader.py --build first.")
            gauntlet = None

    # Load player deck if specified
    deck1 = None
    if args.deck1:
        from digimon_gym.engine.data.deck_loader import parse_deck
        with open(args.deck1, "r") as f:
            deck1 = parse_deck(f.read())
        print(f"  Player deck: {len(deck1)} cards from {args.deck1}")

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
        gauntlet=gauntlet,
        deck1=deck1,
        bounty_threshold=args.bounty_threshold,
        bounty_bonus=args.bounty_bonus,
        use_lstm=args.lstm,
        lstm_hidden_size=args.lstm_hidden_size,
    )


if __name__ == "__main__":
    main()
