## 1. Baseline And Tests

- [x] 1.1 Reconfirm the DNA Omnimon pool and verify there are no live non-comment `kind: raw_rust` entries in its production YAML files.
- [x] 1.2 Inspect BT17-102 printed text, YAML, current ignored test, and DCGO `BT17_102.cs` name-overlay behavior.
- [x] 1.3 Inspect BT23-096 printed text, YAML, current ignored test, and DCGO `BT23_096.cs` ally-attack Delay behavior.
- [x] 1.4 Add or tighten failing behavioral coverage for BT17-102 dynamic source-name aliases, including at least one non-Koromon level-3-or-lower source name.
- [x] 1.5 Add or tighten failing behavioral coverage for BT23-096 Delay-on-ally-attack, including wrong trait or wrong turn negative coverage.

## 2. Dynamic Effective Names

- [x] 2.1 Add a reusable engine representation for permanent effective-name overlays sourced from digivolution cards matching a source filter.
- [x] 2.2 Ensure effective-name queries include printed names, existing static aliases/modifiers, and dynamic source-derived names without duplicating or recursing indefinitely.
- [x] 2.3 Route name predicates and rule helpers that care about a Digimon's names through the effective-name query.
- [x] 2.4 Add DSL parsing/compilation for the source-name overlay primitive.
- [x] 2.5 Update BT17-102 YAML to express its all-turns level-3-and-lower source-name alias declaratively and remove the hardcoded Koromon proxy where it no longer belongs.

## 3. Delay Attack Event Dispatch

- [x] 3.1 Extend Delay lowering so `on_attack`, `on_ally_attack`, and related spellings become event-backed delayed-option triggers.
- [x] 3.2 Carry attacker context through attack event dispatch so Delay active conditions can evaluate attacker predicates.
- [x] 3.3 Fan attack events into delayed-option enqueueing for the correct player and timing window.
- [x] 3.4 Ensure `attacker_trait_has` and related predicates evaluate against ordinary attack context, not only attack-target-change context.
- [x] 3.5 Update BT23-096 YAML to declare the `[Your Turn]` `[CS]` ally-attack Delay clause and reuse its printed de-digivolve body.

## 4. Verification And Trackers

- [x] 4.1 Re-enable the BT17-102 and BT23-096 behavioral tests that were ignored for the two open gaps.
- [x] 4.2 Run targeted Rust tests for BT17-102, BT23-096, DSL lowering, DNA digivolve, and affected name-predicate regressions.
- [x] 4.3 Run the broader Rust engine test slice needed to verify no regressions in card behavior or Delay dispatch.
- [x] 4.4 Update `validated_cards_dsl.json` so BT17-102 and BT23-096 move from `PARTIAL` to `IMPLEMENTED`.
- [x] 4.5 Move or annotate `G-DYNAMIC-NAME-ALIAS-FROM-STACK` and `G-DSL-DELAY-ON-ATTACK-EVENT` as resolved in `qa/dsl-vocab-gaps.md`, `docs/RUST_ENGINE_GAPS.md`, `qa/resolved-gaps.md`, and the DNA Omnimon archetype gap doc.
- [x] 4.6 Re-run the DNA Omnimon raw-Rust scan and record that the archetype still has zero live `raw_rust` escapes.
