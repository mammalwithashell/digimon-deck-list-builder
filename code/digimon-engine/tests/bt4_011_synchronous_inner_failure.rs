include!("bt4_011_nested_abort_scope.rs");

const INITIATOR: &str = r#"
card: TEST-OUTER-INITIATOR
name: Outer Evolution Initiator
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: result
          filter: {card_number_is: BT4-011}
          prompt: Choose outer result
      - effect_initiated_digivolve:
          target: source
          from_hand: result
          cost: {reduce: 0}
      - if:
          condition: {effect_digivolved_any_digimon: true}
          then:
            - gain_memory: 3
"#;

#[test]
fn synchronous_inner_failure_does_not_steal_outer_initiating_success_binding() {
    use digimon_engine::{action::space::{ATTACK_START, PLAY_HAND_START},
        enums::{Expiry, ModifierType}, modifiers::ModifierEntry};
    // Keep the real nested selection path but end the outer payment with a
    // non-conditional +2 marker; only the initiating clause grants the +3.
    let prefix = OBSERVER.split("      - select_own_permanent:\n          filter: {card_number_is: TEST-Q1157-OBSERVER}").next().unwrap();
    let payer = format!("{}      - gain_memory: 2\n", prefix)
        .replace("card_number_is: ST1-12", "card_number_is: TEST-OUTER-INITIATOR");
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../cards/bt4/BT4-011.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-03.yaml")).unwrap()
        .from_dsl_yaml(INNER).unwrap().from_dsl_yaml(INITIATOR).unwrap()
        .from_dsl_yaml(&payer).unwrap()
        .hand(0, &["BT4-011", "TEST-Q1157-INNER"])
        .deck(0, &["ST1-03"; 12]).deck(1, &["ST1-03";12])
        .memory(5).start();
    r.skip_mulligan();
    let outer = r.place_on_field(0, "TEST-OUTER-INITIATOR", Some(0));
    let inner = r.place_on_field(0, "ST1-03", Some(0));
    r.place_on_field(0, "TEST-Q1157-OBSERVER", Some(0));
    r.game.modifiers.add(inner, ModifierEntry::simple(ModifierType::CannotDigivolve,
        0, Expiry::Permanent, 0));
    r.game.current_phase = GamePhase::Main;
    r.game.tick_declarative_effects();
    r.fire_play_event_triggers(0, outer.index as usize, true, false);
    choose(&mut r, PLAY_HAND_START);
    accept(&mut r);
    choose(&mut r, ATTACK_START + 1);
    let parked = r.game.clone();
    let mut digest = None;
    for _ in 0..2 {
        r.game = parked.clone();
        choose(&mut r, PLAY_HAND_START + 1);
        assert!(r.pending_selection().is_none());
        assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data), "BT4-011");
        assert_eq!(r.game.players[0].battle_area[1].top_card().card_id(&r.game.card_data), "ST1-03");
        assert_eq!(r.memory(), 9, "5 plus paid outer tail2 minus reduced cost1 plus initiating success3");
        assert_eq!(r.game.players[0].hand.iter().filter(|c| c.card_id(&r.game.card_data)=="TEST-Q1157-INNER").count(), 1);
        let current = r.game.verification_digest();
        if let Some(previous) = digest { assert_eq!(current, previous); }
        digest = Some(current);
    }
}
