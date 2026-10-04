//! BT20-055 Invisimon

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_engine::action::space::REPLACEMENT_ACCEPT;
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
    // Rule 15-14-5: {Security} effects are active only while FACE UP.
    let idx = runner.game.players[0]
        .security
        .iter()
        .find(|c| c.card_id(&runner.game.card_data) == "BT20-055")
        .map(|c| c.card_index)
        .expect("BT20-055 in security");
    runner.game.players[0].face_up_security.insert(idx);

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

#[test]
fn bt20_055_on_play_flips_opponent_top_face_down_security_face_up() {
    let filler = ["BT1-010"; 5];
    let mut runner = bt20_055_runner()
        .hand(0, &["BT20-055"])
        .security(1, &["BT1-010", "BT1-011"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(11)
        .start();
    let top_security = runner.game.players[1].security[1].handle();

    runner.play(0, 0).expect("play BT20-055");
    runner
        .auto_resolve()
        .expect("BT20-055 On Play body resolves");

    assert!(
        runner.game.players[1]
            .face_up_security
            .contains(&top_security.0),
        "BT20-055 should flip the opponent's top face-down security card face-up"
    );
    assert!(
        runner.pending_selection().is_none(),
        "the security flip rider is deterministic and must not prompt"
    );
}

#[test]
fn bt20_055_face_up_security_check_may_place_top_stacked_card_bottom_security_face_up() {
    let filler = ["BT1-010"; 5];
    let mut runner = bt20_055_runner()
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .security(1, &["BT1-011"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(0)
        .start();
    let invisimon = runner.place_stack(0, &["BT1-010", "BT20-055"]);
    // "This Digimon's top stacked card" = BT20-055 itself (DCGO
    // `AddSecurityCard(card, toTop: false, faceUp: true)`) —
    // G-TOP-STACKED-CARD-TO-SECURITY.
    let moved_source =
        runner.game.players[0].battle_area[invisimon.index as usize].card_sources[1].handle();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let checked_security = runner.game.players[1].security[0].handle();
    runner.game.players[1]
        .face_up_security
        .insert(checked_security.0);

    let _ = runner.attack_player(attacker, 1, true);

    let pending = runner
        .pending_selection()
        .expect("BT20-055 optional security-placement trigger");
    assert_eq!(pending.valid_action_ids, vec![REPLACEMENT_ACCEPT]);
    assert_eq!(pending.selecting_player, 0);

    runner
        .game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept BT20-055 optional trigger");
    runner
        .auto_resolve()
        .expect("finish BT20-055 security-placement trigger");

    assert_eq!(
        runner.game.players[0]
            .security
            .first()
            .expect("bottom security")
            .handle(),
        moved_source,
        "BT20-055 (its own top stacked card) should move to bottom security"
    );
    assert!(
        runner.game.players[0]
            .face_up_security
            .contains(&moved_source.0),
        "the moved BT20-055 should be face-up in security"
    );
    let remaining = &runner.game.players[0].battle_area[invisimon.index as usize].card_sources;
    assert_eq!(
        remaining.len(),
        1,
        "the Digimon stays after moving its top stacked card"
    );
    assert_eq!(
        remaining[0].card_id(&runner.game.card_data),
        "BT1-010",
        "the card that was under BT20-055 is the new top"
    );
}

fn bt20_055_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT20-055")
        .expect("BT20-055 YAML parses and compiles")
        .add_card(make_test_card("BT1-010", "Filler A"))
        .add_card(make_test_card("BT1-011", "Filler B"))
}
