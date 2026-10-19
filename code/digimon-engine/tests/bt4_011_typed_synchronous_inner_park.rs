// Synthetic continuation fixture; no new printed cost requirement is asserted.
include!("bt4_011_nested_abort_scope.rs");
const SYNC_INITIATOR:&str=r#"
card: TEST-SYNC-INITIATOR
name: Outer Source
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          filter: {card_number_is: BT4-011}
          bind_as: result
          prompt: Choose the outer result
      - effect_initiated_digivolve:
          target: source
          from_hand: result
          cost: {reduce: 0}
      - if:
          condition: {effect_digivolved_any_digimon: true}
          then:
            - gain_memory: 3
"#;
const SYNC_PAYER:&str=r#"
card: TEST-SYNC-PAYER
name: Synchronous Payment Source
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    when_any_ally_digivolves_into: {card_number_is: BT4-011}
    amount: 1
    pay_cost:
      - effect_initiated_digivolve:
          target: source
          source: {deck_top: you}
          cost: {reduce: 2}
      - gain_memory: 2
"#;
#[test]fn typed_mandatory_payment_parking_inner_evolution_preserves_outer_result_binding() {
    use digimon_engine::action::space::{PASS,PLAY_HAND_START};
    let observer=INNER_WOULD.replace("card_number_is: ST1-03","card_number_is: TEST-SYNC-PAYER");
    let mut r=DebugRunner::builder().from_dsl_yaml(include_str!("../cards/bt4/BT4-011.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-03.yaml")).unwrap()
        .from_dsl_yaml(INNER).unwrap().from_dsl_yaml(SYNC_INITIATOR).unwrap()
        .from_dsl_yaml(SYNC_PAYER).unwrap().from_dsl_yaml(&observer).unwrap()
        .hand(0,&["BT4-011"]).deck(0,&["ST1-03";12]).deck(1,&["ST1-03";12]).memory(5).start();
    let original=r.game.effect_registry.get("TEST-SYNC-PAYER").unwrap();
    r.register_effect("TEST-SYNC-PAYER",std::sync::Arc::new(TypedPayment(original)));
    r.skip_mulligan();
    let h=r.place_on_field(0,"TEST-SYNC-INITIATOR",Some(0));r.place_on_field(0,"TEST-SYNC-PAYER",Some(0));r.place_on_field(0,"TEST-INNER-WOULD",Some(0));
    let i=r.add_to_hand(0,"TEST-Q1157-INNER");let card=r.game.players[0].hand.remove(i);r.game.players[0].deck.push(card);
    r.game.current_phase=GamePhase::Main;r.game.tick_declarative_effects();r.fire_play_event_triggers(0,h.index as usize,true,false);
    choose(&mut r,PLAY_HAND_START);
    assert_eq!(r.memory(),5,"no enclosing initiating payout before inner decision");
    assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"TEST-SYNC-INITIATOR");
    eprintln!("parked prompt={:?} memory={} field={:?}",r.pending_selection_view(),r.memory(),r.game.players[0].battle_area.iter().map(|p|p.top_card().card_id(&r.game.card_data)).collect::<Vec<_>>());
    let parked=r.game.clone();let accept_id=*r.pending_selection_view().unwrap().valid_action_ids.iter().find(|&&a|a!=PASS).unwrap();
    for choice in [PASS,accept_id] {
        let mut digest=None;
        for _ in 0..2 {
            r.game=parked.clone();choose(&mut r,choice);
            eprintln!("choice={} after prompt={:?} memory={} field={:?}",choice,r.pending_selection_view(),r.memory(),r.game.players[0].battle_area.iter().map(|p|p.top_card().card_id(&r.game.card_data)).collect::<Vec<_>>());
            assert!(r.pending_selection().is_none());
            assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"BT4-011");
            // Accepting the observer draws the exact deck-top result, so
            // that inner declaration loses its required source-zone identity.
            assert_eq!(r.game.players[0].battle_area[1].top_card().card_id(&r.game.card_data),
                if choice==PASS {"TEST-Q1157-INNER"} else {"TEST-SYNC-PAYER"});
            assert_eq!(r.game.players[0].hand.iter().filter(|c|c.card_id(&r.game.card_data)=="TEST-Q1157-INNER").count(),
                if choice==PASS {0} else {1});
            assert_eq!(r.memory(),9,"outer result succeeds: 5+payment2-cost1+initiating3");
            let d=r.game.verification_digest();if let Some(previous)=digest {assert_eq!(d,previous);}digest=Some(d);
        }
    }
}

// Isolate the engine typed-continuation contract from the separate DSL
// first-step classifier: this synthetic reducer explicitly declares that its
// mandatory payment may park, using the established native Effect API.
struct TypedPayment(std::sync::Arc<dyn digimon_engine::effect::CardEffect>);
impl digimon_engine::effect::CardEffect for TypedPayment {
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect> {
  let mut effects=self.0.effects(c);for effect in &mut effects {effect.pay_cost_interactive=true;}effects
 }
}
