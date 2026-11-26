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

#[test]
fn ex12_076_dp_debuff_is_continuous_and_deletes_later_played_opponent_digimon_before_on_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-076 YAML loads")
        .dsl_card("BT24-034")
        .expect("BT24-034 YAML loads")
        .add_card(plain_digimon("RED-SRC", CardColor::Red, 6, 12000))
        .add_card(plain_digimon("BLUE-SRC", CardColor::Blue, 6, 12000))
        .add_card(plain_digimon("P1-SEC", CardColor::Yellow, 3, 2000))
        .hand(1, &["BT24-034"])
        .security(1, &["P1-SEC"])
        .memory(10)
        .start();

    let susanoomon = runner.place_stack(0, &["RED-SRC", "BLUE-SRC", CARD_ID]);
    fire_when_digivolving(&mut runner, susanoomon);
    runner.auto_resolve().expect("resolve EX12-076 continuous DP wave");

    // Later in the same turn, P1 plays BT24-034 (5000 DP) by effect (e.g. <Decode>).
    // The -6000 DP blanket applies immediately, so 0-DP rule processing deletes
    // BT24-034 before its [On Play] can activate.
    runner.game.play_from_hand_with_cost(
        1,
        0,
        digimon_engine::enums::CostDelta::Free,
        digimon_engine::enums::PlaySource::ByEffect,
    );

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "BT24-034 (5000 DP - 6000 continuous = 0 DP) must be deleted by rule check"
    );
    assert!(
        runner.pending_selection().is_none(),
        "deleted BT24-034's [On Play] must not activate after 0-DP rule deletion"
    );
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BT24-034"),
        "BT24-034 was sent to P1's trash"
    );
}
