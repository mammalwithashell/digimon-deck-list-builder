## Why

The pilot training CLI can train a fixed deck against varied opponents, but it does not define a reproducible path for pretraining a broad pilot that learns core gameplay across multiple fully implemented archetypes. A generalist base model should make later archetype-specific fine-tuning faster and easier to compare across tensor profiles.

## What Changes

- Add a generalist pilot training mode that samples the agent deck and opponent deck from fully implemented Rust DSL archetypes.
- Sample archetypes uniformly first, then sample a deck uniformly within the selected archetype, so archetypes with more decklists do not dominate pretraining.
- Add deterministic curriculum controls that separate model/training randomness from deck-pair sampling randomness.
- Add frozen deck-pool snapshots with stable content-addressed deck IDs so A/B runs remain comparable after `data/deck_library.json` changes.
- Validate every training deck source against the Rust implemented-card registry and fail fast with missing card IDs.
- Record generalist training mode, sampling policy, seeds, deck-pool snapshot hash, eligible archetypes, tensor profile, tensor version, and layout hash in model metadata.
- Document the recommended workflow for pretraining a generalist base model and fine-tuning it into an archetype specialist.

## Capabilities

### New Capabilities

- `generalist-pilot-pretraining`: Defines reproducible, implementation-safe pilot pretraining across multiple eligible Rust DSL archetypes, including seeded deck sampling, frozen deck-pool snapshots, metadata, and fine-tune handoff.

### Modified Capabilities

- None.

## Impact

- `code/digimon_gym/agents/pilot_training.py`: CLI flags, seed handling, metadata, model loading/fine-tune flow.
- `code/digimon_gym/agents/gauntlet.py`: shared eligible-deck pool behavior, stable deck IDs, snapshot generation/loading, uniform-archetype sampling support.
- `code/digimon_gym/agents/training_metrics.py`: metadata schema additions for generalist runs and curriculum provenance.
- `code/tests/rl/`: coverage for implemented-card validation, deterministic sampling, snapshot stability, CLI wiring, and metadata.
- `docs/TRAINING_RUNBOOK.md`: generalist pretraining and fine-tuning guidance.
