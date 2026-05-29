## Why

Xros Heart is not implementable faithfully in the Rust engine today because its central game action is not just "play a Digimon with alternate requirements." Printed Xros Heart cards require a DigiXros play transaction: the player may select recipe materials from hand, battle area, trash, and cards under Tamers; those selections reduce play cost; selected materials become the played Digimon's sources; and related effects can modify the transaction before cost is paid.

Current Rust support has useful pieces such as `<Save>`, placing cards under Tamers, basic source selection, alternate paths, and keyword grants, but it lacks a native DigiXros transaction, cast-time material hooks, correct `<Material Save>` timing, and DSL vocabulary for authoring those behaviors without raw Rust. DCGO's implementation shows the missing shape clearly: it centralizes DigiXros selection, temporary zone/count modifiers, pre-attached materials, and cost calculation before the play cost is fixed.

## What Changes

- Add a Rust engine DigiXros play transaction that owns recipe matching, material selection, cost reduction, source attachment, and DigiXros resolution context.
- Add cast-time hooks for effects that modify a pending DigiXros play before cost is paid, including Taiki-style under-Tamer material access and Superior Mode-style pre-attachment plus trash access.
- Correct `<Material Save X>` so it is an optional deletion/removal replacement-style keyword over the printed DigiXros recipe materials, not a `[Main]` activated effect.
- Extend the YAML DSL so Xros Heart cards can declare DigiXros recipes, material zones, transaction modifiers, pre-attached materials, and Material Save behavior declaratively.
- Add focused Rust behavioral tests for BT10-009, BT10-087, BT12-112, and BT10-013 as the initial no-approximations acceptance pool.
- Preserve the current action-space and observation contracts. Any player choice introduced by this change must use existing pending-selection/action-mask machinery.

## Capabilities

### New Capabilities

- `digixros-execution`: The Rust engine can resolve a DigiXros play as a first-class transaction with material choices, cost reduction, source attachment, and transaction-local context.

### Modified Capabilities

- `dsl-card-scripting-vocabulary`: YAML card specs can author DigiXros recipes and transaction modifiers without raw Rust placeholders.
- `permanent-deletion-semantics`: Deletion/removal handling supports `<Material Save X>` and source-rescue flows using deletion snapshots and pending selections.

## Impact

- Affected code: `code/digimon-engine/src/game_actions.rs`, `code/digimon-engine/src/cards/keyword_effects.rs`, `code/digimon-engine/src/effect_context/`, `code/digimon-engine/src/card_spec.rs`, `code/digimon-engine/src/dsl/`, `code/digimon-dsl/`, and `code/digimon-engine/cards/`.
- Affected tests: Rust engine behavioral tests under `code/digimon-engine/tests/`, DSL lowering/authoring tests, and archetype QA fixtures for Xros Heart.
- Affected docs/gap trackers: `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, `qa/dsl-vocab-gaps.md`, and Xros Heart archetype QA notes.
- Compatibility: no `ACTION_SPACE_SIZE`, active tensor profile, PyO3 action contract, or frontend action constants should change as part of this work.
