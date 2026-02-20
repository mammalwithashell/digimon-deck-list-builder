"""Rollout buffer combining LSTM hidden state storage with action mask storage.

Merges RecurrentRolloutBuffer (sequence-aware LSTM state tracking) with
MaskableRolloutBuffer (action mask storage) for use with MaskableRecurrentPPO.
"""

from collections.abc import Generator
from typing import NamedTuple, Optional, Union

import numpy as np
import torch as th
from gymnasium import spaces
from stable_baselines3.common.vec_env import VecNormalize

from sb3_contrib.common.recurrent.buffers import (
    RecurrentRolloutBuffer,
    create_sequencers,
    pad,
    pad_and_flatten,
)
from sb3_contrib.common.recurrent.type_aliases import RNNStates


class MaskableRecurrentRolloutBufferSamples(NamedTuple):
    observations: th.Tensor
    actions: th.Tensor
    old_values: th.Tensor
    old_log_prob: th.Tensor
    advantages: th.Tensor
    returns: th.Tensor
    lstm_states: RNNStates
    episode_starts: th.Tensor
    mask: th.Tensor
    action_masks: th.Tensor


class MaskableRecurrentRolloutBuffer(RecurrentRolloutBuffer):
    """Recurrent rollout buffer that also stores action masks.

    Extends RecurrentRolloutBuffer with action mask storage, combining
    LSTM hidden state tracking (for recurrent policies) with invalid
    action masking (for constrained action spaces).

    :param buffer_size: Max number of elements in the buffer
    :param observation_space: Observation space
    :param action_space: Action space
    :param hidden_state_shape: Shape for LSTM state storage
        (n_steps, lstm.num_layers, n_envs, lstm.hidden_size)
    :param device: PyTorch device
    :param gae_lambda: GAE lambda for advantage estimation
    :param gamma: Discount factor
    :param n_envs: Number of parallel environments
    """

    action_masks: np.ndarray

    def __init__(
        self,
        buffer_size: int,
        observation_space: spaces.Space,
        action_space: spaces.Space,
        hidden_state_shape: tuple[int, int, int, int],
        device: Union[th.device, str] = "auto",
        gae_lambda: float = 1,
        gamma: float = 0.99,
        n_envs: int = 1,
    ):
        self.mask_dims = 0
        super().__init__(
            buffer_size, observation_space, action_space,
            hidden_state_shape, device, gae_lambda, gamma, n_envs,
        )

    def reset(self) -> None:
        if isinstance(self.action_space, spaces.Discrete):
            self.mask_dims = int(self.action_space.n)
        elif isinstance(self.action_space, spaces.MultiDiscrete):
            self.mask_dims = sum(self.action_space.nvec)
        elif isinstance(self.action_space, spaces.MultiBinary):
            assert isinstance(self.action_space.n, int)
            self.mask_dims = 2 * self.action_space.n
        else:
            raise ValueError(f"Unsupported action space {type(self.action_space)}")

        self.action_masks = np.ones(
            (self.buffer_size, self.n_envs, self.mask_dims), dtype=np.float32
        )
        super().reset()

    def add(
        self,
        *args,
        lstm_states: RNNStates,
        action_masks: Optional[np.ndarray] = None,
        **kwargs,
    ) -> None:
        if action_masks is not None:
            self.action_masks[self.pos] = action_masks.reshape(
                self.n_envs, self.mask_dims
            )
        super().add(*args, lstm_states=lstm_states, **kwargs)

    def get(
        self, batch_size: Optional[int] = None
    ) -> Generator[MaskableRecurrentRolloutBufferSamples, None, None]:
        assert self.full, "Rollout buffer must be full before sampling from it"

        # Prepare the data
        if not self.generator_ready:
            # Swap LSTM state axes: (n_steps, num_layers, n_envs, hidden_size)
            # -> (n_steps, n_envs, num_layers, hidden_size)
            for tensor in [
                "hidden_states_pi", "cell_states_pi",
                "hidden_states_vf", "cell_states_vf",
            ]:
                self.__dict__[tensor] = self.__dict__[tensor].swapaxes(1, 2)

            # Flatten: (n_steps, n_envs, *shape) -> (n_envs * n_steps, *shape)
            for tensor in [
                "observations",
                "actions",
                "values",
                "log_probs",
                "advantages",
                "returns",
                "hidden_states_pi",
                "cell_states_pi",
                "hidden_states_vf",
                "cell_states_vf",
                "episode_starts",
                "action_masks",
            ]:
                self.__dict__[tensor] = self.swap_and_flatten(
                    self.__dict__[tensor]
                )
            self.generator_ready = True

        if batch_size is None:
            batch_size = self.buffer_size * self.n_envs

        # Trick to shuffle a bit: keep sequence order but split indices
        split_index = np.random.randint(self.buffer_size * self.n_envs)
        indices = np.arange(self.buffer_size * self.n_envs)
        indices = np.concatenate((indices[split_index:], indices[:split_index]))

        env_change = np.zeros(self.buffer_size * self.n_envs).reshape(
            self.buffer_size, self.n_envs
        )
        env_change[0, :] = 1.0
        env_change = self.swap_and_flatten(env_change)

        start_idx = 0
        while start_idx < self.buffer_size * self.n_envs:
            batch_inds = indices[start_idx : start_idx + batch_size]
            yield self._get_samples(batch_inds, env_change)
            start_idx += batch_size

    def _get_samples(
        self,
        batch_inds: np.ndarray,
        env_change: np.ndarray,
        env: Optional[VecNormalize] = None,
    ) -> MaskableRecurrentRolloutBufferSamples:
        # Retrieve sequence starts and padding utilities
        self.seq_start_indices, self.pad, self.pad_and_flatten = (
            create_sequencers(
                self.episode_starts[batch_inds],
                env_change[batch_inds],
                self.device,
            )
        )

        n_seq = len(self.seq_start_indices)
        max_length = self.pad(self.actions[batch_inds]).shape[1]
        padded_batch_size = n_seq * max_length

        # Retrieve LSTM states at sequence starts
        lstm_states_pi = (
            self.hidden_states_pi[batch_inds][self.seq_start_indices].swapaxes(0, 1),
            self.cell_states_pi[batch_inds][self.seq_start_indices].swapaxes(0, 1),
        )
        lstm_states_vf = (
            self.hidden_states_vf[batch_inds][self.seq_start_indices].swapaxes(0, 1),
            self.cell_states_vf[batch_inds][self.seq_start_indices].swapaxes(0, 1),
        )
        lstm_states_pi = (
            self.to_torch(lstm_states_pi[0]).contiguous(),
            self.to_torch(lstm_states_pi[1]).contiguous(),
        )
        lstm_states_vf = (
            self.to_torch(lstm_states_vf[0]).contiguous(),
            self.to_torch(lstm_states_vf[1]).contiguous(),
        )

        # Pad action masks with 1.0 (all valid) for padding positions.
        # The sequence `mask` field already excludes padded timesteps
        # from the loss, so padding with all-valid is correct.
        action_masks_padded = self.pad(
            self.action_masks[batch_inds], padding_value=1.0
        ).reshape(padded_batch_size, self.mask_dims)

        return MaskableRecurrentRolloutBufferSamples(
            observations=self.pad(self.observations[batch_inds]).reshape(
                (padded_batch_size, *self.obs_shape)
            ),
            actions=self.pad(self.actions[batch_inds]).reshape(
                (padded_batch_size, *self.actions.shape[1:])
            ),
            old_values=self.pad_and_flatten(self.values[batch_inds]),
            old_log_prob=self.pad_and_flatten(self.log_probs[batch_inds]),
            advantages=self.pad_and_flatten(self.advantages[batch_inds]),
            returns=self.pad_and_flatten(self.returns[batch_inds]),
            lstm_states=RNNStates(lstm_states_pi, lstm_states_vf),
            episode_starts=self.pad_and_flatten(self.episode_starts[batch_inds]),
            mask=self.pad_and_flatten(
                np.ones_like(self.returns[batch_inds])
            ),
            action_masks=action_masks_padded,
        )
