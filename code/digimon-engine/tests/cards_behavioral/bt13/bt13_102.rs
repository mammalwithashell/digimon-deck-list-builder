//! BT13-102 Keenan Crier

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt13_102_loads_security_play_slice() {
    DebugRunner::builder()
        .dsl_card("BT13-102")
        .expect("BT13-102 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-OPPONENT-HIDDEN-HAND-CHOICE and G-EFFECT-PLAY-OBSERVER — On Play opponent choice and opponent-turn effect-play memory trigger"]
#[test]
fn bt13_102_hidden_hand_choice_and_effect_play_observer() {}
