use digimon_engine::enums::{CardColor, Keyword, ModifierType};

use super::support::{fire_when_digivolving, plain_digimon, select_first_non_pass, DebugRunner};

const CARD_ID: &str = "EX12-076";

#[test]
fn ex12_076_has_printed_keywords() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-076 YAML loads")
        .start();
    let susanoomon = runner.place_on_field(0, CARD_ID, Some(0));

    for keyword in [Keyword::Rush, Keyword::Raid, Keyword::Blocker] {
        assert!(runner.game.has_keyword(susanoomon, keyword), "{keyword:?}");
    }
}

#[test]
fn ex12_076_on_play_reduces_all_opponent_digimon_by_source_colors() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-076 YAML loads")
        .add_card(plain_digimon("RED-SRC", CardColor::Red, 6, 12000))
        .add_card(plain_digimon("BLUE-SRC", CardColor::Blue, 6, 12000))
        .add_card(plain_digimon("OPP-A", CardColor::Purple, 4, 7000))
        .add_card(plain_digimon("OPP-B", CardColor::Black, 4, 7000))
        .start();
    let susanoomon = runner.place_stack(0, &["RED-SRC", "BLUE-SRC", CARD_ID]);
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));

    fire_when_digivolving(&mut runner, susanoomon);
    runner.auto_resolve().expect("resolve DP wave");

    assert_eq!(
        runner.game.modifiers.sum(opp_a, ModifierType::ChangeDp),
        -6000
    );
    assert_eq!(
        runner.game.modifiers.sum(opp_b, ModifierType::ChangeDp),
        -6000
    );
}

#[test]
fn ex12_076_when_attacking_places_opponent_digimon_as_top_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-076 YAML loads")
        .add_card(plain_digimon("OPP", CardColor::Purple, 4, 7000))
        .add_card(plain_digimon("DEFENDER", CardColor::Purple, 4, 7000))
        .memory(16)
        .start();
    let susanoomon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP", Some(0));
    let defender = runner.place_on_field(1, "DEFENDER", Some(0));

    runner.attack_digimon(susanoomon, defender, false);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve attack effect");

    assert_eq!(runner.battle_area_size(1), 0);
    assert_eq!(
        runner.game.players[1]
            .security
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "OPP"
    );
}
