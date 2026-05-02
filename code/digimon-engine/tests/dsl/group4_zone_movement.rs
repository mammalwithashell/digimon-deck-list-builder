use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledClause, CompiledCostDelta, CompiledPlayerRef,
    CompiledStackPosition, CompiledStep,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_step;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};

#[test]
fn security_stack_steps_parse_and_compile() {
    let yaml = r#"
card: G4-SEC
name: "Group 4 Security Steps"
kind: option
color: [yellow]
cost: 0
effects:
  - when: main_from_hand
    process:
      - select_security:
          of: opponent
          bind_as: picked
          filter: {}
          prompt: "Choose security"
      - add_to_hand_from_security: { of: opponent, card: picked }
      - shuffle_security: { of: opponent }
"#;

    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered effect");
    };

    let CompiledStep::SelectSecurity { .. } = &triggered.process[0] else {
        panic!("expected select_security step");
    };
    assert_eq!(triggered.process.len(), 3);
    assert!(matches!(
        &triggered.process[1],
        CompiledStep::AddToHandFromSecurity {
            of: CompiledPlayerRef::Opponent,
            card: CompiledBindingRef::Named(name),
        } if name == "picked"
    ));
    assert!(matches!(
        triggered.process[2],
        CompiledStep::ShuffleSecurity {
            of: CompiledPlayerRef::Opponent
        }
    ));
}

fn card(card_id: &str) -> CardData {
    make_test_card(card_id, card_id)
}

fn evo_lv4(card_id: &str) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: vec![EvoCost {
            card_color: 0,
            level: 3,
            memory_cost: 0,
        }],
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn stack_on_target(runner: &mut DebugRunner, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card should be registered");
    let card_idx = runner.game.next_card_index();
    let source = digimon_engine::card_source::CardSource::new(data_idx, 0, card_idx);
    runner.game.players[0].battle_area[0]
        .card_sources
        .push(source);
}

#[test]
fn group4_zone_movement_effect_digivolve_accepts_source_yaml() {
    let yaml = r#"
card: G4-DIGI
name: "Group 4 Source Digivolve"
kind: option
color: [red]
cost: 0
effects:
  - when: main_from_hand
    process:
      - effect_initiated_digivolve:
          target: target
          source: picked
          cost: free
          ignore_requirements: true
"#;

    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered effect");
    };

    assert!(matches!(
        &triggered.process[0],
        CompiledStep::EffectInitiatedDigivolve {
            target: CompiledBindingRef::Named(target),
            from_hand: CompiledBindingRef::Named(source),
            cost: CompiledCostDelta::Free,
            ignore_requirements: true,
        } if target == "target" && source == "picked"
    ));
}

#[test]
fn group4_zone_movement_effect_digivolve_step_uses_card_binding_live_source() {
    let mut runner = DebugRunner::builder()
        .add_card(card("BASE3"))
        .add_card(evo_lv4("EVO4"))
        .security(0, &["EVO4"])
        .memory(5)
        .start();

    let target = runner.place_on_field(0, "BASE3", None);
    let picked = runner.game.players[0].security[0].handle();
    let source_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();
    let mut bindings = Bindings::new();
    bindings.insert_permanent("target", target);
    bindings.insert_card("picked", picked);

    let step = CompiledStep::EffectInitiatedDigivolve {
        target: CompiledBindingRef::Named("target".to_string()),
        from_hand: CompiledBindingRef::Named("picked".to_string()),
        cost: CompiledCostDelta::Free,
        ignore_requirements: false,
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert_eq!(runner.game.players[0].security.len(), 0);
    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize]
            .top_card()
            .handle(),
        picked
    );
}

#[test]
fn group4_zone_movement_return_to_deck_include_sources_preserves_stack() {
    let mut runner = DebugRunner::builder()
        .add_card(card("BOTTOM"))
        .add_card(card("MIDDLE"))
        .add_card(card("TOP"))
        .add_card(card("FILLER"))
        .deck(0, &["FILLER"])
        .build();

    let handle = runner.place_on_field(0, "BOTTOM", None);
    stack_on_target(&mut runner, "MIDDLE");
    stack_on_target(&mut runner, "TOP");

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let mut bindings = Bindings::new();
    bindings.insert_permanent("target", handle);

    let step = CompiledStep::ReturnToDeck {
        target: CompiledBindingRef::Named("target".to_string()),
        position: CompiledStackPosition::Top,
        include_sources: true,
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert_eq!(runner.game.players[0].battle_area.len(), 0);
    assert_eq!(runner.game.players[0].trash.len(), 0);

    let deck_ids: Vec<String> = runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        deck_ids,
        vec![
            "FILLER".to_string(),
            "BOTTOM".to_string(),
            "MIDDLE".to_string(),
            "TOP".to_string(),
        ],
        "include_sources returns the whole stack to deck in stack order"
    );
}
