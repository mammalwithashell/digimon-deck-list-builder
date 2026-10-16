// Synthetic capability fixture: the nested effect attempt and following explicit
// selections exercise declaration ownership. This is not a printed card claim
// that a cost requiring successful digivolution can be paid by a failed attempt.
// The ordinary effect step is not a cost:true success requirement.
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
    cost: 2
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
          cost: {reduce: 0}
      - select_own_permanent:
          filter: {card_number_is: TEST-Q1157-OBSERVER}
          bind_as: tail_marker
          prompt: Enclosing payment tail before completion
      - select_own_permanent:
          filter: {card_number_is: TEST-Q1157-OBSERVER}
          bind_as: tail_marker_again
          prompt: Enclosing payment tail remains pending
      - gain_memory: 2
      - if:
          condition:
            any_permanent: {of: you, zone: [battle_area], kind: digimon, card_number_is: ST1-12}
          then:
            - gain_memory: 1
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
        .from_dsl_yaml(INNER_COST).unwrap().from_dsl_yaml(observer).unwrap().from_dsl_yaml(INNER).unwrap().from_dsl_yaml(INNER_WOULD).unwrap()
        .hand(0, if option { &["LM-027", "BT4-011"][..] } else { &["BT4-011"][..] })
        .deck(0, &["ST1-03"; 12]).deck(1, &["ST1-03";12])
        .memory(5).start();
    // Exact printed circle; DebugRunner compiled metadata omits compatibility evo_costs.
    r.game.card_data.iter_mut().find(|c|c.card_id=="BT4-011").unwrap().evo_costs =
        vec![EvoCost {card_color:0, level:3, memory_cost:2}];
    r.game.current_phase = GamePhase::Main;
    r
}



const INNER_COST:&str=r#"
card: TEST-INNER-COST
name: Inner Host Return Payment
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
      - select_own_permanent:
          filter: {card_number_is: ST1-03}
          bind_as: victim
          prompt: Choose inner host to return as payment
      - return_to_hand: {target: victim}
"#;
fn choose(r:&mut DebugRunner,a:u16) {
    let p=r.pending_selection_view().expect("legal decision");
    assert_eq!(build_action_mask(&r.game,p.selecting_player)[a as usize],1.0,"{}",p.prompt);
    r.game.resolve_selection(p.selecting_player,a).unwrap();
}
fn accept(r:&mut DebugRunner) {
    let p=r.pending_selection_view().expect("optional payment");let a=*p.valid_action_ids.iter().find(|&&a|a!=digimon_engine::action::space::PASS).unwrap();choose(r,a);
}
fn inner_departure_preserves_outer(native:bool) {
    use digimon_engine::action::space::{ATTACK_START,PLAY_HAND_START};
    let mut r=setup(false);let outer=r.place_on_field(0,"ST1-12",Some(0));
    r.place_on_field(0,"ST1-03",Some(0));r.place_on_field(0,"TEST-Q1157-OBSERVER",Some(0));r.place_on_field(0,"TEST-INNER-COST",Some(0));
    let inner=r.add_to_hand(0,"TEST-Q1157-INNER");r.game.tick_declarative_effects();
    if native {let a=encode_digivolve(0,0);assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);r.game.decode_action(a,0);}
    else {r.game.effect_initiated_digivolve(0,0,outer,digimon_engine::enums::CostDelta::Reduce(0),false,digimon_engine::enums::PlaySource::ByEffect);}
    accept(&mut r);choose(&mut r,ATTACK_START+1);choose(&mut r,PLAY_HAND_START+inner as u16);
    accept(&mut r);
    assert_eq!(r.game.players[0].battle_area[0].synth_identity(&r.game.card_data,&r.game.modifiers,outer).kind,digimon_engine::enums::CardKind::Digimon,"outer identity is active before inner host departure");
    let parked=r.game.clone();
    let mut completed_digest = None;
    for _ in 0..2 {
        r.game=parked.clone();choose(&mut r,ATTACK_START+1);
        for _ in 0..2 {
        assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"ST1-12","outer must not commit before its payment continuation");
        assert_eq!(r.game.players[0].battle_area[0].synth_identity(&r.game.card_data,&r.game.modifiers,outer).kind,digimon_engine::enums::CardKind::Digimon,"outer treated identity remains active in payment tail");
        choose(&mut r,ATTACK_START+1);
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"BT4-011","inner invalidation must not abort outer declaration");
        assert_eq!(r.memory(),7,"unconditional initiating tail (+2) and outer treated identity remains visible during its payment tail (+1); completed outer payment reduces its cost to 1; failed inner gives no +3 success payout");
        assert_eq!(r.game.players[0].hand.iter().filter(|c|c.card_id(&r.game.card_data)=="TEST-Q1157-INNER").count(),1);
        assert_eq!(r.game.players[0].hand.iter().filter(|c|c.card_id(&r.game.card_data)=="ST1-03").count(),2,"returned inner host plus one outer evolution draw");
        assert!(r.game.modifiers.get(outer,digimon_engine::enums::ModifierType::TreatAsDigimon).is_empty());
        let digest=r.game.verification_digest();
        if let Some(previous)=completed_digest { assert_eq!(digest,previous,"cloned parked continuation must converge exactly"); }
        completed_digest=Some(digest);
    }
}
#[test]
fn inner_effect_payment_departure_preserves_outer_native_treated_declaration() {inner_departure_preserves_outer(true);}
#[test]
fn inner_effect_payment_departure_preserves_outer_effect_treated_declaration() {inner_departure_preserves_outer(false);}
