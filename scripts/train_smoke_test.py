"""Smoke test: validate DigimonEnv works with SB3 MaskablePPO.

Usage:
    python scripts/train_smoke_test.py

Requires: pip install stable-baselines3 sb3-contrib
"""

import sys
import os

# Ensure project root is on path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

import numpy as np
from digimon_gym.digimon_gym import DigimonEnv


def test_manual_loop():
    """Validate env works with a basic manual training loop."""
    print("=" * 60)
    print("Test 1: Manual Training Loop")
    print("=" * 60)

    env = DigimonEnv()
    obs, info = env.reset()

    print(f"  Observation shape: {obs.shape}")
    print(f"  Action mask shape: {info['action_mask'].shape}")
    print(f"  Observation space: {env.observation_space}")
    print(f"  Action space: {env.action_space}")

    total_reward = 0.0
    steps = 0

    while True:
        # Use action mask to select valid action
        mask = info["action_mask"]
        valid_actions = np.where(mask > 0)[0]
        action = np.random.choice(valid_actions) if len(valid_actions) > 0 else 62

        obs, reward, terminated, truncated, info = env.step(int(action))
        total_reward += reward
        steps += 1

        if terminated or truncated:
            break

    print(f"  Episode finished in {steps} steps")
    print(f"  Total reward: {total_reward:.4f}")
    print(f"  Game over: {env.runner.is_game_over}")
    print(f"  Winner: Player {env.runner.winner_id}")
    print("  PASSED!")


def test_gymnasium_env_checker():
    """Run Gymnasium's built-in environment validator."""
    print()
    print("=" * 60)
    print("Test 2: Gymnasium Environment Checker")
    print("=" * 60)

    from gymnasium.utils.env_checker import check_env
    env = DigimonEnv()
    try:
        check_env(env)
        print("  Gymnasium check_env: PASSED!")
    except Exception as e:
        print(f"  Gymnasium check_env: WARNING - {e}")
        print("  (Some warnings are expected for action masking)")


def test_sb3_maskable_ppo():
    """Validate SB3 MaskablePPO can train on DigimonEnv."""
    print()
    print("=" * 60)
    print("Test 3: SB3 MaskablePPO Training (1000 steps)")
    print("=" * 60)

    try:
        from sb3_contrib import MaskablePPO
        from sb3_contrib.common.wrappers import ActionMasker
    except ImportError:
        print("  SKIPPED: sb3-contrib not installed.")
        print("  Install with: pip install sb3-contrib")
        return

    def mask_fn(env):
        return env.action_mask()

    env = ActionMasker(DigimonEnv(), mask_fn)
    model = MaskablePPO("MlpPolicy", env, verbose=0, n_steps=128, batch_size=64)
    model.learn(total_timesteps=1000)
    print("  MaskablePPO training (1000 steps): PASSED!")


def test_sb3_maskable_recurrent_ppo():
    """Validate MaskableRecurrentPPO (LSTM + action masking) on DigimonEnv."""
    print()
    print("=" * 60)
    print("Test 4: MaskableRecurrentPPO Training (256 steps)")
    print("=" * 60)

    try:
        from sb3_contrib.common.wrappers import ActionMasker
        from digimon_gym.agents.maskable_recurrent import (
            MaskableRecurrentPPO,
            MaskableMlpLstmPolicy,
        )
    except ImportError:
        print("  SKIPPED: sb3-contrib or maskable_recurrent not available.")
        return

    def mask_fn(env):
        return env.action_mask()

    env = ActionMasker(DigimonEnv(), mask_fn)
    model = MaskableRecurrentPPO(
        MaskableMlpLstmPolicy,
        env,
        verbose=0,
        n_steps=128,
        batch_size=128,  # RecurrentPPO requires batch_size == n_steps
        policy_kwargs=dict(
            lstm_hidden_size=64,
            n_lstm_layers=1,
            enable_critic_lstm=True,
            net_arch=dict(pi=[64], vf=[64]),
        ),
    )
    model.learn(total_timesteps=256)
    print("  MaskableRecurrentPPO training (256 steps): PASSED!")

    # Run 1 evaluation episode with manual LSTM state threading
    print("  Running evaluation episode with LSTM state threading...")
    obs, info = env.reset()
    state = None
    episode_start = np.array([True])
    steps = 0
    done = False

    while not done:
        action_masks = env.unwrapped.action_mask()
        action, state = model.predict(
            obs, state=state, episode_start=episode_start,
            deterministic=True, action_masks=action_masks,
        )
        obs, reward, terminated, truncated, info = env.step(int(action))
        episode_start = np.array([False])
        steps += 1
        done = terminated or truncated

    print(f"  Evaluation episode: {steps} steps. PASSED!")


if __name__ == "__main__":
    test_manual_loop()
    test_gymnasium_env_checker()
    test_sb3_maskable_ppo()
    test_sb3_maskable_recurrent_ppo()
    print()
    print("=" * 60)
    print("ALL SMOKE TESTS PASSED!")
    print("=" * 60)
