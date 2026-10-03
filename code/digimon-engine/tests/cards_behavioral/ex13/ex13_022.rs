//! EX13-022 AeroVeedramon — Digimon, Lv.5, Blue, DP 7000, Cost 7.
//! Traits: Holy Dragon, CS. Form: Ultimate. Attribute: Vaccine.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-022.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-022.md` agrees)
//!
//! ```text
//! Digivolve: Blue Lv.4 / cost 3.  [Digivolve] Lv.4 w/[CS] trait: Cost 3
//!
//! Effect:
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] You may play 1
//! Tamer card with [Veedramon] in its text from your hand without paying the cost.
//! [All Turns] [Once Per Turn] When any of your Tamers are played, 1 of your
//! opponent's Digimon or Tamers can't suspend until their turn ends.
//!
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When this Digimon with [Veedramon] in its name
//! suspends, it may unsuspend.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Blue/EX13_022.cs
//! - Alt digivolve `HasCSTraits`, level 4, cost 3.
//! - Shared [OP]/[WD]/[WA]: one hash, maxCountPerTurn 1, skippable, gated on an
//!   eligible hand Tamer; `SelectHandEffect(canNoSelect: true)`; nothing picked →
//!   `RemoveUse()`; `PlayPermanentCards(payCost: false)`.
//! - [All Turns] `OnEnterFieldAnyone` (OPT, mandatory): own Tamer played →
//!   `SelectPermanentEffect(opponent Digimon or Tamer, canNoSelect false)` →
//!   `GainCantSuspendUntilOpponentTurnEnd`.
//! - Inherited `OnTappedAnyone` (OPT, optional): carrier top card
//!   `ContainsCardName("Veedramon")` and THIS permanent suspended →
//!   `IUnsuspendPermanents`.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - E three-timing shared OPT (BT23-059 idiom) + optional hand pick → free play,
//!   `refund_opt` on decline (BT25-085 idiom).
//! - F own-Tamer-played observer → opponent permanent `CannotSuspend` lock
//!   (EX12-032 lock idiom).
//! - Inherited self-suspend → may unsuspend (BT24-030 idiom, name-gated).

#![allow(dead_code)]

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause};
use digimon_engine::action::space::{encode_digivolve, HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{PendingSelectionView, SelectionKind};

const CARD_ID: &str = "EX13-022";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn tamer(id: &str, text: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.colors = vec![CardColor::Blue];
    c.effect_text = text.to_string();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-022 YAML parses, compiles and is in the embedded pack")
        .add_card(tamer("VEE-TAMER", "Gain memory with [Veedramon].", 4))
        .add_card(tamer("PLAIN-TAMER", "", 2))
        .add_card({
            let mut c = make_test_card("CS-L4", "CS Champion");
            c.level = Some(4);
            c.colors = vec![CardColor::Yellow];
            c.traits = vec!["CS".to_string()];
            c
        })
        .add_card({
            let mut c = make_test_card("VEEDRAMON-X", "Veedramon X");
            c.level = Some(6);
            c
        })
        .add_card({
            let mut c = make_test_card("OTHER-L6", "Other Six");
            c.level = Some(6);
            c
        })
        .add_card(make_test_card("OPP-DIGI", "Opp Digimon"))
        .add_card(tamer("OPP-TAMER", "", 2))
        .add_card(make_test_card("FILL", "Fill"))
}

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_index(runner: &DebugRunner, id: &str) -> usize {
    runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in hand"))
}

fn find(runner: &DebugRunner, player: u8, id: &str) -> PermanentHandle {
    let index = runner.game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be on player {player}'s field"));
    PermanentHandle {
        player,
        index: index as u8,
    }
}

fn triggered(runner: &DebugRunner) -> Vec<CompiledTriggeredClause> {
    runner
        .compiled_card(CARD_ID)
        .expect("compiled")
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

fn hand_prompt(runner: &mut DebugRunner) -> Option<PendingSelectionView> {
    for _ in 0..4 {
        let view = runner.pending_selection_view()?;
        if view.kind == SelectionKind::Hand {
            return Some(view);
        }
        if view.kind != SelectionKind::EffectChoice && view.kind != SelectionKind::TriggerOrder {
            return None;
        }
        let accept = view.valid_action_ids.iter().copied().find(|&a| a != PASS)?;
        runner.execute_action(view.selecting_player, accept).ok()?;
    }
    None
}

fn offered(runner: &DebugRunner, view: &PendingSelectionView, id: &str) -> bool {
    let slot = hand_index(runner, id) as u16;
    view.valid_action_ids.contains(&(PLAY_HAND_START + slot))
        || view.valid_action_ids.contains(&(HAND_EFFECT_START + slot))
}

fn pick_hand(runner: &mut DebugRunner, id: &str) {
    let view = hand_prompt(runner).expect("hand prompt");
    let slot = hand_index(runner, id) as u16;
    let action = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} must be selectable: {view:?}"));
    runner.execute_action(view.selecting_player, action).expect("pick");
}

/// Pick the opponent permanent `id` at the lock prompt.
fn pick_opp(runner: &mut DebugRunner, id: &str) {
    let view = runner.pending_selection_view().expect("lock-target prompt");
    assert_eq!(view.kind, SelectionKind::OppField, "{view:?}");
    assert!(!view.is_optional, "the lock is mandatory");
    let h = find(runner, 1, id);
    let action = digimon_engine::action::space::ATTACK_START + h.index as u16;
    assert!(view.valid_action_ids.contains(&action), "{id} selectable: {view:?}");
    runner.execute_action(view.selecting_player, action).expect("pick opp");
}

fn locked(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.modifiers.has(h, ModifierType::CannotSuspend)
}

fn base(hand: &[&str], memory: i16) -> DebugRunner {
    let mut runner = builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(memory)
        .start();
    runner.skip_mulligan();
    runner.place_on_field(1, "OPP-DIGI", Some(0));
    runner
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_022_has_the_blue_circle_and_the_cs_trait_alt_digivolve() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(paths, 2, "Blue Lv.4/3 + Lv.4 w/[CS] trait/3");
}

#[test]
fn ex13_022_clause_shape_matches_printed_text() {
    let runner = builder().start();
    let t = triggered(&runner);
    assert_eq!(t.len(), 3);
    let shared = t
        .iter()
        .find(|c| c.when.contains(&CompiledTiming::OnPlay))
        .expect("shared clause");
    assert!(shared.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(shared.when.contains(&CompiledTiming::WhenAttacking));
    assert!(shared.once_per_turn, "one shared [Once Per Turn]");
    assert_eq!(shared.scope, CompiledScope::FaceUp);

    let observer = t
        .iter()
        .find(|c| c.when == vec![CompiledTiming::OnAllyPlayed])
        .expect("tamer-played observer");
    assert!(observer.once_per_turn);
    assert!(!observer.optional, "the lock is mandatory");
    assert_eq!(observer.scope, CompiledScope::FaceUp);

    let ess = t
        .iter()
        .find(|c| c.scope == CompiledScope::Inherited)
        .expect("inherited clause");
    assert_eq!(ess.when, vec![CompiledTiming::OnSuspend]);
    assert!(ess.once_per_turn);
    assert!(ess.optional, "it MAY unsuspend");
}

#[test]
fn ex13_022_digivolves_from_a_cs_trait_lv4_for_3() {
    let mut runner = base(&[CARD_ID], 5);
    let lv4 = runner.place_on_field(0, "CS-L4", Some(0));
    runner.game.decode_action(encode_digivolve(0, lv4.index as u16), 0);
    assert_eq!(field_ids(&runner, 0), vec![CARD_ID]);
    assert_eq!(runner.memory(), 2, "Lv.4 w/[CS] trait: Cost 3");
}

// ─── Section 2/3/5 — shared [OP]/[WD]/[WA] free Tamer play ───────────────────

#[test]
fn ex13_022_on_play_plays_a_veedramon_tamer_free_then_the_lock_fires() {
    let mut runner = base(&[CARD_ID, "VEE-TAMER"], 10);
    let opp = find(&runner, 1, "OPP-DIGI");
    runner.play(0, hand_index(&runner, CARD_ID)).expect("Aero plays");
    let after_play = runner.memory();
    pick_hand(&mut runner, "VEE-TAMER");
    // The Tamer entering play fires the [All Turns] lock.
    pick_opp(&mut runner, "OPP-DIGI");
    let _ = runner.auto_resolve();
    assert!(field_ids(&runner, 0).contains(&"VEE-TAMER".to_string()));
    assert_eq!(runner.memory(), after_play, "without paying the cost");
    assert!(locked(&runner, opp), "opponent Digimon can't suspend");
}

#[test]
fn ex13_022_tamer_without_veedramon_text_is_not_offered() {
    let mut runner = base(&[CARD_ID, "VEE-TAMER", "PLAIN-TAMER"], 10);
    runner.play(0, hand_index(&runner, CARD_ID)).expect("Aero plays");
    let view = hand_prompt(&mut runner).expect("hand prompt");
    assert!(offered(&runner, &view, "VEE-TAMER"));
    assert!(!offered(&runner, &view, "PLAIN-TAMER"));
}

#[test]
fn ex13_022_no_eligible_tamer_installs_no_prompt() {
    let mut runner = base(&[CARD_ID, "PLAIN-TAMER"], 10);
    runner.play(0, hand_index(&runner, CARD_ID)).expect("Aero plays");
    assert!(runner.pending_selection().is_none());
    assert_eq!(hand_ids(&runner, 0), vec!["PLAIN-TAMER"]);
}

#[test]
fn ex13_022_when_digivolving_fires_the_shared_clause() {
    let mut runner = base(&[CARD_ID, "VEE-TAMER"], 10);
    let lv4 = runner.place_on_field(0, "CS-L4", Some(0));
    let slot = hand_index(&runner, CARD_ID) as u16;
    runner.game.decode_action(encode_digivolve(slot, lv4.index as u16), 0);
    pick_hand(&mut runner, "VEE-TAMER");
    let _ = runner.auto_resolve();
    assert!(field_ids(&runner, 0).contains(&"VEE-TAMER".to_string()));
}

#[test]
fn ex13_022_shared_opt_locks_the_when_attacking_after_on_play() {
    let mut runner = base(&[CARD_ID, "VEE-TAMER", "VEE-TAMER"], 10);
    runner.play(0, hand_index(&runner, CARD_ID)).expect("Aero plays");
    pick_hand(&mut runner, "VEE-TAMER");
    let _ = runner.auto_resolve();
    let aero = find(&runner, 0, CARD_ID);
    runner.game.players[0].battle_area[aero.index as usize].turn_played = 0;
    runner.attack_player(aero, 1, false);
    assert!(hand_prompt(&mut runner).is_none(), "shared [Once Per Turn] already used");
    let _ = runner.auto_resolve();
    assert_eq!(hand_ids(&runner, 0), vec!["VEE-TAMER"]);
}

#[test]
fn ex13_022_declining_refunds_the_shared_opt() {
    let mut runner = base(&[CARD_ID, "VEE-TAMER"], 10);
    runner.play(0, hand_index(&runner, CARD_ID)).expect("Aero plays");
    let view = hand_prompt(&mut runner).expect("hand prompt");
    assert!(view.is_optional, "\"you may\"");
    runner.execute_action(view.selecting_player, PASS).expect("decline");
    let _ = runner.auto_resolve();
    let aero = find(&runner, 0, CARD_ID);
    runner.game.players[0].battle_area[aero.index as usize].turn_played = 0;
    runner.attack_player(aero, 1, false);
    assert!(hand_prompt(&mut runner).is_some(), "RemoveUse: OPT not consumed by a decline");
}

// ─── Section 2/3/5 — [All Turns][OPT] own Tamer played → lock ────────────────

fn observer_setup(hand: &[&str]) -> DebugRunner {
    let mut runner = base(hand, 10);
    runner.place_on_field(0, CARD_ID, Some(0));
    runner
}

#[test]
fn ex13_022_own_tamer_played_locks_an_opponent_tamer() {
    let mut runner = observer_setup(&["PLAIN-TAMER"]);
    runner.place_on_field(1, "OPP-TAMER", Some(0));
    runner.play(0, hand_index(&runner, "PLAIN-TAMER")).expect("tamer plays");
    pick_opp(&mut runner, "OPP-TAMER");
    let _ = runner.auto_resolve();
    assert!(locked(&runner, find(&runner, 1, "OPP-TAMER")));
    assert!(!locked(&runner, find(&runner, 1, "OPP-DIGI")));
}

#[test]
fn ex13_022_lock_lasts_until_the_end_of_the_opponents_turn() {
    let mut runner = observer_setup(&["PLAIN-TAMER"]);
    runner.play(0, hand_index(&runner, "PLAIN-TAMER")).expect("tamer plays");
    pick_opp(&mut runner, "OPP-DIGI");
    let _ = runner.auto_resolve();
    let opp = find(&runner, 1, "OPP-DIGI");
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert!(locked(&runner, opp), "still locked during the opponent's turn");
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert!(!locked(&runner, opp), "expired at the end of their turn");
}

#[test]
fn ex13_022_playing_a_digimon_does_not_trigger_the_lock() {
    let mut runner = observer_setup(&["FILL"]);
    runner.play(0, hand_index(&runner, "FILL")).expect("digimon plays");
    assert!(runner.pending_selection().is_none(), "only Tamers trigger it");
    assert!(!locked(&runner, find(&runner, 1, "OPP-DIGI")));
}

#[test]
fn ex13_022_opponent_tamer_played_does_not_trigger_the_lock() {
    let mut runner = observer_setup(&[]);
    let t = runner.place_on_field(1, "OPP-TAMER", Some(0));
    runner.fire_play_event_triggers(1, t.index as usize, false, false);
    assert!(runner.pending_selection().is_none(), "only YOUR Tamers");
}

#[test]
fn ex13_022_tamer_lock_is_once_per_turn() {
    let mut runner = observer_setup(&["PLAIN-TAMER", "PLAIN-TAMER"]);
    runner.play(0, hand_index(&runner, "PLAIN-TAMER")).expect("tamer 1");
    pick_opp(&mut runner, "OPP-DIGI");
    let _ = runner.auto_resolve();
    runner.play(0, hand_index(&runner, "PLAIN-TAMER")).expect("tamer 2");
    assert!(runner.pending_selection().is_none(), "[Once Per Turn] used");
}

// ─── Section 2/3/5 — inherited self-suspend → may unsuspend ──────────────────

fn carrier(top: &str) -> (DebugRunner, PermanentHandle) {
    let mut runner = base(&[], 3);
    let h = runner.place_stack(0, &[CARD_ID, top]);
    (runner, h)
}

fn suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

#[test]
fn ex13_022_veedramon_carrier_may_unsuspend_after_suspending() {
    let (mut runner, h) = carrier("VEEDRAMON-X");
    runner.game.suspend(h);
    let view = runner.pending_selection_view().expect("may-unsuspend prompt");
    assert!(view.is_optional);
    let accept = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
    runner.execute_action(view.selecting_player, accept).expect("accept");
    let _ = runner.auto_resolve();
    assert!(!suspended(&runner, h), "unsuspended");
}

#[test]
fn ex13_022_unsuspend_is_optional() {
    let (mut runner, h) = carrier("VEEDRAMON-X");
    runner.game.suspend(h);
    let view = runner.pending_selection_view().expect("prompt");
    runner.execute_action(view.selecting_player, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert!(suspended(&runner, h));
}

#[test]
fn ex13_022_non_veedramon_carrier_does_not_unsuspend() {
    let (mut runner, h) = carrier("OTHER-L6");
    runner.game.suspend(h);
    assert!(runner.pending_selection().is_none(), "no [Veedramon] in its name");
    assert!(suspended(&runner, h));
}

#[test]
fn ex13_022_another_permanent_suspending_does_not_fire_it() {
    let (mut runner, h) = carrier("VEEDRAMON-X");
    let other = runner.place_on_field(0, "OTHER-L6", Some(0));
    runner.game.suspend(other);
    assert!(runner.pending_selection().is_none(), "only THIS Digimon");
    assert!(!suspended(&runner, h));
}

#[test]
fn ex13_022_unsuspend_is_once_per_turn_and_resets_next_turn() {
    let (mut runner, h) = carrier("VEEDRAMON-X");
    runner.game.suspend(h);
    let view = runner.pending_selection_view().expect("prompt");
    let accept = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
    runner.execute_action(view.selecting_player, accept).expect("accept");
    let _ = runner.auto_resolve();
    runner.game.suspend(h);
    assert!(runner.pending_selection().is_none(), "[Once Per Turn] used");
    assert!(suspended(&runner, h));

    runner.end_turn();
    let _ = runner.auto_resolve();
    runner.game.players[0].battle_area[h.index as usize].is_suspended = false;
    runner.game.suspend(h);
    assert!(runner.pending_selection().is_some(), "fresh turn, fresh use");
}
