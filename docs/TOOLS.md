# Tools Reference

Scripts and modules for managing card data, transpiling C# scripts, running AI reviews, and building/deploying the engine.

---

## 1. Card Data Pipeline

### 1.1 Card Ingester

**Script:** `tools/ingest_cards.py`

Fetches card data from the digimoncard.io API and merges it into `digimon_gym/engine/data/cards.json`. This is the first step when adding a new set — it populates card metadata (name, colors, level, DP, evo costs, traits, effect text) before `build_registry.py` assigns stable indices.

```bash
# Ingest a single set by ID
python tools/ingest_cards.py --set BT26

# Ingest all priority sets missing from cards.json
python tools/ingest_cards.py --bulk
```

**Priority sets** are read from `digimon_gym/scraper/priority_sets.txt`. Bulk mode skips sets already in `cards.json`.

When re-ingesting an existing set, existing `index` and `norm_id` values are preserved on matching card IDs so stable tensor encoding is not corrupted. Genuinely new cards will lack these fields until `build_registry.py` is run. The script warns if cards with existing indices are missing from the API response.

---

### 1.2 Card Registry Builder

**Script:** `tools/build_registry.py`

Assigns stable integer indices to every card in `cards.json`. These indices are used by the RL tensor writer and `nn.Embedding` lookup. Must be run after `ingest_cards.py` to assign indices to newly ingested cards.

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

| Argument | Default | Description |
|---|---|---|
| `--dry-run` | off | Fetch and compute stats without writing |
| `--offline` | off | Skip API fetch; rebuild norm_ids from existing data |
| `--sets` | all known | Override with specific set IDs |
| `--capacity` | 20000 | Registry capacity ceiling |
| `--force` | off | Override reindex safety check |

**Key properties:**

- **Append-only indices**: existing card→index mappings are never changed. New cards get the next available index after the highest existing one.
- **Reindex safety check**: detects if any card would be reassigned a different index (which would break trained RL agent weights). Aborts unless `--force` is used.
- **Output**: `digimon_gym/engine/data/cards.json` — dict-format with `index` and `norm_id` fields per card.
- **Capacity**: 20,000 slots (indices 1–20000). Index 0 is reserved for padding/empty.

---

### 1.3 Card Autoencoder Trainer

**Script:** `tools/train_card_autoencoder.py`

Trains a small autoencoder on 112-float feature vectors (see Card Feature Vectorizer below) to produce compact 16-float embeddings. These embeddings warm-start the `nn.Embedding` table in the RL policy network so new cards start with meaningful representations instead of random noise.

```bash
python -m tools.train_card_autoencoder
python -m tools.train_card_autoencoder --embedding-dim 16 --epochs 500 --lr 1e-3
```

| Argument | Default | Description |
|---|---|---|
| `--embedding-dim` | 16 | Output embedding dimension |
| `--epochs` | 500 | Training epochs |
| `--lr` | 1e-3 | Learning rate |
| `--batch-size` | 256 | Batch size |

**Architecture:**

```
Input (112) → Linear(112, 64) → ReLU → Linear(64, 16) → [embedding]
                                                         ↓
                               Linear(16, 64) → ReLU → Linear(64, 112) → Sigmoid
```

**Outputs:**

| File | Description |
|---|---|
| `digimon_gym/engine/data/card_encoder.pt` | Encoder weights (for re-encoding new cards) |
| `digimon_gym/engine/data/card_embeddings.npy` | Precomputed embedding table, shape `(max_index+1, 16)` |

After training, the script prints a spot-check showing each sampled card and its 3 nearest neighbors by cosine similarity. Cards of similar type/stats should cluster together.

---

## 2. Transpiler

### 2.1 Transpile CLI

**Script:** `tools/transpile_dcgo.py`

Transpiles DCGO C# card effect scripts into Python `CardScript` files compatible with the digimon_gym engine. Reads `.cs` files from a DCGO-Card-Scripts directory and writes Python equivalents to an output directory.

```bash
python tools/transpile_dcgo.py <DCGO_DIR> <OUTPUT_DIR>

# Examples
python tools/transpile_dcgo.py /tmp/dcgo-scripts/CardEffect/BT14 digimon_gym/engine/data/scripts/bt14
python tools/transpile_dcgo.py /tmp/dcgo-scripts/CardEffect/BT24 digimon_gym/engine/data/scripts/bt24
```

The C# source files live at `DCGO/Assets/Scripts/CardEffect/{SET}/{COLOR}/{CARD_ID}.cs` (using underscores, e.g. `BT17_001.cs`).

### 2.2 Transpiler Package

**Package:** `tools/transpiler/`

The transpiler is a full Python package invoked by `transpile_dcgo.py`. Key modules:

| Module | Purpose |
|---|---|
| `cli.py` | CLI entry point and argument handling |
| `models.py` | Data models for C# AST nodes and output scripts |
| `extractors.py` | C# AST parsing — extracts timing, conditions, actions |
| `generators.py` | Python code generation from extracted AST nodes |
| `patterns.py` | Regex/keyword patterns for C# → Python mapping |
| `scoring.py` | Faithfulness scoring weights and heuristics |
| `validation.py` | Forward/reverse/timing issue detection |
| `known_complex_cards.json` | Cards excluded from threshold checks (beyond transpiler capability) |

---

## 3. Script Audit & Promotion

### 3.1 Audit Transpiled Sets

**Script:** `tools/audit_transpiled_sets.py`

Audits generated card scripts for faithfulness to official card text. Validates each script against card metadata from `cards.json` (or the digimoncard.io API) and produces markdown and JSON reports in `tools/audit_reports/`.

```bash
python tools/audit_transpiled_sets.py --sets BT22,EX5,EX6
python tools/audit_transpiled_sets.py --sets ALL --json
python tools/audit_transpiled_sets.py --sets BT22 --use-api --threshold 0.8
```

| Argument | Default | Description |
|---|---|---|
| `--sets` | required | Comma-separated set IDs or `ALL` |
| `--json` | off | Also write JSON report alongside markdown |
| `--use-api` | off | Fetch card text from digimoncard.io (fallback if not in cards.json) |
| `--threshold` | 0.8 | Faithfulness score threshold for flagging cards |
| `--output-dir` | `tools/audit_reports/` | Directory for report files |

**Scoring weights:**

| Component | Weight | Meaning |
|---|---:|---|
| Effects ratio | 0.40 | Script has same number of effect blocks as API text |
| Actions ratio | 0.30 | Script has sufficient engine action calls |
| Forward match | 0.20 | API-mentioned keywords/timings present in script |
| Coroutine coverage | 0.10 | All effect blocks have coroutine callbacks |

Cards in `tools/transpiler/known_complex_cards.json` are excluded from threshold failure counts.

---

### 3.2 Promote Script

**Script:** `tools/promote_script.py`

Promotes a generated card script from the generated lane into the frozen production lane. Calls `digimon_gym.engine.data.script_promotion.promote_script_from_generated` and requires the expected hash of the generated script for safety.

```bash
python tools/promote_script.py \
  --card-id BT22-001 \
  --set-id BT22 \
  --module-name bt22_001 \
  --expected-generated-hash <sha256>
```

On success, prints the promotion result as JSON (frozen path, hash). The script is moved to `digimon_gym/engine/data/scripts/{set_id}/` and the frozen manifest is updated.

---

### 3.3 Check Frozen Integrity

**Script:** `tools/check_frozen_integrity.py`

CI guard that verifies no frozen scripts have been modified outside of the promotion workflow. Computes SHA-256 hashes of all files in `digimon_gym/engine/data/scripts/` (excluding `generated/` and `__pycache__`) and compares them against `_frozen_manifest.json`. Also detects untracked frozen files that are not in the manifest.

```bash
python tools/check_frozen_integrity.py
```

Exits non-zero if any frozen file has been modified, is missing, or is present without a manifest entry. Intended to run in CI on every PR that touches the `scripts/` directory.

---

## 4. AI Review Pipeline

### 4.1 Build Review Batches

**Script:** `tools/build_review_batches.py`

Builds per-card review candidates and groups them into 5-card batches for queuing as `review_batch` AI tasks. Reads generated scripts and the frozen manifest to determine which cards have pending or reviewable scripts. Outputs a JSON plan file consumed by `queue_review_batches.py`.

```bash
python tools/build_review_batches.py --sets EX10,BT13,EX11,BT23,BT21,BT24
python tools/build_review_batches.py --sets BT24 --output data/review/bt24_plan.json
python tools/build_review_batches.py --sets ALL --dry-run
```

Default sets: `EX10, BT13, EX11, BT23, BT21, BT24`. Output plan is written to `data/review/` by default.

---

### 4.2 Queue Review Batches

**Script:** `tools/queue_review_batches.py`

Reads a review plan JSON produced by `build_review_batches.py` and queues each batch as an AI task via the admin API. Requires a running API server and an admin bearer token.

```bash
python tools/queue_review_batches.py \
  --plan data/review/six_set_review_plan.json \
  --base-url http://127.0.0.1:8000 \
  --token $DIGIMON_ADMIN_BEARER_TOKEN

# Dry run (show what would be queued)
python tools/queue_review_batches.py --plan data/review/plan.json --dry-run

# Cap number of batches submitted
python tools/queue_review_batches.py --plan data/review/plan.json --max-tasks 10
```

| Argument | Default | Description |
|---|---|---|
| `--plan` | `data/review/six_set_review_plan.json` | Plan file from `build_review_batches.py` |
| `--base-url` | `$DIGIMON_API_BASE_URL` or `http://127.0.0.1:8000` | API base URL |
| `--token` | `$DIGIMON_ADMIN_BEARER_TOKEN` | Admin bearer token |
| `--task-type` | `review_batch` | AI task type to create |
| `--max-tasks` | 0 (unlimited) | Optional cap on batches submitted |
| `--dry-run` | off | Print tasks without submitting |

---

## 5. Pinecone Vector DB

### 5.1 Ingest Pinecone

**Script:** `tools/ingest_pinecone.py`

Ingests engine docs, card scripts, and card metadata into the `digimon-engine` Pinecone index for sub-agent retrieval. Uses Pinecone integrated inference — text is auto-embedded on upsert. Chunking helpers come from `digimon_gym/ai/retrieval.py`.

Requires `PINECONE_API_KEY` env var.

```bash
python tools/ingest_pinecone.py --all                          # Full rebuild
python tools/ingest_pinecone.py --namespace card-scripts       # Single namespace
python tools/ingest_pinecone.py --namespace card-scripts --set bt10  # Single set
python tools/ingest_pinecone.py --all --dry-run                # Preview counts
```

**Namespaces:**

| Namespace | Content | ~Vectors |
|---|---|---|
| `engine-api` | Engine API reference doc + decomposed engine source (AST-chunked) | ~300 |
| `card-scripts` | Python scripts (frozen + generated) + C# reference scripts | ~6,000 |
| `card-metadata` | Per-card entries from `cards.json` (ID, name, kind, level, colors, traits, effect text) | ~4,000 |
| `rules-docs` | `RULES_CONTEXT.md`, `ACTION_SPEC.md`, `TENSOR_SPEC.md`, `engine-gaps.md` | ~100 |

---

### 5.2 Verify Pinecone

**Script:** `tools/verify_pinecone.py`

Checks Pinecone index health: verifies all expected namespaces exist with non-zero vector counts and runs spot-check queries to confirm retrieval is working.

Requires `PINECONE_API_KEY` env var.

```bash
python tools/verify_pinecone.py
```

Exits non-zero if any expected namespace is empty or spot-check queries return no results.

---

## 6. Meta Analysis

### 6.1 Meta Loader

**Script:** `tools/meta_loader.py`

Scrapes tournament decklists from multiple sources and builds `digimon_gym/engine/data/deck_library.json`. Sources include DigimonMeta.com, Egman Events, DigimonCard.io, and a DigiLab PostgreSQL database. Computes meta share and conversion rate from placement data.

```bash
python tools/meta_loader.py --scrape-digimonmeta URL   # Scrape BT24/EX11 decks
python tools/meta_loader.py --scrape-egman URL          # Scrape Egman tournament decks
python tools/meta_loader.py --scrape-digimoncard-io URL # Scrape DigimonCard.io tournament
python tools/meta_loader.py --scrape-digilab            # Scrape decklists from DigiLab DB
python tools/meta_loader.py --import-file FILE          # Import a local deck file
python tools/meta_loader.py --fetch-meta                # Fetch DigiLab stats (optional)
python tools/meta_loader.py --build                     # Resolve + dedup + compute stats + write
python tools/meta_loader.py --report                    # Print summary of deck_library.json
```

The `--build` step deduplicates decklists, groups them into archetypes, and computes meta_share and conversion_rate fields. The resulting `deck_library.json` is consumed by `GauntletWrapper` and `rank_archetypes.py`.

---

### 6.2 Rank Archetypes

**Script:** `tools/rank_archetypes.py`

Reads `deck_library.json` and prints a ranked table of archetypes by `meta_share`. Useful for deciding which archetypes to prioritize for implementation.

```bash
python tools/rank_archetypes.py
python tools/rank_archetypes.py --top 30
python tools/rank_archetypes.py --top 20 --min-coverage 0.8
```

| Argument | Default | Description |
|---|---|---|
| `--top` | 20 | Number of archetypes to display |
| `--min-coverage` | 0.0 | Filter to archetypes with script_coverage >= this value |

Output columns: archetype name, meta_share, number of decklists, unique card count, script coverage.

---

### 6.3 Resolve Deck

**Module:** `tools/resolve_deck.py`

Resolves an archetype name into an enriched card manifest. Handles alias resolution, deck library lookup, frozen manifest checks, C# script discovery, and card metadata loading. Auto-writes `qa/archetype-qa/{slug}/deck_pool.json`.

**Primary consumer:** skill agents (`/implement-archetype`, `/batch-fix-cards`, `/review-archetype`).

**As a library:**
```python
from tools.resolve_deck import resolve_archetype, resolve_cards

# Full archetype resolution
manifest = resolve_archetype("Royal Knights")
print(manifest.coverage_pct, manifest.frozen_count, manifest.missing_count)
for card in manifest.unique_cards:
    print(card.card_id, card.script_status, card.csharp_path)

# Ad-hoc card enrichment (no archetype context)
cards = resolve_cards(["BT24-017", "BT24-018"])
```

**As a CLI:**
```bash
python tools/resolve_deck.py "Royal Knights"                # Human-readable table
python tools/resolve_deck.py "Royal Knights" --json         # Full JSON manifest
python tools/resolve_deck.py --cards BT24-017,BT24-018     # Explicit card list
python tools/resolve_deck.py --list-archetypes              # List all archetypes
python tools/resolve_deck.py --list-archetypes --min-share 0.01  # Filter by meta share
```

**Return types:**
- `resolve_archetype()` → `ArchetypeManifest` (archetype stats + list of `CardEntry`)
- `resolve_cards()` → `list[CardEntry]` (enriched cards without archetype context)
- `CardEntry` fields: `card_id`, `card_name`, `card_kind`, `level`, `colors`, `traits`, `dp`, `play_cost`, `evo_costs`, `effect_text`, `inherited_text`, `security_text`, `script_status`, `script_path`, `csharp_path`, `deck_frequency`

---

### 6.4 Store Night Recommender

**Script:** `tools/store_night.py`

Evaluates which deck to bring to a specific store's weekly event. Queries the store's local meta from DigiLab, resolves decklists (personal library first, then scraped), simulates matchups, and prints a ranked recommendation with detailed analysis.

```bash
# Basic recommendation
python tools/store_night.py --store "The Card Haven" \
    --archetypes "Rocks,Millenniummon,Dark Masters" --library my_decks.json

# Full analysis with all optional features
python tools/store_night.py --store "The Card Haven" \
    --archetypes "Rocks,Medusamon" \
    --players --trends --decklists --colors --normalize

# Filter to locals and compare two stores
python tools/store_night.py --store "The Card Haven" \
    --archetypes "Rocks" --event-type locals \
    --compare-stores "Boardwalk Games"

# With deck optimization
python tools/store_night.py --store "The Card Haven" \
    --archetypes "Rocks" --library my_decks.json --optimize
```

| Argument | Default | Description |
|---|---|---|
| `--store` | *(required)* | Store name (looked up in DigiLab) |
| `--archetypes` | *(required)* | Comma-separated archetype names to evaluate |
| `--library` | `my_decks.json` | Path to personal deck library JSON |
| `--since` | 3 months ago | Only consider tournaments after this date (ISO) |
| `--games` | 50 | Games per matchup for simulation |
| `--pilot` | `greedy` | Pilot policy path or `"greedy"` |
| `--min-plays` | 3 | Minimum plays for an archetype to count as a threat |
| `--workers` | 1 | Parallel simulation workers |
| `--optimize` | off | Run deck optimization on the top-ranked archetype |
| `--optimize-episodes` | 100 | Architect training episodes when optimizing |

**Optional analysis flags:**

| Flag | Feature | Description |
|---|---|---|
| `--event-type` | Event type filter | Filter tournaments by type (e.g. `"locals"`, `"regional"`) |
| `--players` | Player scouting | Archetype loyalty, skill-weighted threat profiles, attendance detection |
| `--trends` | Meta velocity | Track archetypes rising/falling across time periods |
| `--normalize` | Size normalization | Weight conversion rates by `sqrt(player_count)` |
| `--colors` | Color heatmap | Show primary/secondary color pair distribution |
| `--decklists` | Decklist analysis | Card staples, winning tech differentials, card trends |
| `--compare-stores` | Cross-store comparison | Side-by-side meta shares with another store (comma-separated names) |

**Report sections (when enabled):**

1. **Your Archetypes** — ranked by Expected Tournament Win Rate (ETWR) with per-opponent matchup breakdown
2. **Local Meta Threats** — archetypes sorted by meta share with win rate, conversion rate, and plays (optional: size-normalized conversion column with `--normalize`)
3. **Sleepers** — high-conversion archetypes with sufficient sample size
4. **Meta Trends** (`--trends`) — per-archetype share delta across time periods with sparklines
5. **Top Threats by Player** (`--players`) — players ranked by threat score (win rate × √events)
6. **Player Archetype Loyalty** (`--players`) — primary deck, loyalty percentage, archetype history
7. **Regulars** (`--players`) — players attending ≥25% of events
8. **Color Distribution** (`--colors`) — primary/secondary color pair frequency table
9. **Decklist Analysis** (`--decklists`) — per-threat-archetype card staples (>80%), winning tech (top-4 vs rest differential), rising/falling cards
10. **Cross-Store Comparison** (`--compare-stores`) — side-by-side meta share table with deltas
11. **Optimization** (`--optimize`) — candidate pool breakdown and before/after win rates

**Dependencies:** `digimon_gym.digilab_client` (DigiLab queries), `digimon_gym.agents.architect_simulator` (simulation, only when evaluating), `tools.decklist_analysis` (card-level analysis, only with `--decklists`).

---

### 6.5 Decklist Analysis

**Module:** `tools/decklist_analysis.py`

Pure analysis functions for card-level statistics across decklists. No DB access — operates on `DecklistRecord` objects from `digilab_client`. Used by `store_night.py --decklists` but can also be imported standalone.

```python
from digimon_gym.digilab_client import get_decklists_for_archetype
from tools.decklist_analysis import (
    compute_card_frequencies,
    compute_winning_differentials,
    compute_card_trends,
)

records = get_decklists_for_archetype("Rocks", store_ids=[3], since_date="2025-12-01")

# Card staples: which cards appear in most lists?
freqs = compute_card_frequencies(records)
staples = [f for f in freqs if f.inclusion_rate >= 0.80]

# Winning tech: what do top-4 finishers play that others don't?
diffs = compute_winning_differentials(records, top_n=4)

# Trends: which cards are rising or falling over time?
trends = compute_card_trends(records, periods=3)
```

| Function | Returns | Description |
|---|---|---|
| `digilab_json_to_card_counts(json)` | `Dict[str, int]` | Parse DigiLab decklist JSON to card ID → copy count |
| `compute_card_frequencies(decklists)` | `List[CardFrequency]` | Per-card inclusion rate and average copies, sorted by inclusion |
| `compute_winning_differentials(decklists, top_n)` | `List[CardDifferential]` | Card usage difference between top-N finishers and the rest |
| `compute_card_trends(decklists, periods)` | `List[CardTrend]` | Per-card inclusion rate slope across time buckets |

| Dataclass | Fields |
|---|---|
| `CardFrequency` | `card_id`, `inclusion_rate`, `avg_copies`, `total_lists` |
| `CardDifferential` | `card_id`, `winner_inclusion`, `other_inclusion`, `differential`, `winner_avg_copies`, `other_avg_copies` |
| `CardTrend` | `card_id`, `period_rates`, `trend_slope`, `current_rate` |

---

### 6.6 DigiLab Client

**Module:** `digimon_gym/digilab_client.py`

Pure psycopg2 queries against the DigiLab PostgreSQL database. Standalone — no SQLAlchemy, no app dependencies. All query functions accept the same scope parameters:

| Parameter | Type | Description |
|---|---|---|
| `store_ids` | `List[int]` | Filter to specific store IDs |
| `scene_id` | `int` | Filter to all stores in a scene |
| `since_date` | `str` | ISO date string — only include tournaments on or after |
| `event_type` | `str` | ILIKE filter on `tournaments.event_type` (e.g. `"locals"`) |

**Query functions:**

| Function | Returns | Description |
|---|---|---|
| `list_stores(min_tournaments)` | `List[StoreInfo]` | All stores with tournament counts |
| `list_scenes(min_tournaments)` | `List[SceneInfo]` | All scenes with tournament counts |
| `get_scoped_meta(...)` | `ScopedMetaResult` | Per-archetype meta share, win rate, conversion rate |
| `get_scoped_meta_normalized(...)` | `ScopedMetaResult` | Same but conversion weighted by `sqrt(player_count)` |
| `get_player_history(...)` | `List[PlayerSummary]` | Per-player tournament results and archetype history |
| `get_color_distribution(...)` | `List[ColorDistribution]` | Primary/secondary color pair frequencies |
| `get_meta_over_time(..., periods)` | `List[PeriodMeta]` | Meta snapshots split into time buckets |
| `get_decklists_for_archetype(name, ...)` | `List[DecklistRecord]` | Full decklist JSON for an archetype |

---

## 7. Model Export & Build

### 7.1 ONNX Export

**Script:** `tools/export_onnx.py`

Converts SB3 MaskablePPO / MaskableRecurrentPPO `.zip` checkpoints to ONNX format. Requires PyTorch and SB3 — intended for dev machines, not end-user desktops. The resulting `.onnx` files can be loaded with `onnxruntime` (no PyTorch needed).

```bash
python tools/export_onnx.py --type mlp --input models/mlp_agent.zip --output models/mlp_agent.onnx
python tools/export_onnx.py --type lstm --input models/lstm_agent.zip --output models/lstm_agent.onnx
```

Exported files are consumed by:

- **Hosted API / training**: `OnnxMlpPolicy` / `OnnxLstmPolicy` in `digimon_gym/engine/onnx_policy.py`; served via the `/games/models` API route.
- **Desktop app**: `digimon-engine/src/inference/` loads the same `.onnx` at runtime after it's downloaded from the hosted manifest and cached under `dirs::data_dir()/digimon-tcg/models/<id>/policy.onnx`.

Newly-exported models reach desktop users by being published to the admin model manifest (`/models/manifest.json`) with the correct `tensor_size` / `action_space_size`; the desktop app rejects downloads that mismatch the Rust engine's contract.

---

### 7.2 Desktop Build

The desktop app is **Python-free** — there is no sidecar binary to build. Gameplay, ONNX inference, and deck tooling are statically linked into the Tauri executable via the embedded `digimon-engine` crate.

```bash
cd frontend && npm ci && npm run build -- --mode desktop
cd ../src-tauri && cargo tauri build
```

The installer contains only the Rust binary + frontend assets + icons. Trained AI models are not bundled; users fetch them at runtime from the hosted API's manifest via the in-app Models page.

---

### 7.3 Training Smoke Test

**Script:** `tools/train_smoke_test.py`

Validates that `DigimonEnv` works end-to-end with a manual random-action loop and with SB3 MaskablePPO. Does not validate rule correctness — only checks that the environment initializes, steps without crashing, and produces valid observations and masks.

```bash
python tools/train_smoke_test.py
```

Requires `stable-baselines3` and `sb3-contrib`.

---

## 8. Engine Modules

These modules live inside the engine and are used at runtime as well as by the tools above.

### 8.1 Card Feature Vectorizer

**Module:** `digimon_gym/engine/data/card_features.py`

Converts structured card attributes from `cards.json` into fixed-size float vectors for autoencoder training. Not used at RL inference time — only for generating warm-start embeddings.

**Feature vector layout (112 floats):**

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

```python
from digimon_gym.engine.data.card_features import CardFeatureVectorizer

vectorizer = CardFeatureVectorizer()
features = vectorizer.vectorize_all()  # shape: (max_index+1, 112)
```

---

### 8.2 Card Registry

**Module:** `digimon_gym/engine/data/card_registry.py`

Runtime lookup from card ID strings (e.g. `"BT1-001"`) to integer indices used in the tensor. Auto-initializes from `cards.json` on first use.

| Method | Returns | Description |
|---|---|---|
| `CardRegistry.get_id(card_id)` | `int` | Integer index for tensor encoding (0 = unknown/empty) |
| `CardRegistry.get_norm_id(card_id)` | `float` | Normalized ID (index / 20000) |
| `CardRegistry.get_embedding(card_id)` | `np.ndarray` | Precomputed embedding vector (for warm-start) |
| `CardRegistry.get_string_id(int_id)` | `str` | Reverse lookup: integer → card ID string |
| `CardRegistry.count()` | `int` | Number of registered cards |

| Constant | Value | Description |
|---|---|---|
| `REGISTRY_CAPACITY` | 20,000 | Max cards supported |
| `EMBEDDING_DIM` | 16 | Embedding vector size |

---

### 8.3 Tensor Layout Map

**Module:** `digimon_gym/engine/data/tensor_layout.py`

Computes which positions in the 1375-float observation tensor hold card IDs vs scalar values. Used by `CardEmbeddingExtractor` to split the tensor for GPU-side embedding lookup.

| Name | Value | Description |
|---|---|---|
| `CARD_ID_POSITIONS` | list of 520 ints | Tensor indices holding card IDs |
| `SCALAR_POSITIONS` | list of 855 ints | Tensor indices holding scalar values |
| `NUM_CARD_SLOTS` | 520 | Length of `CARD_ID_POSITIONS` |
| `NUM_SCALAR_SLOTS` | 855 | Length of `SCALAR_POSITIONS` |

All positions are computed deterministically from game constants (`FIELD_SLOTS`, `SLOT_SIZE`, `MAX_SOURCES`, etc.) — nothing is hardcoded. The module asserts that card + scalar positions sum to `TENSOR_SIZE` (1375).

Card ID positions include:
- Top card ID in each battle area slot (28 slots × 1)
- Source card IDs in each digivolution stack (28 slots × 11 sources)
- Hand card IDs (2 × 20)
- Trash card IDs (2 × 45)
- Security card IDs (2 × 10)
- Breeding area card IDs (2 slots × 12 per slot)
- Revealed card IDs (10)

```python
from digimon_gym.engine.data.tensor_layout import CARD_ID_POSITIONS, SCALAR_POSITIONS

# Used internally by CardEmbeddingExtractor
card_ids = observations[:, CARD_ID_POSITIONS].long()   # (batch, 520)
scalars = observations[:, SCALAR_POSITIONS]             # (batch, 855)
```

---

## 9. New Card Set Workflow

When a new Digimon TCG set releases, follow these steps:

### Step 1: Ingest Card Metadata

```bash
python tools/ingest_cards.py --set BT26
```

Fetches card data from digimoncard.io and merges it into `cards.json`. New cards will not yet have `index` or `norm_id` fields.

### Step 2: Assign Stable Registry Indices

```bash
python tools/build_registry.py --sets BT26
```

New cards get append-only indices. Existing indices are preserved, so old trained agents remain valid.

### Step 3: Transpile C# Scripts

```bash
python tools/transpile_dcgo.py /path/to/DCGO/CardEffect/BT26 digimon_gym/engine/data/scripts/generated/bt26
```

Generates Python CardScript files from DCGO C# sources.

### Step 4: Audit Generated Scripts

```bash
python tools/audit_transpiled_sets.py --sets BT26
```

Reports faithfulness scores and flags cards below the 0.8 threshold. Reports are written to `tools/audit_reports/`.

### Step 5: Queue AI Review (Optional)

```bash
python tools/build_review_batches.py --sets BT26 --output data/review/bt26_plan.json
python tools/queue_review_batches.py --plan data/review/bt26_plan.json --token $TOKEN
```

Queues `review_batch` AI tasks for the new set via the admin API.

### Step 6: Promote Reviewed Scripts

```bash
python tools/promote_script.py \
  --card-id BT26-001 \
  --set-id BT26 \
  --module-name bt26_001 \
  --expected-generated-hash <sha256>
```

Moves approved scripts to the frozen lane and updates `_frozen_manifest.json`.

### Step 7: Verify Frozen Integrity

```bash
python tools/check_frozen_integrity.py
```

CI guard — confirms all frozen scripts match their manifest hashes.

### Step 8: Regenerate Warm-Start Embeddings (Optional)

```bash
python -m tools.train_card_autoencoder
```

Retrains the autoencoder on all cards including the new set. Produces updated `card_embeddings.npy` for warm-starting future training runs.

### Step 9: Update Pinecone Index (Optional)

```bash
python tools/ingest_pinecone.py --namespace card-scripts --set bt26
python tools/ingest_pinecone.py --namespace card-metadata
```

Makes the new scripts and card metadata searchable by sub-agents.

### Step 10: Train New Pilot

```bash
python -m digimon_gym.agents.pilot_training --timesteps 500000
```

The `CardEmbeddingExtractor` automatically loads `card_embeddings.npy` if present and uses it to initialize the `nn.Embedding` table. New card indices start with warm-start embeddings instead of random noise.

### What Happens to Old Agents

Old pilots still work — their saved model checkpoint contains the `nn.Embedding` table with weights for the cards they were trained on. New card indices will have untrained embedding rows, which is expected since the pilot needs retraining for new cards anyway.

---

## 10. Archive

**Directory:** `tools/archive/`

One-time migration and backfill scripts that have already been run. Do not re-run these — they are kept for historical reference only.

| Script | Purpose |
|---|---|
| `backfill_xros_req.py` | Backfilled `xros_req` field into cards.json for DigiXros cards |
| `bootstrap_frozen_manifest.py` | Created the initial `_frozen_manifest.json` from existing frozen scripts |
| `bootstrap_roles.py` | One-time DB role migration |
| `build_rag_index.py` | Early RAG index builder (superseded by `ingest_pinecone.py`) |
| `fetch_card_effects.py` | One-time fetch of card effect text |
| `fix_alt_digi_constraints.py` | Bulk fix for alt-digi constraint encoding in generated scripts |
| `fix_cost_reduction_leak.py` | Bulk fix for BeforePayCost cost reduction leaking from field permanents |
| `ingest_bt14_cards.py` | One-time BT14 ingestion before `ingest_cards.py` existed |
| `migrate_set_timing.py` | Migrated timing field format across generated scripts |
| `patch_tamer_memory.py` | Patched memory cost handling for Tamer cards |
| `qa_round1_2.py` | QA round 1+2 batch runner (superseded by `queue_review_batches.py`) |
