//! Declaration-only baseline: COST_ASSIGNMENT_BASELINE=1 lowers each new setter
//! to an existing amount-zero reducer, preserving the real payment and routing.
use digimon_engine::{
    action::{build_action_mask, space::PLAY_HAND_START},
    debug_runner::{make_test_card, DebugRunner},
    enums::{CostDelta, Expiry, ModifierType, PlaySource},
    modifiers::PlayerModifierEntry,
    rules::Rules,
};
fn setter(i: usize, value: u16) -> String {
    let operation = if std::env::var_os("COST_ASSIGNMENT_BASELINE").is_some() {
        "kind: cost_reduction"
    } else {
        "kind: cost_assignment"
    };
    let value = if std::env::var_os("COST_ASSIGNMENT_BASELINE").is_some() {
        "amount: 0".into()
    } else {
        format!("set_cost: {value}")
    };
    format!("card: SET-{i}\nname: Setter {i}\nkind: tamer\ncolor: [white]\ncost: 1\neffects:\n  - {operation}\n    {value}\n    reduction_timing: before_pay_cost\n    when_any_ally_played: {{ name_is: Pay target }}\n    pay_cost:\n      - draw: {{of: you, count: 1}}\n    summary: Setter {i}\n")
}
fn fixture(values: &[u16], reduce: i32) -> DebugRunner {
    let mut rules = Rules::standard();
    rules.memory_range = (-100, 100);
    rules.field_slots = 64;
    let mut target = make_test_card("SET-TARGET", "Pay target");
    target.play_cost = 20;
    let mut b = DebugRunner::builder()
        .with_rules(rules)
        .add_card(target)
        .add_card(make_test_card("SET-DRAW", "Draw marker"));
    for (i, v) in values.iter().enumerate() {
        b = b.from_dsl_yaml(&setter(i, *v)).unwrap();
    }
    if reduce > 0 {
        b=b.from_dsl_yaml(&format!("card: SET-REDUCE\nname: Arithmetic reduction\nkind: tamer\ncolor: [white]\ncost: 1\neffects:\n  - kind: cost_reduction\n    reduction_timing: before_pay_cost\n    when_any_ally_played: {{name_is: Pay target}}\n    amount: {reduce}\n    summary: Arithmetic reduction\n")).unwrap();
    }
    let mut r = b
        .hand(0, &["SET-TARGET"])
        .deck(0, &["SET-DRAW"; 100])
        .deck(1, &["SET-DRAW"; 10])
        .memory(100)
        .start();
    r.skip_mulligan();
    for i in 0..values.len() {
        r.place_on_field(0, &format!("SET-{i}"), None);
    }
    if reduce > 0 {
        r.place_on_field(0, "SET-REDUCE", None);
    }
    r.game.enter_main_phase();
    r
}
fn choose(r: &mut DebugRunner, a: u16) {
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.execute_action(0, a).unwrap();
}
fn label(r: &DebugRunner, wanted: &str) -> u16 {
    let id = wanted.replace("Setter ", "SET-");
    let physical = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == id)
        .unwrap()
        .top_card()
        .handle();
    r.game
        .pending_selection
        .as_ref()
        .expect("real cost prompt")
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .find(|e| e.source_card == Some(physical))
        .expect("all legal physical-source order choices exposed")
        .action_id
}
fn finish(r: &mut DebugRunner) {
    for _ in 0..100 {
        if r.game.pending_selection.is_none() {
            return;
        }
        let a = r
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .iter()
            .copied()
            .find(|a| *a != 62 && *a != 58 && *a != 59)
            .unwrap();
        choose(r, a);
    }
    panic!("payment did not settle");
}
fn start(r: &mut DebugRunner) {
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
}
#[test]
fn two_paid_setters_expose_both_orders_and_clone_preserves_both_payments() {
    let mut r = fixture(&[1, 3], 0);
    start(&mut r);
    assert_eq!(
        r.game
            .pending_selection
            .as_ref()
            .unwrap()
            .effect_choices
            .as_ref()
            .unwrap()
            .len(),
        2
    );
    let saved = r.game.clone();
    for (first, last) in [(0, 3), (1, 1)] {
        r.game = saved.clone();
        let a = label(&r, &format!("Setter {first}"));
        choose(&mut r, a);
        finish(&mut r);
        assert_eq!(r.memory(), 100 - last);
        assert_eq!(
            r.game.players[0].hand.len(),
            2,
            "both selected payment side effects happened"
        );
    }
}
#[test]
fn assignment_survives_both_reduction_prohibitions() {
    for m in [
        ModifierType::CannotReduceCost,
        ModifierType::CannotReducePlayCost,
    ] {
        let mut r = fixture(&[5], 3);
        r.game.modifiers.add_player_modifier(
            0,
            PlayerModifierEntry::simple(m, 0, Expiry::Permanent, None, 1),
        );
        start(&mut r);
        finish(&mut r);
        assert_eq!(
            r.memory(),
            95,
            "Set five applies while reduction three is prohibited"
        );
    }
}
#[test]
fn signed_effect_delta_and_reducer_clamp_only_after_assignment_composition() {
    let mut r = fixture(&[5], 10);
    let _ = r
        .game
        .play_from_hand_with_cost(0, 0, CostDelta::Reduce(-3), PlaySource::ByEffect);
    let a = label(&r, "Setter 0");
    choose(&mut r, a);
    finish(&mut r);
    assert_eq!(r.memory(), 100, "max(0, Set5 +3 -10), not max(0,5-10)+3");
}
#[test]
fn free_effect_play_remains_without_memory_payment() {
    let mut r = fixture(&[5], 0);
    let _ = r
        .game
        .play_from_hand_with_cost(0, 0, CostDelta::Free, PlaySource::ByEffect);
    finish(&mut r);
    assert_eq!(r.memory(), 100);
}
#[test]
fn cost_order_pages_surface_every_effect_and_navigation_has_no_payment() {
    let mut r = fixture(&[1; 35], 0);
    start(&mut r);
    let digest = r.game.verification_digest();
    let hand = r.game.players[0].hand.len();
    assert_eq!(
        r.game
            .pending_selection
            .as_ref()
            .unwrap()
            .effect_choices
            .as_ref()
            .unwrap()
            .iter()
            .filter(|e| e.source_card.is_some())
            .count(),
        28
    );
    choose(&mut r, 59);
    assert_eq!(label(&r, "Setter 34"), 36);
    assert_eq!(r.memory(), 100);
    assert_eq!(r.game.players[0].hand.len(), hand);
    choose(&mut r, 58);
    assert_eq!(r.game.verification_digest(), digest);
}

#[test]
fn vanished_selected_setter_reoffers_other_order_choices_instead_of_substitution() {
    let mut r = fixture(&[1, 3, 5], 0);
    start(&mut r);
    let chosen = label(&r, "Setter 1");
    let initial = r.game.players[0].hand.len();
    // A replacement at the selected slot cannot inherit its declaration key.
    let replacement = r.place_on_field(0, "SET-0", None);
    let fresh = r.game.players[0]
        .battle_area
        .remove(replacement.index as usize);
    r.game.players[0].battle_area[1] = fresh;
    let saved = r.game.clone();
    for _ in 0..2 {
        r.game = saved.clone();
        choose(&mut r, chosen);
        assert!(r
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .prompt
            .contains("resolve next"));
        assert_eq!(
            r.game.players[0].hand.len(),
            initial,
            "vanished choice must not silently pay a different effect"
        );
        assert_eq!(r.memory(), 100);
        assert!(
            r.game
                .pending_selection
                .as_ref()
                .unwrap()
                .effect_choices
                .as_ref()
                .unwrap()
                .len()
                >= 2
        );
    }
}

#[test]
fn completed_assignment_replaces_effect_supplied_fixed_base() {
    let mut r = fixture(&[5], 0);
    let _ = r
        .game
        .play_from_hand_with_cost(0, 0, CostDelta::Fixed(2), PlaySource::ByEffect);
    finish(&mut r);
    assert_eq!(r.memory(), 95, "later Set5 replaces initial Fixed2 base");
}
