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
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
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

#[test]
fn may_dna_digivolve_now_prompts_clone_faithfully() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("SRC-A", 3, Vec::new(), Vec::new()))
        .add_card(digimon("SRC-B", 3, Vec::new(), Vec::new()))
        .add_card(digimon("DNA-RESULT", 4, vec![dna_cost()], Vec::new()))
        .hand(0, &["DNA-RESULT"])
        .memory(5)
        .start();

    let anchor = runner.place_on_field(0, "SRC-A", None);
    let _partner = runner.place_on_field(0, "SRC-B", None);
    runner.game.enter_main_phase();

    let source = runner.game.player(0).battle_area[anchor.index as usize]
        .top_card()
        .handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, source, Some(anchor), 0);
        ctx.may_dna_digivolve_now(
            anchor,
            Arc::new(|game, handle| {
                game.player(handle.player)
                    .battle_area
                    .get(handle.index as usize)
                    .is_some_and(|perm| perm.top_card().card_id(&game.card_data) == "SRC-B")
            }),
            Arc::new(|game, hand_index| {
                game.player(0)
                    .hand
                    .get(hand_index)
                    .is_some_and(|card| card.card_id(&game.card_data) == "DNA-RESULT")
            }),
            0,
            false,
            false,
            None,
            None,
        );
    }

    assert!(
        runner.game.pending_selection_resume.is_some(),
        "may-DNA partner prompt must be resume-driven before cloning"
    );
    let partner_action = runner
        .game
        .pending_selection
        .as_ref()
        .expect("partner prompt installed")
        .valid_action_ids[0];

    let mut clone_at_partner = runner.game.clone();
    clone_at_partner
        .resolve_selection(0, partner_action)
        .expect("clone picks DNA partner");
    assert!(
        clone_at_partner.pending_selection_resume.is_some(),
        "result-card prompt installed by clone must also be resume-driven"
    );
    assert!(
        runner.game.pending_selection.is_some(),
        "resolving the clone leaves the original at the partner prompt"
    );

    let result_action = clone_at_partner
        .pending_selection
        .as_ref()
        .expect("result prompt installed")
        .valid_action_ids[0];
    let mut clone_at_result = clone_at_partner.clone();
    clone_at_result
        .resolve_selection(0, result_action)
        .expect("clone picks DNA result card");

    assert_eq!(
        clone_at_result.player(0).battle_area.len(),
        1,
        "clone performs the DNA merge"
    );
    assert_eq!(
        clone_at_result.player(0).battle_area[0]
            .top_card()
            .card_id(&clone_at_result.card_data),
        "DNA-RESULT",
        "clone's merged stack has the selected result on top"
    );
    assert!(
        clone_at_partner.pending_selection.is_some(),
        "resolving the result-prompt clone leaves its parent clone at the result prompt"
    );
}

#[test]
fn on_digivolve_global_observer_reads_dna_origin_payload() {
    let yaml = r#"
card: DNA-RESULT
name: DNA Result
kind: digimon
level: 4
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_digivolve
    condition: { dna_origin: true }
    process:
      - gain_memory: 4
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

    assert_eq!(runner.game.memory, before + 4);
}

#[test]
fn effect_initiated_dna_sets_effect_origin_on_global_payload() {
    let yaml = r#"
card: DNA-RESULT
name: DNA Result
kind: digimon
level: 4
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_digivolve
    condition:
      all_of:
        - dna_origin: true
        - event_is_effect_initiated: true
    process:
      - gain_memory: 4
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

    assert_eq!(runner.game.memory, before + 4);
}
