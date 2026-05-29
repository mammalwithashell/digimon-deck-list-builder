## Why

Mastemon is a high-priority yellow/purple DNA archetype, but the resolved `Mastemon (Tribal)` pool is not currently implementable in Rust DSL without approximations. The pool resolver now identifies 93 unique cards across 55 decklists, while the Rust lane has only partial coverage for the actual core and `EX6-029 Mastemon` is still a Blast DNA shell with its printed effect body stubbed.

## What Changes

- Add reusable DSL/engine substrate for Mastemon-style security placement and security-cost effects.
- Add or confirm effect-initiated digivolve/play flows from selected security cards for core yellow security-stack cards.
- Implement and test the core Mastemon Rust DSL card set from the resolved deck pool, starting with the boss line and best-deck staples.
- Keep every gameplay-affecting choice visible through existing action masks and pending selections.
- Do not expand `ACTION_SPACE_SIZE` or active tensor contracts as part of this change.

## Capabilities

### New Capabilities
- `mastemon-archetype-coverage`: Tracks readiness and behavioral coverage for the resolved Mastemon Rust DSL archetype pool.

### Modified Capabilities
- `dsl-card-scripting-vocabulary`: Add reusable DSL vocabulary for owner-routed permanent-to-security placement and security-stack cost/result gates needed by Mastemon cards.
- `security-card-effects`: Confirm and extend selected-security play/digivolve flows used by Mastemon support cards.

## Impact

- Rust DSL schema and lowering in `code/digimon-dsl/` and `code/digimon-engine/src/dsl_cards/`.
- Engine effect helpers in `code/digimon-engine/src/effect_context/` and related resolver code.
- Card YAML under `code/digimon-engine/cards/`.
- Behavioral tests under `code/digimon-engine/tests/`.
- Gap trackers in `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, and archetype QA notes.
- Resolved Mastemon deck pool in `qa/archetype-qa/mastemon-tribal/deck_pool.json`.
