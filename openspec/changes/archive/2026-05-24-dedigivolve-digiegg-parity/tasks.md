## 1. Pin Current Behavior

- [x] 1.1 Add a focused BT24-041 regression with an opponent stack `[Digi-Egg Lv2, Lv3, Lv4]` and a computed De-Digivolve amount greater than 1.
- [x] 1.2 Assert the regression fails on current behavior by showing the Lv3 is trashed or a bare Digi-Egg appears in battle area.
- [x] 1.3 Add a shared De-Digivolve primitive/invariant test proving standard `stop_at_level: 3` does not trash a Lv3 with a Digi-Egg beneath it.
- [x] 1.4 Add a defensive cleanup test that intentionally exposes a Digi-Egg as battle-area top and expects the permanent to leave the field.

## 2. Align Standard De-Digivolve Authoring

- [x] 2.1 Update DSL lowering so authored `de_digivolve` steps, including BT24-041's formula-valued step, default to the standard `stop_at_level: 3`.
- [x] 2.2 Audit `code/digimon-engine/cards/**/*.yaml` for standard printed `<De-Digivolve>` steps missing `stop_at_level: 3`.
- [x] 2.3 Preserve intentionally unbounded stack-trash effects through explicit raw Rust/helper calls instead of standard DSL lowering.
- [x] 2.4 Update BT24-041 structural tests to assert the compiled `CompiledStep::DeDigivolve` includes `stop_at_level: Some(3)`.

## 3. Implement Digi-Egg Battle-Area Cleanup

- [x] 3.1 Add or reuse a shared Rust helper that detects battle-area permanents topped by `CardKind::DigiEgg` after stack/source mutation.
- [x] 3.2 Route exposed battle-area Digi-Egg permanents through the rules-aligned leave-field cleanup, moving the Digi-Egg top card to its owner's trash and removing the slot.
- [x] 3.3 Call the helper from `EffectContext::de_digivolve` after each mutation or before returning from a mutation that can expose a new top card.
- [x] 3.4 Audit nearby source-removal helpers and wire the cleanup into any path that can expose a Digi-Egg top in battle area.
- [x] 3.5 Preserve breeding-area Digi-Egg behavior and Digi-Egg-as-source behavior.

## 4. Verify Engine and Debug Surfaces

- [x] 4.1 Run focused Rust behavioral tests for De-Digivolve and BT24-041.
- [x] 4.2 Run the broader affected `cards_behavioral` filters for known De-Digivolve cards such as BT23-096, EX9-013, and BT9-112.
- [x] 4.3 Rebuild the debug MCP binary if required and replay a TS Olympos versus Imperialdramon smoke scenario through the MCP.
- [x] 4.4 Confirm state views, action masks, and legal actions no longer expose a bare Digi-Egg in battle area after the tested stack mutation.

## 5. Documentation and Closeout

- [x] 5.1 Update Rust parity/gap notes if the implementation discovers additional standard De-Digivolve YAML omissions.
- [x] 5.2 Record DCGO reference points used for the implementation in the final summary or relevant parity note.
- [x] 5.3 Run `openspec status --change dedigivolve-digiegg-parity` and confirm the change remains apply-ready.
