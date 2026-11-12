//! ST20-07 Tentomon - Digimon, Lv.3, Green, DP 3000, Cost 3.
//! Traits: Insectoid / ADVENTURE. Attribute: Vaccine.
//! Official Q&A: "This effect prevents digivolution costs from being reduced when
//! the costs are to be paid. This effect affects the opponent player."
//! DCGO: DCGO/Assets/Scripts/CardEffect/ST20/Green/ST20_07.cs
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

fn tentomon_runner() -> DebugRunner {
    let mut registry = digimon_engine::cards::build_registry();
    registry.insert("REDUCER", Arc::new(FieldCostReducer));

    DebugRunner::builder()
        .with_registry(registry)
        .dsl_card("ST20-07")
        .expect("ST20-07 found in embedded DSL pack")
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
fn st20_07_has_metadata_normal_evolution_and_floodgate() {
    let runner = DebugRunner::builder()
        .dsl_card("ST20-07")
        .expect("ST20-07 found in embedded DSL pack")
        .start();

    let compiled = runner
        .compiled_card("ST20-07")
        .expect("ST20-07 compiled card present");

    assert_eq!(compiled.name, "Tentomon");
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(3000));
    assert_eq!(compiled.color, vec![CompiledColor::Green]);
    assert_eq!(compiled.traits, vec!["Insectoid", "ADVENTURE"]);
    assert_eq!(compiled.attribute, ["Vaccine"]);

    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && matches!(path.cost, Some(CompiledCost::Literal(0)))
                && path.from.as_ref().is_some_and(|from| {
                    from.level_eq == Some(2) && from.trait_has.as_deref() == Some("ADVENTURE")
                })
        }),
        "ST20-07 must digivolve from a level 2 with the [ADVENTURE] trait for 0"
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
        "Tentomon must install an opponent-targeted CannotReduceDigivolveCost flood gate"
    );
}

#[test]
fn st20_07_blocks_opponents_digivolution_cost_reductions_but_digivolve_stays_legal() {
    let mut runner = tentomon_runner();

    runner.place_on_field(0, "ST20-07", Some(0));
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
        "Tentomon must not hide the opponent's ordinary digivolve action from the mask"
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
        "Tentomon must suppress the opponent's -3 digivolution cost reducer"
    );
}

#[test]
fn st20_07_baseline_without_tentomon_allows_digivolution_cost_reductions() {
    let mut runner = tentomon_runner();

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
        "ordinary digivolve should remain legal when no Tentomon flood gate is active"
    );
    assert_eq!(
        memory_before - runner.memory(),
        1,
        "digivolution cost reducer should apply normally without Tentomon"
    );
}

/// The gate is `[Opponent's Turn]` and targets the opponent only: on
/// Tentomon's owner's OWN turn, the owner's digivolution cost reductions
/// apply normally (EVO cost 4 - 3 = 1).
#[test]
fn st20_07_owner_own_turn_reductions_are_unaffected() {
    let mut registry = digimon_engine::cards::build_registry();
    registry.insert("REDUCER", Arc::new(FieldCostReducer));
    let mut runner = DebugRunner::builder()
        .with_registry(registry)
        .dsl_card("ST20-07")
        .expect("ST20-07 found in embedded DSL pack")
        .add_card(make_level_digimon("BASE", 3))
        .add_card(make_evo_digimon("EVO", 4, 3, 4))
        .add_card(make_level_digimon("REDUCER", 4))
        .add_card(make_test_card("FILLER", "FILLER"))
        .hand(0, &["EVO"])
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(10)
        .start();

    runner.place_on_field(0, "ST20-07", Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.place_on_field(0, "REDUCER", Some(0));
    runner.game.enter_main_phase();
    runner.game.set_memory(10);
    runner.game.tick_declarative_effects();
    assert_eq!(runner.turn_player(), 0, "it is Tentomon's owner's turn");

    let memory_before = runner.memory();
    let digivolved = runner
        .game
        .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByDigivolve);
    assert!(digivolved, "owner's digivolve is legal");
    assert_eq!(
        memory_before - runner.memory(),
        1,
        "on its owner's own turn Tentomon's gate is off: the -3 reducer applies"
    );
}
