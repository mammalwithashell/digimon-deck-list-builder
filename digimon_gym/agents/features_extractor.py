"""Custom features extractor with learned nn.Embedding for card identities.

The observation tensor is compact (1375 floats). Card identity slots hold integer
registry indices. This extractor:
1. Splits the tensor into card-index positions and scalar positions
2. Looks up card indices in a trainable nn.Embedding table
3. Concatenates the resulting embeddings with scalar features
4. Projects down to features_dim (default 512) for the policy/value heads

The embedding table is part of the model checkpoint — no external files needed
at inference time. Optionally warm-started from autoencoder embeddings.
"""

from typing import Optional

import numpy as np
import torch
import torch.nn as nn
from gymnasium import spaces
from stable_baselines3.common.torch_layers import BaseFeaturesExtractor

from engine_py_legacy.engine.data.tensor_layout import (
    CARD_ID_POSITIONS, SCALAR_POSITIONS, NUM_CARD_SLOTS, NUM_SCALAR_SLOTS,
)  # parity-doc: tensor_layout stays on Python engine
from digimon_engine import REGISTRY_CAPACITY, EMBEDDING_DIM


class CardEmbeddingExtractor(BaseFeaturesExtractor):
    """Extract features by embedding card IDs and concatenating with scalars."""

    def __init__(
        self,
        observation_space: spaces.Box,
        features_dim: int = 512,
        vocab_size: int = REGISTRY_CAPACITY,
        embedding_dim: int = EMBEDDING_DIM,
        pretrained_embeddings: Optional[np.ndarray] = None,
    ):
        super().__init__(observation_space, features_dim)

        # Index tensors for splitting observations (registered as buffers so
        # they move to the correct device automatically with .to())
        self.register_buffer(
            'card_id_indices',
            torch.tensor(CARD_ID_POSITIONS, dtype=torch.long),
        )
        self.register_buffer(
            'scalar_indices',
            torch.tensor(SCALAR_POSITIONS, dtype=torch.long),
        )

        # Trainable card embedding table (padding_idx=0 keeps the zero-vector fixed)
        self.embedding = nn.Embedding(vocab_size, embedding_dim, padding_idx=0)

        # Warm-start from autoencoder embeddings if provided
        if pretrained_embeddings is not None:
            n = min(len(pretrained_embeddings), vocab_size)
            with torch.no_grad():
                self.embedding.weight[:n] = torch.from_numpy(
                    pretrained_embeddings[:n].astype(np.float32)
                )

        # Projection: (NUM_SCALAR_SLOTS + NUM_CARD_SLOTS * embedding_dim) -> features_dim
        combined_dim = NUM_SCALAR_SLOTS + NUM_CARD_SLOTS * embedding_dim
        self.projection = nn.Sequential(
            nn.Linear(combined_dim, features_dim),
            nn.ReLU(),
        )

    def forward(self, observations: torch.Tensor) -> torch.Tensor:
        # Split tensor into card indices and scalar features
        card_ids = observations[:, self.card_id_indices].long().clamp(min=0)
        scalars = observations[:, self.scalar_indices]

        # Embed card IDs: (batch, NUM_CARD_SLOTS) -> (batch, NUM_CARD_SLOTS, embedding_dim)
        card_embeds = self.embedding(card_ids)
        # Flatten: (batch, NUM_CARD_SLOTS * embedding_dim)
        card_embeds = card_embeds.flatten(1)

        # Concatenate and project
        combined = torch.cat([scalars, card_embeds], dim=1)
        return self.projection(combined)
