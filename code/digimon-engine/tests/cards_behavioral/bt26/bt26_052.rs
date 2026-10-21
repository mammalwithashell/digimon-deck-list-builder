//! BT26-052 Pristimon — Digimon, Lv.3, Black, DP 2000, Cost 3.
//! Traits: Puppet, Glowing Dawn, BEATBREAK. Form: Rookie. Attribute: Vaccine.
//! Digivolve: Black Lv.2 / 0; [Digivolve] Lv.2 w/[Glowing Dawn] trait: Cost 0.
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 [Glowing Dawn] trait
//! card and 1 black [BEATBREAK] trait card among them to the hand. Return the
//! rest to the bottom of the deck.
//! Inherited: <Reboot> (This Digimon also unsuspends in your opponent's
//! unsuspend phase.)
//!
//! DCGO: BT26/Black/BT26_052.cs.
//! - Alt-digivolve: level 2 + TopCard EqualsTraits("Glowing Dawn"), cost 0.
//! - On Play: SimplifiedRevealDeckTopCardsAndSelect(3; bucket EqualsTraits
//!   "Glowing Dawn" max 1; bucket black && EqualsTraits "BEATBREAK" max 1;
//!   mutualConditions: true; remainder DeckBottom). Mandatory.
//! - RebootSelfStaticEffect(isInheritedEffect: true).
//!
//! Patterns: BT25-032 Liollmon reveal-3 two-bucket idiom (Group E); inherited
//! static keyword (<Reboot>) driven through the opponent's unsuspend phase.

use super::support::*;
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, PASS, SEL_REVEAL_START};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-052";

fn setup() -> DebugRunner {
    let mut gd_egg = digimon("GDEGG", "GD Egg", CardColor::Yellow, 2, 0, &["Glowing Dawn"]);
    gd_egg.card_kind = CardKind::DigiEgg;
    let mut plain_egg = digimon("EGG", "Plain Egg", CardColor::Red, 2, 0, &[]);
    plain_egg.card_kind = CardKind::DigiEgg;
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-052")
        .add_card(filler("FILLER"))
        .add_card(digimon("GD", "GD", CardColor::Yellow, 3, 3, &["Glowing Dawn"]))
        .add_card(digimon("KBB", "Black BB", CardColor::Black, 4, 5, &["BEATBREAK"]))
        .add_card(digimon("PBB", "Purple BB", CardColor::Purple, 4, 5, &["BEATBREAK"]))
        .add_card(digimon("CARRIER", "Carrier", CardColor::Black, 4, 5, &[]))
        .add_card(digimon("DECKTOP", "DeckTop", CardColor::Red, 3, 3, &[]))
        .add_card(gd_egg)
        .add_card(plain_egg)
        .deck(0, &["FILLER"; 4])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

fn pick_revealed(r: &mut DebugRunner, id: &str) {
    let idx = r
        .game
        .revealed_cards
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not revealed"));
    let action = SEL_REVEAL_START + idx as u16;
    let v = r.pending_selection_view().expect("reveal pick pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{id} not a legal pick (valid {:?})",
        v.valid_action_ids
    );
    assert!(!v.valid_action_ids.contains(&PASS), "the pick is mandatory");
    r.execute_action(0, action).unwrap();
}

fn revealed_legal(r: &DebugRunner, id: &str) -> bool {
    let idx = r
        .game
        .revealed_cards
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap();
    r.pending_selection_view()
        .map(|v| v.valid_action_ids.contains(&(SEL_REVEAL_START + idx as u16)))
        .unwrap_or(false)
}

fn play_from_hand(r: &mut DebugRunner) {
    push_hand(r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    r.play(0, idx).expect("play Pristimon");
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

// ─── [On Play] ──────────────────────────────────────────────────────────────

#[test]
fn bt26_052_on_play_adds_glowing_dawn_and_black_beatbreak_rest_to_bottom() {
    let mut r = setup();
    stack_deck_top(&mut r, 0, &["FILLER", "KBB", "GD"]);
    play_from_hand(&mut r);
    pick_revealed(&mut r, "GD");
    pick_revealed(&mut r, "KBB");
    let _ = r.auto_resolve();
    let hand = hand_ids(&r, 0);
    assert!(hand.contains(&"GD".to_string()), "{hand:?}");
    assert!(hand.contains(&"KBB".to_string()), "{hand:?}");
    assert_eq!(hand.len(), 2);
    assert_eq!(
        deck_ids(&r, 0).first().map(String::as_str),
        Some("FILLER"),
        "the rest returns to the bottom of the deck"
    );
    assert!(r.game.revealed_cards.is_empty());
}

#[test]
fn bt26_052_beatbreak_bucket_rejects_non_black() {
    let mut r = setup();
    stack_deck_top(&mut r, 0, &["FILLER", "PBB", "GD"]);
    play_from_hand(&mut r);
    assert!(!revealed_legal(&r, "PBB"), "a purple [BEATBREAK] is not black");
    pick_revealed(&mut r, "GD");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["GD".to_string()]);
    assert_eq!(r.deck_size(0), 6);
}

#[test]
fn bt26_052_on_play_with_no_match_returns_all_three() {
    let mut r = setup();
    stack_deck_top(&mut r, 0, &["FILLER", "PBB", "DECKTOP"]);
    play_from_hand(&mut r);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(r.deck_size(0), 7);
}

// ─── Alt digivolve ──────────────────────────────────────────────────────────

#[test]
fn bt26_052_digivolves_from_a_glowing_dawn_level_2_for_free() {
    let mut r = setup();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, "GDEGG", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    let action = encode_digivolve(0, host.index as u16);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    let mem = r.memory();
    r.game.decode_action(action, 0);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, host), CARD_ID);
    assert_eq!(r.memory(), mem, "cost 0");
}

#[test]
fn bt26_052_alt_digivolve_needs_the_glowing_dawn_trait() {
    let mut r = setup();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, "EGG", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    let action = encode_digivolve(0, host.index as u16);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 0.0);
}

// ─── Inherited <Reboot> ─────────────────────────────────────────────────────

#[test]
fn bt26_052_inherited_reboot_unsuspends_in_opponents_unsuspend_phase() {
    let mut r = setup();
    let m = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    r.game.players[0].battle_area[m.index as usize].is_suspended = true;
    r.pass_turn(); // → P1's turn: its unsuspend phase
    assert!(!suspended(&r, m), "<Reboot> unsuspends in the opponent's unsuspend phase");
}

#[test]
fn bt26_052_without_pristimon_under_it_no_reboot() {
    let mut r = setup();
    let m = r.place_on_field(0, "CARRIER", Some(0));
    r.game.players[0].battle_area[m.index as usize].is_suspended = true;
    r.pass_turn();
    assert!(suspended(&r, m), "control: no <Reboot>, stays suspended");
}

#[test]
fn bt26_052_face_up_pristimon_has_no_reboot() {
    let mut r = setup();
    let m = r.place_on_field(0, CARD_ID, Some(0));
    r.game.players[0].battle_area[m.index as usize].is_suspended = true;
    r.pass_turn();
    assert!(suspended(&r, m), "<Reboot> is inherited only");
}
