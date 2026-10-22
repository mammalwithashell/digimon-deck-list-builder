//! BT14-046 Togemon — Digimon, Lv.4, Green, DP 3000, Cost 4.
//! Traits: Vegetation. Form: Champion. Attribute: Data.
//! Digivolve: green Lv.3, cost 2.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT14-046.md; card image agrees)
//!
//! ```text
//! [Your Turn][Once Per Turn] When you would play a green Tamer card from your
//! hand, by suspending 1 of your green Digimon, reduce the play cost by 3.
//! Inherited: [Your Turn][Once Per Turn] When this Digimon would digivolve, if
//! you have a green Tamer, reduce the digivolution cost by 1.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT14/Green/BT14_046.cs
//!   - Clause 1: BeforePayCost ActivateClass (maxCountPerTurn 1, isOptional):
//!     on battle area, IsOwnerTurn, CanTriggerWhenPermanentWouldPlay(green
//!     Tamer in the owner's hand); CanActivate = an own green Digimon that can
//!     pay a suspend cost. SelectPermanent (canNoSelect) → SuspendPermanents by
//!     this effect → only if it really suspended: ChangeCost -3 on green Tamer
//!     plays from hand (Root.Hand).
//!   - Inherited: ChangeCostClass -1, maxCountPerTurn 1, IsOwnerTurn, own green
//!     Tamer on field, applies to a digivolution of THIS permanent.
//!
//! # Rules
//! - §15-7-1/-2/-4: "by suspending 1 of your green Digimon" is an optional
//!   processing condition; without it the reduction can't happen, and it can't
//!   be chosen when no green Digimon of yours can suspend.
//! - Inherited text is active only while Togemon is a digivolution card.
//!
//! # Patterns this test covers
//! - D-row field-hosted `cost_reduction` with `when_any_ally_played` + an
//!   interactive `pay_cost` (select own green Digimon → suspend), optional, OPT
//! - inherited `cost_reduction` on this permanent's digivolve
//!   (`source_is_cost_target_permanent`) gated on a green Tamer, OPT

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledClause, CompiledCost, CompiledDeclarativeClause};
use digimon_engine::action::space::{ATTACK_START, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT14-046";

fn digimon(id: &str, color: CardColor, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(4000);
    c.play_cost = 5;
    c
}

fn tamer(id: &str, color: CardColor, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![color];
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c
}

/// Green Lv.5 that digivolves from green Lv.4 for 3.
fn lv5_target(id: &str) -> CardData {
    let mut c = digimon(id, CardColor::Green, 5);
    c.evo_costs = vec![EvoCost {
        card_color: 3,
        level: 4,
        memory_cost: 3,
    }];
    c
}

/// Green Lv.6 that digivolves from green Lv.5 for 3.
fn lv6_target(id: &str) -> CardData {
    let mut c = digimon(id, CardColor::Green, 6);
    c.evo_costs = vec![EvoCost {
        card_color: 3,
        level: 5,
        memory_cost: 3,
    }];
    c
}

fn runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT14-046 in pack")
        .add_card(tamer("G-TAMER", CardColor::Green, 5))
        .add_card(tamer("G-TAMER-2", CardColor::Green, 5))
        .add_card(tamer("R-TAMER", CardColor::Red, 5))
        .add_card(digimon("G-DIGI", CardColor::Green, 4))
        .add_card(digimon("ALLY-G", CardColor::Green, 4))
        .add_card(digimon("ALLY-B", CardColor::Blue, 4))
        .add_card(digimon("TOP-G4", CardColor::Green, 4))
        .add_card(lv5_target("G-LV5"))
        .add_card(lv5_target("G-LV5-B"))
        .add_card(lv6_target("G-LV6"))
        .add_card(digimon("FILLER", CardColor::Blue, 3))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .security(0, &["FILLER"; 5])
        .security(1, &["FILLER"; 5])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.game.enter_main_phase();
    r.game.set_memory(10);
    r
}

fn hand_idx(r: &DebugRunner, player: u8, id: &str) -> usize {
    r.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not in P{player} hand"))
}

fn on_field(r: &DebugRunner, player: u8, id: &str) -> bool {
    r.game.players[player as usize]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == id)
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn own_field_pick(h: PermanentHandle) -> u16 {
    ATTACK_START + h.index as u16
}

/// Start the hand play of `id`; returns the memory before the play.
fn start_play(r: &mut DebugRunner, id: &str) -> i16 {
    r.add_to_hand(0, id);
    let before = r.memory();
    let idx = hand_idx(r, 0, id);
    let _ = r.play(0, idx);
    before
}

/// Accept Togemon's optional reducer and pick `target` as the suspend cost.
/// Accepts an outer accept/decline prompt first when one is installed.
fn accept_and_suspend(r: &mut DebugRunner, target: PermanentHandle) {
    let v = r.pending_selection_view().expect("Togemon's reducer prompt");
    assert!(v.is_optional, "'by suspending' is declinable");
    if !v.valid_action_ids.contains(&own_field_pick(target)) {
        let accept = *v
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .expect("an accept action");
        r.execute_action(0, accept).unwrap();
    }
    let v = r.pending_selection_view().expect("suspend-cost pick");
    assert!(
        v.valid_action_ids.contains(&own_field_pick(target)),
        "target offered as the suspend cost"
    );
    r.execute_action(0, own_field_pick(target)).unwrap();
    let _ = r.auto_resolve();
}

/// The suspend-cost candidates offered once the reducer is accepted.
fn suspend_candidates(r: &mut DebugRunner) -> Vec<u16> {
    let v = r.pending_selection_view().expect("Togemon's reducer prompt");
    let has_field_pick = v
        .valid_action_ids
        .iter()
        .any(|&a| (ATTACK_START..ATTACK_START + 14).contains(&a));
    if !has_field_pick {
        let accept = *v
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .expect("an accept action");
        r.execute_action(0, accept).unwrap();
    }
    r.pending_selection_view()
        .expect("suspend-cost pick")
        .valid_action_ids
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn bt14_046_metadata_and_green_lv3_cost2_circle() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, "Togemon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.dp, Some(3000));
    assert_eq!(card.traits, vec!["Vegetation".to_string()]);
    assert_eq!(card.attribute, ["Data"]);
    assert_eq!(card.alt_paths.len(), 1);
    assert_eq!(card.alt_paths[0].cost, Some(CompiledCost::Literal(2)));
}

#[test]
fn bt14_046_has_two_opt_cost_reductions_one_optional_one_inherited() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.effects.len(), 2);
    let reducers: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(d @ CompiledDeclarativeClause::CostReduction { .. }) => {
                Some(d)
            }
            _ => None,
        })
        .collect();
    assert_eq!(reducers.len(), 2, "two cost_reduction clauses");
    match reducers[0] {
        CompiledDeclarativeClause::CostReduction {
            amount,
            optional,
            once_per_turn,
            ..
        } => {
            assert_eq!(*amount, Some(3));
            assert!(*optional, "'by suspending' reducer is optional");
            assert!(*once_per_turn);
        }
        _ => unreachable!(),
    }
    match reducers[1] {
        CompiledDeclarativeClause::CostReduction {
            amount,
            optional,
            once_per_turn,
            ..
        } => {
            assert_eq!(*amount, Some(1));
            assert!(!*optional, "the digivolve reduction is mandatory");
            assert!(*once_per_turn);
        }
        _ => unreachable!(),
    }
}

// ─── SECTION 2 — Clause 1: green Tamer play -3 by suspending a green Digimon

#[test]
fn bt14_046_green_tamer_play_suspend_togemon_reduces_cost_by_3() {
    let mut r = runner();
    let toge = r.place_on_field(0, CARD_ID, Some(0));
    let before = start_play(&mut r, "G-TAMER");
    accept_and_suspend(&mut r, toge);
    assert!(suspended(&r, toge), "Togemon paid the suspend cost");
    assert!(on_field(&r, 0, "G-TAMER"), "the Tamer is played");
    assert_eq!(before - r.memory(), 2, "cost 5 - 3 = 2");
}

#[test]
fn bt14_046_can_suspend_another_green_digimon_not_a_blue_one() {
    let mut r = runner();
    let toge = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "ALLY-G", Some(0));
    let blue = r.place_on_field(0, "ALLY-B", Some(0));
    let before = start_play(&mut r, "G-TAMER");
    let cands = suspend_candidates(&mut r);
    assert!(cands.contains(&own_field_pick(toge)));
    assert!(cands.contains(&own_field_pick(ally)));
    assert!(!cands.contains(&own_field_pick(blue)), "blue can't pay");
    r.execute_action(0, own_field_pick(ally)).unwrap();
    let _ = r.auto_resolve();
    assert!(suspended(&r, ally));
    assert!(!suspended(&r, toge));
    assert_eq!(before - r.memory(), 2);
}

#[test]
fn bt14_046_declining_pays_the_full_cost_and_suspends_nothing() {
    let mut r = runner();
    let toge = r.place_on_field(0, CARD_ID, Some(0));
    let before = start_play(&mut r, "G-TAMER");
    assert!(r.pending_is_optional());
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    assert!(!suspended(&r, toge));
    assert!(on_field(&r, 0, "G-TAMER"));
    assert_eq!(before - r.memory(), 5, "full cost");
}

#[test]
fn bt14_046_non_green_tamer_play_is_not_reduced() {
    let mut r = runner();
    let toge = r.place_on_field(0, CARD_ID, Some(0));
    let before = start_play(&mut r, "R-TAMER");
    assert!(r.pending_selection().is_none(), "red Tamer: no prompt");
    let _ = r.auto_resolve();
    assert!(!suspended(&r, toge));
    assert_eq!(before - r.memory(), 5);
}

#[test]
fn bt14_046_green_digimon_play_is_not_reduced() {
    let mut r = runner();
    let toge = r.place_on_field(0, CARD_ID, Some(0));
    let before = start_play(&mut r, "G-DIGI");
    assert!(r.pending_selection().is_none(), "Tamer cards only");
    let _ = r.auto_resolve();
    assert!(!suspended(&r, toge));
    assert_eq!(before - r.memory(), 5);
}

/// §15-7-4: no unsuspended green Digimon → the reduction can't be chosen.
#[test]
fn bt14_046_no_unsuspended_green_digimon_no_prompt() {
    let mut r = runner();
    let toge = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "ALLY-B", Some(0));
    r.game.players[0].battle_area[toge.index as usize].is_suspended = true;
    let before = start_play(&mut r, "G-TAMER");
    assert!(r.pending_selection().is_none());
    let _ = r.auto_resolve();
    assert_eq!(before - r.memory(), 5);
}

/// [Your Turn]: a green Tamer played from P0's hand during P1's turn (as by
/// an effect that pays the cost) is not offered the reduction.
#[test]
fn bt14_046_not_on_opponents_turn() {
    let mut r = runner();
    let toge = r.place_on_field(0, CARD_ID, Some(0));
    r.game.turn_player_idx = 1;
    let _ = start_play(&mut r, "G-TAMER");
    assert!(r.pending_selection().is_none(), "[Your Turn]");
    let _ = r.auto_resolve();
    assert!(!suspended(&r, toge));
}

#[test]
fn bt14_046_opponents_green_tamer_play_is_not_reduced() {
    let mut r = runner();
    let toge = r.place_on_field(0, CARD_ID, Some(0));
    r.add_to_hand(1, "G-TAMER");
    let idx = hand_idx(&r, 1, "G-TAMER");
    let _ = r.play(1, idx);
    assert!(
        r.pending_selection().is_none(),
        "'you would play' = Togemon's owner; pending: {:?}",
        r.pending_selection_view()
    );
    let _ = r.auto_resolve();
    assert!(!suspended(&r, toge));
}

#[test]
fn bt14_046_once_per_turn_then_clears_next_own_turn() {
    let mut r = runner();
    let toge = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "ALLY-G", Some(0));
    let _ = start_play(&mut r, "G-TAMER");
    accept_and_suspend(&mut r, toge);

    let before = start_play(&mut r, "G-TAMER-2");
    assert!(r.pending_selection().is_none(), "[Once Per Turn]");
    let _ = r.auto_resolve();
    assert!(!suspended(&r, ally));
    assert_eq!(before - r.memory(), 5, "second green Tamer pays full");

    r.end_turn();
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 0);
    r.game.enter_main_phase();
    r.game.set_memory(10);
    let before = start_play(&mut r, "G-TAMER");
    let v = r.pending_selection_view().expect("offered again next turn");
    assert!(v.is_optional);
    let _ = before;
}

// ─── SECTION 3 — Inherited digivolve -1 with a green Tamer ─────────────────

fn digivolve(r: &mut DebugRunner, carrier: PermanentHandle, id: &str) -> i16 {
    r.add_to_hand(0, id);
    let idx = hand_idx(r, 0, id);
    let before = r.memory();
    let ok = r
        .game
        .digivolve_from_hand(0, idx, carrier.index as usize, PlaySource::ByHand);
    assert!(ok, "digivolve resolves");
    let _ = r.auto_resolve();
    before - r.memory()
}

#[test]
fn bt14_046_inherited_with_green_tamer_reduces_digivolve_cost_by_1() {
    let mut r = runner();
    r.place_on_field(0, "G-TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "TOP-G4"]);
    assert_eq!(digivolve(&mut r, carrier, "G-LV5"), 2, "3 - 1");
}

#[test]
fn bt14_046_inherited_without_a_tamer_pays_full() {
    let mut r = runner();
    let carrier = r.place_stack(0, &[CARD_ID, "TOP-G4"]);
    assert_eq!(digivolve(&mut r, carrier, "G-LV5"), 3);
}

#[test]
fn bt14_046_inherited_with_only_a_red_tamer_pays_full() {
    let mut r = runner();
    r.place_on_field(0, "R-TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "TOP-G4"]);
    assert_eq!(digivolve(&mut r, carrier, "G-LV5"), 3, "needs a GREEN Tamer");
}

#[test]
fn bt14_046_inherited_opponents_green_tamer_does_not_count() {
    let mut r = runner();
    r.place_on_field(1, "G-TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "TOP-G4"]);
    assert_eq!(digivolve(&mut r, carrier, "G-LV5"), 3, "'you have' a green Tamer");
}

#[test]
fn bt14_046_inherited_inactive_while_togemon_is_the_top_card() {
    let mut r = runner();
    r.place_on_field(0, "G-TAMER", Some(0));
    let toge = r.place_on_field(0, CARD_ID, Some(0));
    assert_eq!(digivolve(&mut r, toge, "G-LV5"), 3, "top-card inherited text is inactive");
}

#[test]
fn bt14_046_inherited_does_not_reduce_another_permanents_digivolve() {
    let mut r = runner();
    r.place_on_field(0, "G-TAMER", Some(0));
    r.place_stack(0, &[CARD_ID, "TOP-G4"]);
    let other = r.place_on_field(0, "TOP-G4", Some(0));
    assert_eq!(digivolve(&mut r, other, "G-LV5"), 3, "only THIS Digimon's digivolve");
}

#[test]
fn bt14_046_inherited_once_per_turn() {
    let mut r = runner();
    r.place_on_field(0, "G-TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "TOP-G4"]);
    assert_eq!(digivolve(&mut r, carrier, "G-LV5"), 2);
    assert_eq!(digivolve(&mut r, carrier, "G-LV6"), 3, "[Once Per Turn]");
}
