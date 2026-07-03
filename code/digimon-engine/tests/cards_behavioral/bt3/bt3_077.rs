//! BT3-077 Gazimon — Digimon, Purple, Lv.3, play cost 3, DP 2000.
//! Traits: Mammal. Form: Rookie. Attribute: Virus.
//!
//! Printed text (official Bandai DB / `data/card_bundles/BT3-077.md`):
//! "[All Turns] Your opponent can't gain memory other than by Tamer effects."
//! No inherited or security text.
//!
//! Digivolve requirements (official Bandai DB): Purple Lv.2 / cost 0.
//!
//! # DCGO C# reference
//! None — `find DCGO/Assets/Scripts/CardEffect -name BT3_077.cs` returns no
//! match. Per source-priority rule, the official bundle (printed text) is
//! authoritative here; the card is a static all-turns flood gate with no
//! optionality or selection surface, so no DCGO behavioral tie-break is
//! needed. Structurally and behaviorally this card is a near-exact twin of
//! EX8-030 Tapirmon (`code/digimon-engine/cards/ex8/EX8-030.yaml` /
//! `tests/cards_behavioral/ex8/ex8_030.rs`), differing only in color
//! (Purple vs Yellow), traits (Mammal vs Holy Beast/NSo), attribute (Virus
//! vs Vaccine), and digivolve paths (single Purple Lv.2 path vs a dual
//! yellow/NSo alt-path).
//!
//! # Patterns this test covers
//! - D6 Flood gate / opponent restriction (CannotGainMemoryExceptFromTamers)
//! - All-turns static effect (installs at start, no timing trigger, no OPT)

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

// --- Section 1: Structural assertions ---

#[test]
fn bt3_077_has_printed_stats_evo_path_and_static_memory_floodgate() {
    let runner = DebugRunner::builder()
        .dsl_card("BT3-077")
        .expect("BT3-077 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("BT3-077")
        .expect("BT3-077 compiled card exists");

    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(2000));
    assert_eq!(compiled.color, vec![CompiledColor::Purple]);
    assert_eq!(compiled.traits, vec!["Mammal".to_string()]);

    let digivolve_paths: Vec<_> = compiled
        .alt_paths
        .iter()
        .filter(|path| path.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert!(
        !digivolve_paths.is_empty(),
        "BT3-077 must have a normal purple Lv.2 digivolve path"
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
                    .any(|predicate| predicate.color_is == Some(CompiledColor::Purple))
        }),
        "BT3-077 must retain the normal purple Lv.2 cost-0 digivolve path"
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
        "BT3-077 must compile its all-turns static effect as an opponent player floodgate"
    );
}

#[test]
fn bt3_077_floodgate_is_the_only_declarative_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT3-077")
        .expect("BT3-077 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("BT3-077")
        .expect("BT3-077 compiled card exists");

    let declarative_count = compiled
        .effects
        .iter()
        .filter(|clause| matches!(clause, CompiledClause::Declarative(_)))
        .count();
    assert_eq!(
        declarative_count, 1,
        "BT3-077 prints exactly one static (all-turns) effect, no triggered clauses"
    );

    let triggered_count = compiled
        .effects
        .iter()
        .filter(|clause| matches!(clause, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered_count, 0,
        "BT3-077 has no [When Attacking]/[On Play]/etc. triggered clause — no OPT to test"
    );
}

// --- Section 2/3: Behavioral outcome — integrated via place/attack/end_turn ---

#[test]
fn bt3_077_on_field_blocks_opponent_non_tamer_memory_gain_but_allows_tamer_source_gain() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT3-077")
        .expect("BT3-077 YAML loads")
        .add_card(make_test_card(
            "OPP-DIGIMON-SOURCE",
            "Opponent Digimon Source",
        ))
        .add_card(tamer_card("OPP-TAMER-SOURCE"))
        .start();

    let gazimon = runner.place_on_field(0, "BT3-077", None);
    let opp_digimon = runner.place_on_field(1, "OPP-DIGIMON-SOURCE", None);
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER-SOURCE", None);
    runner.game.tick_declarative_effects();

    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "Gazimon must install CannotGainMemoryExceptFromTamers on its opponent"
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
        "opponent Digimon-sourced memory gain must be blocked by BT3-077"
    );

    runner.game.set_memory(0);
    {
        let mut ctx = EffectContext::new(&mut runner.game, tamer_source, Some(opp_tamer), 1);
        ctx.gain_memory(2);
    }
    assert_eq!(
        runner.game.memory, -2,
        "opponent Tamer-sourced memory gain must remain legal under BT3-077"
    );
}

#[test]
fn bt3_077_floodgate_is_scoped_to_opponent_and_does_not_block_controllers_own_memory_gain() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT3-077")
        .expect("BT3-077 YAML loads")
        .add_card(make_test_card("OWN-DIGIMON-SOURCE", "Own Digimon Source"))
        .start();

    let gazimon = runner.place_on_field(0, "BT3-077", None);
    let own_digimon = runner.place_on_field(0, "OWN-DIGIMON-SOURCE", None);
    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .game
            .modifiers
            .player_has(0, ModifierType::CannotGainMemoryExceptFromTamers),
        "BT3-077's floodgate must target the opponent, not its own controller"
    );

    let own_source = runner.game.player(0).battle_area[own_digimon.index as usize]
        .top_card()
        .handle();

    runner.game.set_memory(0);
    {
        let mut ctx = EffectContext::new(&mut runner.game, own_source, Some(own_digimon), 0);
        ctx.gain_memory(2);
    }
    assert_eq!(
        runner.game.memory, 2,
        "the controller's own non-Tamer memory gain must remain legal under BT3-077"
    );

    // Sanity: Gazimon itself is on the controller's field for the whole test.
    let _ = gazimon;
}

#[test]
fn bt3_077_floodgate_persists_across_turns_all_turns_scope() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT3-077")
        .expect("BT3-077 YAML loads")
        .add_card(make_test_card(
            "OPP-DIGIMON-SOURCE",
            "Opponent Digimon Source",
        ))
        .start();

    runner.place_on_field(0, "BT3-077", None);
    let opp_digimon = runner.place_on_field(1, "OPP-DIGIMON-SOURCE", None);
    runner.game.tick_declarative_effects();

    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "floodgate must be active on player 0's turn"
    );

    runner.end_turn();
    runner.game.tick_declarative_effects();
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "[All Turns] must keep the floodgate active through opponent's turn too"
    );

    let digimon_source = runner.game.player(1).battle_area[opp_digimon.index as usize]
        .top_card()
        .handle();
    runner.game.set_memory(0);
    {
        let mut ctx = EffectContext::new(&mut runner.game, digimon_source, Some(opp_digimon), 1);
        ctx.gain_memory(2);
    }
    assert_eq!(
        runner.game.memory, 0,
        "opponent Digimon-sourced memory gain must still be blocked during opponent's own turn"
    );

    runner.end_turn();
    runner.game.tick_declarative_effects();
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "floodgate must remain active back on player 0's turn (all-turns, not turn-scoped)"
    );
}

#[test]
fn bt3_077_floodgate_clears_when_gazimon_leaves_the_battle_area() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT3-077")
        .expect("BT3-077 YAML loads")
        .add_card(make_test_card(
            "OPP-DIGIMON-SOURCE",
            "Opponent Digimon Source",
        ))
        .start();

    let gazimon = runner.place_on_field(0, "BT3-077", None);
    let opp_digimon = runner.place_on_field(1, "OPP-DIGIMON-SOURCE", None);
    runner.game.tick_declarative_effects();

    assert!(runner
        .game
        .modifiers
        .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers));

    runner.game.players[0]
        .battle_area
        .remove(gazimon.index as usize);
    runner.game.tick_declarative_effects();
    assert!(
        !runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "the static floodgate must clear after Gazimon leaves the battle area"
    );

    let digimon_source = runner.game.player(1).battle_area[opp_digimon.index as usize]
        .top_card()
        .handle();
    runner.game.set_memory(0);
    {
        let mut ctx = EffectContext::new(&mut runner.game, digimon_source, Some(opp_digimon), 1);
        ctx.gain_memory(2);
    }
    assert_eq!(
        runner.game.memory, -2,
        "Digimon-sourced memory gain must work again once BT3-077 is gone"
    );
}
