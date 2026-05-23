## Why

BG Imperial's Rust DSL implementation is substantially complete, but the repository still carries stale `PARTIAL`, `BLOCKED`, approximation, and raw-rust references that contradict current YAML, tests, and resolved-gap notes. This creates false follow-up work and makes future archetype audits less trustworthy.

## What Changes

- Reconcile the BG Imperial card pool against `data/deck_library.json`, `qa/qa-reports/validated_cards_dsl.json`, YAML specs, behavioral tests, and DCGO reference notes.
- Update stale BG Imperial tracker and card comments so resolved gaps are no longer presented as open blockers.
- Correct ledger status for BG Imperial cards whose focused behavioral tests prove faithful implementation.
- Verify that the full BG Imperial pool has no live `raw_rust` YAML clauses or steps.
- Keep adjacent non-BG Imperial raw-rust work, such as `BT13-040 Magnamon`, explicitly out of scope unless a later change expands the target archetype.
- Preserve the no-approximations policy: every remaining player-visible choice must be implemented through DSL/engine selections and action masks, not hidden auto-selection or stubs.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `bg-imperial-archetype-coverage`: require post-closeout readiness reconciliation so the BG Imperial ledger, QA docs, card comments, and raw-rust audit match the current Rust DSL implementation state.

## Impact

- Affected documentation and QA files include BG Imperial archetype trackers under `qa/archetype-qa/`, `qa/dsl-vocab-gaps.md`, `qa/resolved-gaps.md`, and `qa/qa-reports/validated_cards_dsl.json`.
- Affected source comments and ignored-test annotations may include BG Imperial YAML and behavioral tests under `code/digimon-engine/cards/` and `code/digimon-engine/tests/cards_behavioral/`.
- No tensor, action-space, PyO3, frontend, or model metadata contracts are expected to change.
- No new `raw_rust` functions are introduced.
