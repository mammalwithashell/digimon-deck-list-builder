use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlaySource};
use digimon_engine::permanent::PermanentHandle;
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
fn event_target_trait_predicate_still_matches_security_attacker() {
    let yaml = r#"
card: DSL-EVT-TARGET-TRAIT
name: Event Target Trait
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_security_check
    condition: { event_target_trait_has: Dragon }
    process:
      - gain_memory: 3
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card("SEC-VAC", "Security Vaccine", &["Vaccine"], 1000))
        .add_card(digimon_card("ATTACKER", "Attacker", &["Dragon"], 2000))
        .security(1, &["SEC-VAC"])
        .build();
    let observer = runner.place_on_field(1, "DSL-EVT-TARGET-TRAIT", None);
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

#[test]
fn on_move_event_target_trait_predicate_matches_moved_permanent() {
    let yaml = r#"
card: DSL-MOVE-OBS
name: Move Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_move
    condition: { event_target_trait_has: Rock }
    process:
      - gain_memory: 2
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card("BABY-ROCK", "Rock Baby", &["Rock"], 1000))
        .build();
    let observer = runner.place_on_field(0, "DSL-MOVE-OBS", None);
    let moved = runner.place_on_field(0, "BABY-ROCK", None);
    let event_card = runner.game.players[moved.player as usize].battle_area[moved.index as usize]
        .top_card()
        .handle();

    runner.game.enqueue_triggered(
        EffectTiming::OnMove,
        TriggerSource::MovedFromBreeding {
            player: 0,
            permanent: PermanentHandle {
                player: moved.player,
                index: moved.index,
            },
            card: event_card,
        },
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.memory(), 2);
    assert_eq!(observer.player, 0);
}

#[test]
fn on_digivolve_event_card_trait_predicate_matches_new_top_card() {
    let yaml = r#"
card: DSL-DIGI-OBS
name: Digivolve Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_digivolve
    condition: { event_card_trait_has: Mineral }
    process:
      - gain_memory: 3
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card("BASE", "Base", &[], 1000))
        .add_card({
            let mut card = digimon_card("EVO-MINERAL", "Mineral Evo", &["Mineral"], 3000);
            card.level = Some(4);
            card.evo_costs = vec![EvoCost {
                card_color: CardColor::Red as u8,
                level: 3,
                memory_cost: 0,
            }];
            card
        })
        .hand(0, &["EVO-MINERAL"])
        .build();
    runner.place_on_field(0, "DSL-DIGI-OBS", None);
    let target = runner.place_on_field(0, "BASE", None);
    runner.game.enter_main_phase();

    assert!(runner.game.digivolve_from_hand(
        0,
        0,
        target.index as usize,
        PlaySource::ByDigivolve,
    ));

    assert_eq!(runner.memory(), 3);
}

#[test]
fn on_digivolve_event_target_binding_resolves_digivolved_permanent() {
    let yaml = r#"
card: DSL-DIGI-BIND
name: Digivolve Binder
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_digivolve
    process:
      - add_dp_modifier:
          target: event_target
          value: 1000
          expiry: end_of_turn
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card("BASE", "Base", &[], 1000))
        .add_card({
            let mut card = digimon_card("EVO-TARGET", "Target Evo", &[], 3000);
            card.level = Some(4);
            card.evo_costs = vec![EvoCost {
                card_color: CardColor::Red as u8,
                level: 3,
                memory_cost: 0,
            }];
            card
        })
        .hand(0, &["EVO-TARGET"])
        .build();
    let observer = runner.place_on_field(0, "DSL-DIGI-BIND", None);
    let target = runner.place_on_field(0, "BASE", None);
    runner.game.enter_main_phase();

    assert!(runner.game.digivolve_from_hand(
        0,
        0,
        target.index as usize,
        PlaySource::ByDigivolve,
    ));

    assert_eq!(
        runner.effective_dp(target),
        Some(4000),
        "event_target should bind to the just-digivolved permanent"
    );
    assert_eq!(
        runner.effective_dp(observer),
        Some(2000),
        "observer should not receive its own event_target modifier"
    );
}
