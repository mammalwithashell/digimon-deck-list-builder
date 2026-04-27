use std::sync::Arc;

use digimon_engine::card_data::{CardData, DnaCost, DnaRequirement, EvoCost};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, GamePhase};

fn digimon(id: &str, level: u8, dna_costs: Vec<DnaCost>, evo_costs: Vec<EvoCost>) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(level),
        dp: Some(3000),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs,
        dna_costs,
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn red_level_req(level: u8) -> DnaRequirement {
    DnaRequirement {
        level,
        card_colors: vec![CardColor::Red],
        name_contains: String::new(),
        text_contains: String::new(),
    }
}

fn dna_cost() -> DnaCost {
    DnaCost {
        requirement1: red_level_req(3),
        requirement2: red_level_req(3),
        memory_cost: 0,
    }
}

fn register_dsl(runner: &mut DebugRunner, yaml: &str) {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("valid DSL YAML");
    let compiled = digimon_dsl::compile::compile(&spec).expect("DSL compiles");
    let card_id = compiled.card.clone();
    runner.register_effect(&card_id, Arc::new(DslCardEffect::new(Arc::new(compiled))));
}

#[test]
fn effect_initiated_dna_fires_on_dna_and_sets_dna_origin() {
    let yaml = r#"
card: DNA-RESULT
name: DNA Result
kind: digimon
level: 4
color: [red]
cost: 3
dp: 3000
effects:
  - when: when_digivolving
    condition: { dna_origin: true }
    process:
      - gain_memory: 3
  - when: on_dna_digivolve
    process:
      - gain_memory: 2
"#;

    let mut runner = DebugRunner::builder()
        .add_card(digimon("SRC-A", 3, Vec::new(), Vec::new()))
        .add_card(digimon("SRC-B", 3, Vec::new(), Vec::new()))
        .add_card(digimon("DNA-RESULT", 4, vec![dna_cost()], Vec::new()))
        .hand(0, &["DNA-RESULT"])
        .memory(5)
        .start();
    register_dsl(&mut runner, yaml);

    let a = runner.place_on_field(0, "SRC-A", None);
    let b = runner.place_on_field(0, "SRC-B", None);
    let hand_card = runner.game.player(0).hand[0].handle();

    let before = runner.game.memory;
    {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card, None, 0);
        assert!(ctx
            .effect_initiated_dna_digivolve(a, b, hand_card, 0, true)
            .is_some());
    }

    assert_eq!(runner.game.memory, before + 5);
    assert_eq!(runner.game.player(0).battle_area.len(), 1);
}

#[test]
fn non_dna_digivolve_does_not_satisfy_dna_origin_predicate() {
    let yaml = r#"
card: STD-RESULT
name: Standard Result
kind: digimon
level: 4
color: [red]
cost: 3
dp: 3000
effects:
  - when: when_digivolving
    condition: { dna_origin: true }
    process:
      - gain_memory: 3
"#;

    let mut runner = DebugRunner::builder()
        .add_card(digimon("BASE", 3, Vec::new(), Vec::new()))
        .add_card(digimon(
            "STD-RESULT",
            4,
            Vec::new(),
            vec![EvoCost {
                card_color: 0,
                level: 3,
                memory_cost: 0,
            }],
        ))
        .hand(0, &["STD-RESULT"])
        .memory(5)
        .start();
    register_dsl(&mut runner, yaml);

    let target = runner.place_on_field(0, "BASE", None);
    let before = runner.game.memory;
    let source = runner.game.player(0).hand[0].handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
        assert!(ctx.effect_initiated_digivolve(
            0,
            0,
            target,
            digimon_engine::enums::CostDelta::Free,
            true,
        ));
    }

    assert_eq!(runner.game.memory, before);
}

#[test]
fn player_action_dna_selection_executes_and_fires_dna_triggers() {
    let yaml = r#"
card: DNA-RESULT
name: DNA Result
kind: digimon
level: 4
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_dna_digivolve
    process:
      - gain_memory: 2
"#;

    let mut runner = DebugRunner::builder()
        .add_card(digimon("SRC-A", 3, Vec::new(), Vec::new()))
        .add_card(digimon("SRC-B", 3, Vec::new(), Vec::new()))
        .add_card(digimon("DNA-RESULT", 4, vec![dna_cost()], Vec::new()))
        .hand(0, &["DNA-RESULT"])
        .memory(5)
        .start();
    register_dsl(&mut runner, yaml);
    runner.game.enter_main_phase();

    runner.place_on_field(0, "SRC-A", None);
    runner.place_on_field(0, "SRC-B", None);

    let before = runner.game.memory;
    assert!(runner.game.initiate_dna_digivolve(0, 0));
    assert_eq!(runner.game.current_phase, GamePhase::SelectMaterial);
    runner.game.resolve_selection(0, 0).expect("first material");
    assert_eq!(runner.game.current_phase, GamePhase::SelectMaterial);
    runner
        .game
        .resolve_selection(0, 1)
        .expect("second material");

    assert_eq!(runner.game.memory, before + 2);
    assert_eq!(runner.game.player(0).hand.len(), 0);
    assert_eq!(runner.game.player(0).battle_area.len(), 1);
    assert!(runner.game.pending_selection.is_none());
}
