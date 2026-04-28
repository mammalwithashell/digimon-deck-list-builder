"""MaskableRecurrentPPO: PPO with LSTM and action masking.

Combines RecurrentPPO (LSTM hidden state tracking, sequence-aware rollouts)
with MaskablePPO (invalid action masking during collection and training).

collect_rollouts() and train() are copy-modified from RecurrentPPO because
they are monolithic methods with no extension hooks. This is the same pattern
MaskablePPO itself uses (copy-modifying PPO's methods).
"""

from copy import deepcopy
from typing import Any, ClassVar, Optional, TypeVar, Union

import numpy as np
import torch as th
from gymnasium import spaces
from stable_baselines3.common.buffers import RolloutBuffer
from stable_baselines3.common.callbacks import BaseCallback
from stable_baselines3.common.policies import BasePolicy
from stable_baselines3.common.type_aliases import GymEnv, MaybeCallback, Schedule
from stable_baselines3.common.utils import FloatSchedule, explained_variance, obs_as_tensor
from stable_baselines3.common.vec_env import VecEnv

from sb3_contrib.common.maskable.utils import get_action_masks, is_masking_supported
from sb3_contrib.common.recurrent.buffers import RecurrentRolloutBuffer
from sb3_contrib.common.recurrent.policies import RecurrentActorCriticPolicy
from sb3_contrib.common.recurrent.type_aliases import RNNStates
from sb3_contrib.ppo_recurrent import RecurrentPPO

from digimon_gym.agents.maskable_recurrent.buffers import MaskableRecurrentRolloutBuffer
from digimon_gym.agents.maskable_recurrent.policies import (
    MaskableMlpLstmPolicy,
    MaskableRecurrentActorCriticPolicy,
)

SelfMaskableRecurrentPPO = TypeVar(
    "SelfMaskableRecurrentPPO", bound="MaskableRecurrentPPO"
)


class MaskableRecurrentPPO(RecurrentPPO):
    """PPO with LSTM memory and invalid action masking.

    Merges RecurrentPPO's LSTM support with MaskablePPO's action masking.

    :param policy: The policy model to use (MaskableMlpLstmPolicy)
    :param env: The environment to learn from
    :param learning_rate: Learning rate (can be a schedule)
    :param n_steps: Steps per environment per update
    :param batch_size: Minibatch size
    :param n_epochs: PPO epochs per update
    :param gamma: Discount factor
    :param gae_lambda: GAE lambda
    :param clip_range: PPO clipping parameter
    :param clip_range_vf: Value function clipping (None to disable)
    :param normalize_advantage: Whether to normalize advantages
    :param ent_coef: Entropy coefficient
    :param vf_coef: Value function coefficient
    :param max_grad_norm: Max gradient norm for clipping
    :param target_kl: KL divergence limit for early stopping
    :param tensorboard_log: TensorBoard log directory
    :param policy_kwargs: Additional policy kwargs (lstm_hidden_size, etc.)
    :param verbose: Verbosity level
    :param seed: Random seed
    :param device: PyTorch device
    """

    policy_aliases: ClassVar[dict[str, type[BasePolicy]]] = {
        "MaskableMlpLstmPolicy": MaskableMlpLstmPolicy,
        "MlpLstmPolicy": MaskableMlpLstmPolicy,
    }

    def __init__(
        self,
        policy: Union[str, type[MaskableRecurrentActorCriticPolicy]],
        env: Union[GymEnv, str],
        learning_rate: Union[float, Schedule] = 3e-4,
        n_steps: int = 128,
        batch_size: Optional[int] = 128,
        n_epochs: int = 10,
        gamma: float = 0.99,
        gae_lambda: float = 0.95,
        clip_range: Union[float, Schedule] = 0.2,
        clip_range_vf: Union[None, float, Schedule] = None,
        normalize_advantage: bool = True,
        ent_coef: float = 0.0,
        vf_coef: float = 0.5,
        max_grad_norm: float = 0.5,
        target_kl: Optional[float] = None,
        stats_window_size: int = 100,
        tensorboard_log: Optional[str] = None,
        policy_kwargs: Optional[dict[str, Any]] = None,
        verbose: int = 0,
        seed: Optional[int] = None,
        device: Union[th.device, str] = "auto",
        _init_setup_model: bool = True,
    ):
        super().__init__(
            policy,
            env,
            learning_rate=learning_rate,
            n_steps=n_steps,
            batch_size=batch_size,
            n_epochs=n_epochs,
            gamma=gamma,
            gae_lambda=gae_lambda,
            clip_range=clip_range,
            clip_range_vf=clip_range_vf,
            normalize_advantage=normalize_advantage,
            ent_coef=ent_coef,
            vf_coef=vf_coef,
            max_grad_norm=max_grad_norm,
            use_sde=False,
            target_kl=target_kl,
            stats_window_size=stats_window_size,
            tensorboard_log=tensorboard_log,
            policy_kwargs=policy_kwargs,
            verbose=verbose,
            seed=seed,
            device=device,
            _init_setup_model=_init_setup_model,
        )

    def _setup_model(self) -> None:
        self._setup_lr_schedule()
        self.set_random_seed(self.seed)

        self.policy = self.policy_class(
            self.observation_space,
            self.action_space,
            self.lr_schedule,
            use_sde=False,
            **self.policy_kwargs,
        )
        self.policy = self.policy.to(self.device)

        if not isinstance(self.policy, RecurrentActorCriticPolicy):
            raise ValueError("Policy must subclass RecurrentActorCriticPolicy")

        lstm = self.policy.lstm_actor
        single_hidden_state_shape = (
            lstm.num_layers, self.n_envs, lstm.hidden_size
        )
        self._last_lstm_states = RNNStates(
            (
                th.zeros(single_hidden_state_shape, device=self.device),
                th.zeros(single_hidden_state_shape, device=self.device),
            ),
            (
                th.zeros(single_hidden_state_shape, device=self.device),
                th.zeros(single_hidden_state_shape, device=self.device),
            ),
        )

        hidden_state_buffer_shape = (
            self.n_steps, lstm.num_layers, self.n_envs, lstm.hidden_size
        )

        # Use MaskableRecurrentRolloutBuffer instead of RecurrentRolloutBuffer
        self.rollout_buffer = MaskableRecurrentRolloutBuffer(
            self.n_steps,
            self.observation_space,
            self.action_space,
            hidden_state_buffer_shape,
            self.device,
            gamma=self.gamma,
            gae_lambda=self.gae_lambda,
            n_envs=self.n_envs,
        )

        # Initialize schedules for policy/value clipping
        self.clip_range = FloatSchedule(self.clip_range)
        if self.clip_range_vf is not None:
            if isinstance(self.clip_range_vf, (float, int)):
                assert self.clip_range_vf > 0, (
                    "`clip_range_vf` must be positive, "
                    "pass `None` to deactivate vf clipping"
                )
            self.clip_range_vf = FloatSchedule(self.clip_range_vf)

    def collect_rollouts(
        self,
        env: VecEnv,
        callback: BaseCallback,
        rollout_buffer: RolloutBuffer,
        n_rollout_steps: int,
        use_masking: bool = True,
    ) -> bool:
        """Collect rollouts with LSTM state tracking and action masking.

        Copy-modified from RecurrentPPO.collect_rollouts() to add:
        1. Action mask collection via get_action_masks(env)
        2. Passing action_masks to policy.forward() and rollout_buffer.add()
        """
        assert isinstance(rollout_buffer, MaskableRecurrentRolloutBuffer), (
            f"{type(rollout_buffer)} doesn't support recurrent + maskable policy"
        )
        assert self._last_obs is not None, "No previous observation was provided"

        self.policy.set_training_mode(False)

        n_steps = 0
        action_masks = None
        rollout_buffer.reset()

        if use_masking and not is_masking_supported(env):
            raise ValueError(
                "Environment does not support action masking. "
                "Consider using ActionMasker wrapper"
            )

        if self.use_sde:
            self.policy.reset_noise(env.num_envs)

        callback.on_rollout_start()

        lstm_states = deepcopy(self._last_lstm_states)

        while n_steps < n_rollout_steps:
            if (
                self.use_sde
                and self.sde_sample_freq > 0
                and n_steps % self.sde_sample_freq == 0
            ):
                self.policy.reset_noise(env.num_envs)

            with th.no_grad():
                obs_tensor = obs_as_tensor(self._last_obs, self.device)
                episode_starts = th.tensor(
                    self._last_episode_starts,
                    dtype=th.float32,
                    device=self.device,
                )

                # Collect action masks
                if use_masking:
                    action_masks = get_action_masks(env)

                actions, values, log_probs, lstm_states = self.policy.forward(
                    obs_tensor, lstm_states, episode_starts,
                    action_masks=action_masks,
                )

            actions = actions.cpu().numpy()

            clipped_actions = actions
            if isinstance(self.action_space, spaces.Box):
                clipped_actions = np.clip(
                    actions, self.action_space.low, self.action_space.high
                )

            new_obs, rewards, dones, infos = env.step(clipped_actions)

            self.num_timesteps += env.num_envs

            callback.update_locals(locals())
            if not callback.on_step():
                return False

            self._update_info_buffer(infos, dones)
            n_steps += 1

            if isinstance(self.action_space, spaces.Discrete):
                actions = actions.reshape(-1, 1)

            # Handle timeout by bootstrapping with value function
            for idx, done_ in enumerate(dones):
                if (
                    done_
                    and infos[idx].get("terminal_observation") is not None
                    and infos[idx].get("TimeLimit.truncated", False)
                ):
                    terminal_obs = self.policy.obs_to_tensor(
                        infos[idx]["terminal_observation"]
                    )[0]
                    with th.no_grad():
                        terminal_lstm_state = (
                            lstm_states.vf[0][:, idx : idx + 1, :].contiguous(),
                            lstm_states.vf[1][:, idx : idx + 1, :].contiguous(),
                        )
                        episode_starts = th.tensor(
                            [False], dtype=th.float32, device=self.device
                        )
                        terminal_value = self.policy.predict_values(
                            terminal_obs, terminal_lstm_state, episode_starts
                        )[0]
                    rewards[idx] += self.gamma * terminal_value

            rollout_buffer.add(
                self._last_obs,
                actions,
                rewards,
                self._last_episode_starts,
                values,
                log_probs,
                lstm_states=self._last_lstm_states,
                action_masks=action_masks,
            )

            self._last_obs = new_obs
            self._last_episode_starts = dones
            self._last_lstm_states = lstm_states

        with th.no_grad():
            episode_starts = th.tensor(
                dones, dtype=th.float32, device=self.device
            )
            values = self.policy.predict_values(
                obs_as_tensor(new_obs, self.device),
                lstm_states.vf,
                episode_starts,
            )

        rollout_buffer.compute_returns_and_advantage(
            last_values=values, dones=dones
        )

        callback.on_rollout_end()
        return True

    def train(self) -> None:
        """Update policy with action masks passed to evaluate_actions.

        Copy-modified from RecurrentPPO.train() to pass
        rollout_data.action_masks to policy.evaluate_actions().
        """
        self.policy.set_training_mode(True)
        self._update_learning_rate(self.policy.optimizer)
        clip_range = self.clip_range(self._current_progress_remaining)
        if self.clip_range_vf is not None:
            clip_range_vf = self.clip_range_vf(
                self._current_progress_remaining
            )

        entropy_losses = []
        pg_losses, value_losses = [], []
        clip_fractions = []

        continue_training = True

        for epoch in range(self.n_epochs):
            approx_kl_divs = []
            for rollout_data in self.rollout_buffer.get(self.batch_size):
                actions = rollout_data.actions
                if isinstance(self.action_space, spaces.Discrete):
                    actions = rollout_data.actions.long().flatten()

                # Sequence padding mask (True = real data, False = padding)
                mask = rollout_data.mask > 1e-8

                # Pass action_masks to evaluate_actions
                values, log_prob, entropy = self.policy.evaluate_actions(
                    rollout_data.observations,
                    actions,
                    rollout_data.lstm_states,
                    rollout_data.episode_starts,
                    action_masks=rollout_data.action_masks,
                )

                values = values.flatten()
                advantages = rollout_data.advantages
                if self.normalize_advantage:
                    advantages = (
                        (advantages - advantages[mask].mean())
                        / (advantages[mask].std() + 1e-8)
                    )

                ratio = th.exp(log_prob - rollout_data.old_log_prob)

                policy_loss_1 = advantages * ratio
                policy_loss_2 = advantages * th.clamp(
                    ratio, 1 - clip_range, 1 + clip_range
                )
                policy_loss = -th.mean(
                    th.min(policy_loss_1, policy_loss_2)[mask]
                )

                pg_losses.append(policy_loss.item())
                clip_fraction = th.mean(
                    (th.abs(ratio - 1) > clip_range).float()[mask]
                ).item()
                clip_fractions.append(clip_fraction)

                if self.clip_range_vf is None:
                    values_pred = values
                else:
                    values_pred = rollout_data.old_values + th.clamp(
                        values - rollout_data.old_values,
                        -clip_range_vf,
                        clip_range_vf,
                    )

                value_loss = th.mean(
                    ((rollout_data.returns - values_pred) ** 2)[mask]
                )
                value_losses.append(value_loss.item())

                if entropy is None:
                    entropy_loss = -th.mean(-log_prob[mask])
                else:
                    entropy_loss = -th.mean(entropy[mask])

                entropy_losses.append(entropy_loss.item())

                loss = (
                    policy_loss
                    + self.ent_coef * entropy_loss
                    + self.vf_coef * value_loss
                )

                with th.no_grad():
                    log_ratio = log_prob - rollout_data.old_log_prob
                    approx_kl_div = th.mean(
                        ((th.exp(log_ratio) - 1) - log_ratio)[mask]
                    ).cpu().numpy()
                    approx_kl_divs.append(approx_kl_div)

                if (
                    self.target_kl is not None
                    and approx_kl_div > 1.5 * self.target_kl
                ):
                    continue_training = False
                    if self.verbose >= 1:
                        print(
                            f"Early stopping at step {epoch} due to "
                            f"reaching max kl: {approx_kl_div:.2f}"
                        )
                    break

                self.policy.optimizer.zero_grad()
                loss.backward()
                th.nn.utils.clip_grad_norm_(
                    self.policy.parameters(), self.max_grad_norm
                )
                self.policy.optimizer.step()

            if not continue_training:
                break

        self._n_updates += self.n_epochs
        explained_var = explained_variance(
            self.rollout_buffer.values.flatten(),
            self.rollout_buffer.returns.flatten(),
        )

        self.logger.record("train/entropy_loss", np.mean(entropy_losses))
        self.logger.record(
            "train/policy_gradient_loss", np.mean(pg_losses)
        )
        self.logger.record("train/value_loss", np.mean(value_losses))
        self.logger.record("train/approx_kl", np.mean(approx_kl_divs))
        self.logger.record("train/clip_fraction", np.mean(clip_fractions))
        self.logger.record("train/loss", loss.item())
        self.logger.record("train/explained_variance", explained_var)
        if hasattr(self.policy, "log_std"):
            self.logger.record(
                "train/std", th.exp(self.policy.log_std).mean().item()
            )
        self.logger.record(
            "train/n_updates", self._n_updates, exclude="tensorboard"
        )
        self.logger.record("train/clip_range", clip_range)
        if self.clip_range_vf is not None:
            self.logger.record("train/clip_range_vf", clip_range_vf)

    def predict(
        self,
        observation: Union[np.ndarray, dict[str, np.ndarray]],
        state: Optional[tuple[np.ndarray, ...]] = None,
        episode_start: Optional[np.ndarray] = None,
        deterministic: bool = False,
        action_masks: Optional[np.ndarray] = None,
    ) -> tuple[np.ndarray, Optional[tuple[np.ndarray, ...]]]:
        """Get action with LSTM state management and action masking.

        :param observation: Input observation
        :param state: Previous LSTM state (h, c) as numpy, or None
        :param episode_start: Whether this is the start of a new episode
        :param deterministic: Whether to use deterministic actions
        :param action_masks: Boolean mask of valid actions
        :return: (action, new_state)
        """
        return self.policy.predict(
            observation, state, episode_start, deterministic,
            action_masks=action_masks,
        )

    def learn(
        self: SelfMaskableRecurrentPPO,
        total_timesteps: int,
        callback: MaybeCallback = None,
        log_interval: int = 1,
        tb_log_name: str = "MaskableRecurrentPPO",
        reset_num_timesteps: bool = True,
        use_masking: bool = True,
        progress_bar: bool = False,
    ) -> SelfMaskableRecurrentPPO:
        """Learn with custom training loop that supports use_masking.

        Copy-modified from MaskablePPO.learn() to pass use_masking
        to collect_rollouts().
        """
        iteration = 0

        total_timesteps, callback = self._setup_learn(
            total_timesteps,
            callback,
            reset_num_timesteps,
            tb_log_name,
            progress_bar,
        )

        callback.on_training_start(locals(), globals())

        assert self.env is not None

        while self.num_timesteps < total_timesteps:
            continue_training = self.collect_rollouts(
                self.env, callback, self.rollout_buffer,
                self.n_steps, use_masking,
            )

            if not continue_training:
                break

            iteration += 1
            self._update_current_progress_remaining(
                self.num_timesteps, total_timesteps
            )

            if log_interval is not None and iteration % log_interval == 0:
                self.dump_logs(iteration)

            self.train()

        callback.on_training_end()
        return self
