//! EX13-015 Gallantmon — Digimon, Lv.6, Red, DP 12000, Cost 12.
//! Traits: Holy Warrior / Royal Knight. Attribute: Virus.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-015.md`)
//!
//! ＜Raid＞ ＜Progress＞ ＜Blocker＞
//! [On Play] [When Digivolving] [When Attacking] [Counter] [Once Per Turn]
//! Delete 1 of your opponent's 12000 DP or higher Digimon. If this effect
//! didn't delete, trash their top security card.
//! [All Turns] [Once Per Turn] When this Digimon would leave the battle area
//! other than by your effects, by deleting 1 of your opponent's 9000 DP or
//! lower Digimon, it doesn't leave.
//! Assembly -5: Lv.5 × Lv.4 × Lv.3, all w/[Guilmon]/[Growlmon] in name
//! Digivolve: Red Lv.5 / cost 3.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Red/EX13_015.cs (its delete filter is
//! `DP <= 12000`, the inverse of the printed text — printed text governs).
//!
//! # Patterns this test covers
//! - H9 Raid / Progress / H5 Blocker printed keywords.
//! - Delete-with-fallback via the effect result log (BT13-111 idiom),
//!   shared OPT across OP/WD/WA/Counter.
//! - F3 replacement: self leave-prevention paid by deleting an opponent
//!   Digimon, excluded for own-effect removal, OPT.
//! - Assembly with three per-level name-filtered materials.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming, CompiledZone,
};
use digimon_engine::action::space::{
    ATTACK_START, PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-015";

fn digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = level as u16;
    c.colors = vec![CardColor::Red];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-015 YAML loads")
        .add_card(digimon("BIG", "Big", 6, 13000))
        .add_card(digimon("EXACT", "Exact", 6, 12000))
        .add_card(digimon("MID", "Mid", 5, 11000))
        .add_card(digimon("SMALL", "Small", 4, 9000))
        .add_card(digimon("TINY", "Tiny", 3, 3000))
        .add_card(digimon("GROWL5", "WarGrowlmon", 5, 8000))
        .add_card(digimon("GROWL4", "Growlmon", 4, 6000))
        .add_card(digimon("GUIL3", "Guilmon", 3, 3000))
        .add_card(digimon("SEC", "Sec", 3, 3000))
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
fn ex13_015_metadata_and_alt_paths() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(6), Some(12000), Some(12)));
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 1);
    assert_eq!(digi[0].cost, Some(CompiledCost::Literal(3)));
    let a = c
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Assembly)
        .expect("Assembly");
    assert_eq!(a.cost, Some(CompiledCost::Literal(5)));
    assert_eq!(a.materials.len(), 3);
}

#[test]
fn ex13_015_clause_shapes() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    let shared = c
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("shared clause");
    assert_eq!(
        shared.when,
        vec![
            CompiledTiming::OnPlay,
            CompiledTiming::WhenDigivolving,
            CompiledTiming::WhenAttacking,
            CompiledTiming::Counter
        ]
    );
    assert!(shared.once_per_turn && !shared.optional);
    assert!(
        c.effects.iter().any(|c| matches!(c, CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. }))),
        "the leave-replacement clause"
    );
}

#[test]
fn ex13_015_carrier_has_raid_progress_blocker() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    for k in [Keyword::Raid, Keyword::Progress, Keyword::Blocker] {
        assert!(runner.game.has_keyword(me, k), "{k:?}");
    }
}

// ─── Section 2/3 — delete 12000+ or trash security ───────────────────────────

#[test]
fn ex13_015_on_play_deletes_a_12000_or_higher_digimon_and_keeps_security() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .security(1, &["SEC", "SEC"])
        .memory(12)
        .start();
    runner.skip_mulligan();
    let big = runner.place_on_field(1, "BIG", Some(0));
    let mid = runner.place_on_field(1, "MID", Some(0));
    runner.play(0, 0).expect("plays");
    let view = runner.pending_selection_view().expect("delete prompt");
    assert!(!view.is_optional, "mandatory delete");
    assert!(
        !view.valid_action_ids.contains(&(ATTACK_START + mid.index as u16)),
        "11000 DP is below the threshold"
    );
    pick_field(&mut runner, big);
    let _ = runner.auto_resolve();
    assert_eq!(field_ids(&runner, 1), vec!["MID".to_string()]);
    assert_eq!(runner.security_count(1), 2, "deleted → no security trash");
}

#[test]
fn ex13_015_exactly_12000_dp_is_eligible() {
    let mut runner = builder().security(1, &["SEC"]).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let exact = runner.place_on_field(1, "EXACT", Some(0));
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    pick_field(&mut runner, exact);
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(1), 0);
    assert_eq!(runner.security_count(1), 1);
}

#[test]
fn ex13_015_no_12000_target_trashes_their_top_security_card() {
    let mut runner = builder().security(1, &["SEC", "SEC"]).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "MID", Some(0));
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(1), 1, "11000 DP survives");
    assert_eq!(runner.security_count(1), 1, "didn't delete → trash top security");
    assert_eq!(runner.trash_size(1), 1);
}

#[test]
fn ex13_015_when_attacking_fires_and_shares_opt_with_when_digivolving() {
    let mut runner = builder()
        .security(1, &["SEC", "SEC", "SEC"])
        .deck(1, &["SEC"; 3])
        .memory(5)
        .start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    let _ = runner.auto_resolve();
    assert_eq!(runner.security_count(1), 2);
    runner.attack_player(me, 1, false);
    let _ = runner.auto_resolve();
    // Only the attack's own security check removes one more card; the
    // [When Attacking] trigger is locked out by the shared OPT.
    assert_eq!(runner.security_count(1), 1);
}

#[test]
fn ex13_015_when_attacking_fires_when_unused() {
    let mut runner = builder()
        .security(1, &["SEC", "SEC", "SEC"])
        .deck(1, &["SEC"; 3])
        .memory(5)
        .start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let big = runner.place_on_field(1, "BIG", Some(0));
    runner.attack_player(me, 1, false);
    // §16-23-2: the card's <Raid> is a trigger-type effect pending with its
    // [When Attacking] effect in one TriggerOrder; activate the
    // [When Attacking] first.
    let order = runner.pending_selection_view().expect("TriggerOrder");
    assert_eq!(order.kind, SelectionKind::TriggerOrder);
    let when_attacking = order
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .find(|c| c.keyword.is_none())
        .expect("the [When Attacking] branch")
        .action_id;
    runner.execute_action(0, when_attacking).expect("[When Attacking] first");
    pick_field(&mut runner, big);
    let _ = runner.auto_resolve();
    assert!(!field_ids(&runner, 1).contains(&"BIG".to_string()));
}

// ─── Section 2/3/5 — leave-replacement ───────────────────────────────────────

fn replacement_board() -> (DebugRunner, PermanentHandle) {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, me)
}

#[test]
fn ex13_015_opponent_effect_removal_prevented_by_deleting_a_9000_or_lower_digimon() {
    let (mut runner, me) = replacement_board();
    let small = runner.place_on_field(1, "SMALL", Some(0));
    let mid = runner.place_on_field(1, "MID", Some(0));
    runner
        .game
        .delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    let view = runner.pending_selection_view().expect("replacement prompt");
    assert!(view.is_optional, "the replacement may be declined");
    // Accept, then pick the 9000-DP target (11000 is not eligible).
    let mut picked = false;
    for _ in 0..4 {
        let Some(v) = runner.pending_selection_view() else { break };
        if v.valid_action_ids.contains(&(ATTACK_START + small.index as u16)) {
            assert!(
                !v.valid_action_ids.contains(&(ATTACK_START + mid.index as u16)),
                "11000 DP is above 9000"
            );
            pick_field(&mut runner, small);
            picked = true;
            continue;
        }
        let a = v.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        runner.execute_action(v.selecting_player, a).unwrap();
    }
    let _ = runner.auto_resolve();
    assert!(picked, "the 9000-DP cost target was offered");
    assert_eq!(field_ids(&runner, 0), vec![CARD_ID.to_string()], "it doesn't leave");
    assert_eq!(field_ids(&runner, 1), vec!["MID".to_string()]);
}

#[test]
fn ex13_015_declining_the_replacement_lets_it_leave() {
    let (mut runner, me) = replacement_board();
    runner.place_on_field(1, "SMALL", Some(0));
    runner
        .game
        .delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    let p = runner.pending_selection().expect("prompt").selecting_player;
    runner.execute_action(p, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(0), 0);
    assert_eq!(runner.battle_area_size(1), 1);
}

#[test]
fn ex13_015_no_9000_or_lower_target_means_it_leaves() {
    let (mut runner, me) = replacement_board();
    runner.place_on_field(1, "MID", Some(0));
    runner
        .game
        .delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(0), 0, "no payable cost → it leaves (Q&A)");
    assert_eq!(runner.battle_area_size(1), 1);
}

#[test]
fn ex13_015_own_effect_removal_is_not_replaced() {
    let (mut runner, me) = replacement_board();
    runner.place_on_field(1, "SMALL", Some(0));
    runner
        .game
        .delete_permanent_with_cause(me, ReplacementCause::OwnEffect);
    assert!(runner.pending_selection().is_none(), "own effects aren't covered");
    assert_eq!(runner.battle_area_size(0), 0);
}

#[test]
fn ex13_015_battle_removal_is_replaced() {
    let (mut runner, me) = replacement_board();
    let small = runner.place_on_field(1, "SMALL", Some(0));
    runner
        .game
        .delete_permanent_with_cause(me, ReplacementCause::Battle);
    assert!(runner.pending_selection().is_some(), "battle is 'other than by your effects'");
    accept_and_pay(&mut runner, small);
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(0), 1);
}

/// Accept Gallantmon's replacement and pay with `target`. The cost pick is
/// declinable (owner ruling 2026-11-23, DCGO parity), so the payment is
/// chosen explicitly rather than by `auto_resolve`.
fn accept_and_pay(runner: &mut DebugRunner, target: PermanentHandle) {
    pick_replacement(runner, false);
    runner.execute_action(0, REPLACEMENT_ACCEPT).expect("accept");
    pick_field(runner, target);
}

/// While <Evade> can also answer the deletion, the engine first asks which
/// immediate replacement effect applies next (CR 4.3 15-8-5); pick <Evade>
/// (`evade`) or Gallantmon's clause. With one candidate left, its own accept
/// prompt is already pending.
fn pick_replacement(runner: &mut DebugRunner, evade: bool) {
    let v = runner.pending_selection_view().expect("replacement prompt");
    if v.kind == SelectionKind::EffectChoice && v.prompt.contains("immediate replacement") {
        let row = v
            .effect_choices
            .as_ref()
            .and_then(|rows| rows.iter().find(|c| c.label.contains("Evade") == evade))
            .unwrap_or_else(|| panic!("replacement order rows: {:?}", v.effect_choices))
            .action_id;
        runner.execute_action(v.selecting_player, row).expect("order pick");
    }
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Replacement));
}

/// PASS is legal at the pending prompt (the action mask, not the raw ids).
fn pass_legal(runner: &DebugRunner) -> bool {
    let p = runner.pending_selection().expect("prompt").selecting_player;
    digimon_engine::action::build_action_mask(&runner.game, p)[PASS as usize] == 1.0
}

#[test]
fn ex13_015_replacement_is_once_per_turn() {
    let (mut runner, me) = replacement_board();
    let small = runner.place_on_field(1, "SMALL", Some(0));
    runner.place_on_field(1, "TINY", Some(0));
    runner
        .game
        .delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    accept_and_pay(&mut runner, small);
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(0), 1);
    assert_eq!(runner.battle_area_size(1), 1);
    let me = runner.perm_handle(0, 0);
    runner
        .game
        .delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    assert!(
        runner.pending_selection().is_none(),
        "spent: not offered for a second removal this turn though TINY could pay"
    );
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(0), 0, "second removal this turn is not replaced");
}

// ─── [All Turns][OPT] replacement: a declined cost keeps the use ────────────
//
// Owner ruling 2026-11-23 ("parity with DCGO"; qa/rules/mechanics-gaps-2026-10.md
// W14 row (f)). DCGO EX13_015.cs: offered only while an opponent 9000-DP-or-
// lower Digimon exists (`CanActivateCondition`); after the accept, a
// `SelectPermanentEffect` with canNoSelect true. No Digimon chosen ->
// `RemoveUse()`, Gallantmon leaves. A Digimon chosen -> `isUsed` (spent), and
// it doesn't leave only if that Digimon was deleted (Q7247).
//
// <Evade> on Gallantmon keeps it on the field after the would-leave window so
// the use can be observed on a later removal the same turn; it is suspended
// afterwards, so <Evade> is not offered again.

fn grant_evade(runner: &mut DebugRunner, h: PermanentHandle) {
    runner
        .game
        .modifiers
        .grant_keyword(h, Keyword::Evade, digimon_engine::enums::Expiry::Permanent, 0);
}

/// Accept <Evade> for the same deletion; nothing else is asked, so the
/// declined clause is not offered again for this event.
fn accept_evade(runner: &mut DebugRunner) {
    pick_replacement(runner, true);
    runner.execute_action(0, REPLACEMENT_ACCEPT).expect("accept <Evade>");
    assert!(
        runner.pending_selection().is_none(),
        "the same removal does not offer the declined clause again"
    );
}

#[test]
fn ex13_015_replacement_outer_decline_keeps_opt() {
    let (mut runner, me) = replacement_board();
    runner.place_on_field(1, "SMALL", Some(0));
    grant_evade(&mut runner, me);
    runner.game.delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    pick_replacement(&mut runner, false);
    runner.execute_action(0, PASS).expect("decline the replacement");
    accept_evade(&mut runner);
    let me = runner.perm_handle(0, 0);
    runner.game.delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "declined at the accept prompt: offered again this turn"
    );
}

#[test]
fn ex13_015_replacement_accept_then_decline_cost_keeps_opt() {
    let (mut runner, me) = replacement_board();
    let small = runner.place_on_field(1, "SMALL", Some(0));
    grant_evade(&mut runner, me);
    runner.game.delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    pick_replacement(&mut runner, false);
    runner.execute_action(0, REPLACEMENT_ACCEPT).expect("accept");
    let v = runner.pending_selection_view().expect("cost pick");
    assert!(v.valid_action_ids.contains(&(ATTACK_START + small.index as u16)));
    assert!(
        pass_legal(&runner),
        "DCGO EX13_015.cs canNoSelect: the deletion may be declined after the accept"
    );
    runner.execute_action(0, PASS).expect("decline the cost");
    assert_eq!(field_ids(&runner, 1), vec!["SMALL".to_string()], "nothing deleted");
    accept_evade(&mut runner);
    let me = runner.perm_handle(0, 0);
    runner.game.delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "DCGO RemoveUse: the declined cost keeps the [Once Per Turn]"
    );
    runner.execute_action(0, REPLACEMENT_ACCEPT).expect("accept again");
    let v = runner.pending_selection_view().expect("Gallantmon's cost pick again");
    assert!(v.valid_action_ids.contains(&(ATTACK_START + small.index as u16)));
}

#[test]
fn ex13_015_replacement_declined_cost_lets_it_leave() {
    let (mut runner, me) = replacement_board();
    runner.place_on_field(1, "SMALL", Some(0));
    runner.game.delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    runner.execute_action(0, REPLACEMENT_ACCEPT).expect("accept");
    assert!(pass_legal(&runner), "the deletion may be declined after the accept");
    runner.execute_action(0, PASS).expect("decline the cost");
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(0), 0, "not paid: Gallantmon leaves");
    assert_eq!(field_ids(&runner, 1), vec!["SMALL".to_string()]);
}

/// A chosen Digimon whose deletion is prevented spends the use (DCGO sets
/// `isUsed` on the pick) and Gallantmon leaves that removal (Q7247: the "by"
/// condition wasn't met; here <Evade> keeps it instead).
#[test]
fn ex13_015_replacement_prevented_payment_spends_opt() {
    let (mut runner, me) = replacement_board();
    let small = runner.place_on_field(1, "SMALL", Some(0));
    runner.place_on_field(1, "TINY", Some(0));
    runner.game.modifiers.add(
        small,
        digimon_engine::modifiers::ModifierEntry::simple(
            digimon_engine::enums::ModifierType::CannotBeDestroyedByEffect,
            0,
            digimon_engine::enums::Expiry::Permanent,
            1,
        ),
    );
    grant_evade(&mut runner, me);
    runner.game.delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    accept_and_pay(&mut runner, small);
    assert!(field_ids(&runner, 1).contains(&"SMALL".to_string()), "its deletion was prevented");
    accept_evade(&mut runner);
    let me = runner.perm_handle(0, 0);
    runner.game.delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    assert!(
        runner.pending_selection().is_none(),
        "a Digimon was chosen: the use is spent though TINY could pay"
    );
}

// ─── Section 3 — Assembly ────────────────────────────────────────────────────

#[test]
fn ex13_015_assembly_guilmon_growlmon_line_for_7() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(7).start();
    runner.skip_mulligan();
    for id in ["GROWL5", "GROWL4", "GUIL3"] {
        runner.inject_trash(0, id);
    }
    let mem0 = runner.game.memory;
    runner.game.decode_action(PLAY_HAND_START, 0);
    for id in ["GROWL5", "GROWL4", "GUIL3"] {
        assert!(runner.game.pending_selection.is_some(), "Assembly element {id}");
        let idx = trash_index(&runner, 0, id);
        runner.game.decode_action(TRASH_EFFECT_START + idx, 0);
    }
    // [On Play] fires afterwards (no 12000 target, no security) — drain it.
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(), 4);
    assert_eq!(mem0 - runner.game.memory, 7, "12 − 5 = 7");
}
