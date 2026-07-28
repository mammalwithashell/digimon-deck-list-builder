# Tools Reference

Scripts and modules for managing card data, running AI reviews, and building/deploying the engine.

Card-data authority, schemas, failure/freshness rules, committed producer
ownership, preview/plan/apply/check commands, scheduled reconciliation,
rollback, and wrapper-retirement criteria live in
[`CARD_DATA_PIPELINE.md`](CARD_DATA_PIPELINE.md). Pool/campaign and deferred
consumer-cutover semantics live in
[`CARD_POOL_CONTRACTS.md`](CARD_POOL_CONTRACTS.md). `data/cards.json` is a
committed compatibility ABI, not printed-data authority.

---

## 1. Card Data Pipeline

### 1.1 Canonical Sync and Legacy Ingest Preview

**Script:** `code/tools/ingest_cards.py`

This is a deprecated compatibility command. Its accepted mode is a read-only,
secondary-source preview; it does not admit cards, allocate indices, or write
`data/cards.json`:

```bash
# Read-only provisional diagnostic
python code/tools/ingest_cards.py --preview-set BT26
```

The old `--set`, `--bulk`, `--backfill`, and positional `SET NAME` mutation
modes are recognized only to return a nonzero refusal with migration guidance.
They fail before fetching or changing fixed artifacts. Source publication now
uses a complete candidate plus the reviewed canonical transaction:

```bash
python -m tools.card_data sync --plan \
  --repo-root . --candidate-dir <candidate-dir> \
  --source-plan <source-plan.json> --plan-file <publication-plan.json>
python -m tools.card_data sync --apply \
  --repo-root . --candidate-dir <candidate-dir> \
  --plan-file <publication-plan.json>
```

Plan is write-free. Apply is a separate reviewed operation protected by the
repository writer lock, base-revision check, transaction journal, rollback,
and marker-last publication. Private transaction files use short ordinal paths
for Windows safety; the journal fsyncs bounded write-ahead prefixes before
replacement and records rollback progress only after a whole reverse batch is
restored.

### 1.2 Stable Card Registry Compatibility Wrapper

**Script:** `code/tools/build_registry.py`

This legacy command name delegates to the canonical append-only registry
library. It is network-free and manages only registry assignments plus the
derived `index`, `norm_id`, and suffix-derived `card_index` compatibility
values. It never fetches or normalizes printed card data. The committed fixed
paths are transaction-owned: the wrapper may check/dry-run them or write
explicit non-fixed scratch paths, but it cannot publish a revision.

```bash
# Accepted read-only automation against committed fixed paths
python code/tools/build_registry.py --check
python code/tools/build_registry.py --dry-run

# Optional scratch projection; both paths must be non-fixed
python code/tools/build_registry.py \
  --cards-json <scratch-cards-json> \
  --registry <scratch-registry-json>

# Deprecated no-op alias; the command is always offline
python code/tools/build_registry.py --check --offline
```

| Argument | Default | Description |
|---|---|---|
| `--check` | off | Fail when the registry or derived values are stale; no writes |
| `--dry-run` | off | Report candidate changes without writing fixed paths |
| `--offline` | off | Deprecated compatibility alias; operation is always offline |
| `--sets` | n/a | Retired and refused; source discovery belongs to canonical sync |
| `--capacity` | 20000 | Fixed compatibility ceiling; other values are refused |
| `--force` | off | Retired and refused; indices cannot be reassigned |
| `--cards-json` | `data/cards.json` | Compatibility input/output; with a committed marker, a write requires an explicit non-fixed path |
| `--registry` | `data/card_registry.json` | Registry input/output; with a committed marker, a write requires an explicit non-fixed path |

**Key properties:**

- **Append-only indices**: existing card→index mappings are never changed. New cards get the next available index after the highest existing one.
- **Reindex safety check**: any reassignment aborts. `--force` is refused and cannot bypass the tensor/model compatibility contract.
- **Outputs**: explicit scratch paths only. Fixed `data/card_registry.json` and `data/cards.json` changes must be generated in a complete candidate and published by the canonical transaction; no printed fields are changed by this wrapper.
- **Capacity**: 20,000 slots (indices 1–20000). Index 0 is reserved for padding/empty.

---

### 1.3 Official Bundle and Evolution-Cost Compatibility Commands

`code/tools/build_card_bundles.py` and
`code/tools/scrape_official_evo_costs.py` retain their shared official parser
APIs for in-memory callers. Their accepted CLI modes are read-only previews:

```bash
python code/tools/build_card_bundles.py --preview --ids BT13-020 ST9-05
python code/tools/scrape_official_evo_costs.py --ids BT13-020 EX1-014
```

Both emit deprecation guidance on stderr while preserving JSON stdout.
`build_card_bundles.py` refuses the old fixed bundle/official-mirror write, and
`scrape_official_evo_costs.py --out` is retired. Revision-bound official
mirrors, bundles, evolution-cost views, and lexicons are generated together by
the canonical pipeline and published only through reviewed sync apply.

---

### 1.4 Card Autoencoder Trainer

**Script:** `code/tools/train_card_autoencoder.py`

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
| `code/engine_py_legacy/engine/data/card_encoder.pt` | Encoder weights (for re-encoding new cards) |
| `code/engine_py_legacy/engine/data/card_embeddings.npy` | Precomputed embedding table, shape `(max_index+1, 16)` |

After training, the script prints a spot-check showing each sampled card and its 3 nearest neighbors by cosine similarity. Cards of similar type/stats should cluster together.

---

## 2. Script Promotion — RETIRED

The Python frozen/generated card-script promotion lane is **retired**
(shrink-legacy-engine-surface, 2026-06-14). Card scripting is now Rust
DSL-first (rule 28), so there is nothing to promote. The tools
`promote_script.py`, `check_frozen_integrity.py`, `run_qa_batch.py`, and
`archive/bootstrap_frozen_manifest.py` were deleted, and the hosted-API
`/admin/promotions` + `/admin/ai-tasks/{id}/promote` endpoints now return
`410 Gone` (`GET /admin/promotions` still serves historical audit rows).

---

## 3. AI Review Pipeline

### 3.1 Build Review Batches

**Script:** `code/tools/build_review_batches.py`

Builds per-card review candidates and groups them into 5-card batches for queuing as `review_batch` AI tasks. Reads generated scripts and the frozen manifest to determine which cards have pending or reviewable scripts. Outputs a JSON plan file consumed by `queue_review_batches.py`.

```bash
python code/tools/build_review_batches.py --sets EX10,BT13,EX11,BT23,BT21,BT24
python code/tools/build_review_batches.py --sets BT24 --output data/review/bt24_plan.json
python code/tools/build_review_batches.py --sets ALL --dry-run
```

Default sets: `EX10, BT13, EX11, BT23, BT21, BT24`. Output plan is written to `data/review/` by default.

---

### 3.2 Queue Review Batches

**Script:** `code/tools/queue_review_batches.py`

Reads a review plan JSON produced by `build_review_batches.py` and queues each batch as an AI task via the admin API. Requires a running API server and an admin bearer token.

```bash
python code/tools/queue_review_batches.py \
  --plan data/review/six_set_review_plan.json \
  --base-url http://127.0.0.1:8000 \
  --token $DIGIMON_ADMIN_BEARER_TOKEN

# Dry run (show what would be queued)
python code/tools/queue_review_batches.py --plan data/review/plan.json --dry-run

# Cap number of batches submitted
python code/tools/queue_review_batches.py --plan data/review/plan.json --max-tasks 10
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

## 4. Pinecone Vector DB

### 4.1 Ingest Pinecone

**Script:** `code/tools/ingest_pinecone.py`

Ingests engine docs, card scripts, and card metadata into the `digimon-engine` Pinecone index for sub-agent retrieval. Uses Pinecone integrated inference — text is auto-embedded on upsert. Chunking helpers come from `code/server/ai/retrieval.py`.

Requires `PINECONE_API_KEY` env var.

```bash
python code/tools/ingest_pinecone.py --all                          # Full rebuild
python code/tools/ingest_pinecone.py --namespace card-scripts       # Single namespace
python code/tools/ingest_pinecone.py --namespace card-scripts --set bt10  # Single set
python code/tools/ingest_pinecone.py --all --dry-run                # Preview counts
```

**Namespaces:**

| Namespace | Content | ~Vectors |
|---|---|---|
| `engine-api` | Engine API reference doc + decomposed engine source (AST-chunked) | ~300 |
| `card-scripts` | Python scripts (frozen + generated) + C# reference scripts | ~6,000 |
| `card-metadata` | Per-card entries from `cards.json` (ID, name, kind, level, colors, traits, effect text) | ~4,000 |
| `rules-docs` | `RULES_CONTEXT.md`, `ACTION_SPEC.md`, `TENSOR_SPEC.md`, `RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md` | ~100 |

---

### 4.2 Verify Pinecone

**Script:** `code/tools/verify_pinecone.py`

Checks Pinecone index health: verifies all expected namespaces exist with non-zero vector counts and runs spot-check queries to confirm retrieval is working.

Requires `PINECONE_API_KEY` env var.

```bash
python code/tools/verify_pinecone.py
```

Exits non-zero if any expected namespace is empty or spot-check queries return no results.

---

## 5. Meta Analysis

### 5.1 Meta Loader

**Script:** `code/tools/meta_loader.py`

Scrapes tournament decklists from multiple sources and builds `data/deck_library.json`. Sources include DCG Nexus, DigimonMeta.com, Egman Events, DigimonCard.io, and a DigiLab PostgreSQL database. Computes meta share and conversion rate from placement data.

```bash
python code/tools/meta_loader.py --scrape-dcg-nexus --dcg-nexus-format EX12
python code/tools/meta_loader.py --scrape-digimonmeta URL   # Scrape BT24/EX11 decks
python code/tools/meta_loader.py --scrape-egman URL          # Scrape Egman tournament decks
python code/tools/meta_loader.py --scrape-digimoncard-io URL # Scrape DigimonCard.io tournament
python code/tools/meta_loader.py --scrape-digilab            # Scrape decklists from DigiLab DB
python code/tools/meta_loader.py --import-file FILE          # Import a local deck file
python code/tools/meta_loader.py --fetch-meta                # Fetch DigiLab stats (optional)
python code/tools/meta_loader.py --build                     # Resolve + dedup + compute stats + write
python code/tools/meta_loader.py --report                    # Print summary of deck_library.json
```

The `--build` step deduplicates decklists, groups them into archetypes, and computes meta_share and conversion_rate fields. The resulting `deck_library.json` is consumed by `GauntletWrapper` and `rank_archetypes.py`.

#### `stats` vs `format_stats` — pick the right share

Each archetype carries two stat blocks:

- **`stats.meta_share`** — the archetype's fraction of the *entire* corpus, which
  spans several years and rotations. Use for all-time questions.
- **`format_stats["EX12"].meta_share`** — its fraction of the decks played in
  that format only. Use for "what will I face in a competitive room today".

The difference is large enough to change conclusions. In July 2026 the
most-played EX12 deck was **12.19% of EX12 but 1.04% of the merged corpus** —
below the classifier's 2% meta threshold, so the unscoped number tiered the
format's best deck as "rogue". Half the format tiered as unranked.

`format_stats` is keyed on each deck's `format` field, so it only covers sources
that record one (currently DCG Nexus). Untagged decks are excluded rather than
pooled into an "unknown" bucket, so a format's shares always sum to 1.

`server/classifier/meta_tier.py` takes a matching `format_scope` argument, which
scopes both the share and the staple fingerprint:

```python
load_library_from_path(format_scope="EX12")   # current-format tiering
load_library_from_path()                       # all-time, unchanged default
```

#### Source notes

**DCG Nexus** (`dcg-nexus.com`) is the only source that tags each event with its
legal format, so it is the one to reach for when scoping to the current format:

```bash
python code/tools/meta_loader.py --scrape-dcg-nexus \
    --dcg-nexus-since 2026-07-01 --dcg-nexus-format EX12
```

Useful flags: `--dcg-nexus-since YYYY-MM-DD` (filters on the date encoded in the
event slug, before any page is fetched), `--dcg-nexus-max-events N`,
`--dcg-nexus-event URL` (single event), and `--dcg-nexus-delay` (default 0.5s
between requests — the site is a hobby project behind Cloudflare, so do not
lower it without reason). A full-format scrape is roughly 40 event pages plus
~600 decklist pages, so budget ~10 minutes and run it detached.

The scraper reads the decklist payload embedded in each page's export button
rather than `/Deck/Decklist/ExportCSV`, which the site's `robots.txt` disallows.
Note that `robots.txt` also sets `ai-train=no` and blocks named AI crawlers —
see the ingestion caveat before using this data for model training.

**DigiLab** stopped ingesting on 2026-03-12 (newest format EX11). It remains
useful as a historical corpus but contributes nothing to current-format meta.

**Placement caveat:** `_is_top_cut` treats the top 25% of a field as a top cut,
which is right for a 16-player local but flags 94th place at a 398-player
regional. Treat `conversion_rate` as comparable only within similar event sizes.
DCG Nexus also carries only *submitted* decklists (e.g. 33 of 398 at a
regional), which skews toward players who did well.

---

### 5.2 Rank Archetypes

**Script:** `code/tools/rank_archetypes.py`

Reads `deck_library.json` and prints a ranked table of archetypes by `meta_share`. Useful for deciding which archetypes to prioritize for implementation.

```bash
python code/tools/rank_archetypes.py
python code/tools/rank_archetypes.py --top 30
python code/tools/rank_archetypes.py --top 20 --min-coverage 0.8
```

| Argument | Default | Description |
|---|---|---|
| `--top` | 20 | Number of archetypes to display |
| `--min-coverage` | 0.0 | Filter to archetypes with script_coverage >= this value |

Output columns: archetype name, meta_share, number of decklists, unique card count, script coverage.

---

### 5.3 Resolve Deck

**Module:** `code/tools/resolve_deck.py`

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
python code/tools/resolve_deck.py "Royal Knights"                # Human-readable table
python code/tools/resolve_deck.py "Royal Knights" --json         # Full JSON manifest
python code/tools/resolve_deck.py --cards BT24-017,BT24-018     # Explicit card list
python code/tools/resolve_deck.py --list-archetypes              # List all archetypes
python code/tools/resolve_deck.py --list-archetypes --min-share 0.01  # Filter by meta share
```

**Return types:**
- `resolve_archetype()` → `ArchetypeManifest` (archetype stats + list of `CardEntry`)
- `resolve_cards()` → `list[CardEntry]` (enriched cards without archetype context)
- `CardEntry` fields: `card_id`, `card_name`, `card_kind`, `level`, `colors`, `traits`, `dp`, `play_cost`, `evo_costs`, `effect_text`, `inherited_text`, `security_text`, `script_status`, `script_path`, `csharp_path`, `deck_frequency`

---

### 5.4 Store Night Recommender

**Script:** `code/tools/store_night.py`

Evaluates which deck to bring to a specific store's weekly event. Queries the store's local meta from DigiLab, resolves decklists (personal library first, then scraped), simulates matchups, and prints a ranked recommendation with detailed analysis.

```bash
# Basic recommendation
python code/tools/store_night.py --store "The Card Haven" \
    --archetypes "Rocks,Millenniummon,Dark Masters" --library my_decks.json

# Full analysis with all optional features
python code/tools/store_night.py --store "The Card Haven" \
    --archetypes "Rocks,Medusamon" \
    --players --trends --decklists --colors --normalize

# Filter to locals and compare two stores
python code/tools/store_night.py --store "The Card Haven" \
    --archetypes "Rocks" --event-type locals \
    --compare-stores "Boardwalk Games"

# With deck optimization
python code/tools/store_night.py --store "The Card Haven" \
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

**Dependencies:** `server.digilab_client` (DigiLab queries), `digimon_gym.agents.architect_simulator` (simulation, only when evaluating), `tools.decklist_analysis` (card-level analysis, only with `--decklists`).

---

### 5.5 Decklist Analysis

**Module:** `code/tools/decklist_analysis.py`

Pure analysis functions for card-level statistics across decklists. No DB access — operates on `DecklistRecord` objects from `digilab_client`. Used by `store_night.py --decklists` but can also be imported standalone.

```python
from server.digilab_client import get_decklists_for_archetype
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

### 5.6 DigiLab Client

**Module:** `code/server/digilab_client.py`

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

## 6. Model Export & Build

### 6.1 ONNX Export

**Script:** `code/tools/export_onnx.py`

Converts SB3 MaskablePPO / MaskableRecurrentPPO `.zip` checkpoints to ONNX format. Requires PyTorch and SB3 — intended for dev machines, not end-user desktops. The resulting `.onnx` files can be loaded with `onnxruntime` (no PyTorch needed).

```bash
python code/tools/export_onnx.py --type mlp --input models/mlp_agent.zip --output models/mlp_agent.onnx --tensor-profile standard_lite_v2
python code/tools/export_onnx.py --type lstm --input models/lstm_agent.zip --output models/lstm_agent.onnx --tensor-profile standard_lite_v2
```

Export writes a profile metadata sidecar next to the ONNX file, for example
`models/mlp_agent.onnx.meta.json`. The sidecar records the observation profile,
tensor size, feature schema version, layout hash, action-space size, registry
capacity, and embedding dimension so loaders can reject mismatched model/profile
combinations.

Exported files are consumed by:

- **Hosted API / training**: `OnnxMlpPolicy` / `OnnxLstmPolicy` in `code/digimon_gym/inference/onnx_policy.py`; served via the `/games/models` API route.
- **Desktop app**: `code/digimon-engine/src/inference/` loads the same `.onnx` at runtime after it's downloaded from the hosted manifest and cached under `dirs::data_dir()/digimon-tcg/models/<id>/policy.onnx`.

Newly-exported models reach desktop users by being published to the admin model manifest (`/models/manifest.json`). The export sidecar records `observation_profile` and `tensor_layout_hash` for tools, loaders, and workflows that consume that metadata. The hosted manifest / desktop publication path currently records and validates `tensor_size`, `action_space_size`, and the file hash; rejecting profile or layout-hash mismatches from the manifest is future work unless implemented in that publication path.

---

### 6.2 Desktop Build

The desktop app is **Python-free** — there is no sidecar binary to build. Gameplay, ONNX inference, and deck tooling are statically linked into the Tauri executable via the embedded `digimon-engine` crate.

```bash
cd frontend && npm ci && npm run build -- --mode desktop
cd ../src-tauri && cargo tauri build
```

The installer contains only the Rust binary + frontend assets + icons. Trained AI models are not bundled; users fetch them at runtime from the hosted API's manifest via the in-app Models page.

---

### 6.3 Training Smoke Test

**Script:** `code/tools/train_smoke_test.py`

Validates that `DigimonEnv` works end-to-end with a manual random-action loop and with SB3 MaskablePPO. Does not validate rule correctness — only checks that the environment initializes, steps without crashing, and produces valid observations and masks.

```bash
python code/tools/train_smoke_test.py
```

Requires `stable-baselines3` and `sb3-contrib`.

---

### 6.4 Tensor Profile Gauntlet

**Script:** `code/tools/profile_tensor_profiles.py`

Compares board-state tensor profiles with fixed-seed RL profiling metrics:

```powershell
python code/tools/profile_tensor_profiles.py --profiles compact_v1,standard_lite_v2,standard_full_v2 --games 100 --seeds 1000:1100 --policy greedy --out profile_runs/tensor_profiles/latest --require-profiles
```

The default profile set is:

- `compact_v1`, reported as canonical `standard_compact_v1`
- `standard_lite_v2`
- `standard_full_v2`

The gauntlet writes `result.json` and `result.md`. Each profile row includes:

- Steps/sec
- Games/hour
- Win rate versus greedy
- Trigger-order signal accuracy
- Tensor bytes
- Rollout observation memory estimates

For a quick smoke run, use one game and a short step cap:

```powershell
python code/tools/profile_tensor_profiles.py --profiles compact_v1,standard_lite_v2,standard_full_v2 --games 1 --seeds 301:302 --policy greedy --max-steps-per-game 50 --out profile_runs/tensor_profiles/smoke --require-profiles
```

---

### 6.5 Pilot Training

Pilot training defaults to the Rust-backed `standard_lite_v2` observation profile, an `8320`-float fair-information tensor. Pass the profile explicitly in reproducible runs:

```bash
DIGIMON_BACKEND=rust python -m digimon_gym.agents.pilot_training --timesteps 500000 --tensor-profile standard_lite_v2
DIGIMON_BACKEND=rust python -m digimon_gym.agents.pilot_training --lstm --timesteps 500000 --tensor-profile standard_lite_v2
DIGIMON_BACKEND=rust python -m digimon_gym.agents.pilot_training --gauntlet --timesteps 500000 --tensor-profile standard_lite_v2
```

Use `--tensor-profile standard_compact_v1` only for compact compatibility or baseline comparisons.

Profile the opt-in experimental full v2 tensor against the default lite v2 profile before using it for long runs:

```powershell
$env:DIGIMON_BACKEND='rust'
python -m digimon_gym.agents.pilot_training --tensor-profile standard_full_v2 --timesteps 10000
```

Compare `standard_full_v2` against `standard_lite_v2` by both wall-clock throughput and sample efficiency. Full v2 adds `action_id_features[2192][16]`, so fewer steps to learn may still lose overall if environment steps or policy updates slow down enough.

---

## 7. Engine Modules

These modules live inside the engine and are used at runtime as well as by the tools above.

### 7.1 Card Feature Vectorizer

**Module:** `code/engine_py_legacy/engine/data/card_features.py`

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

### 7.2 Card Registry

**Module:** `code/engine_py_legacy/engine/data/card_registry.py`

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

### 7.3 Tensor Profile Metadata

**Default pilot profile:** `code/digimon-engine/src/tensor_profiles/standard/v2_lite.rs`

The Rust tensor profile registry describes which positions in each observation tensor hold card IDs vs scalar values. The default pilot observation profile is `standard_lite_v2`, an `8320`-float fair-information tensor exposed to Python by `digimon_engine.get_observation_layout("standard_lite_v2")` and consumed through `digimon_gym.tensor_profiles.get_tensor_profile("standard_lite_v2")`. `standard_compact_v1` remains the `1375`-float compact compatibility and baseline profile; `standard_v1` and `compact_v1` are accepted only as legacy aliases.

| Name | Value | Description |
|---|---|---|
| `card_id_positions` | list of 542 ints for `standard_lite_v2` | Tensor indices holding card IDs |
| `scalar_positions` | list of 7778 ints for `standard_lite_v2` | Tensor indices holding scalar values |
| `card_id_slot_count` | 542 for `standard_lite_v2` | Length of `card_id_positions` |
| `scalar_slot_count` | 7778 for `standard_lite_v2` | Length of `scalar_positions` |

All positions are computed deterministically from profile-owned sections, slot header fields, source fields, and source stride metadata using named Rust tensor offsets. Each profile asserts that card + scalar positions sum to that profile's `tensor_size`. The module-level `TENSOR_SIZE` remains the compact `1375` compatibility constant, not the default pilot observation size.

`standard_lite_v2` card ID positions include:
- Permanent top-card IDs.
- Permanent source-card IDs.
- Own hand card IDs.
- Known public zone card IDs.
- Pending-choice source-card IDs.

```python
from digimon_gym.tensor_profiles import get_tensor_profile

profile = get_tensor_profile("standard_lite_v2")

# Used internally by CardEmbeddingExtractor
card_ids = observations[:, profile.card_id_positions].long()   # (batch, 542)
scalars = observations[:, profile.scalar_positions]             # (batch, 7778)
```

`digimon_engine.get_tensor_profile()` remains available for compact `standard_compact_v1` compatibility. `code/engine_py_legacy/engine/data/tensor_layout.py` remains a legacy fallback only while migration/parity support remains.

---

## 8. New Card Set Workflow

When a new Digimon TCG set releases, follow these steps:

### Step 1: Preview, Stage, and Review Card Metadata

```bash
# Optional secondary-source diagnostic; never writes or admits cards
python code/tools/ingest_cards.py --preview-set BT26

# Publish only a complete canonical candidate through plan then reviewed apply
python -m tools.card_data sync --plan \
  --repo-root . --candidate-dir <candidate-dir> \
  --source-plan <source-plan.json> --plan-file <publication-plan.json>
python -m tools.card_data sync --apply \
  --repo-root . --candidate-dir <candidate-dir> \
  --plan-file <publication-plan.json>
```

Do not use the retired `ingest_cards.py --set` or `--bulk` writers. Official
observations, completeness checks, reviewed corrections, compatibility
artifacts, and fixed-path publication belong to the canonical revision.
Run `python -m tools.card_data.generate --check` for the network-free freshness
gate. It regenerates from the marker-owned canonical, source, correction,
registry, and narrow compatibility-projection inputs;
`data/cards.json` is only the checked output, never its own generation input.

### Step 2: Assign Stable Registry Indices

Stable assignments are part of the complete canonical candidate. Review their
append-only allocation in the publication plan, then publish them with the same
`sync --apply` shown in Step 1. The compatibility wrapper is useful only for
network-free verification:

```bash
python code/tools/build_registry.py --check
python code/tools/build_registry.py --dry-run
```

New admitted canonical IDs receive indices after the current maximum; existing
indices are never reassigned. Direct fixed-path writes, `--sets`, and `--force`
are refused.

### Step 3: Implement Card Effects with the Rust DSL

Card scripts are primarily authored as YAML specs under `code/digimon-engine/cards/<set>/<CARD_ID>.yaml`, with long-tail bespoke behavior routed through named `raw_rust` functions under `code/digimon-engine/src/cards/raw_rust/`. Use the `batch-implement-cards-rust-dsl` workflow to dispatch the TDD pipeline against the new set, or `assess-rust-engine-archetype` to pre-flight the DSL and engine primitives the set will require.

```bash
# Optional pre-flight: see which DSL and engine primitives the set needs
assess-rust-engine-archetype BT26

# TDD-driven YAML DSL implementation in batches
batch-implement-cards-rust-dsl BT26
```

The C# files at `DCGO/Assets/Scripts/CardEffect/{SET}/{COLOR}/{CARD_ID}.cs` remain available as behavioral implementation references; official printed text and rules govern.

### Step 4: Verify Frozen Integrity (Python sunset only)

```bash
python code/tools/check_frozen_integrity.py
```

Only relevant if Python card scripts were touched. The Python script lane is being sunset alongside the Rust engine migration.

### Step 5: Regenerate Warm-Start Embeddings (Optional)

```bash
python -m tools.train_card_autoencoder
```

Retrains the autoencoder on all cards including the new set. Produces updated `card_embeddings.npy` for warm-starting future training runs.

### Step 6: Update Pinecone Index (Optional)

```bash
python code/tools/ingest_pinecone.py --namespace card-scripts --set bt26
python code/tools/ingest_pinecone.py --namespace card-metadata
```

Makes the new scripts and card metadata searchable by sub-agents.

### Step 7: Train New Pilot

```bash
DIGIMON_BACKEND=rust python -m digimon_gym.agents.pilot_training --timesteps 500000 --tensor-profile standard_lite_v2
```

`standard_lite_v2` is the default pilot profile, but passing it explicitly keeps run logs and copy/pasted commands unambiguous. Use `DIGIMON_BACKEND=rust` for v2 pilot training; `standard_compact_v1` remains available for compact compatibility and baseline runs.

The `CardEmbeddingExtractor` automatically loads `card_embeddings.npy` if present and uses it to initialize the `nn.Embedding` table. New card indices start with warm-start embeddings instead of random noise.

### What Happens to Old Agents

Old pilots still work — their saved model checkpoint contains the `nn.Embedding` table with weights for the cards they were trained on. New card indices will have untrained embedding rows, which is expected since the pilot needs retraining for new cards anyway.

---

## 9. Archive

**Directory:** `code/tools/archive/`

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
