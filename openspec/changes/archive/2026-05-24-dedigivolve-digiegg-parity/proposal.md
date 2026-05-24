## Why

TS Olympos testing exposed a De-Digivolve parity gap: Minervamon can peel an opponent's stack past level 3 and leave a bare Digi-Egg in the battle area. DCGO's `IDegeneration` model stops standard De-Digivolve at a level 3 top card, and the Rust engine needs the same invariant plus cleanup for any stack mutation that exposes a Digi-Egg as an illegal battle-area top card.

## What Changes

- Align standard De-Digivolve resolution with DCGO: standard `<De-Digivolve N>` effects stop before trashing a current level 3 top card.
- Update the DSL lowering for authored `de_digivolve` steps so omitted floors default to the standard level 3 floor, including BT24-041 Minervamon's formula-valued De-Digivolve tail.
- Add an engine invariant that any battle-area permanent whose top card becomes a Digi-Egg through De-Digivolve or related stack mutation automatically leaves the field through the rules/DCGO-aligned disposition.
- Add regression coverage for an `[egg, Lv3, Lv4]` stack targeted by Minervamon so the Lv3 remains and no bare egg appears.
- Add coverage for the defensive cleanup path where an effect exposes a Digi-Egg as battle-area top despite normal De-Digivolve floors.
- Use `DCGO/Assets/Scripts/Script/CardController.cs` `IDegeneration` and `DCGO/Assets/Scripts/CardEffect/BT24/Yellow/BT24_041.cs` as parity references; no production dependency on DCGO is added.

## Capabilities

### New Capabilities
- `dedigivolve-resolution-parity`: Standard De-Digivolve resolution and post-stack-mutation cleanup must match printed rules and DCGO shape, including the level 3 stop and Digi-Egg field-removal invariant.

### Modified Capabilities
- `dsl-card-scripting-vocabulary`: Formula-valued `de_digivolve` steps remain supported, but standard De-Digivolve card YAML must preserve normal stop-at-level caps when using `amount_fn`.

## Impact

- Affected Rust engine areas: `code/digimon-engine/src/effect_context/mod.rs`, battle-area/permanent cleanup helpers, and source/stack mutation call sites that can expose a new top card.
- Affected DSL/engine tests: `code/digimon-engine/tests/dsl/phase2c_permanent_mutations.rs`, `code/digimon-engine/tests/cards_behavioral/bt24/bt24_041.rs`, and shared De-Digivolve behavioral tests.
- Affected docs/spec tracking: OpenSpec capability specs, and any Rust engine gap/parity notes that currently imply formula-valued De-Digivolve is complete without the level 3 and Digi-Egg cleanup invariants.
- No action-space, tensor, model metadata, Python API, or frontend contract changes are expected.
