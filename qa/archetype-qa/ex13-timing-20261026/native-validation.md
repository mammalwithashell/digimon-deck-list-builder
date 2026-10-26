# Native validation receipt

Completed suites: **1156 passed, 5 pre-existing ignored**. No unresolved failure in these scoped completed runs.

Of those, 1023 passes used the final library. The other 133 integration passes precede only the native Delay incarnation guard, as recorded in the JSON receipt.

| Suite | Passed | Ignored | Filtered | Log |
| --- | ---: | ---: | ---: | --- |
| combat | 299 | 1 | 0 | `combat-final.log` |
| dsl_focused | 102 | 0 | 1495 | `dsl-focused-final.log` |
| cards_behavioral_35_modules | 457 | 4 | 12312 | `cards-behavioral-final.log` |
| option_flow | 143 | 0 | 0 | `timing-options-final-verified.log` |
| training_pool_timing_audit | 22 | 0 | 0 | `timing-options-final-verified.log` |
| archetype_audit_regressions (before final native guard) | 12 | 0 | 0 | `native-integration.log` |
| effects (before final native guard) | 18 | 0 | 0 | `native-integration.log` |
| end_of_attack_scope (before final native guard) | 5 | 0 | 0 | `native-integration.log` |
| event_emission (before final native guard) | 25 | 0 | 0 | `native-integration.log` |
| phase_flow (before final native guard) | 73 | 0 | 0 | `native-integration.log` |

The full DSL attempt was deliberately interrupted after 478 passing tests to prioritize the native repair and focused validation. Its process-termination result is retained; it is not a full-suite pass. The two initial combat failures were broad timing witness fixtures and are fixed; assertions and attack paths were retained.

Four ignored card tests belong to BT21-029; the combat suite has one other existing ignored test. See the JSON receipt for exact test names.

The 35 selected card modules include all 22 authored EndOfAttack cards, the four corrected event-kind cards, Asuna, Bacchusmon, five authored Scramble cards, and EX3-024/ST24-14 controls. Passing modules do not constitute complete card or archetype certification.

No training/runtime deployment or action/tensor contract changed.
