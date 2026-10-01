//! EX13-037 Dynasmon — Digimon, Lv.6, Yellow/Red, DP 12000, Cost 12.
//! Traits: Holy Warrior / Royal Knight. Attribute: Data.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-037.md`)
//!
//! ＜Raid＞ ＜Piercing＞ ＜Blocker＞
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] Trash your
//! top security card and this Digimon gets +10000 DP until your opponent's
//! turn ends. Then, if you have 3 or fewer security cards, trash their top
//! security card.
//! [All Turns] [Once Per Turn] When security stacks are removed from, 1 of
//! your opponent's Digimon gets -12000 DP until their turn ends. Then, if you
//! have 3 or fewer security cards, ＜Recovery +1＞.
//! Assembly -5: Lv.5 × Lv.4 × Lv.3, all w/[Witchelny] in text
//! Digivolve: Yellow Lv.5 / 4; Red Lv.5 / 4; Lv.5 w/[Witchelny] in text: 3.
//!
//! # DCGO C# reference
//! None at the submodule (no EX13_037.cs); printed text governs.
//!
//! # Patterns this test covers
//! - H9 Raid / H3 Piercing / H5 Blocker printed keywords.
//! - Security-trash cost-like step + self buff + security-count gate.
//! - Security-removed observer (either stack) with OPT, mandatory -DP and a
//!   gated Recovery.
//! - Assembly with "[X] in text" materials.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledCost, CompiledTiming};
use digimon_engine::action::space::{ATTACK_START, PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-037";

fn digimon(id: &str, name: &str, level: u8, dp: i32, text: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = level as u16;
    c.colors = vec![CardColor::Yellow];
    c.effect_text = text.to_string();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-037 YAML loads")
        .add_card(digimon("OPP", "Opp", 6, 15000, ""))
        .add_card(digimon("SEC", "Sec", 3, 3000, ""))
        .add_card(digimon("W5", "Witch Five", 5, 7000, "[On Play] Reveal ... [Witchelny] trait ..."))
        .add_card({
            let mut c = digimon("W4", "Witch Four", 4, 5000, "");
            c.traits = vec!["Witchelny".to_string()];
            c
        })
        .add_card(digimon("W3", "Witch Three", 3, 3000, "[When Digivolving] if you have a [Witchelny] Tamer ..."))
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn pick_field(runner: &mut DebugRunner, target: PermanentHandle) {
    let view = runner.pending_selection_view().expect("field prompt");
    let id = ATTACK_START + target.index as u16;
    assert!(view.valid_action_ids.contains(&id), "{target:?} not selectable: {view:?}");
    runner.execute_action(view.selecting_player, id).expect("pick");
}

/// Resolve a simultaneous-trigger ordering prompt, if any: both the own- and
/// opponent-security-removed legs of the [All Turns] observer can queue
/// together (its OPT then locks the second).
fn resolve_trigger_order(runner: &mut DebugRunner) {
    while runner.pending_kind() == Some(SelectionKind::TriggerOrder) {
        let v = runner.pending_selection_view().unwrap();
        runner.execute_action(v.selecting_player, v.valid_action_ids[0]).unwrap();
    }
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
fn ex13_037_metadata_and_alt_paths() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(6), Some(12000), Some(12)));
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 3);
    assert_eq!(
        digi.iter().filter(|p| p.cost == Some(CompiledCost::Literal(4))).count(),
        2
    );
    let a = c
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Assembly)
        .expect("Assembly");
    assert_eq!(a.cost, Some(CompiledCost::Literal(5)));
    assert_eq!(a.materials.len(), 3);
}

#[test]
fn ex13_037_clause_shapes() {
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
    assert_eq!(t.len(), 2);
    assert!(t.iter().all(|x| x.once_per_turn && !x.optional));
}

#[test]
fn ex13_037_carrier_has_raid_piercing_blocker() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    for k in [Keyword::Raid, Keyword::Piercing, Keyword::Blocker] {
        assert!(runner.game.has_keyword(me, k), "{k:?}");
    }
}

// ─── Section 2/3 — OP/WD/WA ──────────────────────────────────────────────────

#[test]
fn ex13_037_with_5_security_trashes_own_top_and_buffs_but_keeps_theirs() {
    let mut runner = builder()
        .security(0, &["SEC"; 5])
        .security(1, &["SEC"; 5])
        .start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    // The security removal triggers the [All Turns] clause (-12000 target).
    if runner.pending_selection().is_some() {
        pick_field(&mut runner, opp);
    }
    let _ = runner.auto_resolve();
    assert_eq!(runner.security_count(0), 4, "own top security trashed (4 > 3 → no recovery)");
    assert_eq!(runner.security_count(1), 5, "4 security left → theirs is kept");
    assert_eq!(runner.effective_dp(me), Some(22000));
    assert_eq!(runner.effective_dp(opp), Some(3000), "security removal → -12000");
}

#[test]
fn ex13_037_with_4_security_trashes_both_tops_then_recovers() {
    let mut runner = builder()
        .security(0, &["SEC"; 4])
        .security(1, &["SEC"; 5])
        .deck(0, &["SEC"; 5])
        .start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    resolve_trigger_order(&mut runner);
    if runner.pending_selection().is_some() {
        pick_field(&mut runner, opp);
    }
    let _ = runner.auto_resolve();
    assert_eq!(runner.security_count(1), 4, "3 left → trash their top security");
    assert_eq!(
        runner.security_count(0),
        4,
        "4 → 3 by the trash, then <Recovery +1> from the [All Turns] clause"
    );
    assert_eq!(runner.effective_dp(opp), Some(3000));
}

#[test]
fn ex13_037_empty_security_still_buffs() {
    let mut runner = builder().security(1, &["SEC"; 2]).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(me), Some(22000));
}

#[test]
fn ex13_037_op_wd_wa_share_the_once_per_turn() {
    let mut runner = builder()
        .security(0, &["SEC"; 5])
        .security(1, &["SEC"; 5])
        .deck(1, &["SEC"; 5])
        .memory(5)
        .start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    let _ = runner.auto_resolve();
    assert_eq!(runner.security_count(0), 4);
    runner.attack_player(me, 1, false);
    let _ = runner.auto_resolve();
    assert_eq!(runner.security_count(0), 4, "[When Attacking] is locked out");
}

// ─── Section 2/3/5 — security removed observer ───────────────────────────────

#[test]
fn ex13_037_opponent_security_check_triggers_minus_12000() {
    let mut runner = builder()
        .security(0, &["SEC"; 5])
        .security(1, &["SEC"; 5])
        .deck(1, &["SEC"; 5])
        .memory(5)
        .start();
    let attacker = runner.place_on_field(0, "OPP", Some(0));
    runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));
    runner.attack_player(attacker, 1, false);
    let mut picked = false;
    for _ in 0..6 {
        let Some(v) = runner.pending_selection_view() else { break };
        if v.kind == SelectionKind::OppField {
            pick_field(&mut runner, opp);
            picked = true;
            continue;
        }
        let a = v.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap_or(PASS);
        runner.execute_action(v.selecting_player, a).unwrap();
    }
    assert!(picked, "the opponent's security removal fires the observer");
    assert_eq!(runner.effective_dp(opp), Some(3000));
    assert_eq!(runner.security_count(0), 5, "5 security → no recovery");
}

#[test]
fn ex13_037_observer_with_no_opponent_digimon_still_recovers_at_3_or_fewer() {
    let mut runner = builder()
        .security(0, &["SEC"; 4])
        .deck(0, &["SEC"; 5])
        .security(1, &["SEC"; 5])
        .start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    let _ = runner.auto_resolve();
    assert_eq!(runner.security_count(1), 4);
    assert_eq!(
        runner.security_count(0),
        4,
        "no -DP target, but the Recovery still resolves (4 → 3 → 4)"
    );
}

#[test]
fn ex13_037_observer_is_once_per_turn() {
    let mut runner = builder()
        .security(0, &["SEC"; 5])
        .security(1, &["SEC"; 5])
        .deck(1, &["SEC"; 5])
        .memory(5)
        .start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(0, "OPP", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    resolve_trigger_order(&mut runner);
    pick_field(&mut runner, opp);
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(opp), Some(3000));
    runner.attack_player(attacker, 1, false);
    for _ in 0..6 {
        let Some(v) = runner.pending_selection_view() else { break };
        assert_ne!(v.kind, SelectionKind::OppField, "observer OPT already spent");
        let a = v.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap_or(PASS);
        runner.execute_action(v.selecting_player, a).unwrap();
    }
    assert_eq!(runner.security_count(1), 4, "the security check happened");
    assert_eq!(runner.effective_dp(opp), Some(3000));
}

// ─── Section 3 — Assembly ────────────────────────────────────────────────────

#[test]
fn ex13_037_assembly_witchelny_text_line_for_7() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .security(0, &["SEC"; 5])
        .security(1, &["SEC"; 5])
        .memory(7)
        .start();
    runner.skip_mulligan();
    for id in ["W5", "W4", "W3"] {
        runner.inject_trash(0, id);
    }
    let mem0 = runner.game.memory;
    runner.game.decode_action(PLAY_HAND_START, 0);
    for id in ["W5", "W4", "W3"] {
        assert!(runner.game.pending_selection.is_some(), "Assembly element {id}");
        let idx = trash_index(&runner, 0, id);
        runner.game.decode_action(TRASH_EFFECT_START + idx, 0);
    }
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(), 4);
    assert_eq!(mem0 - runner.game.memory, 7);
}
