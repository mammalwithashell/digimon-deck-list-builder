use digimon_engine::card_data::{CardData, DnaCost, DnaRequirement, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, GamePhase, PlaySource};

fn dna_result_card(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.dna_costs = vec![DnaCost {
        requirement1: DnaRequirement {
            level: 3,
            card_colors: vec![CardColor::Red],
            name_contains: String::new(),
            text_contains: String::new(),
        },
        requirement2: DnaRequirement {
            level: 3,
            card_colors: vec![CardColor::Red],
            name_contains: String::new(),
            text_contains: String::new(),
        },
        memory_cost: 0,
    }];
    card
}

fn normal_evo_card(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(4);
    card.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 0,
    }];
    card
}

fn on_dna_gain_one_yaml(card_id: &str) -> String {
    format!(
        r#"
card: {card_id}
name: DNA Trigger
kind: digimon
level: 4
color: [red]
cost: 0
dp: 4000
effects:
  - when: on_dna_digivolve
    process:
      - gain_memory: 1
"#
    )
}

#[test]
fn effect_initiated_dna_fires_on_dna_digivolve() {
    let yaml = on_dna_gain_one_yaml("DNA-EFFECT");
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .unwrap()
        .add_card(dna_result_card("DNA-EFFECT"))
        .add_card(make_test_card("MAT-A", "Material A"))
        .add_card(make_test_card("MAT-B", "Material B"))
        .hand(0, &["DNA-EFFECT"])
        .memory(0)
        .start();

    let target_a = runner.place_on_field(0, "MAT-A", None);
    let target_b = runner.place_on_field(0, "MAT-B", None);
    let from_hand = runner.game.players[0].hand[0].handle();

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, from_hand, None, 0);
        ctx.effect_initiated_dna_digivolve(target_a, target_b, from_hand, 0, true)
    };

    assert!(result.is_some());
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    assert_eq!(runner.memory(), 1);
}

#[test]
fn user_action_dna_fires_on_dna_digivolve() {
    let yaml = on_dna_gain_one_yaml("DNA-ACTION");
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .unwrap()
        .add_card(dna_result_card("DNA-ACTION"))
        .add_card(make_test_card("MAT-A", "Material A"))
        .add_card(make_test_card("MAT-B", "Material B"))
        .hand(0, &["DNA-ACTION"])
        .memory(0)
        .start();

    runner.place_on_field(0, "MAT-A", None);
    runner.place_on_field(0, "MAT-B", None);
    runner.game.current_phase = GamePhase::Main;

    assert!(runner.game.initiate_dna_digivolve(0, 0));
    assert_eq!(
        runner.pending_kind(),
        Some(digimon_engine::selection::SelectionKind::Material)
    );
    runner
        .execute_action(0, 0)
        .expect("select first DNA material");
    assert_eq!(
        runner.pending_kind(),
        Some(digimon_engine::selection::SelectionKind::Material)
    );
    runner
        .execute_action(0, 1)
        .expect("select second DNA material");

    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    assert_eq!(runner.game.players[0].hand.len(), 0);
    assert_eq!(runner.memory(), 1);
}

#[test]
fn non_dna_digivolve_does_not_fire_on_dna_digivolve() {
    let yaml = on_dna_gain_one_yaml("NORMAL-EVO");
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .unwrap()
        .add_card(normal_evo_card("NORMAL-EVO"))
        .add_card(make_test_card("BASE", "Base"))
        .hand(0, &["NORMAL-EVO"])
        .memory(5)
        .start();

    runner.place_on_field(0, "BASE", None);
    runner.game.current_phase = GamePhase::Main;
    assert!(runner.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand));

    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    assert_eq!(runner.memory(), 5);
}
