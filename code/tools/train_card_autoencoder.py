"""Train a card autoencoder and generate frozen embedding lookup table.

Usage:
    python -m tools.train_card_autoencoder [--embedding-dim 16] [--epochs 500] [--lr 1e-3]

Outputs (under <repo_root>/models/):
    card_encoder.pt     - Encoder weights (for re-encoding new cards)
    card_embeddings.npy - Precomputed embedding table (vocab_size+1, dim)

The embeddings are an optional warm-start consumed by pilot_training. The old
digimon_gym/engine/data/ output path no longer exists (Rust pivot), so the
artifacts now live under models/ — the path pilot_training's warm-start loader
reads (shrink-legacy-engine-surface).
"""

import argparse
from pathlib import Path

import numpy as np
import torch
import torch.nn as nn
from torch.utils.data import DataLoader, TensorDataset

from tools.card_features import CardFeatureVectorizer, FEATURE_DIM

DATA_DIR = Path(__file__).resolve().parent.parent.parent / 'models'


class CardAutoencoder(nn.Module):
    def __init__(self, input_dim: int, embedding_dim: int):
        super().__init__()
        hidden = max(embedding_dim * 4, 64)
        self.encoder = nn.Sequential(
            nn.Linear(input_dim, hidden),
            nn.ReLU(),
            nn.Linear(hidden, embedding_dim),
        )
        self.decoder = nn.Sequential(
            nn.Linear(embedding_dim, hidden),
            nn.ReLU(),
            nn.Linear(hidden, input_dim),
            nn.Sigmoid(),
        )

    def forward(self, x):
        z = self.encoder(x)
        return self.decoder(z), z

    def encode(self, x):
        return self.encoder(x)


def train(embedding_dim: int = 16, epochs: int = 500, lr: float = 1e-3,
          batch_size: int = 256):
    vectorizer = CardFeatureVectorizer()
    features = vectorizer.vectorize_all()  # (max_index+1, FEATURE_DIM)

    # Only train on non-zero rows (actual cards, skip index 0 padding)
    mask = features.sum(axis=1) > 0
    card_features = features[mask]
    card_indices = np.where(mask)[0]

    print(f"Training autoencoder: {len(card_features)} cards, "
          f"input_dim={FEATURE_DIM}, embedding_dim={embedding_dim}")

    dataset = TensorDataset(torch.from_numpy(card_features))
    loader = DataLoader(dataset, batch_size=batch_size, shuffle=True)

    model = CardAutoencoder(FEATURE_DIM, embedding_dim)
    optimizer = torch.optim.Adam(model.parameters(), lr=lr)
    loss_fn = nn.MSELoss()

    for epoch in range(epochs):
        total_loss = 0.0
        for (batch,) in loader:
            recon, _ = model(batch)
            loss = loss_fn(recon, batch)
            optimizer.zero_grad()
            loss.backward()
            optimizer.step()
            total_loss += loss.item() * len(batch)

        avg_loss = total_loss / len(card_features)
        if (epoch + 1) % 100 == 0 or epoch == 0:
            print(f"  Epoch {epoch+1:4d}/{epochs}: loss={avg_loss:.6f}")

    # Generate embedding table
    model.eval()
    with torch.no_grad():
        all_features_t = torch.from_numpy(features)
        embeddings = model.encode(all_features_t).numpy()

    # Zero out padding row
    embeddings[0] = 0.0

    # Save
    encoder_path = DATA_DIR / 'card_encoder.pt'
    embeddings_path = DATA_DIR / 'card_embeddings.npy'

    torch.save({
        'model_state_dict': model.encoder.state_dict(),
        'input_dim': FEATURE_DIM,
        'embedding_dim': embedding_dim,
    }, encoder_path)

    np.save(embeddings_path, embeddings)

    print(f"\nSaved encoder weights: {encoder_path}")
    print(f"Saved embeddings: {embeddings_path} shape={embeddings.shape}")
    print(f"Final reconstruction loss: {avg_loss:.6f}")

    # Quick quality check: find most similar pairs
    _quality_check(embeddings, features)


def _quality_check(embeddings: np.ndarray, features: np.ndarray):
    """Spot-check embedding quality by comparing known-similar cards."""
    import json

    cards_path = DATA_DIR / 'cards.json'
    with open(cards_path) as f:
        cards = json.load(f)

    # Build index → card_id map
    idx_to_id = {}
    for card in cards.values():
        idx_to_id[card['index']] = card['card_id']

    # Compute cosine similarity matrix for a sample
    norms = np.linalg.norm(embeddings, axis=1, keepdims=True)
    norms = np.maximum(norms, 1e-8)
    normed = embeddings / norms

    # Find 5 closest pairs among non-zero embeddings
    nonzero_mask = embeddings.sum(axis=1) != 0
    nonzero_idx = np.where(nonzero_mask)[0]

    if len(nonzero_idx) < 10:
        return

    # Sample a few cards and find their nearest neighbors
    rng = np.random.default_rng(42)
    sample_indices = rng.choice(nonzero_idx, size=min(5, len(nonzero_idx)), replace=False)

    print("\nEmbedding quality spot-check:")
    for idx in sample_indices:
        sims = normed[idx] @ normed[nonzero_idx].T
        top_k = np.argsort(sims)[-4:-1][::-1]  # top 3 excluding self
        neighbors = [idx_to_id.get(nonzero_idx[k], '?') for k in top_k]
        card_id = idx_to_id.get(idx, '?')
        sim_vals = [f"{sims[k]:.3f}" for k in top_k]
        print(f"  {card_id} → nearest: {', '.join(f'{n}({s})' for n, s in zip(neighbors, sim_vals))}")


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Train card autoencoder')
    parser.add_argument('--embedding-dim', type=int, default=16)
    parser.add_argument('--epochs', type=int, default=500)
    parser.add_argument('--lr', type=float, default=1e-3)
    parser.add_argument('--batch-size', type=int, default=256)
    args = parser.parse_args()

    train(
        embedding_dim=args.embedding_dim,
        epochs=args.epochs,
        lr=args.lr,
        batch_size=args.batch_size,
    )
