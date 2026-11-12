//! ST21-02 Gomamon — Digimon, Lv.3, Blue, DP 2000, Cost 3.
//! Traits: Sea Beast / ADVENTURE. Attribute: Vaccine.
//!
//! Printed (official observation): [Digivolve] Lv.2 w/[ADVENTURE] trait: Cost 0.
//! [All Turns] Your opponent can't gain memory other than by Tamer effects.
//! Official Q&A: "your opponent's memory will no longer increase by any effects
//! except Tamer effects. This effect affects the opponent player."
//! DCGO: DCGO/Assets/Scripts/CardEffect/ST21/Blue/ST21_02.cs
//! Same flood gate as EX8-030 Tapirmon / BT3-077 Gazimon
//! (`CannotGainMemoryExceptFromTamers`, target_player: opponent).

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
fn st21_02_has_printed_stats_evo_paths_and_static_memory_floodgate() {
    let runner = DebugRunner::builder()
        .dsl_card("ST21-02")
        .expect("ST21-02 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("ST21-02")
        .expect("ST21-02 compiled card exists");

    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(2000));
    assert_eq!(compiled.color, vec![CompiledColor::Blue]);
    assert_eq!(
        compiled.traits,
        vec!["Sea Beast".to_string(), "ADVENTURE".to_string()]
    );

    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(0))
                && path.from.as_ref().is_some_and(|from| {
                    let preds: Vec<_> = std::iter::once(&**from).chain(from.all_of.iter()).collect();
                    preds.iter().any(|q| q.level_eq == Some(2))
                        && preds
                            .iter()
                            .any(|q| q.trait_has.as_deref() == Some("ADVENTURE"))
                })
        }),
        "ST21-02 must digivolve from a Lv.2 with the [ADVENTURE] trait for cost 0"
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
        "ST21-02 must compile its all-turns static effect as an opponent player floodgate"
    );
}

#[test]
fn st21_02_blocks_opponent_non_tamer_memory_gain_but_allows_tamer_source_gain() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST21-02")
        .expect("ST21-02 YAML loads")
        .add_card(make_test_card(
            "OPP-DIGIMON-SOURCE",
            "Opponent Digimon Source",
        ))
        .add_card(tamer_card("OPP-TAMER-SOURCE"))
        .start();

    let gomamon = runner.place_on_field(0, "ST21-02", None);
    let opp_digimon = runner.place_on_field(1, "OPP-DIGIMON-SOURCE", None);
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER-SOURCE", None);
    runner.game.tick_declarative_effects();

    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "Gomamon must install CannotGainMemoryExceptFromTamers on its opponent"
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
        "opponent Digimon-sourced memory gain must be blocked by ST21-02"
    );

    runner.game.set_memory(0);
    {
        let mut ctx = EffectContext::new(&mut runner.game, tamer_source, Some(opp_tamer), 1);
        ctx.gain_memory(2);
    }
    assert_eq!(
        runner.game.memory, -2,
        "opponent Tamer-sourced memory gain must remain legal under ST21-02"
    );

    runner.game.players[0]
        .battle_area
        .remove(gomamon.index as usize);
    runner.game.tick_declarative_effects();
    assert!(
        !runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "the static floodgate must clear after Gomamon leaves the battle area"
    );

    runner.game.set_memory(0);
    {
        let mut ctx = EffectContext::new(&mut runner.game, digimon_source, Some(opp_digimon), 1);
        ctx.gain_memory(2);
    }
    assert_eq!(
        runner.game.memory, -2,
        "Digimon-sourced memory gain must work again once ST21-02 is gone"
    );
}
