## Why

`unblock-mastemon-rust-dsl` made the resolved Mastemon best deck authorable, but the full `Mastemon (Tribal)` pool still has 57 cards without production Rust YAML and most of those have no behavioral tests. Several remaining cards are not just throughput work: they require reusable DSL/engine substrate for CS source-placement observers, choice-shaped security costs, aggregate play-cost budget selections, and conditional effect/attack locks.

## What Changes

- Extend Mastemon coverage from best-deck readiness to full resolved-pool readiness for all 93 unique `Mastemon (Tribal)` cards.
- Add or confirm reusable DSL/engine substrate for:
  - effect-created digivolution-source placement observer context used by the CS package;
  - top-or-bottom security-card costs that gate follow-up effects;
  - total play-cost budget selection for playing multiple cards from trash or other visible zones;
  - conditional attack/effect suppression for Venusmon-style locks;
  - temporary name/original-name mutation if needed by KingSukamon-style effects.
- Implement and test the remaining Mastemon full-pool cards only after the relevant substrate is proven by focused failing tests.
- Keep every gameplay-affecting decision visible through existing actions or `PendingSelection` surfaces.
- Do not expand `ACTION_SPACE_SIZE` or active tensor contracts unless a blocker proves unavoidable; if unavoidable, stop and split the action/tensor contract work into its own change.

## Capabilities

### New Capabilities
- `mastemon-full-pool-coverage`: Tracks full resolved-pool Rust DSL coverage for `Mastemon (Tribal)`, distinguishing completed best-deck readiness from remaining full-pool cards and substrate blockers.

### Modified Capabilities
- `dsl-card-scripting-vocabulary`: Add reusable DSL vocabulary/lowering for source-placement observers, choice-shaped security costs, aggregate play-cost budget selections, conditional locks, and temporary identity mutation as needed by the full pool.
- `security-card-effects`: Extend security-stack effect support for selected/position-choice security costs and effect-trashed security follow-up activation shapes used by non-best-deck Mastemon cards.

## Impact

- Rust DSL schema, compiler, validator, and lowering in `code/digimon-dsl/` and `code/digimon-engine/src/dsl_cards/`.
- Rust engine effect context, source-placement event dispatch, modifiers, selections, and security-stack helpers under `code/digimon-engine/src/`.
- Card YAML under `code/digimon-engine/cards/`.
- Behavioral and DSL tests under `code/digimon-engine/tests/`.
- Mastemon QA notes and gap trackers in `qa/archetype-qa/mastemon-tribal/`, `qa/dsl-vocab-gaps.md`, `qa/archetype-qa/engine-gaps.md`, and `docs/RUST_ENGINE_GAPS.md`.
- Resolver baseline from `code/tools/resolve_deck.py "Mastemon (Tribal)"` and `qa/archetype-qa/mastemon-tribal/deck_pool.json`.
