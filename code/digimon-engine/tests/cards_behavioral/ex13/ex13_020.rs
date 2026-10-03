//! EX13-020 Magnamon — Digimon, Lv.4, Blue/Yellow, DP 7000, Cost 7.
//! Traits: Holy Warrior / Royal Knight. Attribute: Free.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-020.md`)
//!
//! ＜Blocker＞ ＜Armor Purge＞
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] This Digimon
//! gets +1000 DP until your opponent's turn ends for each color in trashes.
//! Then, to 1 of your opponent's Digimon, give -4000 DP until their turn ends
//! for every 5000 DP this Digimon has.
//! [End of Your Turn] [Once Per Turn] 1 of your [Free] or [Royal Knight] trait
//! Digimon may unsuspend.
//! Inherited: [End of Your Turn] [Once Per Turn] 1 of your Digimon with the
//! [Free] or [Royal Knight] trait may unsuspend.
//! Assembly -2: [Veemon]
//! Digivolve: Blue Lv.3 / 4; Yellow Lv.3 / 4; [Veemon]: Cost 3.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Blue/EX13_020.cs
//!
//! # Patterns this test covers
//! - H5 Blocker + Armor Purge printed keywords.
//! - D1 formula DP buff (distinct colours across both trashes) followed by a
//!   formula debuff scaled off the carrier's post-buff DP (G-DSL-FORMULA-MULTIPLY).
//! - E3-adjacent: shared OPT across [On Play]/[When Digivolving]/[When Attacking].
//! - F7-adjacent: End of Your Turn optional unsuspend (face-up + inherited).
//! - C-group: Assembly alt-play with one named trash material.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming, CompiledZone,
};
use digimon_engine::action::space::{ATTACK_START, PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-020";

fn digimon(id: &str, name: &str, level: u8, dp: i32, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-020 YAML loads")
        .add_card(digimon("VEEMON", "Veemon", 3, 4000, CardColor::Blue, &["Mini Dragon"]))
        .add_card(digimon("RED-T", "Red Trash", 3, 3000, CardColor::Red, &[]))
        .add_card(digimon("BLUE-T", "Blue Trash", 3, 3000, CardColor::Blue, &[]))
        .add_card(digimon("GREEN-T", "Green Trash", 3, 3000, CardColor::Green, &[]))
        .add_card(digimon("OPP-A", "Opp A", 5, 12000, CardColor::Purple, &[]))
        .add_card(digimon("OPP-B", "Opp B", 5, 12000, CardColor::Purple, &[]))
        .add_card(digimon("RK", "Royal Ally", 6, 11000, CardColor::Yellow, &["Royal Knight"]))
        .add_card(digimon("FREE", "Free Ally", 5, 7000, CardColor::Yellow, &["Free"]))
        .add_card(digimon("PLAIN", "Plain Ally", 5, 7000, CardColor::Yellow, &["Beast"]))
        .add_card(digimon("FILL", "Fill", 3, 3000, CardColor::Black, &[]))
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn pick_perm(runner: &mut DebugRunner, target: PermanentHandle) {
    let view = runner.pending_selection_view().expect("permanent prompt");
    let id = ATTACK_START + target.index as u16;
    assert!(
        view.valid_action_ids.contains(&id),
        "{target:?} not selectable: {view:?}"
    );
    runner.execute_action(view.selecting_player, id).expect("pick");
}

fn stack_ids(runner: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

// ─── Section 1 — Structural ───────────────────────────────────────────────────

#[test]
fn ex13_020_metadata_and_alt_paths() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(7000));
    assert_eq!(card.cost, Some(7));
    for t in ["Holy Warrior", "Royal Knight"] {
        assert!(card.traits.iter().any(|x| x == t), "trait {t}");
    }
    let circles: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(circles.len(), 3, "Blue Lv.3/4, Yellow Lv.3/4, [Veemon]/3");
    assert!(circles
        .iter()
        .any(|c| c.cost == Some(CompiledCost::Literal(3))
            && c.from.as_ref().and_then(|f| f.name_is.as_deref()) == Some("Veemon")));
    let assembly = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Assembly)
        .expect("Assembly -2");
    assert_eq!(assembly.cost, Some(CompiledCost::Literal(2)));
    assert_eq!(assembly.materials.len(), 1);
    assert_eq!(assembly.materials[0].filter.name_is.as_deref(), Some("Veemon"));
    assert!(assembly.materials[0].stack_under);
    assert!(assembly.materials[0].zones.contains(&CompiledZone::Trash));
}

#[test]
fn ex13_020_clause_shapes() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 3);
    let shared = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("shared");
    assert_eq!(
        shared.when,
        vec![
            CompiledTiming::OnPlay,
            CompiledTiming::WhenDigivolving,
            CompiledTiming::WhenAttacking
        ]
    );
    assert!(shared.once_per_turn);
    assert!(!shared.optional, "the DP clause is mandatory");
    let eot: Vec<_> = triggered
        .iter()
        .filter(|t| !t.when.contains(&CompiledTiming::OnPlay))
        .collect();
    assert_eq!(eot.len(), 2);
    assert!(eot.iter().all(|t| t.once_per_turn));
    assert!(eot.iter().any(|t| t.scope == CompiledScope::FaceUp));
    assert!(eot.iter().any(|t| t.scope == CompiledScope::Inherited));
}

#[test]
fn ex13_020_carrier_has_blocker_and_armor_purge() {
    let mut runner = builder().start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(runner.game.has_keyword(perm, Keyword::Blocker));
    assert!(runner.game.has_keyword(perm, Keyword::ArmorPurge));
}

// ─── Section 3 — OP/WD/WA DP clause ──────────────────────────────────────────

#[test]
fn ex13_020_on_play_counts_distinct_colors_across_both_trashes_then_scales_debuff() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(10).start();
    runner.skip_mulligan();
    runner.inject_trash(0, "RED-T");
    runner.inject_trash(0, "BLUE-T");
    runner.inject_trash(1, "BLUE-T");
    runner.inject_trash(1, "GREEN-T");
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    let b = runner.place_on_field(1, "OPP-B", Some(0));

    let field = runner.play(0, 0).expect("Magnamon plays");
    let me = PermanentHandle { player: 0, index: field as u8 };
    // red/blue/green → +3000 → 10000 DP.
    assert_eq!(runner.effective_dp(me), Some(10000));
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert!(!runner.pending_is_optional(), "the debuff target is mandatory");
    pick_perm(&mut runner, b);
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(b), Some(12000 - 8000), "10000/5000 = 2 → -8000");
    assert_eq!(runner.effective_dp(a), Some(12000), "only 1 Digimon debuffed");
}

#[test]
fn ex13_020_empty_trashes_give_no_buff_and_a_single_4000_debuff() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(10).start();
    runner.skip_mulligan();
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    let field = runner.play(0, 0).expect("plays");
    let me = PermanentHandle { player: 0, index: field as u8 };
    assert_eq!(runner.effective_dp(me), Some(7000));
    pick_perm(&mut runner, a);
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(a), Some(8000), "7000/5000 = 1 → -4000");
}

#[test]
fn ex13_020_buff_lasts_through_opponents_turn_and_expires_after() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();
    runner.skip_mulligan();
    runner.inject_trash(0, "RED-T");
    let field = runner.play(0, 0).expect("plays");
    let me = PermanentHandle { player: 0, index: field as u8 };
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(me), Some(8000));
    end_my_turn(&mut runner);
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1);
    assert_eq!(runner.effective_dp(me), Some(8000), "still active on the opponent's turn");
    end_my_turn(&mut runner);
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 0);
    assert_eq!(runner.effective_dp(me), Some(7000), "expired after the opponent's turn");
}

#[test]
fn ex13_020_when_digivolving_and_when_attacking_share_the_once_per_turn() {
    let mut runner = builder()
        .security(1, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    runner.inject_trash(0, "RED-T");
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));

    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    pick_perm(&mut runner, a);
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(me), Some(8000));
    assert_eq!(runner.effective_dp(a), Some(8000));

    runner.attack_player(me, 1, false);
    assert_ne!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "[When Attacking] is locked out by the shared OPT"
    );
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(a), Some(8000), "no second debuff");
}

#[test]
fn ex13_020_when_attacking_fires_when_not_used_this_turn() {
    let mut runner = builder()
        .security(1, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    runner.attack_player(me, 1, false);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    pick_perm(&mut runner, a);
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(a), Some(8000));
}

#[test]
fn ex13_020_no_opponent_digimon_still_buffs() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(10).start();
    runner.skip_mulligan();
    runner.inject_trash(1, "GREEN-T");
    let field = runner.play(0, 0).expect("plays");
    let me = PermanentHandle { player: 0, index: field as u8 };
    let _ = runner.auto_resolve();
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.effective_dp(me), Some(8000));
}

// ─── Section 3 — End of Your Turn unsuspend ──────────────────────────────────

fn eot_setup(ally: &str, face_up: bool) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder()
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(3)
        .start();
    runner.skip_mulligan();
    if face_up {
        runner.place_on_field(0, CARD_ID, Some(0));
    } else {
        runner.place_stack(0, &[CARD_ID, "PLAIN"]);
    }
    let target = runner.place_on_field(0, ally, Some(0));
    runner.game.suspend(target);
    (runner, target)
}

/// End the active player's turn with memory on the opponent's side (-3), so
/// the next player starts with 3 memory and their turn does not auto-end.
fn end_my_turn(runner: &mut DebugRunner) {
    runner.game.set_memory(-3);
    runner.end_turn();
}

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

#[test]
fn ex13_020_end_of_turn_may_unsuspend_a_royal_knight() {
    let (mut runner, rk) = eot_setup("RK", true);
    end_my_turn(&mut runner);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    assert!(runner.pending_is_optional(), "'may unsuspend'");
    pick_perm(&mut runner, rk);
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1, "checked before player 0's own unsuspend phase");
    assert!(!is_suspended(&runner, rk));
}

#[test]
fn ex13_020_end_of_turn_accepts_a_free_attribute_digimon() {
    let (mut runner, free) = eot_setup("FREE", true);
    end_my_turn(&mut runner);
    pick_perm(&mut runner, free);
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1);
    assert!(!is_suspended(&runner, free));
}

#[test]
fn ex13_020_end_of_turn_ignores_non_free_non_royal_knight() {
    let (mut runner, plain) = eot_setup("PLAIN", true);
    end_my_turn(&mut runner);
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1);
    assert!(is_suspended(&runner, plain), "a [Beast] Digimon is not eligible");
}

#[test]
fn ex13_020_end_of_turn_decline_leaves_it_suspended() {
    let (mut runner, rk) = eot_setup("RK", true);
    end_my_turn(&mut runner);
    assert!(runner.pending_is_optional());
    let p = runner.pending_selection().expect("prompt").selecting_player;
    runner.execute_action(p, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1);
    assert!(is_suspended(&runner, rk));
}

#[test]
fn ex13_020_inherited_end_of_turn_unsuspend_from_the_stack() {
    let (mut runner, rk) = eot_setup("RK", false);
    end_my_turn(&mut runner);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    pick_perm(&mut runner, rk);
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1, "checked before player 0's own unsuspend phase");
    assert!(!is_suspended(&runner, rk));
}

// ─── Section 3 — Assembly ────────────────────────────────────────────────────

#[test]
fn ex13_020_assembly_plays_for_5_and_stacks_veemon_from_trash() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
    runner.skip_mulligan();
    runner.inject_trash(0, "VEEMON");
    let mem0 = runner.game.memory;
    runner.game.decode_action(PLAY_HAND_START, 0);
    assert!(runner.game.pending_selection.is_some(), "Assembly flow surfaces");
    runner.game.decode_action(TRASH_EFFECT_START, 0);
    let _ = runner.auto_resolve();
    let perm = PermanentHandle { player: 0, index: 0 };
    let stack = stack_ids(&runner, perm);
    assert_eq!(stack.last().map(String::as_str), Some(CARD_ID));
    assert!(stack.contains(&"VEEMON".to_string()));
    assert_eq!(mem0 - runner.game.memory, 5, "7 − 2 = 5");
}
