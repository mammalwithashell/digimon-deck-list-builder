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

// ─── {Security} [Once Per Turn] (G-ENGINE-SECURITY-ZONE-OPT-UNTRACKED) ──────

/// P0: a face-up EX12-074 as its only security card, two [Shambala] attackers
/// and two [Shambala] digivolution targets in hand; P1: three plain security
/// cards. Returns the runner and the two attackers.
fn ex12_074_opt_board() -> (
    DebugRunner,
    digimon_engine::permanent::PermanentHandle,
    digimon_engine::permanent::PermanentHandle,
) {
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
        .hand(0, &["SHAMBALA-EVO", "SHAMBALA-EVO"])
        .security(0, &[CARD_ID])
        .security(1, &["SEC", "SEC", "SEC"])
        .deck(0, &["SEC"; 6])
        .deck(1, &["SEC"; 6])
        .memory(10)
        .start();
    let security_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(security_index);
    let a1 = runner.place_on_field(0, "ATTACKER", Some(0));
    let a2 = runner.place_on_field(0, "ATTACKER", Some(0));
    (runner, a1, a2)
}

/// CR 4.3 §15-14-1-1 / §15-14-1-2: EX12-074's {Security} [Your Turn]
/// [Once Per Turn] digivolve is used once, so a second [Shambala] attack the
/// same turn is not offered it, with a legal target still in hand.
#[test]
fn ex12_074_security_digivolve_is_offered_once_per_turn() {
    let (mut runner, a1, a2) = ex12_074_opt_board();
    runner.attack_player(a1, 1, false);
    select_hand_card(&mut runner, 0, "SHAMBALA-EVO");
    runner.auto_resolve().expect("resolve the first digivolve");
    assert_eq!(top_card_id(&runner, 0, a1.index as usize), "SHAMBALA-EVO");
    assert!(hand_contains(&runner, 0, "SHAMBALA-EVO"), "a second target remains");

    runner.attack_player(a2, 1, false);
    assert!(
        runner.game.players[0].battle_area[a2.index as usize].is_suspended,
        "the second [Shambala] attack was declared"
    );
    assert!(
        runner.pending_selection().is_none(),
        "the [Once Per Turn] is used: no second offer this turn: {:?}",
        runner.pending_selection()
    );
    assert_eq!(top_card_id(&runner, 0, a2.index as usize), "ATTACKER");
}

/// Official Q&A Q1818 / §15-14-1-4: declining the whole-effect "may" is
/// choosing not to activate it, so the next [Shambala] attack the same turn
/// is offered it again (the folded-"may" refund, for a {Security} clause).
#[test]
fn ex12_074_declined_security_digivolve_is_offered_again_same_turn_q1818() {
    let (mut runner, a1, a2) = ex12_074_opt_board();
    runner.attack_player(a1, 1, false);
    let sel = runner.pending_selection().expect("the first offer");
    assert!(sel.is_optional);
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline");
    runner.auto_resolve().expect("finish the first attack");
    assert_eq!(top_card_id(&runner, 0, a1.index as usize), "ATTACKER");

    runner.attack_player(a2, 1, false);
    assert!(
        runner.pending_selection().is_some(),
        "Q1818: the declined [Once Per Turn] effect is offered again"
    );
}

/// §15-14-1-5-1: the use count resets when the turn changes, so on P0's next
/// turn the [Shambala] attack is offered the digivolve again.
#[test]
fn ex12_074_security_digivolve_rearms_next_turn() {
    let (mut runner, a1, a2) = ex12_074_opt_board();
    runner.attack_player(a1, 1, false);
    select_hand_card(&mut runner, 0, "SHAMBALA-EVO");
    runner.auto_resolve().expect("resolve the first digivolve");

    runner.end_turn(); // -> P1
    runner.end_turn(); // -> P0
    assert_eq!(runner.game.turn_player(), 0);
    assert!(!runner.game.game_over);
    runner.attack_player(a2, 1, false);
    assert!(
        runner.pending_selection().is_some(),
        "a new turn re-arms the [Once Per Turn]"
    );
}
