# Regression baseline (captured 2026-06-07, pre-refactor)

`cargo test --manifest-path code/digimon-engine/Cargo.toml`

**Bar:** introduce no NEW failures beyond these 7 pre-existing `cards_behavioral`
failures (all DP-modifier/aura behavior, unrelated to facade structure):

- bt21_072_all_turns_dp_bonus_applies_on_opponents_turn
- bt21_072_all_turns_dp_plus1000_with_one_digivolution_card
- bt21_072_all_turns_dp_plus2000_with_two_digivolution_cards
- bt21_072_all_turns_dp_recomputes_after_digivolution_card_removed
- ex7_030_when_attacking_gives_one_opponent_digimon_minus_6000_dp
- p_134_inherited_when_attacking_gives_minus_2000_dp
- p_197_inherited_when_attacking_gives_minus_2000_dp_once_per_turn

cards_behavioral: **3548 passed; 7 failed; 62 ignored**. All other binaries green.
Memory-extraction cross-check: identical 3548/7 → behavior-preserving confirmed.
