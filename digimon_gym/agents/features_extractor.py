"""Custom features extractor for embedding-based observation tensors.

With card embeddings, the observation tensor is ~6,891 floats. Feeding this
directly into an LSTM would create ~7M parameters in the input gate alone.
This extractor projects the flat tensor down to a manageable features_dim
(default 512) before the policy/value heads.
"""

import torch
import torch.nn as nn
from gymnasium import spaces
from stable_baselines3.common.torch_layers import BaseFeaturesExtractor


class CardEmbeddingExtractor(BaseFeaturesExtractor):
    """Project the large embedding-based tensor down for efficient policy learning."""

    def __init__(self, observation_space: spaces.Box, features_dim: int = 512):
        super().__init__(observation_space, features_dim)
        self.net = nn.Sequential(
            nn.Linear(observation_space.shape[0], features_dim),
            nn.ReLU(),
        )

    def forward(self, observations: torch.Tensor) -> torch.Tensor:
        return self.net(observations)
