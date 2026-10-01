//! EX13-050 Bokomon — Digimon, Lv.3, Black, DP 1000, Play Cost 3.
//! Traits: Mutant. Form: Rookie. Attribute: Vaccine.
//! Digivolve: Black Lv.2 / cost 0.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-050.md`;
//! per-card JSON `code/digimon-engine/cards/ex13/EX13-050.json`)
//!
//! [All Turns] Players can't gain memory other than by Tamer effects.
//! Inherited: ＜Blocker＞ (This Digimon can block in the blocker timing.)
//!
//! Official Q&A: "each player's memory can no longer be increased by any
//! effects except Tamer effects. This effect affects both players."
//!
//! # DCGO C# reference
//! None at the submodule (no EX13_050.cs). Printed text + official Q&A govern.
//!
//! # Pattern rows
//! - F-floodgate (player-scoped static restriction, BOTH players — `target_player: any`)
//! - H5 inherited <Blocker>

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledPlayerRef, CompiledScope,
};
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "EX13-050";

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c
}

fn baby(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(2);
    c.dp = None;
    c.colors = vec![color];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-050 YAML loads from the embedded pack")
        .add_card(make_test_card("DIGI-SRC", "Digimon Source"))
        .add_card(tamer("TAMER-SRC"))
        .add_card(baby("BLACK-L2", CardColor::Black))
        .add_card(baby("RED-L2", CardColor::Red))
        .add_card({
            let mut c = make_test_card("TOP-L4", "Top Four");
            c.level = Some(4);
            c
        })
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
}

/// Run `gain_memory(amount)` as `player`'s effect sourced from the permanent `src`.
fn gain_from(runner: &mut DebugRunner, src: PermanentHandle, player: u8, amount: i16) {
    let card = runner.game.players[src.player as usize].battle_area[src.index as usize]
        .top_card()
        .handle();
    let mut ctx = EffectContext::new(&mut runner.game, card, Some(src), player);
    ctx.gain_memory(amount);
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_050_printed_metadata_and_digivolve_path() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(3), Some(1000), Some(3)));
    assert_eq!(c.color, vec![CompiledColor::Black]);
    assert_eq!(c.traits, vec!["Mutant".to_string()]);
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 1, "single Black Lv.2 circle");
    assert_eq!(digi[0].cost, Some(CompiledCost::Literal(0)));
}

#[test]
fn ex13_050_static_floodgate_targets_both_players() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    let gates: Vec<_> = c
        .effects
        .iter()
        .filter_map(|cl| match cl {
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
                modifier,
                target_player,
                ..
            }) => Some((modifier.clone(), *target_player)),
            _ => None,
        })
        .collect();
    assert_eq!(
        gates,
        vec![(
            "CannotGainMemoryExceptFromTamers".to_string(),
            Some(CompiledPlayerRef::Any)
        )],
        "\"Players can't gain memory\" — one floodgate over BOTH players"
    );
}

#[test]
fn ex13_050_inherited_blocker_clause() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    let inherited_blocker = c.effects.iter().any(|cl| {
        matches!(
            cl,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::Inherited,
                ..
            }) if keyword == "Blocker"
        )
    });
    assert!(inherited_blocker, "inherited <Blocker>");
}

// ─── Section 2 — Digivolve gating ────────────────────────────────────────────

#[test]
fn ex13_050_digivolves_from_black_lv2_for_0() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(3).start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "BLACK-L2", Some(0));
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    let top = runner.game.players[0].battle_area[base.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(top, CARD_ID);
    assert_eq!(runner.memory(), 3, "cost 0");
}

#[test]
fn ex13_050_cannot_digivolve_from_red_lv2() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(3).start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "RED-L2", Some(0));
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    let top = runner.game.players[0].battle_area[base.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(top, "RED-L2", "only the Black circle exists");
}

// ─── Section 3 — [All Turns] floodgate behaviour ─────────────────────────────

#[test]
fn ex13_050_played_bokomon_blocks_both_players_digimon_sourced_memory_gain() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
    runner.skip_mulligan();
    let own_src = runner.place_on_field(0, "DIGI-SRC", Some(0));
    let opp_src = runner.place_on_field(1, "DIGI-SRC", Some(0));
    runner.play(0, 0).expect("Bokomon plays");
    let _ = runner.auto_resolve();
    runner.game.tick_declarative_effects();

    assert!(runner
        .game
        .modifiers
        .player_has(0, ModifierType::CannotGainMemoryExceptFromTamers));
    assert!(runner
        .game
        .modifiers
        .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers));

    // Controller (player 0): Digimon-sourced gain is blocked.
    runner.game.set_memory(0);
    gain_from(&mut runner, own_src, 0, 2);
    assert_eq!(runner.game.memory, 0, "Bokomon's own controller can't gain by a Digimon effect");

    // Opponent (player 1): Digimon-sourced gain is blocked.
    gain_from(&mut runner, opp_src, 1, 2);
    assert_eq!(runner.game.memory, 0, "the opponent can't gain by a Digimon effect either");
}

#[test]
fn ex13_050_tamer_sourced_memory_gain_still_works_for_both_players() {
    let mut runner = builder().memory(0).start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    let own_tamer = runner.place_on_field(0, "TAMER-SRC", Some(0));
    let opp_tamer = runner.place_on_field(1, "TAMER-SRC", Some(0));
    runner.game.tick_declarative_effects();

    runner.game.set_memory(0);
    gain_from(&mut runner, own_tamer, 0, 2);
    assert_eq!(runner.game.memory, 2, "Tamer effects may still give the controller memory");

    runner.game.set_memory(0);
    gain_from(&mut runner, opp_tamer, 1, 2);
    assert_eq!(runner.game.memory, -2, "Tamer effects may still give the opponent memory");
}

#[test]
fn ex13_050_floodgate_lifts_when_bokomon_leaves() {
    let mut runner = builder().memory(0).start();
    runner.skip_mulligan();
    let boko = runner.place_on_field(0, CARD_ID, Some(0));
    let own_src = runner.place_on_field(0, "DIGI-SRC", Some(0));
    runner.game.tick_declarative_effects();
    assert!(runner
        .game
        .modifiers
        .player_has(0, ModifierType::CannotGainMemoryExceptFromTamers));

    runner.game.players[0].battle_area.remove(boko.index as usize);
    runner.game.tick_declarative_effects();
    assert!(!runner
        .game
        .modifiers
        .player_has(0, ModifierType::CannotGainMemoryExceptFromTamers));
    assert!(!runner
        .game
        .modifiers
        .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers));

    let own_src = PermanentHandle { player: 0, index: 0 };
    runner.game.set_memory(0);
    gain_from(&mut runner, own_src, 0, 2);
    assert_eq!(runner.game.memory, 2, "gains work again once Bokomon is gone");
}

#[test]
fn ex13_050_floodgate_is_not_inherited() {
    // As a digivolution source the [All Turns] effect does not apply.
    let mut runner = builder().memory(0).start();
    runner.skip_mulligan();
    runner.place_stack(0, &[CARD_ID, "TOP-L4"]);
    runner.game.tick_declarative_effects();
    assert!(!runner
        .game
        .modifiers
        .player_has(0, ModifierType::CannotGainMemoryExceptFromTamers));
    assert!(!runner
        .game
        .modifiers
        .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers));
}

// ─── Section 4 — Inherited <Blocker> ─────────────────────────────────────────

#[test]
fn ex13_050_inherited_blocker_on_digimon_above_it() {
    let mut runner = builder().start();
    runner.skip_mulligan();
    let stack = runner.place_stack(0, &[CARD_ID, "TOP-L4"]);
    runner.game.tick_declarative_effects();
    assert!(runner.game.has_keyword(stack, Keyword::Blocker));
}

#[test]
fn ex13_050_face_up_bokomon_has_no_blocker() {
    let mut runner = builder().start();
    runner.skip_mulligan();
    let boko = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();
    assert!(
        !runner.game.has_keyword(boko, Keyword::Blocker),
        "<Blocker> is inherited-only"
    );
}
