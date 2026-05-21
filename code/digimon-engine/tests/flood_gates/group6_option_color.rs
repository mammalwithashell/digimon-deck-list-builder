use digimon_engine::action::PLAY_HAND_START;
use digimon_engine::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;

/// A plain white Option card with NO `use_requirement` — it genuinely needs a
/// matching-color board source (or a player-scoped `IgnoreColorRequirement`
/// modifier) to be playable from hand. P-206 is unsuitable as this fixture
/// because it now carries an unconditional intrinsic color-ignore
/// `use_requirement` (G-IGNORE-COLOR-MASK from-hand closure).
fn plain_white_option() -> CardData {
    let mut c = make_test_card("PLAIN-OPT", "Plain Option");
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::White];
    c.play_cost = 4;
    c
}

fn runner_with_plain_option_in_hand() -> DebugRunner {
    DebugRunner::builder()
        .add_card(plain_white_option())
        .hand(0, &["PLAIN-OPT"])
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
    let mut runner = runner_with_plain_option_in_hand();
    assert_eq!(runner.game.turn_player(), 0);
    assert_eq!(runner.game.memory, 10);
    assert_eq!(runner.game.player(0).battle_area.len(), 0);

    let before = build_action_mask(&runner.game, 0);
    assert_eq!(
        before[PLAY_HAND_START as usize], 0.0,
        "a plain white Option should not be usable from hand without matching board color or bypass"
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
    let mut runner = runner_with_plain_option_in_hand();

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
