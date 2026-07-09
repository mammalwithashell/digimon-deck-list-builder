use digimon_engine::enums::{CardColor, CardKind};

use super::support::{
    field_contains, hand_index, plain_digimon, select_first_non_pass, DebugRunner,
};

const CARD_ID: &str = "EX12-034";

#[test]
fn ex12_034_on_play_may_play_kotenken_token() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-034 YAML loads")
        .hand(0, &[CARD_ID])
        .memory(12)
        .start();

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-034");
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve token play");

    assert!(field_contains(&runner, 0, "TOKEN_KOTENKEN"));
    let token = runner.game.players[0]
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&runner.game.card_data) == "TOKEN_KOTENKEN")
        .expect("Kotenken token");
    assert_eq!(
        token.top_card().card_kind(&runner.game.card_data),
        CardKind::Token
    );
}

#[test]
fn ex12_034_ally_played_bottom_decks_opponent_lowest_level_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-034 YAML loads")
        .add_card(plain_digimon("ALLY", CardColor::Black, 3, 3000))
        .add_card(plain_digimon("LOW", CardColor::Black, 3, 3000))
        .add_card(plain_digimon("HIGH", CardColor::Black, 5, 7000))
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner.place_on_field(1, "LOW", Some(0));
    runner.place_on_field(1, "HIGH", Some(0));

    runner.fire_play_event_triggers(ally.player, ally.index as usize, true, false);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve bottom-deck");

    assert_eq!(runner.battle_area_size(1), 1);
    assert_eq!(
        runner.game.players[1]
            .deck
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "LOW"
    );
}
