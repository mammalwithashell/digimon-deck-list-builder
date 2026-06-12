# tools

CLI utilities for the card pipeline, training, scenarios, ingest, and ops. Detailed reference: [`docs/TOOLS.md`](../../docs/TOOLS.md).

## Layout

- `archive/` — one-time migration scripts
- `dsl-lint/` — DSL linter (Cargo workspace member; consumes [`digimon-dsl`](../digimon-dsl/))
- `dsl-schema-export/` — DSL JSON-Schema generator (Cargo workspace member)
- `scraper/` — meta-deck scraping helpers

## Notable tools

### Card pipeline
- `ingest_cards.py` — refresh `data/cards.json` from upstream API
- `build_card_meta.py` — derive card-meta artifacts
- `build_registry.py` — build the card-script registry
- `build_tested_cards.py` — regenerate `data/tested_cards.json` from the Rust engine's implemented-card registry (`digimon-engine-cli pool`)
- `xros_req_parser.py` — parse the Xros / requirement clauses
- `transpile_dcgo.py` — generate Python script stubs from DCGO C# (sunset alongside the Python engine)
- `promote_script.py` / `check_frozen_integrity.py` — frozen-script promotion + integrity gate

### Decks / archetypes
- `resolve_deck.py`, `decklist_analysis.py`
- `rank_archetypes.py`, `backfill_deck_meta_tier.py`
- `build_review_batches.py`, `queue_review_batches.py`, `run_qa_batch.py`

### Pinecone
- `ingest_pinecone.py` — push `engine-api`, `card-scripts`, `card-metadata`, `rules-docs` namespaces
- `verify_pinecone.py` — index sanity check

### Models
- `export_onnx.py` — SB3 → ONNX (MLP / LSTM)
- `export_random_onnx.py`, `export_null_agent.py` — baselines
- `publish_model.py` — push to hosted API model store
- `publish_release_smoke.py` — smoke-check a published model
- `train_card_autoencoder.py`, `train_smoke_test.py`
- `run_training_job.py` — DB-backed training orchestration

### Scenarios
- `generate_scenarios.py` — author YAML scenarios from card text
- `run_scenario.py` — execute a single scenario

### Ops
- `provision_ci_release_user.py` — bootstrap CI release credentials
- `store_night.py` — late-night store ingestion job

## Pinecone reference

The `digimon-engine` Pinecone index has four namespaces (`engine-api`, `card-scripts`, `card-metadata`, `rules-docs`). See [`docs/TOOLS.md`](../../docs/TOOLS.md) §5 for ingestion + verification commands.
