# Tools Reference

Scripts and modules for managing the card registry, embeddings, and tensor layout.

---

## 1. Card Registry Builder

**Script:** `tools/build_registry.py`

Fetches all Digimon TCG cards from the DigimonCard.io API and assigns stable integer indices used by the RL tensor writer and `nn.Embedding` lookup.

### Usage

```bash
# Full build from API (fetches all known sets)
python tools/build_registry.py

# Dry run — fetch + stats, no write
python tools/build_registry.py --dry-run

# Rebuild norm_ids only (no API fetch)
python tools/build_registry.py --offline

# Fetch specific sets only
python tools/build_registry.py --sets BT25 EX12
```

### Arguments

| Argument | Default | Description |
|---|---|---|
| `--dry-run` | off | Fetch and compute stats without writing |
| `--offline` | off | Skip API fetch; rebuild norm_ids from existing data |
| `--sets` | all known | Override with specific set IDs |
| `--capacity` | 20000 | Registry capacity ceiling |
| `--force` | off | Override reindex safety check |

### Key Properties

- **Append-only indices**: existing card→index mappings are never changed. New cards get the next available index after the highest existing one.
- **Reindex safety check**: detects if any card would be reassigned a different index (which would break trained RL agent weights). Aborts unless `--force` is used.
- **Output**: `digimon_gym/engine/data/cards.json` — dict-format with `index` and `norm_id` fields per card.
- **Capacity**: 20,000 slots (indices 1–20000). Index 0 is reserved for padding/empty.

---

## 2. Card Feature Vectorizer

**Module:** `digimon_gym/engine/data/card_features.py`

Converts structured card attributes from `cards.json` into fixed-size float vectors for autoencoder training. Not used at RL inference time — only for generating warm-start embeddings.

### Feature Vector Layout (112 floats)

| Section | Size | Encoding |
|---|---:|---|
| Card kind | 4 | One-hot (Digimon, Tamer, Option, DigiEgg) |
| Colors | 8 | Multi-hot (Red, Blue, Yellow, Green, White, Black, Purple, NoColor) |
| Level | 1 | Normalized (0–7 → 0.0–1.0) |
| Play cost | 1 | Normalized (0–20 → 0.0–1.0) |
| DP | 1 | Normalized (0–17000 → 0.0–1.0) |
| Forms | 16 | One-hot (Rookie, Champion, Ultimate, Mega, ...) |
| Attributes | 18 | One-hot (Vaccine, Virus, Data, Free, ...) |
| Keywords | 39 | Multi-hot (Alliance, Blocker, Jamming, Piercing, ...) |
| Flags | 3 | has_inherited, has_security, is_ace |
| Evolution costs | 20 | 2 slots × (color_8 + level + cost) |

### Usage

```python
from digimon_gym.engine.data.card_features import CardFeatureVectorizer

vectorizer = CardFeatureVectorizer()
features = vectorizer.vectorize_all()  # shape: (max_index+1, 112)
```

---

## 3. Card Autoencoder Trainer

**Script:** `tools/train_card_autoencoder.py`

Trains a small autoencoder on the 112-float feature vectors to produce compact 16-float embeddings. These embeddings are used to warm-start the `nn.Embedding` table in the RL policy network.

### Usage

```bash
python -m tools.train_card_autoencoder
python -m tools.train_card_autoencoder --embedding-dim 16 --epochs 500 --lr 1e-3
```

### Arguments

| Argument | Default | Description |
|---|---|---|
| `--embedding-dim` | 16 | Output embedding dimension |
| `--epochs` | 500 | Training epochs |
| `--lr` | 1e-3 | Learning rate |
| `--batch-size` | 256 | Batch size |

### Architecture

```
Input (112) → Linear(112, 64) → ReLU → Linear(64, 16) → [embedding]
                                                         ↓
                               Linear(16, 64) → ReLU → Linear(64, 112) → Sigmoid
```

### Outputs

| File | Description |
|---|---|
| `digimon_gym/engine/data/card_encoder.pt` | Encoder weights (for re-encoding new cards) |
| `digimon_gym/engine/data/card_embeddings.npy` | Precomputed embedding table, shape `(max_index+1, 16)` |

### Quality Check

After training, the script prints a spot-check showing each sampled card and its 3 nearest neighbors by cosine similarity. Cards of similar type/stats should cluster together.

---

## 4. Tensor Layout Map

**Module:** `digimon_gym/engine/data/tensor_layout.py`

Computes which positions in the 1375-float observation tensor hold card IDs vs scalar values. Used by `CardEmbeddingExtractor` to split the tensor for GPU-side embedding lookup.

### Exports

| Name | Value | Description |
|---|---|---|
| `CARD_ID_POSITIONS` | list of 520 ints | Tensor indices holding card IDs |
| `SCALAR_POSITIONS` | list of 855 ints | Tensor indices holding scalar values |
| `NUM_CARD_SLOTS` | 520 | Length of `CARD_ID_POSITIONS` |
| `NUM_SCALAR_SLOTS` | 855 | Length of `SCALAR_POSITIONS` |

### How It Works

All positions are computed deterministically from game constants (`FIELD_SLOTS`, `SLOT_SIZE`, `MAX_SOURCES`, etc.) — nothing is hardcoded. The module asserts that card + scalar positions sum to `TENSOR_SIZE` (1375).

Card ID positions include:
- Top card ID in each battle area slot (28 slots × 1)
- Source card IDs in each digivolution stack (28 slots × 11 sources)
- Hand card IDs (2 × 20)
- Trash card IDs (2 × 45)
- Security card IDs (2 × 10)
- Breeding area card IDs (2 slots × 12 per slot)
- Revealed card IDs (10)

### Usage

```python
from digimon_gym.engine.data.tensor_layout import CARD_ID_POSITIONS, SCALAR_POSITIONS

# Used internally by CardEmbeddingExtractor
card_ids = observations[:, CARD_ID_POSITIONS].long()   # (batch, 520)
scalars = observations[:, SCALAR_POSITIONS]             # (batch, 855)
```

---

## 5. Card Registry

**Module:** `digimon_gym/engine/data/card_registry.py`

Runtime lookup from card ID strings (e.g. `"BT1-001"`) to integer indices used in the tensor. Auto-initializes from `cards.json` on first use.

### Key Methods

| Method | Returns | Description |
|---|---|---|
| `CardRegistry.get_id(card_id)` | `int` | Integer index for tensor encoding (0 = unknown/empty) |
| `CardRegistry.get_norm_id(card_id)` | `float` | Normalized ID (index / 20000) |
| `CardRegistry.get_embedding(card_id)` | `np.ndarray` | Precomputed embedding vector (for warm-start) |
| `CardRegistry.get_string_id(int_id)` | `str` | Reverse lookup: integer → card ID string |
| `CardRegistry.count()` | `int` | Number of registered cards |

### Constants

| Constant | Value | Description |
|---|---|---|
| `REGISTRY_CAPACITY` | 20,000 | Max cards supported |
| `EMBEDDING_DIM` | 16 | Embedding vector size |

---

## 6. New Card Set Workflow

When a new Digimon TCG set releases, follow these steps:

### Step 1: Update the Registry

```bash
python tools/build_registry.py --sets BT26
```

New cards get append-only indices. Existing indices are preserved, so old trained agents remain valid.

### Step 2: Regenerate Warm-Start Embeddings (Optional)

```bash
python -m tools.train_card_autoencoder
```

Retrains the autoencoder on all cards including the new set. Produces updated `card_embeddings.npy` for warm-starting future training runs.

### Step 3: Train New Pilot

```bash
python -m digimon_gym.agents.pilot_training --timesteps 500000
```

The `CardEmbeddingExtractor` automatically loads `card_embeddings.npy` if present and uses it to initialize the `nn.Embedding` table. New card indices start with warm-start embeddings instead of random noise.

### What Happens to Old Agents

Old pilots still work — their saved model checkpoint contains the `nn.Embedding` table with weights for the cards they were trained on. New card indices will have untrained embedding rows, which is expected since the pilot needs retraining for new cards anyway.
