//! EX8-030 Tapirmon — Digimon, Yellow, Lv.3, play cost 3, DP 2000.
//! Traits: Holy Beast / NSo. Attribute: Vaccine.
//!
//! Printed text from `data/cards.json`:
//! "[All Turns] Your opponent can't gain memory other than by Tamer effects."
//! No inherited or security text.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, ModifierType};

fn tamer_card(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}

#[test]
fn ex8_030_has_printed_stats_evo_paths_and_static_memory_floodgate() {
    let runner = DebugRunner::builder()
        .dsl_card("EX8-030")
        .expect("EX8-030 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("EX8-030")
        .expect("EX8-030 compiled card exists");

    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(2000));
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(
        compiled.traits,
        vec!["Holy Beast".to_string(), "NSo".to_string()]
    );

    let digivolve_paths: Vec<_> = compiled
        .alt_paths
        .iter()
        .filter(|path| path.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert!(
        digivolve_paths.len() >= 2,
        "EX8-030 must have normal yellow Lv.2 and alternate NSo Lv.2 digivolve paths"
    );
    assert!(digivolve_paths
        .iter()
        .all(|path| path.cost == Some(CompiledCost::Literal(0))));
    assert!(
        digivolve_paths.iter().any(|path| {
            let Some(from) = path.from.as_ref() else {
                return false;
            };
            from.all_of
                .iter()
                .any(|predicate| predicate.level_eq == Some(2))
                && from
                    .all_of
                    .iter()
                    .any(|predicate| predicate.color_is == Some(CompiledColor::Yellow))
        }),
        "EX8-030 must retain the normal yellow Lv.2 cost-0 digivolve path"
    );
    assert!(
        digivolve_paths.iter().any(|path| {
            let Some(from) = path.from.as_ref() else {
                return false;
            };
            from.all_of
                .iter()
                .any(|predicate| predicate.level_eq == Some(2))
                && from
                    .all_of
                    .iter()
                    .any(|predicate| predicate.trait_has.as_deref() == Some("NSo"))
        }),
        "EX8-030 must also digivolve from a Lv.2 with the NSo trait for cost 0"
    );

    let floodgate = compiled.effects.iter().find_map(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
            modifier,
            target_player,
            ..
        }) => Some((modifier.as_str(), *target_player)),
        _ => None,
    });
    assert_eq!(
        floodgate,
        Some((
            "CannotGainMemoryExceptFromTamers",
            Some(digimon_dsl::compiled::CompiledPlayerRef::Opponent),
        )),
        "EX8-030 must compile its all-turns static effect as an opponent player floodgate"
    );
}

#[test]
fn ex8_030_blocks_opponent_non_tamer_memory_gain_but_allows_tamer_source_gain() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-030")
        .expect("EX8-030 YAML loads")
        .add_card(make_test_card(
            "OPP-DIGIMON-SOURCE",
            "Opponent Digimon Source",
        ))
        .add_card(tamer_card("OPP-TAMER-SOURCE"))
        .start();

    let tapirmon = runner.place_on_field(0, "EX8-030", None);
    let opp_digimon = runner.place_on_field(1, "OPP-DIGIMON-SOURCE", None);
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER-SOURCE", None);
    runner.game.tick_declarative_effects();

    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "Tapirmon must install CannotGainMemoryExceptFromTamers on its opponent"
    );

    let digimon_source = runner.game.player(1).battle_area[opp_digimon.index as usize]
        .top_card()
        .handle();
    let tamer_source = runner.game.player(1).battle_area[opp_tamer.index as usize]
        .top_card()
        .handle();

    runner.game.set_memory(0);
    {
        let mut ctx = EffectContext::new(&mut runner.game, digimon_source, Some(opp_digimon), 1);
        ctx.gain_memory(2);
    }
    assert_eq!(
        runner.game.memory, 0,
        "opponent Digimon-sourced memory gain must be blocked by EX8-030"
    );

    runner.game.set_memory(0);
    {
        let mut ctx = EffectContext::new(&mut runner.game, tamer_source, Some(opp_tamer), 1);
        ctx.gain_memory(2);
    }
    assert_eq!(
        runner.game.memory, -2,
        "opponent Tamer-sourced memory gain must remain legal under EX8-030"
    );

    runner.game.players[0]
        .battle_area
        .remove(tapirmon.index as usize);
    runner.game.tick_declarative_effects();
    assert!(
        !runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "the static floodgate must clear after Tapirmon leaves the battle area"
    );

    runner.game.set_memory(0);
    {
        let mut ctx = EffectContext::new(&mut runner.game, digimon_source, Some(opp_digimon), 1);
        ctx.gain_memory(2);
    }
    assert_eq!(
        runner.game.memory, -2,
        "Digimon-sourced memory gain must work again once EX8-030 is gone"
    );
}
