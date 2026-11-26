//! ST25-01 Tokomon — Digi-Egg Lv.2 Red
//! Inherited: [Your Turn] While your opponent has 3 or more memory, this Digimon with [Gemmon] in its text gets +3000 DP.

use crate::alysion_common::{advance_turn, fixture, start};

#[test]
fn st25_01_inherited_dp_requires_opponent_3_memory_your_turn_and_gemmon_text() {
    let mut r = start(fixture(&["ST25-01", "ST25-02"]));
    let gemmon = r.place_stack(0, &["ST25-01", "ST25-02"]);
    let plain = r.place_stack(0, &["ST25-01", "PLAIN-5K"]);

    // Own turn, opponent has 0 memory (our memory = +10) -> no bonus.
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(gemmon), Some(1000));
    assert_eq!(r.effective_dp(plain), Some(5000));

    // Own turn, opponent has 2 memory (our memory = -2) -> still < 3.
    r.game.set_memory(-2);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(gemmon), Some(1000));

    // Own turn, opponent has 3 memory (our memory = -3) -> +3000 DP only on [Gemmon] in text.
    r.game.set_memory(-3);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(gemmon), Some(4000));
    assert_eq!(r.effective_dp(plain), Some(5000));

    // Opponent's turn -> inactive even if turn player has 3+ memory.
    advance_turn(&mut r);
    r.game.set_memory(5);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(gemmon), Some(1000));
}
