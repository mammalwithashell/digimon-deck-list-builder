use digimon_dsl::common::PlayerRef;
use digimon_dsl::compiled::{CompiledAggregateSelector, CompiledFormula, CompiledPlayerRef};
use digimon_dsl::formula::{AggregateFormulaSpec, AggregateSelector, CompoundFormula, FormulaSpec};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::dsl_cards::formula_eval;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};

fn test_card(id: &str, level: u8, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(level),
        dp: Some(dp),
        play_cost: level as u16,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
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
fn lowest_dp_defaults_to_controller_scope_for_compatibility() {
    let mut runner = DebugRunner::builder()
        .add_card(test_card("YOU-LOW", 3, 3000))
        .add_card(test_card("YOU-HIGH", 5, 5000))
        .add_card(test_card("OPP-LOW", 4, 1000))
        .build();
    let target = runner.place_on_field(0, "YOU-LOW", None);
    runner.place_on_field(0, "YOU-HIGH", None);
    runner.place_on_field(1, "OPP-LOW", None);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);
    let formula = CompiledFormula::Aggregate(CompiledAggregateSelector::LowestDp);

    assert_eq!(formula_eval::evaluate(&formula, &ctx, target), 3000);
}

#[test]
fn lowest_dp_can_scan_opponent_scope() {
    let mut runner = DebugRunner::builder()
        .add_card(test_card("YOU-LOW", 3, 3000))
        .add_card(test_card("YOU-HIGH", 5, 5000))
        .add_card(test_card("OPP-LOW", 4, 1000))
        .add_card(test_card("OPP-HIGH", 6, 7000))
        .build();
    let target = runner.place_on_field(0, "YOU-LOW", None);
    runner.place_on_field(0, "YOU-HIGH", None);
    runner.place_on_field(1, "OPP-LOW", None);
    runner.place_on_field(1, "OPP-HIGH", None);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);
    let formula = CompiledFormula::AggregateScoped {
        selector: CompiledAggregateSelector::LowestDp,
        scope: CompiledPlayerRef::Opponent,
    };

    assert_eq!(formula_eval::evaluate(&formula, &ctx, target), 1000);
}

#[test]
fn highest_level_can_scan_any_scope() {
    let mut runner = DebugRunner::builder()
        .add_card(test_card("YOU-L3", 3, 3000))
        .add_card(test_card("YOU-L5", 5, 5000))
        .add_card(test_card("OPP-L7", 7, 7000))
        .build();
    let target = runner.place_on_field(0, "YOU-L3", None);
    runner.place_on_field(0, "YOU-L5", None);
    runner.place_on_field(1, "OPP-L7", None);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);
    let formula = CompiledFormula::AggregateScoped {
        selector: CompiledAggregateSelector::HighestLevel,
        scope: CompiledPlayerRef::Any,
    };

    assert_eq!(formula_eval::evaluate(&formula, &ctx, target), 7);
}

#[test]
fn aggregate_surface_parses_scoped_payload() {
    let formula: FormulaSpec = serde_yml::from_str(
        r#"
aggregate:
  selector: lowest_dp
  scope: opponent
"#,
    )
    .expect("formula parses");

    match formula {
        FormulaSpec::Compound(CompoundFormula::AggregateScoped(AggregateFormulaSpec {
            selector: AggregateSelector::LowestDp,
            scope: PlayerRef::Opponent,
        })) => {}
        other => panic!("expected scoped aggregate payload, got {other:?}"),
    }
}
