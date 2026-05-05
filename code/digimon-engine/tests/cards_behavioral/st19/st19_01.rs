//! ST19-01 Kyaromon.
//! Printed text from data/cards.json:
//! Inherited Effect [When Attacking] [Once Per Turn] If you have another
//! Digimon, <Draw 1> (Draw 1 card from your deck.)

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledPlayerRef, CompiledScope, CompiledStep,
    CompiledTiming, CompiledZone,
};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::{EffectTiming, TriggerSource};

const KYAROMON_YAML: &str = include_str!("../../../cards/st19/ST19-01.yaml");

#[test]
fn st19_01_yaml_compiles_as_inherited_when_attacking_draw_one() {
    let spec: CardSpec = serde_yml::from_str(KYAROMON_YAML).expect("ST19-01 YAML parses");
    let compiled = compile(&spec).expect("ST19-01 YAML compiles");

    assert_eq!(compiled.card, "ST19-01");
    assert_eq!(compiled.name, "Kyaromon");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|effect| match effect {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect();

    assert_eq!(triggered.len(), 1, "ST19-01 has one inherited clause");
    let clause = triggered[0];
    assert_eq!(clause.scope, CompiledScope::Inherited);
    assert_eq!(clause.when, vec![CompiledTiming::WhenAttacking]);
    assert!(
        clause.once_per_turn,
        "printed [Once Per Turn] must be encoded"
    );
    assert!(
        !clause.optional,
        "Draw 1 is mandatory when the another-Digimon condition is met"
    );
    assert_eq!(
        clause.process,
        vec![CompiledStep::Draw {
            of: CompiledPlayerRef::You,
            count: 1,
        }]
    );
}

#[test]
fn st19_01_condition_requires_another_own_battle_area_digimon() {
    let spec: CardSpec = serde_yml::from_str(KYAROMON_YAML).expect("ST19-01 YAML parses");
    let compiled = compile(&spec).expect("ST19-01 YAML compiles");
    let clause = compiled
        .effects
        .iter()
        .find_map(|effect| match effect {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("inherited clause exists");

    let condition = clause.condition.as_ref().expect("another Digimon gate");
    let any = condition
        .any_permanent
        .as_ref()
        .expect("condition must use any_permanent");

    assert_eq!(any.of, CompiledPlayerRef::You);
    assert_eq!(any.predicate.kind, Some(CompiledCardKind::Digimon));
    assert_eq!(any.predicate.zone, vec![CompiledZone::BattleArea]);
    assert_eq!(
        any.predicate.other,
        Some(true),
        "another Digimon must exclude the attacking carrier"
    );
}

#[test]
fn st19_01_inherited_when_attacking_draws_one_with_another_digimon() {
    let mut runner = kyaromon_runner()
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("ALLY", "Ally"))
        .add_card(make_test_card("DRAW", "Draw"))
        .deck(0, &["DRAW"])
        .start();

    let carrier = runner.place_stack(0, &["ST19-01", "CARRIER"]);
    runner.place_on_field(0, "ALLY", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.hand_size(0), 1, "Kyaromon must draw exactly 1");
    assert_eq!(runner.deck_size(0), 0, "drawn card leaves deck");
    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert_eq!(hand_ids, vec!["DRAW".to_string()]);
}

#[test]
fn st19_01_inherited_when_attacking_does_not_draw_without_another_digimon() {
    let mut runner = kyaromon_runner()
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("DRAW", "Draw"))
        .deck(0, &["DRAW"])
        .start();

    let carrier = runner.place_stack(0, &["ST19-01", "CARRIER"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.hand_size(0), 0, "no other Digimon means no draw");
    assert_eq!(runner.deck_size(0), 1, "deck is unchanged");
}

#[test]
fn st19_01_inherited_opt_blocks_second_draw_same_turn() {
    let mut runner = kyaromon_runner()
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("ALLY", "Ally"))
        .add_card(make_test_card("DRAW-1", "Draw 1"))
        .add_card(make_test_card("DRAW-2", "Draw 2"))
        .deck(0, &["DRAW-2", "DRAW-1"])
        .start();

    let carrier = runner.place_stack(0, &["ST19-01", "CARRIER"]);
    runner.place_on_field(0, "ALLY", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.hand_size(0), 1, "OPT must block the second draw");
    assert_eq!(runner.deck_size(0), 1, "second card must remain in deck");
}

fn kyaromon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(KYAROMON_YAML)
        .expect("ST19-01 YAML loads")
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
