//! EX13-060 Alphamon — Digimon, Lv.6, Black/Yellow, DP 13000, Cost 13.
//! Traits: Holy Warrior / X Antibody / Royal Knight / Chronicle. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-060.md`)
//!
//! [When Digivolving] 1 of your opponent's Digimon gets -8000 DP until their
//! turn ends. Then, if they have 5 or more memory, gain 2 memory.
//! [Your Turn] [Once Per Turn] When any of your [Chronicle] trait Digimon or
//! Tamers are played, 1 of your Digimon may attack. Then, you may activate 1
//! of this Digimon's [When Digivolving] effects.
//! [End of Your Turn] [Once Per Turn] You may play 1 [Chronicle] trait card
//! without [Alphamon] in its name from your hand with the cost reduced by 6.
//! It gains ＜Rush＞ for the turn.
//! Assembly -5: Lv.5 × Lv.4 × Lv.3, all w/[Chronicle] trait
//! Digivolve: Black Lv.5 / 5; Yellow Lv.5 / 5; [Grademon]/Lv.5 w/[Chronicle]: 4.
//!
//! # DCGO C# reference
//! None at the submodule (no EX13_060.cs); printed text governs.
//!
//! # Patterns this test covers
//! - D1-adjacent: mandatory -DP then a memory-gated memory gain (opponent ≥5).
//! - B3: trigger on another permanent's play ([Chronicle] Digimon/Tamer).
//! - F1-adjacent: optional effect attack + refire of this Digimon's
//!   [When Digivolving] effect.
//! - End of Your Turn optional cost-reduced play from hand + Rush grant, OPT
//!   refunded on decline.
//! - Assembly with three per-level trait-filtered materials.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming, CompiledZone,
};
use digimon_engine::action::space::{
    ATTACK_START, HAND_EFFECT_START, PASS, PLAY_HAND_START, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-060";

fn card(id: &str, name: &str, kind: CardKind, level: Option<u8>, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = kind;
    c.level = level;
    c.dp = level.map(|l| 1000 * l as i32);
    c.play_cost = cost;
    c.colors = vec![CardColor::Black];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-060 YAML loads")
        .add_card(card("CHR-TAMER", "Chronicle Tamer", CardKind::Tamer, None, 3, &["Chronicle"]))
        .add_card(card("CHR-TAMER-2", "Chronicle Tamer 2", CardKind::Tamer, None, 3, &["Chronicle"]))
        .add_card(card("PLAIN-TAMER", "Plain Tamer", CardKind::Tamer, None, 3, &[]))
        .add_card(card("CHR-D7", "Chronicle Seven", CardKind::Digimon, Some(5), 8, &["Chronicle"]))
        .add_card(card("ALPHA-X", "Alphamon Ouryuken", CardKind::Digimon, Some(7), 8, &["Chronicle"]))
        .add_card(card("CHR-L5", "Chronicle Five", CardKind::Digimon, Some(5), 7, &["Chronicle"]))
        .add_card(card("CHR-L4", "Chronicle Four", CardKind::Digimon, Some(4), 5, &["Chronicle"]))
        .add_card(card("CHR-L3", "Chronicle Three", CardKind::Digimon, Some(3), 3, &["Chronicle"]))
        .add_card(card("ALLY", "Ally", CardKind::Digimon, Some(5), 5, &[]))
        .add_card({
            let mut c = card("OPP", "Opp", CardKind::Digimon, Some(6), 9, &[]);
            c.dp = Some(12000);
            c
        })
        .add_card(card("FILL", "Fill", CardKind::Digimon, Some(3), 3, &[]))
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} in hand"))
}

fn hand_action(runner: &DebugRunner, card_id: &str) -> Option<u16> {
    let view = runner.pending_selection_view()?;
    let slot = hand_index(runner, 0, card_id) as u16;
    [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
}

fn pick_field(runner: &mut DebugRunner, target: PermanentHandle) {
    let view = runner.pending_selection_view().expect("field prompt");
    let id = ATTACK_START + target.index as u16;
    assert!(view.valid_action_ids.contains(&id), "{target:?} not selectable: {view:?}");
    runner.execute_action(view.selecting_player, id).expect("pick");
}

/// Player 0's memory from their own perspective (the gauge flips sign when
/// the turn passes after an effect leaves memory on the opponent's side).
fn p0_memory(runner: &DebugRunner) -> i16 {
    if runner.turn_player() == 0 {
        runner.game.memory
    } else {
        -runner.game.memory
    }
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn trash_index(runner: &DebugRunner, player: u8, card_id: &str) -> u16 {
    runner.game.players[player as usize]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} in trash")) as u16
}

// ─── Section 1 — Structural ───────────────────────────────────────────────────

#[test]
fn ex13_060_metadata_and_alt_paths() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(6), Some(13000), Some(13)));
    for t in ["Holy Warrior", "X Antibody", "Royal Knight", "Chronicle"] {
        assert!(c.traits.iter().any(|x| x == t), "trait {t}");
    }
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 4, "2 circles + [Grademon] + Lv.5 [Chronicle]");
    assert_eq!(
        digi.iter().filter(|p| p.cost == Some(CompiledCost::Literal(5))).count(),
        2
    );
    assert_eq!(
        digi.iter().filter(|p| p.cost == Some(CompiledCost::Literal(4))).count(),
        2
    );
    let a = c
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Assembly)
        .expect("Assembly -5");
    assert_eq!(a.cost, Some(CompiledCost::Literal(5)));
    assert_eq!(a.materials.len(), 3);
}

#[test]
fn ex13_060_clause_shapes() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 3);
    let wd = t.iter().find(|x| x.when == vec![CompiledTiming::WhenDigivolving]).expect("WD");
    assert!(!wd.optional && !wd.once_per_turn);
    let played = t.iter().find(|x| x.when == vec![CompiledTiming::OnAllyPlayed]).expect("played");
    assert!(played.once_per_turn);
    let eot = t.iter().find(|x| x.when == vec![CompiledTiming::EndOfYourTurn]).expect("EOT");
    assert!(eot.once_per_turn);
}

// ─── Section 2/3 — [When Digivolving] ────────────────────────────────────────

#[test]
fn ex13_060_wd_debuffs_and_gains_2_memory_when_opponent_has_5_or_more() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));
    runner.game.set_memory(-5);
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    assert!(!runner.pending_is_optional(), "the -8000 target is mandatory");
    pick_field(&mut runner, opp);
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(opp), Some(12000 - 8000));
    assert_eq!(p0_memory(&runner), -3, "gain 2 memory");
}

#[test]
fn ex13_060_wd_no_memory_gain_when_opponent_has_4() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));
    runner.game.set_memory(-4);
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    pick_field(&mut runner, opp);
    let _ = runner.auto_resolve();
    assert_eq!(p0_memory(&runner), -4, "opponent has only 4 → no gain");
}

#[test]
fn ex13_060_wd_with_no_opponent_digimon_still_checks_memory() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.set_memory(-6);
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    let _ = runner.auto_resolve();
    assert_eq!(p0_memory(&runner), -4);
}

// ─── Section 2/3/5 — [Your Turn][OPT] Chronicle played ───────────────────────

#[test]
fn ex13_060_chronicle_tamer_play_lets_a_digimon_attack_then_refires_wd() {
    let mut runner = builder()
        .hand(0, &["CHR-TAMER"])
        .security(1, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"; 3])
        .memory(10)
        .start();
    runner.skip_mulligan();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));
    let slot = hand_index(&runner, 0, "CHR-TAMER");
    runner.play(0, slot).expect("tamer plays");

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField), "choose an attacker");
    assert!(runner.pending_is_optional(), "'may attack'");
    pick_field(&mut runner, ally);
    // The attack-target prompt: attack the opponent's security.
    let view = runner.pending_selection_view().expect("attack target prompt");
    let sec = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .max()
        .expect("a target");
    runner.execute_action(view.selecting_player, sec).expect("attack");
    // Resolve the attack; stop at the refire prompt (optional) for Alphamon.
    let mut saw_refire_target = false;
    for _ in 0..12 {
        let Some(v) = runner.pending_selection_view() else { break };
        if v.kind == SelectionKind::OppField && v.valid_action_ids.contains(&(ATTACK_START + opp.index as u16)) {
            saw_refire_target = true;
            runner.execute_action(v.selecting_player, ATTACK_START + opp.index as u16).unwrap();
            continue;
        }
        let a = v.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap_or(PASS);
        runner.execute_action(v.selecting_player, a).unwrap();
    }
    assert!(
        runner.game.players[0].battle_area[ally.index as usize].is_suspended,
        "the chosen Digimon attacked"
    );
    assert!(saw_refire_target, "Alphamon's [When Digivolving] was activated");
    assert_eq!(runner.effective_dp(opp), Some(12000 - 8000));
    let _ = me;
}

#[test]
fn ex13_060_declining_the_attack_still_offers_the_wd_refire() {
    let mut runner = builder().hand(0, &["CHR-TAMER"]).memory(10).start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));
    let slot = hand_index(&runner, 0, "CHR-TAMER");
    runner.play(0, slot).expect("tamer plays");
    runner.execute_action(0, PASS).expect("decline the attack");
    // Next: the optional refire.
    assert!(runner.pending_selection().is_some(), "refire offered");
    assert!(runner.pending_is_optional(), "'you may activate'");
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(opp), Some(12000 - 8000));
}

#[test]
fn ex13_060_declining_the_refire_leaves_the_opponent_untouched() {
    let mut runner = builder().hand(0, &["CHR-TAMER"]).memory(10).start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));
    let slot = hand_index(&runner, 0, "CHR-TAMER");
    runner.play(0, slot).expect("tamer plays");
    runner.execute_action(0, PASS).expect("decline the attack");
    runner.execute_action(0, PASS).expect("decline the refire");
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(opp), Some(12000));
}

#[test]
fn ex13_060_non_chronicle_play_does_not_trigger() {
    let mut runner = builder().hand(0, &["PLAIN-TAMER"]).memory(10).start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "ALLY", Some(0));
    let slot = hand_index(&runner, 0, "PLAIN-TAMER");
    runner.play(0, slot).expect("tamer plays");
    assert!(runner.pending_selection().is_none());
}

#[test]
fn ex13_060_chronicle_trigger_is_once_per_turn() {
    let mut runner = builder()
        .hand(0, &["CHR-TAMER", "CHR-TAMER-2"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    let slot = hand_index(&runner, 0, "CHR-TAMER");
    runner.play(0, slot).expect("tamer plays");
    let _ = runner.auto_resolve();
    let slot = hand_index(&runner, 0, "CHR-TAMER-2");
    runner.play(0, slot).expect("second tamer plays");
    assert!(runner.pending_selection().is_none(), "OPT spent");
}

#[test]
fn ex13_060_chronicle_trigger_does_not_fire_on_the_opponents_turn() {
    let mut runner = builder()
        .hand(1, &["CHR-TAMER"])
        .deck(1, &["FILL"; 4])
        .deck(0, &["FILL"; 4])
        .memory(10)
        .start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.set_memory(-5);
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1);
    let slot = hand_index(&runner, 1, "CHR-TAMER");
    runner.play(1, slot).expect("opp tamer");
    assert!(runner.pending_selection().is_none());
}

// ─── Section 2/3 — [End of Your Turn][OPT] play Chronicle −6 with Rush ───────

fn end_turn_with(hand: &[&str]) -> DebugRunner {
    let mut runner = builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(2)
        .start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.end_turn();
    runner
}

#[test]
fn ex13_060_end_of_turn_plays_a_chronicle_card_for_6_less_with_rush() {
    let mut runner = end_turn_with(&["CHR-D7"]);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    assert!(runner.pending_is_optional());
    let a = hand_action(&runner, "CHR-D7").expect("CHR-D7 selectable");
    let mem0 = runner.game.memory;
    runner.execute_action(0, a).expect("pick");
    assert!(field_ids(&runner, 0).contains(&"CHR-D7".to_string()));
    assert_eq!(mem0 - runner.game.memory, 2, "8 − 6 = 2");
    let idx = field_ids(&runner, 0).iter().position(|c| c == "CHR-D7").unwrap();
    let h = PermanentHandle { player: 0, index: idx as u8 };
    assert!(runner.game.has_keyword(h, Keyword::Rush), "gains <Rush> for the turn");
}

#[test]
fn ex13_060_end_of_turn_excludes_alphamon_named_and_non_chronicle_cards() {
    let runner = end_turn_with(&["ALPHA-X", "ALLY", "CHR-L3"]);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    assert!(hand_action(&runner, "ALPHA-X").is_none(), "[Alphamon] in its name is excluded");
    assert!(hand_action(&runner, "ALLY").is_none(), "non-[Chronicle] excluded");
    assert!(hand_action(&runner, "CHR-L3").is_some());
}

#[test]
fn ex13_060_end_of_turn_decline_plays_nothing() {
    let mut runner = end_turn_with(&["CHR-D7"]);
    runner.execute_action(0, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert!(!field_ids(&runner, 0).contains(&"CHR-D7".to_string()));
}

#[test]
fn ex13_060_end_of_turn_with_no_eligible_hand_card_prompts_nothing() {
    let runner = end_turn_with(&["ALLY"]);
    assert_ne!(runner.pending_kind(), Some(SelectionKind::Hand));
}

// ─── Section 3 — Assembly ────────────────────────────────────────────────────

#[test]
fn ex13_060_assembly_three_chronicle_levels_for_8() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(8).start();
    runner.skip_mulligan();
    for id in ["CHR-L5", "CHR-L4", "CHR-L3"] {
        runner.inject_trash(0, id);
    }
    let mem0 = runner.game.memory;
    runner.game.decode_action(PLAY_HAND_START, 0);
    for id in ["CHR-L5", "CHR-L4", "CHR-L3"] {
        assert!(runner.game.pending_selection.is_some(), "Assembly element {id}");
        let idx = trash_index(&runner, 0, id);
        runner.game.decode_action(TRASH_EFFECT_START + idx, 0);
    }
    let _ = runner.auto_resolve();
    let stack = &runner.game.players[0].battle_area[0].card_sources;
    assert_eq!(stack.len(), 4);
    assert_eq!(mem0 - runner.game.memory, 8, "13 − 5 = 8");
}
