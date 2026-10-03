//! BT8-095 Fire Rocket — Option, Red, Cost 1.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT8-095.md)
//!
//! While you have a Digimon with the [Armor Form] trait, you can ignore this
//! card's color requirements.
//! [Main] 1 of your multicolored Digimon gains <Security A. +1> for the turn.
//! [Security] Delete 1 of your opponent's Digimon with <Blocker>.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT8/Red/BT8_095.cs
//!   - IgnoreColorConditionClass on this card while owner has a battle-area
//!     Digimon whose TopCard traits contain "Armor Form".
//!   - OptionSkill: SelectPermanentEffect (canNoSelect: false) over own Digimon
//!     with TopCard.CardColors.Count >= 2 → ChangeDigimonSAttack(+1,
//!     UntilEachTurnEnd).
//!   - SecuritySkill: SelectPermanentEffect (canNoSelect: false, Destroy) over
//!     opponent Digimon with HasBlocker.
//!
//! # Patterns
//! - use_requirement colour bypass (BT19-089 idiom) — Armor Form is a FORM
//!   folded into traits by the card loader, so `trait_has: "Armor Form"`.
//! - select_own_permanent `self_color_count_gte: 2` → SecurityAttackChange +1
//!   end_of_turn
//! - on_security select_opponent_permanent `has_keyword: Blocker` → delete

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_engine::action::space::{encode_attack, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "BT8-095";

fn digimon(id: &str, colors: &[CardColor], traits: &[&str], keywords: &[Keyword]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = colors.to_vec();
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c.keywords = keywords.to_vec();
    c
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT8-095 compiles")
        .add_card(digimon("ARMOR-BLUE", &[CardColor::Blue], &["Armor Form"], &[]))
        .add_card(digimon("PLAIN-BLUE", &[CardColor::Blue], &["Beast"], &[]))
        .add_card(digimon("RED", &[CardColor::Red], &[], &[]))
        .add_card(digimon("RED-YEL", &[CardColor::Red, CardColor::Yellow], &[], &[]))
        .add_card(digimon("BLU-GRN", &[CardColor::Blue, CardColor::Green], &[], &[]))
        .add_card(digimon("OPP-BLOCKER", &[CardColor::Black], &[], &[Keyword::Blocker]))
        .add_card(digimon("OPP-BLOCKER2", &[CardColor::Black], &[], &[Keyword::Blocker]))
        .add_card(digimon("OPP-PLAIN", &[CardColor::Black], &[], &[]))
        .add_card(digimon("FILLER", &[CardColor::Red], &[], &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
}

fn main_runner() -> DebugRunner {
    let mut r = builder().hand(0, &[CARD_ID]).memory(5).start();
    r.set_first_player(0);
    r.game.enter_main_phase();
    r
}

fn sa(r: &DebugRunner, h: PermanentHandle) -> i32 {
    r.game.modifiers.sum(h, ModifierType::SecurityAttackChange)
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

// ─── Colour requirement bypass ───────────────────────────────────────────────

#[test]
fn bt8_095_off_colour_without_armor_form_is_not_playable() {
    let mut r = main_runner();
    r.place_on_field(0, "PLAIN-BLUE", Some(0));
    let mask = digimon_engine::action::build_action_mask(&r.game, 0);
    assert_eq!(mask[PLAY_HAND_START as usize], 0.0);
    assert_eq!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Invalid);
    assert_eq!(r.game.players[0].hand.len(), 1);
}

#[test]
fn bt8_095_armor_form_digimon_lets_you_ignore_colour() {
    let mut r = main_runner();
    r.place_on_field(0, "ARMOR-BLUE", Some(0));
    let mask = digimon_engine::action::build_action_mask(&r.game, 0);
    assert_eq!(mask[PLAY_HAND_START as usize], 1.0);
    assert_ne!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Invalid);
}

#[test]
fn bt8_095_red_permanent_plays_normally() {
    let mut r = main_runner();
    r.place_on_field(0, "RED", Some(0));
    assert_ne!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Invalid);
}

// ─── [Main] ──────────────────────────────────────────────────────────────────

#[test]
fn bt8_095_main_grants_security_attack_to_chosen_multicolour_digimon() {
    let mut r = main_runner();
    let mono = r.place_on_field(0, "RED", Some(0));
    let a = r.place_on_field(0, "RED-YEL", Some(0));
    let b = r.place_on_field(0, "BLU-GRN", Some(0));
    assert_ne!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Invalid);
    let v = r.pending_selection_view().expect("mandatory own-field pick");
    assert_eq!(v.kind, SelectionKind::OwnField);
    assert!(!r.pending_is_optional(), "canNoSelect: false");
    let mut offered = v.valid_action_ids.clone();
    offered.sort();
    let mut want = vec![encode_attack(0, a.index as u16), encode_attack(0, b.index as u16)];
    want.sort();
    assert_eq!(offered, want, "only 2+ colour Digimon are eligible");
    r.execute_action(0, encode_attack(0, b.index as u16)).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(sa(&r, b), 1, "<Security A. +1>");
    assert_eq!(sa(&r, a), 0);
    assert_eq!(sa(&r, mono), 0);
}

#[test]
fn bt8_095_main_security_attack_expires_at_end_of_turn() {
    let mut r = main_runner();
    let a = r.place_on_field(0, "RED-YEL", Some(0));
    r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(sa(&r, a), 1);
    r.end_turn();
    assert_eq!(sa(&r, a), 0, "for the turn only");
}

#[test]
fn bt8_095_main_no_multicolour_digimon_no_effect() {
    let mut r = main_runner();
    let mono = r.place_on_field(0, "RED", Some(0));
    let res = r.game.play_option_from_hand(0, 0);
    assert_ne!(res, OptionPlayResult::Invalid, "the Option can still be used");
    let offered: Vec<u16> = r
        .pending_selection_view()
        .map(|v| v.valid_action_ids.into_iter().filter(|&a| a != PASS).collect())
        .unwrap_or_default();
    assert!(offered.is_empty(), "no eligible target");
    let _ = r.auto_resolve();
    assert_eq!(sa(&r, mono), 0);
}

// ─── [Security] ──────────────────────────────────────────────────────────────

fn security_runner() -> (DebugRunner, PermanentHandle) {
    let mut r = builder().security(0, &[CARD_ID]).memory(5).start();
    r.set_first_player(0);
    r.game.enter_main_phase();
    let attacker = r.place_on_field(1, "OPP-PLAIN", Some(0));
    (r, attacker)
}

#[test]
fn bt8_095_security_deletes_chosen_opponent_blocker() {
    let (mut r, attacker) = security_runner();
    let b1 = r.place_on_field(1, "OPP-BLOCKER", Some(0));
    let b2 = r.place_on_field(1, "OPP-BLOCKER2", Some(0));
    r.attack_player(attacker, 0, false);
    let v = r.pending_selection_view().expect("[Security] delete pick");
    assert_eq!(v.kind, SelectionKind::OppField);
    let mut offered: Vec<u16> = v
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    offered.sort();
    let mut want = vec![encode_attack(0, b1.index as u16), encode_attack(0, b2.index as u16)];
    want.sort();
    assert_eq!(offered, want, "only <Blocker> Digimon are eligible");
    r.execute_action(v.selecting_player, encode_attack(0, b2.index as u16))
        .unwrap();
    let _ = r.auto_resolve();
    let opp = field_ids(&r, 1);
    assert!(!opp.contains(&"OPP-BLOCKER2".to_string()), "chosen Blocker deleted");
    assert!(opp.contains(&"OPP-BLOCKER".to_string()));
    assert!(opp.contains(&"OPP-PLAIN".to_string()), "non-Blocker untouched");
}

#[test]
fn bt8_095_security_without_opponent_blocker_does_nothing() {
    let (mut r, attacker) = security_runner();
    r.attack_player(attacker, 0, false);
    let offered: Vec<u16> = r
        .pending_selection_view()
        .map(|v| v.valid_action_ids.into_iter().filter(|&a| a != PASS).collect())
        .unwrap_or_default();
    assert!(offered.is_empty(), "no <Blocker> target");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&"OPP-PLAIN".to_string()));
}
