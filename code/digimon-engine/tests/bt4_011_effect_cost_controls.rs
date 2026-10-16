const OBSERVER: &str = r#"
card: TEST-Q1157-OBSERVER
name: Would Evolve Cost Observer
kind: tamer
color: [white]
cost: 2
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    optional: true
    when_any_ally_digivolves_into: {card_number_is: BT4-011}
    condition:
      any_permanent: {of: you, zone: [battle_area], kind: digimon}
    amount: 1
    pay_cost:
      - suspend: {target: source}
"#;
use digimon_engine::{action::{build_action_mask, encode_digivolve, space::PLAY_HAND_START},
    card_data::EvoCost, debug_runner::DebugRunner, enums::{CardKind, GamePhase}};
fn setup(option: bool) -> DebugRunner { setup_observer(option, OBSERVER) }
fn setup_observer(option: bool, observer: &str) -> DebugRunner {
    let path = std::env::var("BT4_011_TEST_YAML").unwrap_or_else(|_|
        format!("{}/cards/bt4/BT4-011.yaml", env!("CARGO_MANIFEST_DIR")));
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&std::fs::read_to_string(path).unwrap()).unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-12.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/st2/ST2-12.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-03.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/LM/LM-027.yaml")).unwrap()
        .from_dsl_yaml(observer).unwrap()
        .hand(0, if option { &["LM-027", "BT4-011"][..] } else { &["BT4-011"][..] })
        .deck(0, &["ST1-03"; 12]).deck(1, &["ST1-03";12])
        .memory(5).start();
    // Exact printed circle; DebugRunner compiled metadata omits compatibility evo_costs.
    r.game.card_data.iter_mut().find(|c|c.card_id=="BT4-011").unwrap().evo_costs =
        vec![EvoCost {card_color:0, level:3, memory_cost:2}];
    r.game.current_phase = GamePhase::Main;
    r
}


fn check(base: &str) {
    use digimon_engine::action::space::PASS;
    let mut r=setup(false);let h=r.place_on_field(0,base,None);
    r.place_on_field(0,"TEST-Q1157-OBSERVER",None);r.game.tick_declarative_effects();
    let a=encode_digivolve(0,0);assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);
    begin_effect(&mut r);
    let p=r.game.pending_selection.as_ref().expect("Digimon-scoped would-evolve reducer must see declared treated-as base");
    let choices:Vec<_>=p.valid_action_ids.iter().copied().filter(|a|*a!=PASS).collect();
    assert_eq!(choices.len(),1,"one optional reducer accept action");
    let mask=build_action_mask(&r.game,0);assert_eq!(mask[PASS as usize],1.0);assert_eq!(mask[choices[0] as usize],1.0);
    if base=="ST1-12" {
        let id=r.game.players[0].battle_area[0].synth_identity(&r.game.card_data,&r.game.modifiers,h);
        assert_eq!(id.kind,CardKind::Digimon);assert_eq!(id.level,Some(3));assert_eq!(id.dp,None);
    }
    for (choice,memory) in [(choices[0],5),(PASS,4)] {
        let mut g=r.game.clone();let mut twin=g.clone();
        g.resolve_selection(0,choice).unwrap();twin.resolve_selection(0,choice).unwrap();
        assert_eq!(g.verification_digest(),twin.verification_digest());
        assert!(g.pending_selection.is_none());assert_eq!(g.memory,memory);
        assert_eq!(g.players[0].battle_area[1].is_suspended,choice!=PASS,"only accepted reducer pays the suspension cost");
        assert_eq!(g.players[0].battle_area[0].top_card().card_id(&g.card_data),"BT4-011");
        assert_eq!(g.players[0].battle_area[0].synth_identity(&g.card_data,&g.modifiers,h).dp,Some(5000));
    }
}
#[test]
fn effect_reduced_q1157_cost_window_ordinary_digimon_control() {check("ST1-03");}
#[test]
fn effect_reduced_q1157_cost_window_treated_tamer_accept_decline_clones() {check("ST1-12");}

fn begin_effect(r:&mut DebugRunner) {
    use digimon_engine::{enums::{CostDelta,PlaySource},permanent::PermanentHandle};
    let _=r.game.effect_initiated_digivolve(0,0,PermanentHandle{player:0,index:0},CostDelta::Reduce(1),false,PlaySource::ByEffect);
}

#[test]
fn effect_late_cost_departure_does_not_move_result_or_charge_memory() {
    use digimon_engine::action::space::PASS;
    let observer=OBSERVER.replace("color: [white]","color: [red]").replace("suspend: {target: source}","return_to_hand: {target: source}");
    let mut r=setup_observer(false,&observer);r.place_on_field(0,"TEST-Q1157-OBSERVER",None);
    r.game.tick_declarative_effects();begin_effect(&mut r);
    let pick=r.game.pending_selection.as_ref().expect("self-return optional cost offered");
    let accept=*pick.valid_action_ids.iter().find(|a|**a!=PASS).unwrap();
    let mut twin=r.game.clone();r.game.resolve_selection(0,accept).unwrap();twin.resolve_selection(0,accept).unwrap();
    assert_eq!(r.game.verification_digest(),twin.verification_digest());assert_eq!(r.memory(),5);
    assert!(r.game.pending_selection.is_none());assert!(r.game.players[0].battle_area.is_empty());
    let ids:Vec<_>=r.game.players[0].hand.iter().map(|c|c.card_id(&r.game.card_data)).collect();assert_eq!(ids,vec!["BT4-011","TEST-Q1157-OBSERVER"]);
}
