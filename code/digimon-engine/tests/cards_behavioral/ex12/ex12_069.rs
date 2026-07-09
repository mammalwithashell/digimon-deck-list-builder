use digimon_engine::enums::CardColor;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

use super::support::{
    field_contains, hand_contains, hand_index, plain_digimon, select_first_non_pass,
    select_hand_card, vb_digimon, DebugRunner,
};

const CARD_ID: &str = "EX12-069";

#[test]
fn ex12_069_main_adds_bottom_security_to_hand_and_places_self_face_up_bottom_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-069 YAML loads")
        .add_card(vb_digimon("VB-ENABLER", CardColor::Yellow, 4, 5000))
        .add_card(plain_digimon("BOTTOM", CardColor::Yellow, 3, 2000))
        .hand(0, &[CARD_ID])
        .security(0, &["BOTTOM"])
        .memory(5)
        .start();

    runner.place_on_field(0, "VB-ENABLER", Some(0));
    let option_slot = hand_index(&runner, 0, CARD_ID);
    let result = runner.game.play_option_from_hand(0, option_slot);
    assert_ne!(result, OptionPlayResult::Invalid);
    runner.auto_resolve().expect("resolve Virus Busters main");

    assert!(hand_contains(&runner, 0, "BOTTOM"));
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        CARD_ID
    );
    let security_index = runner.game.players[0].security[0].card_index;
    assert!(
        runner.game.players[0]
            .face_up_security
            .contains(&security_index),
        "Virus Busters should be face-up in security"
    );
}

#[test]
fn ex12_069_face_up_security_plays_same_level_vb_from_hand_with_cost_reduced() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-069 YAML loads")
        .add_card(vb_digimon("ATTACKER", CardColor::Yellow, 4, 5000))
        .add_card(vb_digimon("SAME-LV4", CardColor::Yellow, 4, 5000))
        .add_card(vb_digimon("WRONG-LV5", CardColor::Yellow, 5, 7000))
        .add_card(plain_digimon("SEC", CardColor::Purple, 3, 2000))
        .hand(0, &["SAME-LV4", "WRONG-LV5"])
        .security(0, &[CARD_ID])
        .security(1, &["SEC"])
        .memory(5)
        .start();
    runner.set_turn(1);

    let security_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(security_index);
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let memory_before = runner.memory();

    runner.attack_player(attacker, 1, false);
    let offer = runner
        .pending_selection_view()
        .expect("face-up Virus Busters should offer play trigger");
    assert!(offer.is_optional);
    select_first_non_pass(&mut runner);

    let hand = runner
        .pending_selection_view()
        .expect("accepting should expose same-level VB hand cards");
    assert_eq!(hand.kind, SelectionKind::Hand);
    select_hand_card(&mut runner, 0, "SAME-LV4");
    runner.auto_resolve().expect("resolve face-up security play");

    assert!(field_contains(&runner, 0, "SAME-LV4"));
    assert!(
        !field_contains(&runner, 0, "WRONG-LV5"),
        "different-level VB card must not be selectable"
    );
    assert_eq!(memory_before - runner.memory(), 2, "cost 5 reduced by 3");
}

#[test]
fn ex12_069_security_plays_cost_5_or_lower_vb_from_hand_or_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-069 YAML loads")
        .add_card(plain_digimon("ATTACKER", CardColor::Purple, 4, 6000))
        .add_card(vb_digimon("SEC-PLAY", CardColor::Yellow, 4, 5000))
        .hand(1, &["SEC-PLAY"])
        .security(1, &[CARD_ID])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    let view = runner
        .pending_selection_view()
        .expect("Virus Busters security effect should prompt");
    assert!(matches!(view.kind, SelectionKind::UnionZone { .. }));
    select_first_non_pass(&mut runner);
    runner
        .auto_resolve()
        .expect("resolve Virus Busters security free play");

    assert!(field_contains(&runner, 1, "SEC-PLAY"));
}
