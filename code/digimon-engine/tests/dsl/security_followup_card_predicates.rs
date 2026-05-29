use digimon_engine::action::space::{HAND_EFFECT_START, PASS, SEL_MY_SECURITY_START};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardColor;
use digimon_engine::{EffectTiming, TriggerSource};

fn yellow_card(card_id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(card_id, name);
    card.colors = vec![CardColor::Yellow];
    card
}

fn zone_ids(
    cards: &[digimon_engine::card_source::CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

#[test]
fn binding_card_matches_gates_followup_on_selected_security_card() {
    let yaml = r#"
card: TEST-BINDING-CARD-MATCHES
name: Binding Card Matches
kind: digimon
color: [yellow]
level: 3
cost: 3
dp: 1000
effects:
  - when: on_play
    process:
      - select_security:
          of: you
          bind_as: picked
          optional: true
          filter: {}
          prompt: "Pick any security card"
      - if:
          condition: { binding_present: picked }
          then:
            - add_to_hand_from_security: { of: you, card: picked }
      - if:
          condition:
            binding_card_matches:
              binding: picked
              filter: { color_is: yellow }
          then:
            - recover: { of: you, count: 1 }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("binding-card predicate YAML loads")
        .add_card(make_test_card("RED", "Red Card"))
        .add_card(yellow_card("YELLOW", "Yellow Card"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .security(0, &["RED", "YELLOW"])
        .deck(0, &["RECOVER"])
        .memory(0)
        .start();
    let source = runner.place_on_field(0, "TEST-BINDING-CARD-MATCHES", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect("security prompt");
    assert_eq!(
        view.valid_action_ids,
        vec![SEL_MY_SECURITY_START, SEL_MY_SECURITY_START + 1]
    );
    runner
        .execute_action(0, SEL_MY_SECURITY_START + 1)
        .expect("pick yellow security");

    assert_eq!(
        zone_ids(&runner.game.players[0].security, &runner.game.card_data),
        vec!["RED", "RECOVER"]
    );
    assert!(
        zone_ids(&runner.game.players[0].hand, &runner.game.card_data)
            .contains(&"YELLOW".to_string())
    );
}

#[test]
fn binding_card_matches_fails_for_nonmatching_security_card() {
    let yaml = r#"
card: TEST-BINDING-CARD-MATCHES-NO
name: Binding Card Matches No
kind: digimon
color: [yellow]
level: 3
cost: 3
dp: 1000
effects:
  - when: on_play
    process:
      - select_security:
          of: you
          bind_as: picked
          optional: true
          filter: {}
          prompt: "Pick any security card"
      - if:
          condition: { binding_present: picked }
          then:
            - add_to_hand_from_security: { of: you, card: picked }
      - if:
          condition:
            binding_card_matches:
              binding: picked
              filter: { color_is: yellow }
          then:
            - recover: { of: you, count: 1 }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("binding-card predicate YAML loads")
        .add_card(make_test_card("RED", "Red Card"))
        .add_card(yellow_card("YELLOW", "Yellow Card"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .security(0, &["RED", "YELLOW"])
        .deck(0, &["RECOVER"])
        .memory(0)
        .start();
    let source = runner.place_on_field(0, "TEST-BINDING-CARD-MATCHES-NO", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
    runner
        .execute_action(0, SEL_MY_SECURITY_START)
        .expect("pick red security");

    assert_eq!(
        zone_ids(&runner.game.players[0].security, &runner.game.card_data),
        vec!["YELLOW"]
    );
    assert!(
        !zone_ids(&runner.game.players[0].security, &runner.game.card_data)
            .contains(&"RECOVER".to_string())
    );
}

#[test]
fn add_top_security_to_hand_cost_gates_body_on_accept_or_decline() {
    let yaml = r#"
card: TEST-ADD-TOP-SEC-COST
name: Add Top Security Cost
kind: tamer
color: [yellow]
cost: 3
effects:
  - when: on_play
    process:
      - if_add_top_security_to_hand_cost:
          of: you
          prompt: "Add top security to hand?"
          then:
            - gain_memory: 1
"#;
    let mut decline = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("cost YAML loads")
        .add_card(make_test_card("SEC", "Security"))
        .security(0, &["SEC"])
        .memory(0)
        .start();
    let source = decline.place_on_field(0, "TEST-ADD-TOP-SEC-COST", Some(0));
    decline
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    decline.game.drain_effect_queue();
    decline.execute_action(0, PASS).expect("decline cost");
    assert_eq!(decline.memory(), 0);
    assert_eq!(decline.security_count(0), 1);

    let mut accept = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("cost YAML loads")
        .add_card(make_test_card("SEC", "Security"))
        .security(0, &["SEC"])
        .memory(0)
        .start();
    let source = accept.place_on_field(0, "TEST-ADD-TOP-SEC-COST", Some(0));
    accept
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    accept.game.drain_effect_queue();
    let view = accept.pending_selection_view().expect("cost prompt");
    assert_eq!(view.valid_action_ids, vec![HAND_EFFECT_START]);
    accept
        .execute_action(0, HAND_EFFECT_START)
        .expect("accept cost");
    assert_eq!(accept.memory(), 1);
    assert_eq!(accept.security_count(0), 0);
    assert!(
        zone_ids(&accept.game.players[0].hand, &accept.game.card_data).contains(&"SEC".to_string())
    );
}
