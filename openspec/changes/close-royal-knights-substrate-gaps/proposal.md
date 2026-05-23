## Why

Royal Knights is close to becoming a faithful Rust/DSL archetype, but several cards still sit behind stale gap comments, raw Rust escape hatches, or small reusable primitives that affect player-visible choices. Closing these gaps now will turn the remaining King Drasil, Omnimon, Jesmon, and Gallantmon patterns into reusable substrate instead of one-off card work.

## What Changes

- Add reusable DSL/engine support for optional breeding-permanent selection so printed "you may" effects targeting a breeding-area King Drasil can expose a decline path.
- Use existing and newly completed source-selection primitives to migrate Royal Knights breeding-source plays from stubs/raw Rust to native YAML, including different-name source picks, On Play suppression, and Rush grants.
- Migrate budgeted opponent-target deletion patterns to native budgeted multi-select DSL where the primitive already exists.
- Add or verify card-shaped Royal Knights coverage for the remaining high-value cards whose substrate is already closed but whose YAML/tests have not caught up.
- Refresh gap trackers and stale comments so future work distinguishes true engine/DSL gaps from card-authoring backlog.

## Capabilities

### New Capabilities

- `royal-knights-substrate-closure`: Reusable Rust engine and DSL primitives needed by Royal Knights cards, especially breeding selection, breeding-source play, budgeted target selection, and event-bound keyword grants.
- `royal-knights-card-coverage`: Production Royal Knights YAML and behavioral coverage that consumes the closed primitives without hidden auto-selections or raw no-op placeholders.

### Modified Capabilities

- None.

## Impact

- Affected code: `code/digimon-engine/`, `code/digimon-dsl/`, Royal Knights YAML under `code/digimon-engine/cards/`, and behavioral tests under `code/digimon-engine/tests/`.
- Affected documentation and QA trackers: `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, `qa/archetype-qa/engine-gaps.md`, and `qa/archetype-qa/dsl/royal-knights-2026-05-03-dsl-engine-gaps.md`.
- Action/tensor contracts should not change unless a task explicitly discovers a new player-visible action range requirement; any such discovery must update `docs/ACTION_SPEC.md`, `docs/TENSOR_SPEC.md`, exports, wrappers, and metadata in the same change.
