const INNER:&str=r#"
card: TEST-Q1157-INNER
name: Inner Evolution Result
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
alt_paths:
  - kind: digivolve
    from: {kind: digimon, level_eq: 3, color_is: red}
    cost: 0
"#;
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
      any_permanent: {of: you, zone: [battle_area], kind: digimon, card_number_is: ST1-12}
    amount: 1
    pay_cost:
      - select_own_permanent:
          filter: {card_number_is: ST1-03}
          bind_as: inner_base
          prompt: Choose the inner base
      - select_hand:
          of: you
          filter: {card_number_is: TEST-Q1157-INNER}
          bind_as: inner_card
          prompt: Choose the inner evolution
      - effect_initiated_digivolve:
          target: inner_base
          from_hand: inner_card
          cost: free
      - if:
          condition: {effect_digivolved_any_digimon: true}
          then:
            - gain_memory: 3
"#;
const INNER_WOULD:&str=r#"
card: TEST-INNER-WOULD
name: Inner Would Observer
kind: tamer
color: [white]
cost: 2
effects:
  - kind: replacement
    trigger: when_would_digivolve
    optional: true
    active_when: {card_number_is: ST1-03}
    process:
      - draw: {of: you, count: 1}
"#;
use digimon_engine::{action::{build_action_mask, encode_digivolve},
    card_data::EvoCost, debug_runner::DebugRunner, enums::GamePhase};
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
        .from_dsl_yaml(observer).unwrap().from_dsl_yaml(INNER).unwrap().from_dsl_yaml(INNER_WOULD).unwrap()
        .hand(0, if option { &["LM-027", "BT4-011"][..] } else { &["BT4-011"][..] })
        .deck(0, &["ST1-03"; 12]).deck(1, &["ST1-03";12])
        .memory(5).start();
    // Exact printed circle; DebugRunner compiled metadata omits compatibility evo_costs.
    r.game.card_data.iter_mut().find(|c|c.card_id=="BT4-011").unwrap().evo_costs =
        vec![EvoCost {card_color:0, level:3, memory_cost:2}];
    r.game.current_phase = GamePhase::Main;
    r
}


fn nested_effect_evolution_payment_preserves_inner_result(native:bool, initial:i16) {
    use digimon_engine::action::space::{PASS,ATTACK_START,PLAY_HAND_START};
    let mut r=setup(false);let outer=r.place_on_field(0,"ST1-12",None);
    r.place_on_field(0,"ST1-03",None);r.place_on_field(0,"TEST-Q1157-OBSERVER",None);r.place_on_field(0,"TEST-INNER-WOULD",None);
    let inner_hand=r.add_to_hand(0,"TEST-Q1157-INNER");assert_eq!(inner_hand,1);
    r.game.set_memory(initial);r.game.tick_declarative_effects();let a=encode_digivolve(0,0);
    if native {assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);}
    if native {r.game.decode_action(a,0);} else {r.game.effect_initiated_digivolve(0,0,outer,digimon_engine::enums::CostDelta::Reduce(0),false,digimon_engine::enums::PlaySource::ByEffect);}
    let p=r.game.pending_selection.as_ref().expect("outer treated declaration exposes field payment");
    let accept=*p.valid_action_ids.iter().find(|a|**a!=PASS).unwrap();
    assert_eq!(build_action_mask(&r.game,0)[accept as usize],1.0);r.game.resolve_selection(0,accept).unwrap();
    assert_eq!(build_action_mask(&r.game,0)[(ATTACK_START+1) as usize],1.0);
    r.game.resolve_selection(0,ATTACK_START+1).unwrap();
    assert_eq!(build_action_mask(&r.game,0)[(PLAY_HAND_START+inner_hand as u16) as usize],1.0);
    r.game.resolve_selection(0,PLAY_HAND_START+inner_hand as u16).unwrap();
    assert!(r.game.pending_selection.is_some(),"inner evolution has a would decision");
    assert_eq!(r.memory(),initial,"inner result marker must wait for its declaration to finish");
    assert_eq!(r.game.players[0].hand.len(),2);
    let mut twin=r.game.clone();
    assert_eq!(build_action_mask(&r.game,0)[PASS as usize],1.0);
    r.game.resolve_selection(0,PASS).unwrap();twin.resolve_selection(0,PASS).unwrap();
    assert_eq!(r.game.verification_digest(),twin.verification_digest());
    assert!(r.game.pending_selection.is_none()); let p0_memory=if r.game.turn_player()==0 {r.memory()}else{-r.memory()}; assert_eq!(p0_memory,initial+2,"inner successful evolution must fund outer payment before its commit; memory is measured relative to player zero");
    assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"BT4-011");
    assert_eq!(r.game.players[0].battle_area[1].top_card().card_id(&r.game.card_data),"TEST-Q1157-INNER");
    assert_eq!(r.game.players[0].battle_area[0].card_sources[0].card_id(&r.game.card_data),"ST1-12");
    assert_eq!(r.game.players[0].battle_area[1].card_sources[0].card_id(&r.game.card_data),"ST1-03");
    assert_eq!(r.game.players[0].hand.len(),2,"both evolution bonus draws occur");
    assert!(r.game.modifiers.get(outer,digimon_engine::enums::ModifierType::TreatAsDigimon).is_empty());
    assert_eq!(r.game.players[0].battle_area[0].synth_identity(&r.game.card_data,&r.game.modifiers,outer).dp,Some(5000));
}

#[test]
fn native_outer_cost_inner_effect_would_retains_result_and_tail(){nested_effect_evolution_payment_preserves_inner_result(true,5);}
#[test]
fn effect_outer_cost_inner_effect_would_retains_result_and_tail(){nested_effect_evolution_payment_preserves_inner_result(false,5);}

#[test]
fn inner_effect_result_tail_funds_outer_payment_before_outer_commit(){nested_effect_evolution_payment_preserves_inner_result(false,-10);}
