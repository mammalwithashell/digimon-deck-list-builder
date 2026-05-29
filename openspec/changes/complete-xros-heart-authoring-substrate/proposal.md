## Why

The resolved Xros Heart deck pool is now clear: the first two Xros Heart
substrate changes closed base DigiXros and the first reusable under-Tamer/source
flow wave, but the remaining high-frequency BT19 and AD1 cards still need a few
substrate primitives before they can be authored without raw Rust or hidden
approximations. Addressing those primitives now lets Xros Heart authoring proceed
as ordinary YAML work while preserving action-mask visibility for every player
choice.

## What Changes

- Add source-zone effect digivolve support for effects that digivolve a Digimon
  using a named or predicate-matched card stored under one of the player's
  Tamers.
- Add stack-derived selector and formula primitives for source-count targeting,
  source-color counting, and comparisons against the acting Digimon's current
  DP.
- Add temporary effect lockout primitives for effects that prevent selected
  Digimon or Tamers from activating specific timing-triggered effects and/or
  unsuspending until a printed expiry.
- Add reveal-pool free-play routing so selected revealed cards can be played
  declaratively without hidden hand/search approximations.
- Extend DSL vocabulary so the above behaviors can be authored declaratively and
  rejected explicitly when unsupported.
- Use the remaining Xros Heart deck-pool cards as acceptance fixtures, led by
  `BT19-008`, `BT19-057`, `BT19-014`, `BT19-038`, `BT19-051`, `BT19-035`,
  `AD1-006`, `AD1-013`, `BT19-079`, `BT19-026`, and `BT21-030`.
- Preserve `ACTION_SPACE_SIZE`, active tensor profiles, PyO3 action contracts,
  and frontend action constants.

## Capabilities

### New Capabilities

- `source-zone-effect-digivolve`: Effect-initiated digivolution from cards stored
  in source-like zones such as cards under Tamers.
- `stack-derived-effect-metrics`: Selectors and formulas that inspect source
  stacks, count source colors, compare against current DP, or target by fewest
  sources.
- `temporary-effect-lockouts`: Temporary status modifiers that suppress printed
  timing-effect activation and/or unsuspend behavior until an explicit expiry.

### Modified Capabilities

- `dsl-card-scripting-vocabulary`: Add declarative authoring vocabulary for
  source-zone effect digivolve, reveal-pool free play, stack-derived metrics,
  and temporary lockouts.

## Impact

- Affected code: `code/digimon-engine/src/effect_context/`,
  `code/digimon-engine/src/dsl_cards/`, `code/digimon-engine/src/game_actions.rs`,
  `code/digimon-engine/src/combat.rs`, `code/digimon-dsl/`, and card YAML under
  `code/digimon-engine/cards/`.
- Affected tests: Rust behavioral tests for representative Xros Heart fixtures
  and DSL parser/lowering tests for the new vocabulary.
- Affected docs/gap trackers: `docs/RUST_ENGINE_GAPS.md`,
  `qa/archetype-qa/engine-gaps.md`, `qa/dsl-vocab-gaps.md`, and Xros Heart QA
  deck-pool/readiness notes.
- Compatibility: no action-space or tensor-profile contract changes are
  authorized by this change.
