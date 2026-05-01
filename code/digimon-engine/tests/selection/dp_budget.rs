use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::GamePhase;
use digimon_engine::selection::{SelectionError, SelectionKind};

#[test]
fn dp_budget_selection_picks_multiple_opponent_digimon_until_pass() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("LOW", "Low"))
        .add_card(make_test_card("MID", "Mid"))
        .add_card(make_test_card("HIGH", "High"))
        .start();
    let p0 = 0;
    let p1 = 1;
    let source = r.place_on_field(p0, "SRC", Some(0));
    r.force_base_dp("LOW", 3000);
    r.force_base_dp("MID", 4000);
    r.force_base_dp("HIGH", 8000);
    let low = r.place_on_field(p1, "LOW", Some(0));
    let mid = r.place_on_field(p1, "MID", Some(0));
    let high = r.place_on_field(p1, "HIGH", Some(0));

    let picked = Arc::new(Mutex::new(Vec::new()));
    let picked_slot = Arc::clone(&picked);
    {
        let source_card = r.top_card(source);
        let mut ctx = EffectContext::new(&mut r.game, source_card, Some(source), p0);
        ctx.select_opponent_permanents_by_dp_budget(
            "delete up to 7000 DP",
            7000,
            0,
            |_, _| true,
            move |_, handles| {
                *picked_slot.lock().unwrap() = handles;
            },
        );
    }

    assert_eq!(r.game.current_phase, GamePhase::SelectBudgeted);
    assert_eq!(
        r.game.pending_selection.as_ref().unwrap().kind,
        SelectionKind::DpBudget {
            remaining_dp: 7000,
            picked: 0,
        }
    );
    assert!(r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids
        .contains(&encode_attack(0, low.index as u16)));
    assert!(r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids
        .contains(&encode_attack(0, mid.index as u16)));
    assert!(!r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids
        .contains(&encode_attack(0, high.index as u16)));
    assert!(r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids
        .contains(&PASS));

    r.game
        .resolve_selection(p0, encode_attack(0, low.index as u16))
        .expect("pick low");
    assert_eq!(
        r.game.pending_selection.as_ref().unwrap().kind,
        SelectionKind::DpBudget {
            remaining_dp: 4000,
            picked: 1,
        }
    );
    r.game
        .resolve_selection(p0, encode_attack(0, mid.index as u16))
        .expect("pick mid");

    assert_eq!(picked.lock().unwrap().as_slice(), &[low, mid]);
}

#[test]
fn dp_budget_selection_hides_pass_until_minimum_pick_is_met() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("LOW", "Low"))
        .add_card(make_test_card("MID", "Mid"))
        .start();

    let p0 = 0;
    let p1 = 1;
    let source = r.place_on_field(p0, "SRC", Some(0));
    r.force_base_dp("LOW", 3000);
    r.force_base_dp("MID", 3000);
    let low = r.place_on_field(p1, "LOW", Some(0));
    let _mid = r.place_on_field(p1, "MID", Some(0));

    let picked = Arc::new(Mutex::new(Vec::new()));
    let picked_slot = Arc::clone(&picked);

    {
        let source_card = r.top_card(source);
        let mut ctx = EffectContext::new(&mut r.game, source_card, Some(source), p0);
        ctx.select_opponent_permanents_by_dp_budget(
            "delete at least one",
            6000,
            1,
            |_, _| true,
            move |_, handles| {
                *picked_slot.lock().unwrap() = handles;
            },
        );
    }

    let selection = r
        .game
        .pending_selection
        .as_ref()
        .expect("DP-budget selection should be pending");
    assert!(
        !selection.valid_action_ids.contains(&PASS),
        "PASS must be hidden before mandatory minimum is met"
    );
    assert!(!selection.is_optional);
    let err = r.game.resolve_selection(p0, PASS).unwrap_err();
    assert_eq!(err, SelectionError::InvalidAction);

    r.game
        .resolve_selection(p0, encode_attack(0, low.index as u16))
        .expect("pick first DP-budget target");

    let selection = r
        .game
        .pending_selection
        .as_ref()
        .expect("DP-budget selection should continue after first pick");
    assert!(
        selection.valid_action_ids.contains(&PASS),
        "PASS should be available after mandatory minimum is met"
    );
    assert!(selection.is_optional);

    r.game
        .resolve_selection(p0, PASS)
        .expect("PASS after minimum should commit selection");

    assert_eq!(picked.lock().unwrap().as_slice(), &[low]);
    assert!(r.game.pending_selection.is_none());
}
