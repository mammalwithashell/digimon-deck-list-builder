// Capability regression: a nested reducer choice is owned by the inner
// declaration; declining it cannot undo an accepted enclosing payment.
include!("bt4_011_nested_abort_scope.rs");

fn decline_inner_reducer_preserves_outer(native: bool, abort_payment: bool) {
    use digimon_engine::action::space::{ATTACK_START, PLAY_HAND_START, PASS};
    let mut r = if abort_payment {setup_with_abort_cost(false, OBSERVER)} else {setup(false)};
    let outer = r.place_on_field(0, "ST1-12", Some(0));
    r.place_on_field(0, "ST1-03", Some(0));
    r.place_on_field(0, "TEST-Q1157-OBSERVER", Some(0));
    r.place_on_field(0, "TEST-INNER-COST", Some(0));
    let inner = r.add_to_hand(0, "TEST-Q1157-INNER");
    if abort_payment { r.add_to_hand(0, "ST1-03"); }
    r.game.tick_declarative_effects();
    if native {
        let a = encode_digivolve(0, 0);
        assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
        r.game.decode_action(a, 0);
    } else {
        r.game.effect_initiated_digivolve(0, 0, outer,
            digimon_engine::enums::CostDelta::Reduce(0), false,
            digimon_engine::enums::PlaySource::ByEffect);
    }
    accept(&mut r);
    choose(&mut r, ATTACK_START + 1);
    choose(&mut r, PLAY_HAND_START + inner as u16);
    let parked = r.game.clone();
    let mut digest = None;
    for _ in 0..2 {
        r.game = parked.clone();
        assert_eq!(r.pending_kind(), Some(if abort_payment {
            digimon_engine::selection::SelectionKind::Hand
        } else { digimon_engine::selection::SelectionKind::EffectChoice }));
        // A pay_cost containing an optional cost:true selection is self-gated;
        // its Hand choice is the consent point, without another outer yes.
        choose(&mut r, PASS);
        assert_eq!(r.game.players[0].battle_area[1].top_card().card_id(&r.game.card_data), "TEST-Q1157-INNER");
        for _ in 0..2 {
            assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data), "ST1-12");
            choose(&mut r, ATTACK_START + 2);
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data), "BT4-011");
        assert_eq!(r.memory(), 8, "5 minus full inner cost2 plus outer payment tail6 minus reduced outer cost1");
        assert_eq!(r.game.players[0].hand.len(), if abort_payment {3} else {2}, "both evolution bonus draws survive");
        let current = r.game.verification_digest();
        if let Some(previous) = digest { assert_eq!(current, previous); }
        digest = Some(current);
    }
}

#[test]
fn native_outer_keeps_credit_when_inner_reducer_gate_is_declined() {
    decline_inner_reducer_preserves_outer(true, false);
}
#[test]
fn effect_outer_keeps_credit_when_inner_reducer_gate_is_declined() {
    decline_inner_reducer_preserves_outer(false, false);
}

fn setup_with_abort_cost(option: bool, observer: &str) -> DebugRunner {
    let path = std::env::var("BT4_011_TEST_YAML").unwrap_or_else(|_|
        format!("{}/cards/bt4/BT4-011.yaml", env!("CARGO_MANIFEST_DIR")));
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&std::fs::read_to_string(path).unwrap()).unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-12.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/st2/ST2-12.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-03.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/LM/LM-027.yaml")).unwrap()
        .from_dsl_yaml(ABORT_COST).unwrap().from_dsl_yaml(observer).unwrap().from_dsl_yaml(INNER).unwrap().from_dsl_yaml(INNER_WOULD).unwrap()
        .hand(0, if option { &["LM-027", "BT4-011"][..] } else { &["BT4-011"][..] })
        .deck(0, &["ST1-03"; 12]).deck(1, &["ST1-03";12])
        .memory(5).start();
    // Exact printed circle; DebugRunner compiled metadata omits compatibility evo_costs.
    r.game.card_data.iter_mut().find(|c|c.card_id=="BT4-011").unwrap().evo_costs =
        vec![EvoCost {card_color:0, level:3, memory_cost:2}];
    r.game.current_phase = GamePhase::Main;
    r
}




const ABORT_COST: &str = r#"
card: TEST-INNER-COST
name: Inner Optional Discard Payment
kind: tamer
color: [white]
cost: 2
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    optional: true
    when_any_ally_digivolves_into: {card_number_is: TEST-Q1157-INNER}
    amount: 1
    pay_cost:
      - select_hand:
          of: you
          filter: {card_number_is: ST1-03}
          optional: true
          cost: true
          bind_as: discard
          prompt: Pay inner reduction by discarding
      - trash_from_hand_by_index: {of: you, hand_index: discard}
"#;
#[test]
fn native_outer_keeps_credit_when_inner_payment_clause_aborts() {
    decline_inner_reducer_preserves_outer(true, true);
}
#[test]
fn effect_outer_keeps_credit_when_inner_payment_clause_aborts() {
    decline_inner_reducer_preserves_outer(false, true);
}
