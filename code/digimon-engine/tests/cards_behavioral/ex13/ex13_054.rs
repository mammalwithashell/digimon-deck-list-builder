//! EX13-054 Nanimon — Digimon, Lv.4, Black, DP 3000, Play Cost 3.
//! Traits: Invader (+ (Rule) Trait: Has [Mutant] Type). Form: Champion.
//! Attribute: Virus. Digivolve: Black Lv.3 / 2; Yellow Lv.3 / 2.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-054.md`;
//! per-card JSON `code/digimon-engine/cards/ex13/EX13-054.json`)
//!
//! [Security] At the end of the battle, play this card without paying the cost.
//! [On Play] [On Deletion] 1 of your opponent's Digimon can't attack players
//! until their turn ends. (Rule) Trait: Has [Mutant] Type.
//! Inherited: [All Turns] This Digimon gets +1000 DP.
//!
//! # DCGO C# reference
//! None at the submodule (no EX13_054.cs). Printed text governs; the security
//! clause follows the shipped BT21-043 / BT21-015 `play_from_security` idiom
//! (DCGO `PlaySelfDigimonAfterBattleSecurityEffect`).
//!
//! # Pattern rows
//! - G-security play-self-after-battle
//! - [On Play]/[On Deletion] shared body: mandatory opp pick + CannotAttackPlayer
//!   until the end of the opponent's turn
//! - inherited static self +DP

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{encode_digivolve, ATTACK_START, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-054";

fn digimon(id: &str, level: u8, dp: i32, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    c.dp = Some(dp);
    c.colors = vec![color];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-054 YAML loads from the embedded pack")
        .add_card(digimon("OPP-A", 4, 5000, CardColor::Red))
        .add_card(digimon("OPP-B", 4, 6000, CardColor::Red))
        .add_card(digimon("BLACK-L3", 3, 3000, CardColor::Black))
        .add_card(digimon("YELLOW-L3", 3, 3000, CardColor::Yellow))
        .add_card(digimon("RED-L3", 3, 3000, CardColor::Red))
        .add_card(digimon("TOP-L5", 5, 7000, CardColor::Black))
        .add_card(digimon("ATTACKER", 5, 9000, CardColor::Red))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
}

fn pick_opp(runner: &mut DebugRunner, target: PermanentHandle) {
    let view = runner.pending_selection_view().expect("opponent-field prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    let id = ATTACK_START + target.index as u16;
    assert!(view.valid_action_ids.contains(&id), "{target:?} not offered: {view:?}");
    runner.execute_action(view.selecting_player, id).expect("pick");
}

fn cannot_attack_player(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.modifiers.has(h, ModifierType::CannotAttackPlayer)
}

fn rotate_to(runner: &mut DebugRunner, target: u8) {
    for _ in 0..8 {
        if runner.turn_player() == target {
            return;
        }
        let _ = runner.auto_resolve();
        runner.pass_turn();
        let _ = runner.auto_resolve();
        if runner.game.current_phase == GamePhase::EndOfTurnAction {
            runner.game.pass_end_of_turn_action();
            let _ = runner.auto_resolve();
        }
    }
    assert_eq!(runner.turn_player(), target);
}

fn top_id(runner: &DebugRunner, h: PermanentHandle) -> String {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_054_printed_metadata_traits_and_paths() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(4), Some(3000), Some(3)));
    assert_eq!(c.color, vec![CompiledColor::Black]);
    assert!(c.traits.iter().any(|t| t == "Invader"));
    assert!(
        c.traits.iter().any(|t| t == "Mutant"),
        "(Rule) Trait: Has [Mutant] Type"
    );
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 2);
    assert!(digi.iter().all(|p| p.cost == Some(CompiledCost::Literal(2))));
}

#[test]
fn ex13_054_clause_shapes() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = c
        .effects
        .iter()
        .filter_map(|cl| match cl {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(triggered
        .iter()
        .any(|t| t.when.contains(&CompiledTiming::OnSecurity) && !t.optional));
    let shared = triggered
        .iter()
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay) && t.when.contains(&CompiledTiming::OnDeletion)
        })
        .expect("[On Play][On Deletion] shared clause");
    assert!(!shared.optional, "mandatory");
    assert!(!shared.once_per_turn);
    assert_eq!(shared.scope, CompiledScope::FaceUp);
    let inherited_dp = c.effects.iter().any(|cl| {
        matches!(
            cl,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Inherited,
                dp_modifier: Some(1000),
                ..
            })
        )
    });
    assert!(inherited_dp, "inherited +1000 DP aura");
}

// ─── Section 2 — Digivolve gating ────────────────────────────────────────────

#[test]
fn ex13_054_digivolves_from_black_or_yellow_lv3_for_2() {
    for base_id in ["BLACK-L3", "YELLOW-L3"] {
        let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
        runner.skip_mulligan();
        let base = runner.place_on_field(0, base_id, Some(0));
        runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
        let _ = runner.auto_resolve();
        assert_eq!(top_id(&runner, base), CARD_ID, "from {base_id}");
        assert_eq!(runner.memory(), 3, "cost 2 from {base_id}");
    }
}

#[test]
fn ex13_054_cannot_digivolve_from_red_lv3() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "RED-L3", Some(0));
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    assert_eq!(top_id(&runner, base), "RED-L3");
}

// ─── Section 3 — [On Play] ───────────────────────────────────────────────────

#[test]
fn ex13_054_on_play_target_cannot_attack_players() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
    runner.skip_mulligan();
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    let b = runner.place_on_field(1, "OPP-B", Some(0));
    runner.play(0, 0).expect("Nanimon plays");
    let view = runner.pending_selection_view().expect("pick prompt");
    assert!(!view.valid_action_ids.contains(&PASS), "mandatory pick");
    pick_opp(&mut runner, b);
    let _ = runner.auto_resolve();
    assert!(cannot_attack_player(&runner, b), "chosen Digimon can't attack players");
    assert!(!cannot_attack_player(&runner, a), "only 1 Digimon is affected");
    assert!(
        !runner.game.modifiers.has(b, ModifierType::CannotAttack),
        "it may still attack Digimon"
    );
}

#[test]
fn ex13_054_on_play_without_opponent_digimon_is_a_no_op() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
    runner.skip_mulligan();
    runner.play(0, 0).expect("Nanimon plays");
    assert!(runner.pending_selection().is_none());
}

#[test]
fn ex13_054_restriction_lasts_through_the_opponents_turn_then_clears() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
    runner.skip_mulligan();
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    runner.play(0, 0).expect("Nanimon plays");
    pick_opp(&mut runner, a);
    let _ = runner.auto_resolve();
    rotate_to(&mut runner, 1);
    assert!(cannot_attack_player(&runner, a), "still restricted on their turn");
    rotate_to(&mut runner, 0);
    assert!(!cannot_attack_player(&runner, a), "cleared after their turn ends");
}

// ─── Section 4 — [On Deletion] ───────────────────────────────────────────────

#[test]
fn ex13_054_on_deletion_target_cannot_attack_players() {
    let mut runner = builder().start();
    runner.skip_mulligan();
    let nani = runner.place_on_field(0, CARD_ID, Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    runner
        .game
        .delete_permanent_with_cause(nani, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();
    let a = PermanentHandle { player: 1, index: a.index };
    pick_opp(&mut runner, a);
    let _ = runner.auto_resolve();
    assert!(cannot_attack_player(&runner, a));
}

#[test]
fn ex13_054_on_deletion_during_opponents_turn_expires_at_that_turns_end() {
    let mut runner = builder().start();
    runner.skip_mulligan();
    rotate_to(&mut runner, 1);
    let nani = runner.place_on_field(0, CARD_ID, Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    runner
        .game
        .delete_permanent_with_cause(nani, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();
    pick_opp(&mut runner, a);
    let _ = runner.auto_resolve();
    assert!(cannot_attack_player(&runner, a));
    rotate_to(&mut runner, 0);
    assert!(
        !cannot_attack_player(&runner, a),
        "\"until their turn ends\" = the end of the turn in progress"
    );
}

// ─── Section 5 — [Security] ──────────────────────────────────────────────────

#[test]
fn ex13_054_security_plays_nanimon_after_the_battle_and_fires_on_play() {
    let mut runner = builder().memory(5).start();
    runner.skip_mulligan();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == CARD_ID)
            .expect("EX13-054 in card_data");
        let next = runner.game.next_card_index();
        let src = CardSource::new(idx, 1, next);
        runner.game.players[1].security.push(src);
    }
    let mem0 = runner.memory();
    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();
    assert_eq!(runner.security_count(1), 0);
    assert_eq!(runner.battle_area_size(1), 1, "Nanimon is played to its owner's field");
    let nani = PermanentHandle { player: 1, index: 0 };
    assert_eq!(top_id(&runner, nani), CARD_ID);
    assert_eq!(runner.memory(), mem0, "played without paying the cost");
    assert!(
        cannot_attack_player(&runner, attacker),
        "its [On Play] restricts 1 of the attacking player's Digimon"
    );
}

// ─── Section 6 — Inherited +1000 DP ──────────────────────────────────────────

#[test]
fn ex13_054_inherited_plus_1000_dp() {
    let mut runner = builder().start();
    runner.skip_mulligan();
    let stack = runner.place_stack(0, &[CARD_ID, "TOP-L5"]);
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(stack), Some(8000));
}

#[test]
fn ex13_054_face_up_nanimon_does_not_get_its_inherited_buff() {
    let mut runner = builder().start();
    runner.skip_mulligan();
    let nani = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(nani), Some(3000));
}
