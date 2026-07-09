use digimon_engine::enums::{CardColor, Keyword};

use super::support::{
    field_contains, hand_contains, plain_digimon, select_first_non_pass, DebugRunner,
};

const CARD_ID: &str = "EX12-024";

#[test]
fn ex12_024_on_play_returns_opponent_level_4_or_lower_to_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-024 YAML loads")
        .add_card(plain_digimon("OPP-L4", CardColor::Blue, 4, 4000))
        .start();

    let garurumon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-L4", Some(0));
    runner.fire_on_play(0, garurumon.index as usize);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve EX12-024 on-play");

    assert!(!field_contains(&runner, 1, "OPP-L4"));
    assert!(hand_contains(&runner, 1, "OPP-L4"));
}

#[test]
fn ex12_024_grants_printed_jamming() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-024 YAML loads")
        .start();
    let handle = runner.place_on_field(0, CARD_ID, Some(0));

    runner.game.tick_declarative_effects();
    assert!(runner.game.has_keyword(handle, Keyword::Jamming));
}
