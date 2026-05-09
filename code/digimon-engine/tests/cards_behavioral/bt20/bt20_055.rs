//! BT20-055 Invisimon

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_engine::debug_runner::{make_test_card, DebugRunner};

#[test]
fn bt20_055_security_end_of_opponents_turn_plays_self_from_security() {
    let filler = ["BT1-010"; 5];
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-055")
        .expect("BT20-055 YAML parses and compiles")
        .add_card(make_test_card("BT1-010", "Filler"))
        .security(0, &["BT20-055", "BT1-010"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(3)
        .start();

    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 1);
    assert!(
        runner.game.players[0].battle_area.is_empty(),
        "BT20-055 must wait until the opponent's turn ends"
    );

    runner.end_turn();

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "BT20-055"),
        "BT20-055 should play itself from security at end of opponent's turn"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        1,
        "only BT20-055 should leave security"
    );
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        "BT1-010",
        "the other security card should remain"
    );
}
