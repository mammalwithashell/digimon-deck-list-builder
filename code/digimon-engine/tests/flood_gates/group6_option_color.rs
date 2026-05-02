use digimon_engine::action::PLAY_HAND_START;
use digimon_engine::build_action_mask;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{Expiry, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;

const P_206_YAML: &str = include_str!("../../cards/p/P-206.yaml");

fn runner_with_p206_in_hand() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(P_206_YAML)
        .expect("P-206 DSL fixture must compile")
        .hand(0, &["P-206"])
        .memory(10)
        .start()
}

fn install_ignore_color_requirement(runner: &mut DebugRunner) {
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::IgnoreColorRequirement,
            0,
            Expiry::EndOfTurn,
            None,
            0,
        ),
    );
}

#[test]
fn player_modifier_bypasses_option_color_requirement_in_mask() {
    let mut runner = runner_with_p206_in_hand();
    assert_eq!(runner.game.turn_player(), 0);
    assert_eq!(runner.game.memory, 10);
    assert_eq!(runner.game.player(0).battle_area.len(), 0);

    let before = build_action_mask(&runner.game, 0);
    assert_eq!(
        before[PLAY_HAND_START as usize], 0.0,
        "P-206 should not be usable from hand without matching board color or bypass"
    );

    install_ignore_color_requirement(&mut runner);

    let after = build_action_mask(&runner.game, 0);
    assert_eq!(
        after[PLAY_HAND_START as usize], 1.0,
        "IgnoreColorRequirement should make the Option hand-play bit legal"
    );
}

#[test]
fn decode_rejects_option_without_color_or_bypass_and_accepts_with_bypass() {
    let mut runner = runner_with_p206_in_hand();

    runner.game.decode_action(PLAY_HAND_START, 0);
    assert_eq!(
        runner.game.player(0).hand.len(),
        1,
        "decoder must reject an Option without matching board color or bypass"
    );

    install_ignore_color_requirement(&mut runner);

    runner.game.decode_action(PLAY_HAND_START, 0);
    assert_eq!(
        runner.game.player(0).hand.len(),
        0,
        "decoder should use the Option when IgnoreColorRequirement is active"
    );
}
