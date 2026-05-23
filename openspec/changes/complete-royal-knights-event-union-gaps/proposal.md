## Why

The Royal Knights closeout moved the archetype to full YAML/test coverage, but the remaining incomplete cards cluster around two reusable substrate families: event-context observers and cross-zone union/source flows. Capturing those as shared capabilities keeps the next work focused on engine/DSL primitives instead of one-off card patches.

## What Changes

- Add event-context coverage for Royal Knights observers that depend on security removed/added/trashed payloads, event-target self predicates, opponent-played level branches, same-level X digivolution triggers, effect-play origin, and all-turns filter verification.
- Add union/source flow coverage for heterogeneous selections across hand, trash, breeding-area sources, and existing sources, including source-placement costs, source-play effects, and binding played/placed cards for follow-up attach-self behavior.
- Use the new primitives to complete the Royal Knights cards currently blocked or partial because of those two families, with priority on `BT13-019`, `BT20-021`, `EX11-053`, `BT20-056`, `BT15-084`, `BT20-060`, `BT23-035`, `BT23-047`, `BT8-090`, `BT9-092`, `BT13-095`, `BT21-086`, `RB1-035`, and `EX11-069`.
- Keep aggregate/formula/security-option lifecycle gaps out of scope unless they are required to make an event-context or union/source target faithful.
- Preserve the no-approximations policy: every player-visible choice must flow through action masks or pending selections, with no hidden auto-selection.

## Capabilities

### New Capabilities

- `royal-knights-event-context-coverage`: Event payload, predicate, and observer behavior required by remaining Royal Knights effects.
- `royal-knights-union-source-flows`: Cross-zone union/source selection, cost, play, and attach behavior required by remaining Royal Knights effects.

### Modified Capabilities

- None.

## Impact

- Affected systems: `code/digimon-engine/` event dispatch, trigger evaluation, pending selections, action masking, and behavioral tests.
- Affected DSL surface: `code/digimon-dsl/` lowering and YAML vocabulary for event predicates, security event payloads, and union/source operations.
- Affected card specs: Royal Knights YAML under `code/digimon-engine/cards/`, especially the listed blocked and partial cards.
- Affected trackers: `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, `qa/dsl-vocab-gaps.md`, and Royal Knights archetype QA notes.
