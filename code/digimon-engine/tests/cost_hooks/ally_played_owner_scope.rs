//! `when_any_ally_played` cost reducers are owner-scoped
//! (G-COST-REDUCTION-ALLY-PLAYED-OWNER).
//!
//! The BeforePayCost scan walks BOTH players' fields, so a field-hosted
//! "[Your Turn] When you would play a ... card" reducer used to be offered for
//! the OPPONENT's matching play too (BT14-046 Togemon: P1 playing a green
//! Tamer during P0's turn got P0's reducer prompt, with P1 selecting). The
//! played card must belong to the reducer's controller.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const REDUCER_YAML: &str = r#"
card: TEST-ALLY-REDUCER
name: Ally Reducer
kind: tamer
color: [green]
cost: 3
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    active_when: { your_turn: true }
    when_any_ally_played: { kind: digimon }
    optional: true
    amount: 2
    summary: "[Your Turn] When you would play a Digimon card, reduce the play cost by 2"
"#;

fn five_cost_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c
}

fn runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(REDUCER_YAML)
        .expect("inline reducer compiles")
        .add_card(five_cost_digimon("DIGI-5"))
        .hand(0, &["DIGI-5"])
        .hand(1, &["DIGI-5"])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.game.enter_main_phase();
    r.game.set_memory(10);
    r.place_on_field(0, "TEST-ALLY-REDUCER", Some(0));
    r
}

#[test]
fn ally_reducer_is_offered_for_its_controllers_play() {
    let mut r = runner();
    let before = r.memory();
    let _ = r.play(0, 0);
    let v = r.pending_selection_view().expect("P0's own play is offered the reducer");
    assert_eq!(v.selecting_player, 0);
    assert!(v.is_optional);
    let accept = *v
        .valid_action_ids
        .iter()
        .find(|&&a| a != digimon_engine::action::space::PASS)
        .expect("accept action");
    r.execute_action(0, accept).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(before - r.memory(), 3, "5 - 2: P0's own Digimon play is reduced");
}

#[test]
fn ally_reducer_does_not_apply_to_the_opponents_play() {
    // Pre-fix this installed P0's reducer prompt with P1 as the selecting
    // player (the BT14-046 Togemon failure).
    let mut r = runner();
    let _ = r.play(1, 0);
    assert!(
        r.pending_selection().is_none(),
        "no reducer prompt for the opponent's play: {:?}",
        r.pending_selection_view()
    );
}
