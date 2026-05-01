use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

fn digimon_card(id: &str, name: &str, traits: &[&str], dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(dp),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: traits.iter().map(|t| t.to_string()).collect(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

#[test]
fn event_target_kind_predicate_matches_digivolving_permanent() {
    let yaml = r#"
card: DSL-EVT-KIND
name: Event Kind
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: when_digivolving
    condition: { event_target_kind: digimon }
    process:
      - gain_memory: 2
"#;
    let mut runner = DebugRunner::builder().from_dsl_yaml(yaml).unwrap().build();
    let target = runner.place_on_field(0, "DSL-EVT-KIND", None);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(target),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.memory(), 2);
}

#[test]
fn event_card_trait_predicate_matches_revealed_security_card() {
    let yaml = r#"
card: DSL-EVT-OBS
name: Event Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_security_check
    condition: { event_card_trait_has: Vaccine }
    process:
      - gain_memory: 3
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card(
            "SEC-VAC",
            "Security Vaccine",
            &["Vaccine"],
            1000,
        ))
        .add_card(digimon_card("ATTACKER", "Attacker", &[], 2000))
        .security(1, &["SEC-VAC"])
        .build();
    let observer = runner.place_on_field(1, "DSL-EVT-OBS", None);
    let attacker = runner.place_on_field(0, "ATTACKER", None);
    let revealed = runner.game.players[1].security[0].handle();

    runner.game.enqueue_triggered(
        EffectTiming::OnSecurityCheck,
        TriggerSource::OnSecurityCheck {
            attacker,
            defender: observer.player,
            revealed_card: revealed,
            was_face_up: false,
        },
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.memory(), 3);
}

#[test]
fn event_target_binding_resolves_trigger_permanent() {
    let yaml = r#"
card: DSL-EVT-TARGET
name: Event Target
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: when_digivolving
    process:
      - add_dp_modifier:
          target: event_target
          value: 1000
          expiry: end_of_turn
"#;
    let mut runner = DebugRunner::builder().from_dsl_yaml(yaml).unwrap().build();
    let target = runner.place_on_field(0, "DSL-EVT-TARGET", None);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(target),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.effective_dp(target), Some(3000));
}

#[test]
fn event_card_binding_resolves_trigger_card() {
    let yaml = r#"
card: DSL-EVT-CARD
name: Event Card
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_security_check
    process:
      - mark_security_face_up:
          of: you
          card: event_card
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card(
            "SEC-CARD",
            "Security Card",
            &["Vaccine"],
            1000,
        ))
        .add_card(digimon_card("ATTACKER", "Attacker", &[], 2000))
        .security(1, &["SEC-CARD"])
        .build();
    let observer = runner.place_on_field(1, "DSL-EVT-CARD", None);
    let attacker = runner.place_on_field(0, "ATTACKER", None);
    let revealed = runner.game.players[1].security[0].handle();

    runner.game.enqueue_triggered(
        EffectTiming::OnSecurityCheck,
        TriggerSource::OnSecurityCheck {
            attacker,
            defender: observer.player,
            revealed_card: revealed,
            was_face_up: false,
        },
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.game.players[1]
            .face_up_security
            .contains(&revealed.0),
        "event_card binding should let the observer mark the revealed card face-up"
    );
}
