//! BT3-054 Blossomon — Digimon, Lv.5, Green, DP 7000, Cost 7.
//! Traits: Vegetation. Form: Ultimate. Attribute: Data.
//! Digivolve: green Lv.4, cost 3.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT3-054.md; card image agrees)
//!
//! ```text
//! ＜Digisorption -3＞ (When one of your Digimon digivolves into this card from
//! your hand, you may suspend 1 of your Digimon to reduce the memory cost of
//! the digivolution by 3.)
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT3/Green/BT3_054.cs — BeforePayCost
//! ActivateClass (optional), only for a digivolution INTO this hand card by
//! the owner; SelectPermanent over the owner's Digimon that can be suspended
//! (`CanTapWhenAbsorbEvolution`, the digivolving Digimon included), Mode.Tap,
//! then "Digivolution Cost -3".
//!
//! # Rules (Ver. 4.3)
//! - §16-10-1/-2: an immediate-type effect of the HAND card, offered when a
//!   digivolution into it would occur, before the cost is paid.
//! - §16-10-3: suspending is an optional processing condition; once done the
//!   reduction is mandatory.
//! - §16-10-5: the Digimon that will become a digivolution card may be the
//!   one suspended.
//!
//! # Patterns this test covers
//! - H-row printed keyword `<Digisorption -3>` (`grant_keyword` with `value`),
//!   read off the hand card by the digivolve reducer chain

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::action::space::{encode_digivolve, ATTACK_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT3-054";

fn digimon(id: &str, color: CardColor, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

fn runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT3-054 in pack")
        .add_card(digimon("G-LV4", CardColor::Green, 4))
        .add_card(digimon("ALLY", CardColor::Blue, 3))
        .add_card(digimon("OPP", CardColor::Red, 4))
        .add_card(make_test_card("F1", "F1"))
        .deck(0, &["F1"; 10])
        .deck(1, &["F1"; 10])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.game.enter_main_phase();
    r.game.set_memory(5);
    r
}

fn hand_idx(r: &DebugRunner, id: &str) -> u16 {
    r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not in hand")) as u16
}

/// P0 digivolves `base` into Blossomon from the hand (the player action).
fn digivolve_into_blossomon(r: &mut DebugRunner, base: PermanentHandle) {
    r.add_to_hand(0, CARD_ID);
    let h = hand_idx(r, CARD_ID);
    r.game.decode_action(encode_digivolve(h, base.index as u16), 0);
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn bt3_054_metadata_and_green_lv4_cost3_circle() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, "Blossomon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(7000));
    assert_eq!(card.traits, vec!["Vegetation".to_string()]);
    assert_eq!(card.attribute, ["Data"]);
    assert_eq!(card.alt_paths.len(), 1);
    assert_eq!(card.alt_paths[0].cost, Some(CompiledCost::Literal(3)));
}

#[test]
fn bt3_054_is_exactly_one_digisorption_3_keyword() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.effects.len(), 1);
    match &card.effects[0] {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            value,
            scope,
            active_when,
            ..
        }) => {
            assert_eq!(keyword, "Digisorption");
            assert_eq!(*value, Some(3));
            assert_eq!(*scope, CompiledScope::FaceUp);
            assert!(active_when.is_none());
        }
        other => panic!("expected <Digisorption -3>, got {other:?}"),
    }
}

// ─── SECTION 2 — <Digisorption -3> ─────────────────────────────────────────

/// Offered before the cost is paid; suspending another Digimon reduces the
/// cost 3 → 0.
#[test]
fn bt3_054_suspending_another_digimon_reduces_the_cost_by_3() {
    let mut r = runner();
    let base = r.place_on_field(0, "G-LV4", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    digivolve_into_blossomon(&mut r, base);
    let v = r.pending_selection_view().expect("<Digisorption> offered");
    assert!(v.is_optional && v.selecting_player == 0);
    assert_eq!(r.memory(), 5, "nothing paid yet (§16-10-2)");
    r.accept_optional_trigger().unwrap();
    r.execute_action(0, ATTACK_START + ally.index as u16).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, base), CARD_ID);
    assert!(suspended(&r, ally));
    assert_eq!(r.memory(), 5, "3 - 3 = 0");
}

/// §16-10-5: the Digimon digivolving can be the one suspended; the new
/// Blossomon stays suspended.
#[test]
fn bt3_054_can_suspend_the_digivolving_digimon_itself() {
    let mut r = runner();
    let base = r.place_on_field(0, "G-LV4", Some(0));
    digivolve_into_blossomon(&mut r, base);
    r.accept_optional_trigger().unwrap();
    let v = r.pending_selection_view().expect("suspend pick");
    assert!(v.valid_action_ids.contains(&(ATTACK_START + base.index as u16)));
    r.execute_action(0, ATTACK_START + base.index as u16).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, base), CARD_ID);
    assert!(suspended(&r, base));
    assert_eq!(r.memory(), 5);
}

/// "you may": declining pays the full cost 3 and suspends nothing.
#[test]
fn bt3_054_declining_pays_full_cost() {
    let mut r = runner();
    let base = r.place_on_field(0, "G-LV4", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    digivolve_into_blossomon(&mut r, base);
    r.decline_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, base), CARD_ID);
    assert!(!suspended(&r, ally) && !suspended(&r, base));
    assert_eq!(r.memory(), 2, "5 - 3");
}

/// "1 of YOUR Digimon": the opponent's Digimon are not offered; a suspended
/// Digimon can't be suspended.
#[test]
fn bt3_054_only_own_unsuspended_digimon_can_pay() {
    let mut r = runner();
    let base = r.place_on_field(0, "G-LV4", Some(0));
    let tapped = r.place_on_field(0, "ALLY", Some(0));
    r.game.players[0].battle_area[tapped.index as usize].is_suspended = true;
    let opp = r.place_on_field(1, "OPP", Some(0));
    digivolve_into_blossomon(&mut r, base);
    r.accept_optional_trigger().unwrap();
    let v = r.pending_selection_view().expect("suspend pick");
    assert_eq!(v.valid_action_ids, vec![ATTACK_START + base.index as u16], "{v:?}");
    let _ = opp;
}

/// §15-7-4: with no Digimon of yours able to suspend, it isn't offered.
#[test]
fn bt3_054_not_offered_when_nothing_can_be_suspended() {
    let mut r = runner();
    let base = r.place_on_field(0, "G-LV4", Some(0));
    r.game.players[0].battle_area[base.index as usize].is_suspended = true;
    digivolve_into_blossomon(&mut r, base);
    assert!(r.pending_selection().is_none(), "{:?}", r.pending_selection_view());
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, base), CARD_ID);
    assert_eq!(r.memory(), 2, "full cost");
}

/// Only a DIGIVOLUTION into this card: playing Blossomon from the hand is
/// not reduced and offers nothing.
#[test]
fn bt3_054_playing_it_is_not_reduced() {
    let mut r = runner();
    r.place_on_field(0, "ALLY", Some(0));
    r.game.set_memory(10);
    r.add_to_hand(0, CARD_ID);
    let h = hand_idx(&r, CARD_ID) as usize;
    let _ = r.play(0, h);
    assert!(r.pending_selection().is_none(), "no <Digisorption> on a play");
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 3, "10 - 7");
}
