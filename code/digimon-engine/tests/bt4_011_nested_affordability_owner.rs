// A clone-only affordability probe must not credit its declined inner payment.
include!("bt4_011_nested_abort_scope.rs");
const EXTRA_ROUTE:&str=r#"
  - kind: digivolve
    from: {kind: digimon, level_eq: 3, color_is: red}
    cost: 3
"#;
const PROBE_COST:&str=r#"
card: TEST-INNER-COST
name: Destructive Inner Payment
kind: tamer
color: [white]
cost: 2
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    optional: true
    when_any_ally_digivolves_into: {card_number_is: TEST-Q1157-INNER}
    amount: 3
    pay_cost:
      - select_hand:
          of: you
          filter: {card_number_is: TEST-Q1157-INNER}
          optional: true
          cost: true
          bind_as: discard
          prompt: Pay by discarding the declared result
      - trash_from_hand_by_index: {of: you, hand_index: discard}
"#;
fn setup_probe_owner(option: bool, observer: &str) -> DebugRunner {
    let path = std::env::var("BT4_011_TEST_YAML").unwrap_or_else(|_|
        format!("{}/cards/bt4/BT4-011.yaml", env!("CARGO_MANIFEST_DIR")));
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&std::fs::read_to_string(path).unwrap()).unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-12.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/st2/ST2-12.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-03.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/LM/LM-027.yaml")).unwrap()
        .from_dsl_yaml(PROBE_COST).unwrap().from_dsl_yaml(observer).unwrap().from_dsl_yaml(&format!("{}{}",INNER,EXTRA_ROUTE)).unwrap().from_dsl_yaml(INNER_WOULD).unwrap()
        .hand(0, if option { &["LM-027", "BT4-011"][..] } else { &["BT4-011"][..] })
        .deck(0, &["ST1-03"; 12]).deck(1, &["ST1-03";12])
        .memory(5).start();
    // Exact printed circle; DebugRunner compiled metadata omits compatibility evo_costs.
    r.game.card_data.iter_mut().find(|c|c.card_id=="BT4-011").unwrap().evo_costs =
        vec![EvoCost {card_color:0, level:3, memory_cost:2}];
    r.game.current_phase = GamePhase::Main;
    r
}
#[test] fn nested_multi_route_probe_does_not_credit_declined_payment_from_outer_scope() {
    use digimon_engine::action::space::{ATTACK_START,PLAY_HAND_START};
    let mut r=setup_probe_owner(false,OBSERVER);r.game.set_memory(-9);
    let outer=r.place_on_field(0,"ST1-12",Some(0));r.place_on_field(0,"ST1-03",Some(0));
    r.place_on_field(0,"TEST-Q1157-OBSERVER",Some(0));r.place_on_field(0,"TEST-INNER-COST",Some(0));
    let inner=r.add_to_hand(0,"TEST-Q1157-INNER");r.game.tick_declarative_effects();
    r.game.effect_initiated_digivolve(0,0,outer,digimon_engine::enums::CostDelta::Reduce(2),false,digimon_engine::enums::PlaySource::ByEffect);
    accept(&mut r);choose(&mut r,ATTACK_START+1);
    let parked=r.game.clone();let mut digest=None;
    for _ in 0..2 {
        r.game=parked.clone();choose(&mut r,PLAY_HAND_START+inner as u16);
        let pending=r.pending_selection_view().unwrap();
        assert_eq!(pending.prompt,"Enclosing payment tail before completion","decline leaves costs2/3 unaffordable; accepting destroys the declared result card, so no inner route is legal");
        for _ in 0..2 {choose(&mut r,ATTACK_START+2);}
        assert_eq!(r.game.players[0].battle_area[1].top_card().card_id(&r.game.card_data),"ST1-03");
        assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"BT4-011");
        assert_eq!(r.game.players[0].hand.iter().filter(|c|c.card_id(&r.game.card_data)=="TEST-Q1157-INNER").count(),1);
        let d=r.game.verification_digest();if let Some(previous)=digest {assert_eq!(d,previous);}digest=Some(d);
    }
}

#[test]fn top_level_multi_route_probe_does_not_offer_unpayable_destructive_cost(){
 let mut r=setup_probe_owner(false,OBSERVER);r.game.set_memory(-9);
 let host=r.place_on_field(0,"ST1-03",Some(0));r.place_on_field(0,"TEST-INNER-COST",Some(0));
 let card=r.add_to_hand(0,"TEST-Q1157-INNER");r.game.tick_declarative_effects();
 assert!(!r.game.effect_initiated_digivolve(0,card,host,digimon_engine::enums::CostDelta::Reduce(0),false,digimon_engine::enums::PlaySource::ByEffect));
 assert!(r.pending_selection().is_none(),"no feasible payment path exists even without an enclosing declaration");
 assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"ST1-03");assert_eq!(r.memory(),-9);
}
