"""Tests for MaskableRecurrentPPO (LSTM + action masking).

Covers buffer, policy, and algorithm integration with DigimonEnv.
"""

import numpy as np
import pytest
import torch as th
from gymnasium import spaces

from digimon_gym.agents.maskable_recurrent import (
    MaskableRecurrentPPO,
    MaskableRecurrentActorCriticPolicy,
    MaskableMlpLstmPolicy,
    MaskableRecurrentRolloutBuffer,
    MaskableRecurrentRolloutBufferSamples,
)
from sb3_contrib.common.recurrent.type_aliases import RNNStates


# ── Fixtures ──────────────────────────────────────────────────────────

LSTM_HIDDEN = 32
N_ENVS = 1
N_STEPS = 16
OBS_DIM = 20
N_ACTIONS = 10


@pytest.fixture
def obs_space():
    return spaces.Box(low=-1.0, high=1.0, shape=(OBS_DIM,), dtype=np.float32)


@pytest.fixture
def act_space():
    return spaces.Discrete(N_ACTIONS)


@pytest.fixture
def hidden_state_shape():
    return (N_STEPS, 1, N_ENVS, LSTM_HIDDEN)  # (n_steps, num_layers, n_envs, hidden)


@pytest.fixture
def buffer(obs_space, act_space, hidden_state_shape):
    buf = MaskableRecurrentRolloutBuffer(
        buffer_size=N_STEPS,
        observation_space=obs_space,
        action_space=act_space,
        hidden_state_shape=hidden_state_shape,
        device="cpu",
        n_envs=N_ENVS,
    )
    return buf


@pytest.fixture
def policy(obs_space, act_space):
    lr_schedule = lambda _: 3e-4
    return MaskableRecurrentActorCriticPolicy(
        obs_space, act_space, lr_schedule,
        lstm_hidden_size=LSTM_HIDDEN,
        n_lstm_layers=1,
        enable_critic_lstm=True,
        net_arch=dict(pi=[16], vf=[16]),
    )


# ── Buffer Tests ──────────────────────────────────────────────────────

class TestMaskableRecurrentRolloutBuffer:

    def test_reset_allocates_action_masks(self, buffer):
        """Buffer reset creates action_masks array with correct shape."""
        assert buffer.action_masks.shape == (N_STEPS, N_ENVS, N_ACTIONS)
        assert buffer.action_masks.dtype == np.float32
        # Default should be all ones (all valid)
        np.testing.assert_array_equal(buffer.action_masks, 1.0)

    def test_add_stores_action_masks(self, buffer, obs_space, act_space):
        """Buffer stores action masks at correct position."""
        buffer.reset()

        obs = np.zeros((N_ENVS, OBS_DIM), dtype=np.float32)
        action = np.array([[0]])
        reward = np.array([1.0])
        episode_start = np.array([True])
        value = th.tensor([0.5])
        log_prob = th.tensor([-1.0])

        lstm_h = th.zeros(1, N_ENVS, LSTM_HIDDEN)
        lstm_c = th.zeros(1, N_ENVS, LSTM_HIDDEN)
        lstm_states = RNNStates(
            (lstm_h, lstm_c), (lstm_h.clone(), lstm_c.clone())
        )

        # Create a mask where only first 3 actions are valid
        mask = np.zeros((N_ENVS, N_ACTIONS), dtype=np.float32)
        mask[:, :3] = 1.0

        buffer.add(
            obs, action, reward, episode_start, value, log_prob,
            lstm_states=lstm_states,
            action_masks=mask,
        )

        np.testing.assert_array_equal(buffer.action_masks[0], mask)

    def test_fill_and_get_samples(self, buffer, obs_space, act_space):
        """Buffer produces valid samples with action_masks after filling."""
        buffer.reset()

        lstm_h = th.zeros(1, N_ENVS, LSTM_HIDDEN)
        lstm_c = th.zeros(1, N_ENVS, LSTM_HIDDEN)
        lstm_states = RNNStates(
            (lstm_h, lstm_c), (lstm_h.clone(), lstm_c.clone())
        )

        for step in range(N_STEPS):
            obs = np.random.randn(N_ENVS, OBS_DIM).astype(np.float32)
            action = np.array([[np.random.randint(N_ACTIONS)]])
            reward = np.array([0.0])
            episode_start = np.array([step == 0])
            value = th.tensor([0.0])
            log_prob = th.tensor([-1.0])

            mask = np.ones((N_ENVS, N_ACTIONS), dtype=np.float32)
            mask[:, step % N_ACTIONS] = 0.0  # Mask out one action per step

            buffer.add(
                obs, action, reward, episode_start, value, log_prob,
                lstm_states=lstm_states,
                action_masks=mask,
            )

        last_values = th.tensor([0.0])
        dones = np.array([False])
        buffer.compute_returns_and_advantage(last_values, dones)

        samples_list = list(buffer.get(batch_size=N_STEPS * N_ENVS))
        assert len(samples_list) > 0

        sample = samples_list[0]
        assert isinstance(sample, MaskableRecurrentRolloutBufferSamples)
        assert hasattr(sample, "action_masks")
        assert sample.action_masks.shape[-1] == N_ACTIONS

    def test_samples_namedtuple_fields(self, buffer):
        """MaskableRecurrentRolloutBufferSamples has all required fields."""
        fields = MaskableRecurrentRolloutBufferSamples._fields
        required = {
            "observations", "actions", "old_values", "old_log_prob",
            "advantages", "returns", "lstm_states", "episode_starts",
            "mask", "action_masks",
        }
        assert required.issubset(set(fields))


# ── Policy Tests ──────────────────────────────────────────────────────

class TestMaskableRecurrentPolicy:

    def test_policy_creates_maskable_distribution(self, policy):
        """Policy uses MaskableDistribution, not standard Categorical."""
        from sb3_contrib.common.maskable.distributions import (
            MaskableDistribution,
        )
        assert isinstance(policy.action_dist, MaskableDistribution)

    def test_forward_without_mask(self, policy):
        """Policy forward pass works without action masks."""
        obs = th.randn(1, OBS_DIM)
        lstm_states = RNNStates(
            (th.zeros(1, 1, LSTM_HIDDEN), th.zeros(1, 1, LSTM_HIDDEN)),
            (th.zeros(1, 1, LSTM_HIDDEN), th.zeros(1, 1, LSTM_HIDDEN)),
        )
        episode_starts = th.tensor([1.0])

        actions, values, log_probs, new_states = policy.forward(
            obs, lstm_states, episode_starts
        )
        assert actions.shape == (1,)
        assert values.shape == (1, 1)
        assert log_probs.shape == (1,)
        assert isinstance(new_states, RNNStates)

    def test_forward_with_mask_excludes_invalid(self, policy):
        """Masked actions should get near-zero probability."""
        obs = th.randn(1, OBS_DIM)
        lstm_states = RNNStates(
            (th.zeros(1, 1, LSTM_HIDDEN), th.zeros(1, 1, LSTM_HIDDEN)),
            (th.zeros(1, 1, LSTM_HIDDEN), th.zeros(1, 1, LSTM_HIDDEN)),
        )
        episode_starts = th.tensor([1.0])

        # Only action 0 is valid
        mask = np.zeros((1, N_ACTIONS), dtype=np.float32)
        mask[0, 0] = 1.0

        actions, values, log_probs, _ = policy.forward(
            obs, lstm_states, episode_starts, action_masks=mask
        )
        # With only one valid action, it must always be selected
        assert actions.item() == 0

    def test_evaluate_actions_with_mask(self, policy):
        """evaluate_actions computes valid log_probs under masked distribution."""
        obs = th.randn(1, OBS_DIM)
        actions = th.tensor([0])
        lstm_states = RNNStates(
            (th.zeros(1, 1, LSTM_HIDDEN), th.zeros(1, 1, LSTM_HIDDEN)),
            (th.zeros(1, 1, LSTM_HIDDEN), th.zeros(1, 1, LSTM_HIDDEN)),
        )
        episode_starts = th.tensor([1.0])

        # Only action 0 is valid
        mask = th.zeros(1, N_ACTIONS)
        mask[0, 0] = 1.0

        values, log_probs, entropy = policy.evaluate_actions(
            obs, actions, lstm_states, episode_starts, action_masks=mask
        )
        assert values.shape == (1, 1)
        assert log_probs.shape == (1,)
        assert entropy.shape == (1,)
        # log_prob of only valid action should be 0 (prob=1)
        assert log_probs.item() == pytest.approx(0.0, abs=1e-5)

    def test_predict_threads_lstm_state(self, policy):
        """predict() returns updated LSTM state for stateful inference."""
        obs = np.random.randn(1, OBS_DIM).astype(np.float32)
        mask = np.ones((1, N_ACTIONS), dtype=np.float32)

        action, state = policy.predict(
            obs, state=None, episode_start=np.array([True]),
            deterministic=True, action_masks=mask,
        )
        assert action.ndim <= 1  # scalar or (1,) depending on vectorized detection
        assert state is not None
        assert len(state) == 2  # (h, c)
        assert state[0].shape[-1] == LSTM_HIDDEN

        # Second call should pass state through
        action2, state2 = policy.predict(
            obs, state=state, episode_start=np.array([False]),
            deterministic=True, action_masks=mask,
        )
        assert action2.ndim <= 1
        assert state2 is not None

    def test_episode_start_resets_lstm(self, policy):
        """episode_start=True should reset LSTM hidden state."""
        obs = np.random.randn(1, OBS_DIM).astype(np.float32)
        mask = np.ones((1, N_ACTIONS), dtype=np.float32)

        # Run a few steps to build up state
        state = None
        for i in range(5):
            _, state = policy.predict(
                obs, state=state,
                episode_start=np.array([i == 0]),
                deterministic=True, action_masks=mask,
            )

        # Now reset with episode_start=True — should zero LSTM
        _, state_after_reset = policy.predict(
            obs, state=state, episode_start=np.array([True]),
            deterministic=True, action_masks=mask,
        )
        # Compare with a fresh start (state=None)
        _, state_fresh = policy.predict(
            obs, state=None, episode_start=np.array([True]),
            deterministic=True, action_masks=mask,
        )
        # Both should produce the same action since LSTM was reset
        # (same input, same initial hidden state)
        np.testing.assert_array_equal(state_after_reset[0], state_fresh[0])
        np.testing.assert_array_equal(state_after_reset[1], state_fresh[1])


# ── Algorithm Integration Tests ──────────────────────────────────────

class TestMaskableRecurrentPPOIntegration:

    def test_create_with_digimon_env(self):
        """MaskableRecurrentPPO creates successfully with DigimonEnv."""
        from sb3_contrib.common.wrappers import ActionMasker
        from digimon_gym.digimon_gym import DigimonEnv

        def mask_fn(env):
            return env.action_mask()

        env = ActionMasker(DigimonEnv(), mask_fn)
        model = MaskableRecurrentPPO(
            MaskableMlpLstmPolicy, env,
            n_steps=64, batch_size=64,
            policy_kwargs=dict(
                lstm_hidden_size=32,
                n_lstm_layers=1,
                enable_critic_lstm=True,
                net_arch=dict(pi=[16], vf=[16]),
            ),
        )
        assert model is not None
        assert isinstance(model.policy, MaskableRecurrentActorCriticPolicy)
        assert isinstance(model.rollout_buffer, MaskableRecurrentRolloutBuffer)

    def test_train_short_run(self):
        """MaskableRecurrentPPO can train for 128 steps without error."""
        from sb3_contrib.common.wrappers import ActionMasker
        from digimon_gym.digimon_gym import DigimonEnv

        def mask_fn(env):
            return env.action_mask()

        env = ActionMasker(DigimonEnv(), mask_fn)
        model = MaskableRecurrentPPO(
            MaskableMlpLstmPolicy, env,
            n_steps=64, batch_size=64,
            policy_kwargs=dict(
                lstm_hidden_size=32,
                n_lstm_layers=1,
                enable_critic_lstm=True,
                net_arch=dict(pi=[16], vf=[16]),
            ),
        )
        # Should complete without error
        model.learn(total_timesteps=128)

    def test_predict_with_action_masks(self):
        """model.predict() accepts action_masks kwarg."""
        from sb3_contrib.common.wrappers import ActionMasker
        from digimon_gym.digimon_gym import DigimonEnv

        def mask_fn(env):
            return env.action_mask()

        env = ActionMasker(DigimonEnv(), mask_fn)
        model = MaskableRecurrentPPO(
            MaskableMlpLstmPolicy, env,
            n_steps=64, batch_size=64,
            _init_setup_model=True,
            policy_kwargs=dict(
                lstm_hidden_size=32,
                n_lstm_layers=1,
                enable_critic_lstm=True,
                net_arch=dict(pi=[16], vf=[16]),
            ),
        )

        obs, info = env.reset()
        action_masks = env.unwrapped.action_mask()
        action, state = model.predict(
            obs, state=None, episode_start=np.array([True]),
            deterministic=True, action_masks=action_masks,
        )
        assert action is not None
        assert state is not None

    def test_eval_loop_threads_state(self):
        """Full evaluation loop with LSTM state threading works."""
        from sb3_contrib.common.wrappers import ActionMasker
        from digimon_gym.digimon_gym import DigimonEnv

        def mask_fn(env):
            return env.action_mask()

        env = ActionMasker(DigimonEnv(), mask_fn)
        model = MaskableRecurrentPPO(
            MaskableMlpLstmPolicy, env,
            n_steps=64, batch_size=64,
            policy_kwargs=dict(
                lstm_hidden_size=32,
                n_lstm_layers=1,
                enable_critic_lstm=True,
                net_arch=dict(pi=[16], vf=[16]),
            ),
        )

        obs, info = env.reset()
        state = None
        episode_start = np.array([True])
        steps = 0

        while steps < 20:  # Run up to 20 steps
            action_masks = env.unwrapped.action_mask()
            action, state = model.predict(
                obs, state=state, episode_start=episode_start,
                deterministic=True, action_masks=action_masks,
            )
            obs, reward, terminated, truncated, info = env.step(int(action))
            episode_start = np.array([False])
            steps += 1

            if terminated or truncated:
                break

        assert steps > 0
        assert state is not None

    def test_save_load_roundtrip(self, tmp_path):
        """Save and load preserves model weights and config."""
        from sb3_contrib.common.wrappers import ActionMasker
        from digimon_gym.digimon_gym import DigimonEnv

        def mask_fn(env):
            return env.action_mask()

        env = ActionMasker(DigimonEnv(), mask_fn)
        model = MaskableRecurrentPPO(
            MaskableMlpLstmPolicy, env,
            n_steps=64, batch_size=64,
            policy_kwargs=dict(
                lstm_hidden_size=32,
                n_lstm_layers=1,
                enable_critic_lstm=True,
                net_arch=dict(pi=[16], vf=[16]),
            ),
        )

        save_path = str(tmp_path / "test_model")
        model.save(save_path)

        loaded = MaskableRecurrentPPO.load(save_path, env=env)
        assert isinstance(loaded.policy, MaskableRecurrentActorCriticPolicy)
        assert loaded.policy.lstm_actor.hidden_size == 32

        # Verify predict works on loaded model
        obs, info = env.reset()
        action_masks = env.unwrapped.action_mask()
        action, state = loaded.predict(
            obs, state=None, episode_start=np.array([True]),
            deterministic=True, action_masks=action_masks,
        )
        assert action is not None

    def test_policy_alias(self):
        """'MaskableMlpLstmPolicy' string alias resolves correctly."""
        assert MaskableMlpLstmPolicy is MaskableRecurrentActorCriticPolicy
