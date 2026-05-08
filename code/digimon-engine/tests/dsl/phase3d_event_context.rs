use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{
    CardColor, CardKind, CardSourceRef, CostDelta, DelayTrigger, EffectTiming, PlaySource,
    StackPosition,
};
use digimon_engine::permanent::{Permanent, PermanentHandle};
use digimon_engine::selection::{OptionPlayResult, TriggerSource};
use std::sync::Arc;

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
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

fn option_card(card_id: &str, name: &str, traits: &[&str]) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Option,
        level: None,
        dp: None,
        play_cost: 0,
        colors: vec![CardColor::Red],
        traits: traits.iter().map(|t| t.to_string()).collect(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

struct DelayOptionNoop;
impl CardEffect for DelayOptionNoop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .option_main()
            .name("Delay noop")
            .delay(DelayTrigger::EndOfYourNextTurn)
            .process(|_ctx| {})
            .build()]
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

    assert_eq!(
        runner.memory(),
        -3,
        "observer belongs to player 1, so gain_memory moves the gauge toward that controller"
    );
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
        .add_card(digimon_card(
            "SEC-VAC",
            "Security Vaccine",
            &["Vaccine"],
            1000,
        ))
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

    assert_eq!(
        runner.memory(),
        -3,
        "observer belongs to player 1, so gain_memory moves the gauge toward that controller"
    );
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
fn on_move_real_breeding_dispatch_supplies_moved_card_context() {
    let yaml = r#"
card: DSL-MOVE-REAL-OBS
name: Real Move Observer
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
effects:
  - when: on_move
    condition: { event_target_trait_has: Rock }
    process:
      - gain_memory: 2
"#;

    let moved_card = digimon_card("ROOKIE-ROCK", "Rock Rookie", &["Rock"], 1000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(moved_card)
        .digitama(0, &["ROOKIE-ROCK"])
        .memory(10)
        .build();

    runner.place_on_field(0, "DSL-MOVE-REAL-OBS", None);

    assert!(runner.game.hatch(0), "hatch Rock rookie into breeding");
    let after_hatch = runner.memory();

    assert!(
        runner.game.move_from_breeding(0),
        "move Rock rookie from breeding to battle"
    );
    assert_eq!(
        runner.memory(),
        after_hatch + 2,
        "real move dispatch should expose moved card trait context"
    );
}

#[test]
fn hatch_does_not_fire_on_move_observers() {
    let yaml = r#"
card: DSL-MOVE-HATCH-OBS
name: Hatch Observer
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
effects:
  - when: on_move
    condition: { event_target_trait_has: Rock }
    process:
      - gain_memory: 2
"#;

    let mut moved_card = digimon_card("BABY-HATCH-ROCK", "Hatch Rock Baby", &["Rock"], 1000);
    moved_card.level = Some(2);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(moved_card)
        .digitama(0, &["BABY-HATCH-ROCK"])
        .memory(10)
        .build();

    runner.place_on_field(0, "DSL-MOVE-HATCH-OBS", None);
    let before = runner.memory();

    assert!(runner.game.hatch(0), "hatch Rock baby into breeding");
    assert_eq!(
        runner.memory(),
        before,
        "hatch into breeding must not fire OnMove"
    );
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

    assert!(runner
        .game
        .digivolve_from_hand(0, 0, target.index as usize, PlaySource::ByDigivolve,));

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

    assert!(runner
        .game
        .digivolve_from_hand(0, 0, target.index as usize, PlaySource::ByDigivolve,));

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

#[test]
fn event_is_effect_initiated_matches_effect_play_only() {
    let yaml = r#"
card: DSL-EFFECT-PLAY-OBS
name: Effect Play Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_enter_field_anyone
    condition: { event_is_effect_initiated: true }
    process:
      - gain_memory: 2
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card("NORMAL-PLAY", "Normal Play", &[], 1000))
        .add_card(digimon_card("EFFECT-PLAY", "Effect Play", &[], 1000))
        .hand(0, &["NORMAL-PLAY", "EFFECT-PLAY"])
        .build();
    runner.place_on_field(0, "DSL-EFFECT-PLAY-OBS", None);
    runner.game.enter_main_phase();

    assert_eq!(
        runner
            .game
            .play_from_hand_with_cost(0, 0, CostDelta::Free, PlaySource::ByHand),
        Some(1),
        "normal play precondition"
    );
    assert_eq!(
        runner.memory(),
        0,
        "normal main-phase play must not satisfy event_is_effect_initiated"
    );

    let effect_card = runner.game.players[0].hand[0].handle();
    assert_eq!(
        runner.game.play_card_from_effect_without_cost(0, effect_card),
        Some(PermanentHandle {
            player: 0,
            index: 2,
        }),
        "effect play precondition"
    );
    assert_eq!(
        runner.memory(),
        2,
        "effect play should satisfy event_is_effect_initiated"
    );
}

#[test]
fn event_is_effect_initiated_matches_effect_digivolve_only() {
    let yaml = r#"
card: DSL-EFFECT-DIGI-OBS
name: Effect Digivolve Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_digivolve
    condition:
      all_of:
        - event_is_effect_initiated: true
        - event_card_trait_has: Mineral
    process:
      - gain_memory: 3
"#;
    let evo = |id: &str| {
        let mut card = digimon_card(id, "Mineral Evo", &["Mineral"], 3000);
        card.level = Some(4);
        card.evo_costs = vec![EvoCost {
            card_color: CardColor::Red as u8,
            level: 3,
            memory_cost: 0,
        }];
        card
    };
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card("BASE-NORMAL", "Normal Base", &[], 1000))
        .add_card(digimon_card("BASE-EFFECT", "Effect Base", &[], 1000))
        .add_card(evo("NORMAL-EVO"))
        .add_card(evo("EFFECT-EVO"))
        .hand(0, &["NORMAL-EVO", "EFFECT-EVO"])
        .build();
    runner.place_on_field(0, "DSL-EFFECT-DIGI-OBS", None);
    let normal_target = runner.place_on_field(0, "BASE-NORMAL", None);
    let effect_target = runner.place_on_field(0, "BASE-EFFECT", None);
    runner.game.enter_main_phase();

    assert!(runner.game.digivolve_from_hand(
        0,
        0,
        normal_target.index as usize,
        PlaySource::ByDigivolve,
    ));
    assert_eq!(
        runner.memory(),
        0,
        "normal digivolve must not satisfy event_is_effect_initiated"
    );

    assert!(runner.game.effect_initiated_digivolve(
        0,
        0,
        effect_target,
        digimon_engine::enums::CostDelta::Free,
        false,
        PlaySource::ByEffect,
    ));
    assert_eq!(
        runner.memory(),
        3,
        "effect-initiated digivolve should carry event card plus effect-origin flag"
    );
}

#[test]
fn event_target_owner_predicate_compiles() {
    let yaml = r#"
card: DSL-EVT-OWNER-COMPILE
name: Event Owner Compile
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_digivolve
    condition: { event_target_owner: you }
    process:
      - gain_memory: 1
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");
    let digimon_dsl::compiled::CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let condition = triggered.condition.as_ref().expect("condition");
    assert_eq!(
        condition.event_target_owner,
        Some(digimon_dsl::compiled::CompiledPlayerRef::You)
    );
}

#[test]
fn event_permanent_is_source_predicate_compiles() {
    let yaml = r#"
card: DSL-EVT-SOURCE
name: Event Source
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_suspend
    condition: { event_permanent_is_source: true }
    process:
      - gain_memory: 1
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");
    let digimon_dsl::compiled::CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let condition = triggered.condition.as_ref().expect("condition");
    assert_eq!(condition.event_permanent_is_source, Some(true));
}

#[test]
fn event_permanent_is_source_matches_suspended_subject_to_observer_source() {
    let yaml = r#"
card: DSL-SELF-SUSPEND
name: Self Suspend Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_suspend
    condition: { event_permanent_is_source: true }
    process:
      - gain_memory: 2
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card("ALLY-SUSPEND", "Ally Suspend", &[], 1000))
        .build();
    let observer = runner.place_on_field(0, "DSL-SELF-SUSPEND", None);
    let ally = runner.place_on_field(0, "ALLY-SUSPEND", None);

    runner.game.suspend(ally);
    assert_eq!(
        runner.memory(),
        0,
        "ally suspending must not satisfy event_permanent_is_source for the observer"
    );

    runner.game.suspend(observer);
    assert_eq!(
        runner.memory(),
        2,
        "self suspending should satisfy event_permanent_is_source"
    );
}

#[test]
fn source_trash_host_and_trashed_source_predicates_compile() {
    let yaml = r#"
card: DSL-SOURCE-PREDS-COMPILE
name: Source Predicate Compile
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_digivolution_card_trashed
    condition:
      all_of:
        - host_permanent_trait_has: Rock
        - trashed_source_trait_has: Mineral
        - trashed_source_card_id_is: UNDER-MINERAL
    process:
      - gain_memory: 1
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");
    let digimon_dsl::compiled::CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let condition = triggered.condition.as_ref().expect("condition");
    assert!(condition
        .all_of
        .iter()
        .any(|p| p.host_permanent_trait_has.as_deref() == Some("Rock")));
    assert!(condition
        .all_of
        .iter()
        .any(|p| p.trashed_source_trait_has.as_deref() == Some("Mineral")));
    assert!(condition
        .all_of
        .iter()
        .any(|p| p.trashed_source_card_id_is.as_deref() == Some("UNDER-MINERAL")));
}

#[test]
fn on_enter_field_anyone_event_card_trait_predicate_matches_entering_card() {
    let yaml = r#"
card: DSL-ENTER-OBS
name: Enter Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_enter_field_anyone
    condition: { event_card_trait_has: Royal Knight }
    process:
      - gain_memory: 4
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card({
            let mut card = digimon_card("RK-ENTER", "Royal Knight", &["Royal Knight"], 3000);
            card.play_cost = 0;
            card
        })
        .hand(1, &["RK-ENTER"])
        .memory(10)
        .build();
    runner.place_on_field(0, "DSL-ENTER-OBS", None);

    let before = runner.memory();
    assert_eq!(runner.play(1, 0), Some(0));

    assert_eq!(
        runner.memory(),
        before + 4,
        "observer should see the entering card traits through event context"
    );
}

#[test]
fn on_any_digimon_played_alias_uses_enter_field_payload() {
    let yaml = r#"
card: DSL-ANY-PLAYED-OBS
name: Any Played Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_any_digimon_played
    condition: { event_card_trait_has: Royal Knight }
    process:
      - gain_memory: 4
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card({
            let mut card = digimon_card("RK-ANY-PLAYED", "Royal Knight", &["Royal Knight"], 3000);
            card.play_cost = 0;
            card
        })
        .hand(1, &["RK-ANY-PLAYED"])
        .memory(10)
        .build();
    runner.place_on_field(0, "DSL-ANY-PLAYED-OBS", None);

    let before = runner.memory();
    assert_eq!(runner.play(1, 0), Some(0));

    assert_eq!(
        runner.memory(),
        before + 4,
        "alias should share OnEnterFieldAnyone subject and event_card payload"
    );
}

#[test]
fn on_option_placed_event_card_trait_predicate_matches_placed_option() {
    let yaml = r#"
card: DSL-OPT-OBS
name: Option Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_option_placed
    condition: { event_card_trait_has: Royal Knight }
    process:
      - gain_memory: 2
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(option_card(
            "RK-OPTION",
            "Royal Knight Option",
            &["Royal Knight"],
        ))
        .hand(0, &["RK-OPTION"])
        .build();
    runner.register_effect("RK-OPTION", Arc::new(DelayOptionNoop));
    runner.place_on_field(0, "DSL-OPT-OBS", None);
    runner.game.enter_main_phase();

    let before = runner.memory();
    let battle_before = runner.battle_area_size(0);
    let result = runner.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Trashed);

    assert_eq!(
        runner.battle_area_size(0),
        battle_before + 1,
        "Delay option should be placed before OnOptionPlaced observers resolve"
    );

    assert_eq!(runner.memory(), before + 2);
}

#[test]
fn on_place_security_event_card_trait_predicate_matches_placed_card() {
    let yaml = r#"
card: DSL-PLACE-SEC-OBS
name: Place Security Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_place_security
    condition: { event_card_trait_has: Royal Knight }
    process:
      - gain_memory: 3
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card(
            "RK-SECURITY-PLACED",
            "Royal Knight Security",
            &["Royal Knight"],
            3000,
        ))
        .memory(5)
        .build();
    let observer = runner.place_on_field(0, "DSL-PLACE-SEC-OBS", None);
    let placed_data = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == "RK-SECURITY-PLACED")
        .expect("registered security placement card");
    let placed = CardSource::new(placed_data, 0, runner.game.next_card_index());
    runner.game.players[0].trash.push(placed);

    let before = runner.memory();
    let source_card = runner.game.players[observer.player as usize].battle_area
        [observer.index as usize]
        .top_card()
        .handle();
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(observer), 0);
    assert!(ctx.place_on_security(0, CardSourceRef::Trash(0, 0), StackPosition::Top, false));

    assert_eq!(
        runner.memory(),
        before + 3,
        "DSL on_place_security should expose the placed card as event_card"
    );
}

#[test]
fn on_added_to_security_alias_uses_place_security_payload() {
    let yaml = r#"
card: DSL-ADDED-SEC-OBS
name: Added Security Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_added_to_security
    condition: { event_card_trait_has: Royal Knight }
    process:
      - gain_memory: 3
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card(
            "RK-SECURITY-ADDED",
            "Royal Knight Security",
            &["Royal Knight"],
            3000,
        ))
        .memory(5)
        .build();
    let observer = runner.place_on_field(0, "DSL-ADDED-SEC-OBS", None);
    let placed_data = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == "RK-SECURITY-ADDED")
        .expect("registered security placement card");
    let placed = CardSource::new(placed_data, 0, runner.game.next_card_index());
    runner.game.players[0].trash.push(placed);

    let before = runner.memory();
    let source_card = runner.game.players[observer.player as usize].battle_area
        [observer.index as usize]
        .top_card()
        .handle();
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(observer), 0);
    assert!(ctx.place_on_security(0, CardSourceRef::Trash(0, 0), StackPosition::Top, false));

    assert_eq!(
        runner.memory(),
        before + 3,
        "alias should share OnPlaceSecurity event_card payload"
    );
}

#[test]
fn on_discard_security_event_cause_predicate_matches_effect_trash() {
    let yaml = r#"
card: DSL-DISCARD-SEC
name: Discard Security
kind: option
color: [yellow]
cost: 0
effects:
  - when: on_discard_security
    condition: { event_cause: opponent_effect }
    process:
      - gain_memory: 4
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card("TRASHER", "Security Trasher", &[], 3000))
        .security(1, &["DSL-DISCARD-SEC"])
        .memory(0)
        .build();
    let trasher = runner.place_on_field(0, "TRASHER", None);
    let source_card = runner.game.players[trasher.player as usize].battle_area
        [trasher.index as usize]
        .top_card()
        .handle();
    runner.game.set_effect_source_player_for_test(Some(0));
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(trasher), 0);
    assert!(ctx.trash_top_security(1));
    runner.game.set_effect_source_player_for_test(None);

    assert_eq!(
        runner.memory(),
        -4,
        "DSL on_discard_security should see opponent-effect security trash payload"
    );
}

#[test]
fn on_digivolution_card_trashed_event_card_trait_predicate_matches_trashed_source() {
    let yaml = r#"
card: DSL-SOURCE-TRASH-OBS
name: Source Trash Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_digivolution_card_trashed
    condition: { event_card_trait_has: Mineral }
    process:
      - gain_memory: 3
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card("TOP", "Top", &[], 4000))
        .add_card(digimon_card(
            "UNDER-MINERAL",
            "Under Mineral",
            &["Mineral"],
            1000,
        ))
        .build();
    runner.place_on_field(0, "DSL-SOURCE-TRASH-OBS", None);
    let host = {
        let g = runner.game_mut();
        let turn = g.turn_count;
        let under_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "UNDER-MINERAL")
            .unwrap();
        let top_idx = g.card_data.iter().position(|c| c.card_id == "TOP").unwrap();
        let under = CardSource::new(under_idx, 0, g.next_card_index());
        let top = CardSource::new(top_idx, 0, g.next_card_index());
        let mut permanent = Permanent::new(under, turn);
        permanent.card_sources.push(top);
        g.players[0].battle_area.push(permanent);
        PermanentHandle {
            player: 0,
            index: (g.players[0].battle_area.len() - 1) as u8,
        }
    };

    assert!(runner.game_mut().return_to_hand(host).is_some());

    assert_eq!(runner.memory(), 3);
}

#[test]
fn on_digivolution_card_trashed_host_and_trashed_source_predicates_match_event_context() {
    let yaml = r#"
card: DSL-SOURCE-TRASH-CONTEXT
name: Source Trash Context
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_digivolution_card_trashed
    condition:
      all_of:
        - event_target_owner: you
        - host_permanent_trait_has: Rock
        - trashed_source_trait_has: Mineral
        - trashed_source_card_id_is: UNDER-MINERAL
    process:
      - gain_memory: 4
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card("BASE", "Base", &[], 1000))
        .add_card(digimon_card(
            "UNDER-MINERAL",
            "Under Mineral",
            &["Mineral"],
            1000,
        ))
        .add_card(digimon_card("HOST-ROCK", "Host Rock", &["Rock"], 4000))
        .build();
    runner.place_on_field(0, "DSL-SOURCE-TRASH-CONTEXT", None);
    let (host, host_card, trashed_card) = {
        let g = runner.game_mut();
        let turn = g.turn_count;
        let base_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "BASE")
            .unwrap();
        let under_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "UNDER-MINERAL")
            .unwrap();
        let host_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "HOST-ROCK")
            .unwrap();
        let base = CardSource::new(base_idx, 0, g.next_card_index());
        let under = CardSource::new(under_idx, 0, g.next_card_index());
        let top = CardSource::new(host_idx, 0, g.next_card_index());
        let trashed_card = under.handle();
        let host_card = top.handle();
        let mut permanent = Permanent::new(base, turn);
        permanent.card_sources.push(under);
        permanent.card_sources.push(top);
        g.players[0].battle_area.push(permanent);
        (
            PermanentHandle {
                player: 0,
                index: (g.players[0].battle_area.len() - 1) as u8,
            },
            host_card,
            trashed_card,
        )
    };

    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolutionCardTrashed,
        TriggerSource::SourceTrashedFromStack {
            player: 0,
            host,
            host_card,
            card: trashed_card,
            cause: digimon_engine::trigger_context::EventCause::EffectDeletion,
        },
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.memory(), 4);
}

#[test]
fn source_trash_event_target_owner_uses_trashed_stack_host_owner_not_observer() {
    let yaml = r#"
card: DSL-SOURCE-TRASH-OWNER
name: Source Trash Owner
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_digivolution_card_trashed
    condition:
      all_of:
        - event_target_owner: opponent
        - host_permanent_trait_has: Rock
        - trashed_source_trait_has: Mineral
    process:
      - gain_memory: 4
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card("BASE", "Base", &[], 1000))
        .add_card(digimon_card(
            "UNDER-MINERAL",
            "Under Mineral",
            &["Mineral"],
            1000,
        ))
        .add_card(digimon_card("HOST-ROCK", "Host Rock", &["Rock"], 4000))
        .build();
    runner.place_on_field(0, "DSL-SOURCE-TRASH-OWNER", None);
    let (host, host_card, trashed_card) = {
        let g = runner.game_mut();
        let turn = g.turn_count;
        let base_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "BASE")
            .unwrap();
        let under_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "UNDER-MINERAL")
            .unwrap();
        let host_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "HOST-ROCK")
            .unwrap();
        let base = CardSource::new(base_idx, 1, g.next_card_index());
        let under = CardSource::new(under_idx, 1, g.next_card_index());
        let top = CardSource::new(host_idx, 1, g.next_card_index());
        let trashed_card = under.handle();
        let host_card = top.handle();
        let mut permanent = Permanent::new(base, turn);
        permanent.card_sources.push(under);
        permanent.card_sources.push(top);
        g.players[1].battle_area.push(permanent);
        (
            PermanentHandle {
                player: 1,
                index: (g.players[1].battle_area.len() - 1) as u8,
            },
            host_card,
            trashed_card,
        )
    };

    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolutionCardTrashed,
        TriggerSource::SourceTrashedFromStack {
            player: 1,
            host,
            host_card,
            card: trashed_card,
            cause: digimon_engine::trigger_context::EventCause::EffectDeletion,
        },
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.memory(), 4);
}

struct DeleteSelfThenPlayNext;

impl CardEffect for DeleteSelfThenPlayNext {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Delete self then play next card")
            .process(|ctx| {
                if let Some(handle) = ctx.source_permanent {
                    ctx.delete_permanent(handle);
                }
                let _ = ctx.play_from_hand_free(ctx.player, 0);
            })
            .build()]
    }
}

#[test]
fn on_enter_field_event_target_trait_uses_event_card_when_entered_handle_is_stale() {
    let yaml = r#"
card: DSL-ENTER-STALE-OBS
name: Enter Stale Observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_enter_field_anyone
    condition: { event_target_trait_has: Royal Knight }
    process:
      - gain_memory: 5
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card({
            let mut card = digimon_card("SELF-ROYAL", "Self Royal", &["Royal Knight"], 3000);
            card.play_cost = 0;
            card
        })
        .add_card({
            let mut card = digimon_card("DECOY", "Decoy", &[], 1000);
            card.play_cost = 0;
            card
        })
        .hand(1, &["SELF-ROYAL", "DECOY"])
        .memory(10)
        .build();
    runner.register_effect("SELF-ROYAL", Arc::new(DeleteSelfThenPlayNext));
    runner.place_on_field(0, "DSL-ENTER-STALE-OBS", None);

    let before = runner.memory();
    assert_eq!(runner.play(1, 0), Some(0));

    assert_eq!(
        runner.memory(),
        before + 5,
        "event_target_trait_has should not inspect a different permanent through a stale event_permanent handle"
    );
}
