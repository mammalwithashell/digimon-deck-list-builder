"""Recurrent actor-critic policy with action masking support.

Combines RecurrentActorCriticPolicy (LSTM forward pass, hidden state management)
with MaskableDistribution (invalid action masking via logit suppression).
"""

from typing import Any, Optional, Union

import numpy as np
import torch as th
from gymnasium import spaces
from stable_baselines3.common.torch_layers import (
    BaseFeaturesExtractor,
    FlattenExtractor,
)
from stable_baselines3.common.type_aliases import Schedule
from torch import nn

from sb3_contrib.common.maskable.distributions import (
    MaskableDistribution,
    make_masked_proba_distribution,
)
from sb3_contrib.common.recurrent.policies import RecurrentActorCriticPolicy
from sb3_contrib.common.recurrent.type_aliases import RNNStates


class MaskableRecurrentActorCriticPolicy(RecurrentActorCriticPolicy):
    """LSTM policy with action masking for partially observable environments.

    Extends RecurrentActorCriticPolicy to use MaskableDistribution instead
    of standard CategoricalDistribution, enabling invalid action masking
    during both data collection and policy updates.

    :param observation_space: Observation space
    :param action_space: Action space
    :param lr_schedule: Learning rate schedule
    :param net_arch: Policy/value network specification
    :param activation_fn: Activation function
    :param ortho_init: Whether to use orthogonal initialization
    :param lstm_hidden_size: LSTM hidden units per layer
    :param n_lstm_layers: Number of LSTM layers
    :param shared_lstm: Whether actor and critic share the LSTM
    :param enable_critic_lstm: Whether critic has its own LSTM
    :param lstm_kwargs: Additional LSTM constructor kwargs
    """

    def __init__(
        self,
        observation_space: spaces.Space,
        action_space: spaces.Space,
        lr_schedule: Schedule,
        net_arch: Optional[Union[list[int], dict[str, list[int]]]] = None,
        activation_fn: type[nn.Module] = nn.Tanh,
        ortho_init: bool = True,
        use_sde: bool = False,
        log_std_init: float = 0.0,
        full_std: bool = True,
        use_expln: bool = False,
        squash_output: bool = False,
        features_extractor_class: type[BaseFeaturesExtractor] = FlattenExtractor,
        features_extractor_kwargs: Optional[dict[str, Any]] = None,
        share_features_extractor: bool = True,
        normalize_images: bool = True,
        optimizer_class: type[th.optim.Optimizer] = th.optim.Adam,
        optimizer_kwargs: Optional[dict[str, Any]] = None,
        lstm_hidden_size: int = 256,
        n_lstm_layers: int = 1,
        shared_lstm: bool = False,
        enable_critic_lstm: bool = True,
        lstm_kwargs: Optional[dict[str, Any]] = None,
    ):
        super().__init__(
            observation_space,
            action_space,
            lr_schedule,
            net_arch,
            activation_fn,
            ortho_init,
            use_sde,
            log_std_init,
            full_std,
            use_expln,
            squash_output,
            features_extractor_class,
            features_extractor_kwargs,
            share_features_extractor,
            normalize_images,
            optimizer_class,
            optimizer_kwargs,
            lstm_hidden_size,
            n_lstm_layers,
            shared_lstm,
            enable_critic_lstm,
            lstm_kwargs,
        )

        # Replace standard distribution with maskable distribution.
        # Parent's __init__ already created action_dist as CategoricalDistribution.
        # We swap it for MaskableCategoricalDistribution and rebuild action_net.
        self.action_dist = make_masked_proba_distribution(action_space)
        self.action_net = self.action_dist.proba_distribution_net(
            latent_dim=self.mlp_extractor.latent_dim_pi
        )
        self.action_net = self.action_net.to(self.device)

        # Rebuild optimizer to include the new action_net parameters
        if optimizer_kwargs is None:
            optimizer_kwargs = {}
            if optimizer_class == th.optim.Adam:
                optimizer_kwargs["eps"] = 1e-5
        self.optimizer = optimizer_class(
            self.parameters(), lr=lr_schedule(1), **optimizer_kwargs
        )

    def _get_action_dist_from_latent(
        self, latent_pi: th.Tensor
    ) -> MaskableDistribution:
        """Get action distribution from latent actor representation.

        Overrides parent's version which doesn't recognize MaskableDistribution.
        """
        action_logits = self.action_net(latent_pi)
        return self.action_dist.proba_distribution(action_logits=action_logits)

    def forward(
        self,
        obs: th.Tensor,
        lstm_states: RNNStates,
        episode_starts: th.Tensor,
        deterministic: bool = False,
        action_masks: Optional[np.ndarray] = None,
    ) -> tuple[th.Tensor, th.Tensor, th.Tensor, RNNStates]:
        """Forward pass with optional action masking.

        :param obs: Observation tensor
        :param lstm_states: Previous LSTM hidden/cell states
        :param episode_starts: Whether observations start new episodes
        :param deterministic: Whether to use deterministic actions
        :param action_masks: Boolean mask of valid actions
        :return: (actions, values, log_probs, new_lstm_states)
        """
        features = self.extract_features(obs)
        if self.share_features_extractor:
            pi_features = vf_features = features
        else:
            pi_features, vf_features = features

        latent_pi, lstm_states_pi = self._process_sequence(
            pi_features, lstm_states.pi, episode_starts, self.lstm_actor
        )
        if self.lstm_critic is not None:
            latent_vf, lstm_states_vf = self._process_sequence(
                vf_features, lstm_states.vf, episode_starts, self.lstm_critic
            )
        elif self.shared_lstm:
            latent_vf = latent_pi.detach()
            lstm_states_vf = (
                lstm_states_pi[0].detach(),
                lstm_states_pi[1].detach(),
            )
        else:
            latent_vf = self.critic(vf_features)
            lstm_states_vf = lstm_states_pi

        latent_pi = self.mlp_extractor.forward_actor(latent_pi)
        latent_vf = self.mlp_extractor.forward_critic(latent_vf)

        values = self.value_net(latent_vf)
        distribution = self._get_action_dist_from_latent(latent_pi)

        if action_masks is not None:
            distribution.apply_masking(action_masks)

        actions = distribution.get_actions(deterministic=deterministic)
        log_prob = distribution.log_prob(actions)
        return actions, values, log_prob, RNNStates(lstm_states_pi, lstm_states_vf)

    def evaluate_actions(
        self,
        obs: th.Tensor,
        actions: th.Tensor,
        lstm_states: RNNStates,
        episode_starts: th.Tensor,
        action_masks: Optional[th.Tensor] = None,
    ) -> tuple[th.Tensor, th.Tensor, th.Tensor]:
        """Evaluate actions with optional masking.

        :param obs: Observation tensor
        :param actions: Actions to evaluate
        :param lstm_states: LSTM states at sequence starts
        :param episode_starts: Episode boundary flags
        :param action_masks: Boolean mask of valid actions
        :return: (values, log_probs, entropy)
        """
        features = self.extract_features(obs)
        if self.share_features_extractor:
            pi_features = vf_features = features
        else:
            pi_features, vf_features = features

        latent_pi, _ = self._process_sequence(
            pi_features, lstm_states.pi, episode_starts, self.lstm_actor
        )
        if self.lstm_critic is not None:
            latent_vf, _ = self._process_sequence(
                vf_features, lstm_states.vf, episode_starts, self.lstm_critic
            )
        elif self.shared_lstm:
            latent_vf = latent_pi.detach()
        else:
            latent_vf = self.critic(vf_features)

        latent_pi = self.mlp_extractor.forward_actor(latent_pi)
        latent_vf = self.mlp_extractor.forward_critic(latent_vf)

        distribution = self._get_action_dist_from_latent(latent_pi)

        if action_masks is not None:
            distribution.apply_masking(action_masks)

        log_prob = distribution.log_prob(actions)
        values = self.value_net(latent_vf)
        return values, log_prob, distribution.entropy()

    def get_distribution(
        self,
        obs: th.Tensor,
        lstm_states: tuple[th.Tensor, th.Tensor],
        episode_starts: th.Tensor,
        action_masks: Optional[np.ndarray] = None,
    ) -> tuple[MaskableDistribution, tuple[th.Tensor, ...]]:
        """Get the current policy distribution with optional masking.

        :param obs: Observation tensor
        :param lstm_states: Previous actor LSTM states
        :param episode_starts: Episode boundary flags
        :param action_masks: Boolean mask of valid actions
        :return: (distribution, new_lstm_states)
        """
        from stable_baselines3.common.policies import ActorCriticPolicy

        features = super(ActorCriticPolicy, self).extract_features(
            obs, self.pi_features_extractor
        )
        latent_pi, lstm_states = self._process_sequence(
            features, lstm_states, episode_starts, self.lstm_actor
        )
        latent_pi = self.mlp_extractor.forward_actor(latent_pi)
        distribution = self._get_action_dist_from_latent(latent_pi)

        if action_masks is not None:
            distribution.apply_masking(action_masks)

        return distribution, lstm_states

    def _predict(
        self,
        observation: th.Tensor,
        lstm_states: tuple[th.Tensor, th.Tensor],
        episode_starts: th.Tensor,
        deterministic: bool = False,
        action_masks: Optional[np.ndarray] = None,
    ) -> tuple[th.Tensor, tuple[th.Tensor, ...]]:
        """Get action from observation with masking.

        :param observation: Observation tensor
        :param lstm_states: Previous actor LSTM states
        :param episode_starts: Episode boundary flags
        :param deterministic: Whether to use deterministic actions
        :param action_masks: Boolean mask of valid actions
        :return: (actions, new_lstm_states)
        """
        distribution, lstm_states = self.get_distribution(
            observation, lstm_states, episode_starts, action_masks
        )
        return distribution.get_actions(deterministic=deterministic), lstm_states

    def predict(
        self,
        observation: Union[np.ndarray, dict[str, np.ndarray]],
        state: Optional[tuple[np.ndarray, ...]] = None,
        episode_start: Optional[np.ndarray] = None,
        deterministic: bool = False,
        action_masks: Optional[np.ndarray] = None,
    ) -> tuple[np.ndarray, Optional[tuple[np.ndarray, ...]]]:
        """Get action from observation (numpy in/out) with masking and state.

        Handles LSTM state initialization, tensor conversion, and passes
        action_masks through to the distribution.

        :param observation: Input observation (numpy)
        :param state: Previous LSTM state tuple (h, c) as numpy, or None
        :param episode_start: Whether this is the start of a new episode
        :param deterministic: Whether to use deterministic actions
        :param action_masks: Boolean mask of valid actions
        :return: (action, new_state) where state is (h, c) numpy tuple
        """
        self.set_training_mode(False)

        observation, vectorized_env = self.obs_to_tensor(observation)

        if isinstance(observation, dict):
            n_envs = observation[next(iter(observation.keys()))].shape[0]
        else:
            n_envs = observation.shape[0]

        if state is None:
            state = np.concatenate(
                [np.zeros(self.lstm_hidden_state_shape) for _ in range(n_envs)],
                axis=1,
            )
            state = (state, state)

        if episode_start is None:
            episode_start = np.array([False for _ in range(n_envs)])

        with th.no_grad():
            states = (
                th.tensor(state[0], dtype=th.float32, device=self.device),
                th.tensor(state[1], dtype=th.float32, device=self.device),
            )
            episode_starts = th.tensor(
                episode_start, dtype=th.float32, device=self.device
            )
            actions, states = self._predict(
                observation,
                lstm_states=states,
                episode_starts=episode_starts,
                deterministic=deterministic,
                action_masks=action_masks,
            )
            states = (states[0].cpu().numpy(), states[1].cpu().numpy())

        actions = actions.cpu().numpy()

        if isinstance(self.action_space, spaces.Box):
            if self.squash_output:
                actions = self.unscale_action(actions)
            else:
                actions = np.clip(
                    actions, self.action_space.low, self.action_space.high
                )

        if not vectorized_env:
            actions = actions.squeeze(axis=0)

        return actions, states


MaskableMlpLstmPolicy = MaskableRecurrentActorCriticPolicy
