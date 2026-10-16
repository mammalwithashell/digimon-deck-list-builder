//! Official Q1157: the would-digivolve event must classify the declared Hybrid base as Digimon.
const OBSERVER: &str = r#"
card: TEST-Q1157-OBSERVER
name: Would Evolve Observer
kind: tamer
color: [white]
cost: 2
effects:
  - kind: replacement
    trigger: when_would_digivolve
    optional: true
    active_when: { kind: digimon }
    process:
      - draw: {of: you, count: 1}
"#;
use digimon_engine::{
    action::{build_action_mask, encode_digivolve, space::PLAY_HAND_START},
    card_data::EvoCost,
    debug_runner::DebugRunner,
    enums::{CardKind, GamePhase},
};
fn setup(option: bool) -> DebugRunner {
    setup_observer(option, OBSERVER)
}
fn setup_observer(option: bool, observer: &str) -> DebugRunner {
    let path = std::env::var("BT4_011_TEST_YAML")
        .unwrap_or_else(|_| format!("{}/cards/bt4/BT4-011.yaml", env!("CARGO_MANIFEST_DIR")));
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&std::fs::read_to_string(path).unwrap())
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-12.yaml"))
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/st2/ST2-12.yaml"))
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-03.yaml"))
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/LM/LM-027.yaml"))
        .unwrap()
        .from_dsl_yaml(observer)
        .unwrap()
        .hand(
            0,
            if option {
                &["LM-027", "BT4-011"][..]
            } else {
                &["BT4-011"][..]
            },
        )
        .deck(0, &["ST1-03"; 12])
        .deck(1, &["ST1-03"; 12])
        .memory(5)
        .start();
    // Exact printed circle; DebugRunner compiled metadata omits compatibility evo_costs.
    r.game
        .card_data
        .iter_mut()
        .find(|c| c.card_id == "BT4-011")
        .unwrap()
        .evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 2,
    }];
    r.game.current_phase = GamePhase::Main;
    r
}


const LATE_GATE: &str = r#"
  - kind: flood_gate
    modifier: CannotDigivolve
    active_when: {source_is_unsuspended: false}
    target: {of: you, zone: [battle_area], kind: digimon}
"#;
fn late_would_gate(native: bool, cost_window: bool) {
    use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
    let observer = format!("{}{}", if cost_window {COST_OBSERVER.to_string()} else {OBSERVER.replace("draw: {of: you, count: 1}", "suspend: {target: source}")}, LATE_GATE);
    let mut r = setup_observer(false, &observer);
    let host = r.place_on_field(0, "ST1-12", Some(0));
    r.place_on_field(0, "TEST-Q1157-OBSERVER", Some(0));
    r.game.tick_declarative_effects();
    if native {
        let action = encode_digivolve(0, 0);
        assert_eq!(build_action_mask(&r.game,0)[action as usize],1.0);
        r.game.decode_action(action,0);
    } else {
        r.game.effect_initiated_digivolve(0,0,host,digimon_engine::enums::CostDelta::Reduce(0),false,digimon_engine::enums::PlaySource::ByEffect);
    }
    let saved = r.game.clone();
    let accept = *saved.pending_selection.as_ref().unwrap().valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    for choice in [accept, PASS] {
        let mut expected_digest = None;
        for _ in 0..2 {
            r.game = saved.clone();
            assert_eq!(build_action_mask(&r.game,0)[choice as usize],1.0);
            r.game.resolve_selection(0,choice).unwrap();
            assert!(r.pending_selection().is_none());
            let denied = choice == accept;
            assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data), if denied {"ST1-12"} else {"BT4-011"});
            assert_eq!(r.memory(), if denied {5} else {3});
            assert_eq!(r.game.players[0].hand.iter().filter(|c|c.card_id(&r.game.card_data)=="BT4-011").count(),usize::from(denied));
            assert!(r.game.modifiers.get(host,digimon_engine::enums::ModifierType::TreatAsDigimon).is_empty());
            if denied {
                assert_eq!(r.game.players[0].battle_area[0].synth_identity(&r.game.card_data,&r.game.modifiers,host).kind,CardKind::Tamer);
                assert!(!r.game.modifiers.has(host,digimon_engine::enums::ModifierType::CannotDigivolve), "the Digimon-only materialized gate clears on restored Tamer identity");
                assert_eq!(build_action_mask(&r.game,0)[encode_digivolve(0,0) as usize],0.0, "a new declaration still reconstructs the treated route and rejects the active Digimon-only gate");
            }
            let digest = r.game.verification_digest();
            if let Some(previous) = expected_digest { assert_eq!(digest, previous); }
            expected_digest = Some(digest);
        }
    }
}
#[test] fn native_evolution_rechecks_late_digimon_gate_before_clearing_treatment(){late_would_gate(true, false);}
#[test] fn effect_evolution_rechecks_late_digimon_gate_before_clearing_treatment(){late_would_gate(false, false);}

const COST_OBSERVER: &str = r#"
card: TEST-Q1157-OBSERVER
name: Late Cost Gate Observer
kind: tamer
color: [white]
cost: 2
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    optional: true
    when_any_ally_digivolves_into: {card_number_is: BT4-011}
    amount: 1
    pay_cost:
      - suspend: {target: source}
"#;
#[test] fn native_evolution_rechecks_late_cost_digimon_gate(){late_would_gate(true, true);}
#[test] fn effect_evolution_rechecks_late_cost_digimon_gate(){late_would_gate(false, true);}
