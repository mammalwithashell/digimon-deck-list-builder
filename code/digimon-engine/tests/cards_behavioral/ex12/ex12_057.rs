use digimon_engine::enums::{CardColor, CardKind, ModifierType};

use super::support::{
    field_contains, hand_index, plain_digimon, select_first_non_pass, DebugRunner,
};

const CARD_ID: &str = "EX12-057";

#[test]
fn ex12_057_on_play_may_play_paishu_token() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-057 YAML loads")
        .hand(0, &[CARD_ID])
        .memory(12)
        .start();

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-057");
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve token play");

    assert!(field_contains(&runner, 0, "TOKEN_PAISHU"));
    let token = runner.game.players[0]
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&runner.game.card_data) == "TOKEN_PAISHU")
        .expect("Paishu token");
    assert_eq!(
        token.top_card().card_kind(&runner.game.card_data),
        CardKind::Token
    );
}

#[test]
fn ex12_057_ally_played_de_digivolves_then_dp_minus() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-057 YAML loads")
        .add_card(plain_digimon("ALLY", CardColor::Black, 3, 3000))
        .add_card(plain_digimon("OPP-BASE", CardColor::Black, 3, 7000))
        .add_card(plain_digimon("OPP-SRC", CardColor::Black, 4, 5000))
        .add_card(plain_digimon("OPP-TOP", CardColor::Black, 5, 9000))
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    let opp = runner.place_stack(1, &["OPP-BASE", "OPP-SRC", "OPP-TOP"]);

    runner.fire_play_event_triggers(ally.player, ally.index as usize, true, false);
    select_first_non_pass(&mut runner);
    select_first_non_pass(&mut runner);
    runner
        .auto_resolve()
        .expect("resolve de-digivolve and DP minus");

    assert_eq!(
        runner.game.modifiers.sum(opp, ModifierType::ChangeDp),
        -6000
    );
}
