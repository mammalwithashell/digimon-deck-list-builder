## Why

The Xros Heart DigiXros substrate is now in place, but current competitive Xros
Heart lists still depend on reusable behaviors that sit around DigiXros rather
than inside the base transaction: cards moving under Tamers, playing cards from
under Tamers, leave-battle source rescue, wildcard material substitution, and
effect-driven attacks. Capturing these as reusable primitives keeps Xros Heart
authoring honest while also unblocking Blue Flare, Twilight, Bagra Army, and
future Save-era cards that use the same patterns.

## What Changes

- Add under-Tamer card-flow primitives for placing cards from hand/trash under
  a Tamer, selecting cards from under one or more Tamers, and playing selected
  cards from that stash with free or reduced costs.
- Add follow-up DigiXros transaction primitives for turn-scoped wildcard
  material substitution, where a specified card may replace one printed
  requirement for the next or current DigiXros.
- Add source-stack payoff primitives for leave-battle source rescue, moving all
  or filtered source cards under Tamers, counting moved sources for later cost
  reduction, and targeting opponent stack sources for trashing.
- Add event-driven attack primitives for effects that let a just-played or
  just-digivolved Digimon attack, or that unsuspend named bodies and then
  initiate an attack through pending selections.
- Extend YAML DSL vocabulary so the above behaviors can be authored without
  raw Rust placeholders, with Xros Heart cards serving as acceptance fixtures.
- Preserve action-space and tensor contracts. New player-visible choices must
  use existing pending-selection and action-mask surfaces.

## Capabilities

### New Capabilities

- `under-tamer-card-flow`: Moving cards into Tamer stacks, selecting cards from
  under Tamers, and playing or reusing those cards as game objects.
- `digixros-transaction-followups`: DigiXros transaction extensions beyond the
  base recipe flow, especially wildcard requirement replacement and scoped
  transaction modifiers.
- `source-stack-payoff-effects`: Source-stack movement, rescue, counting, and
  opponent stack-trashing effects that are not limited to Material Save's
  recipe-filtered case.
- `event-driven-attack-effects`: Effects that grant or initiate attacks outside
  ordinary attack selection, including immediate may-attack windows and
  effect-driven attack prompts.

### Modified Capabilities

- `dsl-card-scripting-vocabulary`: Add declarative authoring vocabulary for the
  new primitives and reject unsupported fields explicitly.
- `permanent-deletion-semantics`: Generalize snapshot-backed source rescue
  beyond recipe-filtered Material Save.

## Impact

- Affected code: `code/digimon-engine/src/effect_context/`,
  `code/digimon-engine/src/game_actions.rs`, `code/digimon-engine/src/combat.rs`,
  `code/digimon-engine/src/dsl_cards/`, `code/digimon-dsl/`, and card YAML under
  `code/digimon-engine/cards/`.
- Affected tests: Rust behavioral tests for representative Xros Heart cards and
  DSL parser/lowering tests for the new vocabulary.
- Affected docs/gap trackers: `docs/RUST_ENGINE_GAPS.md`,
  `qa/archetype-qa/engine-gaps.md`, `qa/dsl-vocab-gaps.md`, and Xros Heart QA
  readiness notes.
- Compatibility: no `ACTION_SPACE_SIZE`, active tensor profile, PyO3 action
  contract, or frontend action constants should change as part of this work.
