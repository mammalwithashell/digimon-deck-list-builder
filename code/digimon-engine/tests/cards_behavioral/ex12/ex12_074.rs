use crate::dsl_card_data::compiled;

use digimon_engine::enums::CardColor;
use digimon_engine::selection::OptionPlayResult;
use digimon_engine::selection::SelectionKind;

use super::support::{
    field_contains, hand_contains, hand_index, plain_digimon, select_first_non_pass,
    select_hand_card, tb_digimon, top_card_id, with_evo_cost, DebugRunner,
};

const CARD_ID: &str = "EX12-074";

#[test]
fn ex12_074_has_shambala_use_requirement() {
    let card = compiled(CARD_ID);
    let requirement = card
        .use_requirement
        .as_ref()
        .and_then(|pred| pred.any_field_permanent.as_ref())
        .expect("EX12-074 prints Use Req. ([Shambala] trait)");

    assert_eq!(
        requirement.predicate.trait_has.as_deref(),
        Some("Shambala"),
        "Use Req. should look for an own [Shambala] trait permanent"
    );
}

#[test]
fn ex12_074_main_moves_bottom_security_places_self_face_up_and_plays_shambala() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-074 YAML loads")
        .add_card(plain_digimon("WHITE", CardColor::White, 3, 1000))
        .add_card(plain_digimon("BOTTOM", CardColor::Yellow, 3, 3000))
        .add_card(tb_digimon("PLAY", CardColor::Yellow, 4, 5000))
        .hand(0, &[CARD_ID, "PLAY"])
        .security(0, &["BOTTOM"])
        .memory(10)
        .start();

    runner.place_on_field(0, "WHITE", Some(0));
    let option_slot = hand_index(&runner, 0, CARD_ID);
    assert_eq!(
        runner.game.play_option_from_hand(0, option_slot),
        OptionPlayResult::Pending
    );
    select_hand_card(&mut runner, 0, "PLAY");
    runner.auto_resolve().expect("resolve EX12-074 main");

    assert!(hand_contains(&runner, 0, "BOTTOM"));
    assert!(field_contains(&runner, 0, "PLAY"));
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        CARD_ID
    );
    let security_index = runner.game.players[0].security[0].card_index;
    assert!(
        runner.game.players[0]
            .face_up_security
            .contains(&security_index),
        "EX12-074 should be face-up in bottom security"
    );
}

#[test]
fn ex12_074_face_up_security_digivolves_attacking_shambala_from_hand_cost_reduced() {
    let evo = with_evo_cost(
        tb_digimon("SHAMBALA-EVO", CardColor::Yellow, 5, 7000),
        CardColor::Yellow,
        4,
        3,
    );

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-074 YAML loads")
        .add_card(tb_digimon("ATTACKER", CardColor::Yellow, 4, 5000))
        .add_card(evo)
        .add_card(plain_digimon("SEC", CardColor::Purple, 3, 3000))
        .hand(0, &["SHAMBALA-EVO"])
        .security(0, &[CARD_ID])
        .security(1, &["SEC"])
        .memory(5)
        .start();

    let security_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(security_index);
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    select_hand_card(&mut runner, 0, "SHAMBALA-EVO");
    runner
        .auto_resolve()
        .expect("resolve EX12-074 security digivolve");

    assert_eq!(
        top_card_id(&runner, 0, attacker.index as usize),
        "SHAMBALA-EVO"
    );
}

#[test]
fn ex12_074_security_plays_low_cost_shambala_from_hand_or_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-074 YAML loads")
        .add_card(plain_digimon("ATTACKER", CardColor::Purple, 4, 6000))
        .add_card(tb_digimon("SEC-PLAY", CardColor::Yellow, 4, 5000))
        .hand(1, &["SEC-PLAY"])
        .security(1, &[CARD_ID])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    let view = runner
        .pending_selection_view()
        .expect("EX12-074 security effect should prompt");
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { .. }),
        "security effect should use a hand-or-trash union-zone pick"
    );
    select_first_non_pass(&mut runner);
    runner
        .auto_resolve()
        .expect("resolve EX12-074 security free play");

    assert!(field_contains(&runner, 1, "SEC-PLAY"));
}
