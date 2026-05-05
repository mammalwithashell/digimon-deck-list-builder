//! BT5-033 Cutemon - Digimon, Lv.3, Yellow, DP 3000, Cost 3.
//! Traits: Fairy. Attribute: Vaccine.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [Opponent's Turn] Your opponent can't reduce digivolution costs.
//! ```
//!
//! # Patterns this test covers
//! - D6 flood gate / opponent restriction (opponent-turn, cannot-reduce-digivolve-cost)

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledPlayerRef,
};
use digimon_engine::action::{build_action_mask, encode_digivolve};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};

struct FieldCostReducer;

impl CardEffect for FieldCostReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card).cost_reduction(3).build()]
    }
}

fn make_level_digimon(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Yellow];
    card
}

fn make_evo_digimon(id: &str, level: u8, from_level: u8, cost: u16) -> CardData {
    let mut card = make_level_digimon(id, level);
    card.evo_costs = vec![EvoCost {
        level: from_level,
        memory_cost: cost,
        card_color: CardColor::Yellow as u8,
    }];
    card
}

fn cutemon_runner() -> DebugRunner {
    let mut registry = digimon_engine::cards::build_registry();
    registry.insert("REDUCER", Arc::new(FieldCostReducer));

    DebugRunner::builder()
        .with_registry(registry)
        .dsl_card("BT5-033")
        .expect("BT5-033 found in embedded DSL pack")
        .add_card(make_level_digimon("BASE", 3))
        .add_card(make_evo_digimon("EVO", 4, 3, 4))
        .add_card(make_level_digimon("REDUCER", 4))
        .add_card(make_test_card("FILLER", "FILLER"))
        .hand(1, &["EVO"])
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(10)
        .start()
}

#[test]
fn bt5_033_has_metadata_normal_evolution_and_floodgate() {
    let runner = DebugRunner::builder()
        .dsl_card("BT5-033")
        .expect("BT5-033 found in embedded DSL pack")
        .start();

    let compiled = runner
        .compiled_card("BT5-033")
        .expect("BT5-033 compiled card present");

    assert_eq!(compiled.name, "Cutemon");
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(3000));
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.traits, vec!["Fairy"]);
    assert_eq!(compiled.attribute.as_deref(), Some("Vaccine"));

    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && matches!(path.cost, Some(CompiledCost::Literal(0)))
                && path.from.as_ref().is_some_and(|from| {
                    from.level_eq == Some(2) && from.color_is == Some(CompiledColor::Yellow)
                })
        }),
        "BT5-033 must digivolve from yellow level 2 for 0"
    );

    let flood_gate = compiled.effects.iter().find_map(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
            modifier,
            target_player,
            ..
        }) => Some((modifier.as_str(), *target_player)),
        _ => None,
    });

    assert_eq!(
        flood_gate,
        Some((
            "CannotReduceDigivolveCost",
            Some(CompiledPlayerRef::Opponent),
        )),
        "Cutemon must install an opponent-targeted CannotReduceDigivolveCost flood gate"
    );
}

#[test]
fn bt5_033_blocks_opponents_digivolution_cost_reductions_but_digivolve_stays_legal() {
    let mut runner = cutemon_runner();

    runner.place_on_field(0, "BT5-033", Some(0));
    runner.place_on_field(1, "BASE", Some(0));
    runner.place_on_field(1, "REDUCER", Some(0));
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.game.set_memory(10);
    runner.game.tick_declarative_effects();

    let action = encode_digivolve(0, 0);
    let mask = build_action_mask(&runner.game, 1);
    assert_eq!(
        mask[action as usize], 1.0,
        "Cutemon must not hide the opponent's ordinary digivolve action from the mask"
    );

    let memory_before = runner.memory();
    let digivolved = runner
        .game
        .digivolve_from_hand(1, 0, 0, PlaySource::ByDigivolve);

    assert!(
        digivolved,
        "opponent's ordinary digivolve must remain legal"
    );
    assert_eq!(
        memory_before - runner.memory(),
        4,
        "Cutemon must suppress the opponent's -3 digivolution cost reducer"
    );
}

#[test]
fn bt5_033_baseline_without_cutemon_allows_digivolution_cost_reductions() {
    let mut runner = cutemon_runner();

    runner.place_on_field(1, "BASE", Some(0));
    runner.place_on_field(1, "REDUCER", Some(0));
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.game.set_memory(10);
    runner.game.tick_declarative_effects();

    let action = encode_digivolve(0, 0);
    let mask = build_action_mask(&runner.game, 1);
    assert_eq!(
        mask[action as usize], 1.0,
        "ordinary digivolve should be mask-legal when the reducer applies normally"
    );

    let memory_before = runner.memory();
    let digivolved = runner
        .game
        .digivolve_from_hand(1, 0, 0, PlaySource::ByDigivolve);

    assert!(
        digivolved,
        "ordinary digivolve should remain legal when no Cutemon flood gate is active"
    );
    assert_eq!(
        memory_before - runner.memory(),
        1,
        "digivolution cost reducer should apply normally without Cutemon"
    );
}
