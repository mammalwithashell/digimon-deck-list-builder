//! BT26-061 Chiropmon — Digimon, Lv.3, Purple, DP 2000, Cost 3.
//! Traits: Mammal, Glowing Dawn, BEATBREAK. Form: Rookie. Attribute: Virus.
//! Digivolve: Purple Lv.2 / 0; [Digivolve] Lv.2 w/[Glowing Dawn] trait: Cost 0.
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 [Glowing Dawn] trait
//! card and 1 purple [BEATBREAK] trait card among them to the hand. Return the
//! rest to the bottom of the deck.
//! Inherited: [When Attacking] [Once Per Turn] <Draw 1> and trash 1 card in
//! your hand.
//!
//! DCGO: BT26/Purple/BT26_061.cs.
//! - Alt-digivolve: level 2 + TopCard EqualsTraits("Glowing Dawn"), cost 0.
//! - On Play: SimplifiedRevealDeckTopCardsAndSelect(3; bucket EqualsTraits
//!   "Glowing Dawn" max 1 AddHand; bucket HasCardColor(Purple) && EqualsTraits
//!   "BEATBREAK" max 1 AddHand; remainder DeckBottom). Mandatory.
//! - Inherited OnAllyAttack (maxCount 1, not optional): Draw 1 → if hand >= 1
//!   a mandatory (canNoSelect:false) Discard pick.
//!
//! Patterns: BT25-032 Liollmon reveal-3 two-bucket idiom (Group E); inherited
//! When Attacking OPT draw/discard (Group G + OPT lockout).

use super::support::*;
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, PASS, SEL_REVEAL_START};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};

const CARD_ID: &str = "BT26-061";

fn setup() -> DebugRunner {
    let mut gd_egg = digimon("GDEGG", "GD Egg", CardColor::Purple, 2, 0, &["Glowing Dawn"]);
    gd_egg.card_kind = CardKind::DigiEgg;
    let mut plain_egg = digimon("EGG", "Plain Egg", CardColor::Red, 2, 0, &[]);
    plain_egg.card_kind = CardKind::DigiEgg;
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-061")
        .add_card(filler("FILLER"))
        .add_card(digimon("GD", "GD", CardColor::Yellow, 3, 3, &["Glowing Dawn"]))
        .add_card(digimon("PBB", "Purple BB", CardColor::Purple, 4, 5, &["BEATBREAK"]))
        .add_card(digimon("YBB", "Yellow BB", CardColor::Yellow, 4, 5, &["BEATBREAK"]))
        .add_card(digimon(
            "BOTH",
            "Purple GD BB",
            CardColor::Purple,
            4,
            5,
            &["Glowing Dawn", "BEATBREAK"],
        ))
        .add_card(digimon("CARRIER", "Carrier", CardColor::Purple, 4, 5, &[]))
        .add_card(digimon("DECKTOP", "DeckTop", CardColor::Red, 3, 3, &[]))
        .add_card(gd_egg)
        .add_card(plain_egg)
        .deck(0, &["FILLER"; 4])
        .deck(1, &["FILLER"; 6])
        .security(1, &["FILLER"; 3])
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
    r.play(0, idx).expect("play Chiropmon");
}

// ─── [On Play] ──────────────────────────────────────────────────────────────

#[test]
fn bt26_061_on_play_adds_glowing_dawn_and_purple_beatbreak_rest_to_bottom() {
    let mut r = setup();
    // Top 3 (last pushed = top): FILLER, PBB, GD.
    stack_deck_top(&mut r, 0, &["FILLER", "PBB", "GD"]);
    play_from_hand(&mut r);
    pick_revealed(&mut r, "GD");
    pick_revealed(&mut r, "PBB");
    let _ = r.auto_resolve();
    let hand = hand_ids(&r, 0);
    assert!(hand.contains(&"GD".to_string()), "{hand:?}");
    assert!(hand.contains(&"PBB".to_string()), "{hand:?}");
    assert_eq!(hand.len(), 2);
    assert_eq!(
        deck_ids(&r, 0).first().map(String::as_str),
        Some("FILLER"),
        "the rest returns to the bottom of the deck"
    );
    assert!(r.game.revealed_cards.is_empty());
}

#[test]
fn bt26_061_beatbreak_bucket_rejects_non_purple() {
    let mut r = setup();
    stack_deck_top(&mut r, 0, &["FILLER", "YBB", "GD"]);
    play_from_hand(&mut r);
    assert!(!revealed_legal(&r, "YBB"), "a yellow [BEATBREAK] is not purple");
    pick_revealed(&mut r, "GD");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["GD".to_string()]);
    assert_eq!(r.deck_size(0), 6, "YBB + FILLER back under the deck");
}

#[test]
fn bt26_061_one_card_cannot_fill_both_buckets() {
    let mut r = setup();
    stack_deck_top(&mut r, 0, &["FILLER", "FILLER", "BOTH"]);
    play_from_hand(&mut r);
    pick_revealed(&mut r, "BOTH");
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(hand_ids(&r, 0), vec!["BOTH".to_string()], "added once");
}

#[test]
fn bt26_061_on_play_with_no_match_returns_all_three() {
    let mut r = setup();
    stack_deck_top(&mut r, 0, &["FILLER", "YBB", "DECKTOP"]);
    play_from_hand(&mut r);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(r.deck_size(0), 7);
    assert!(r.game.revealed_cards.is_empty());
}

// ─── Alt digivolve ──────────────────────────────────────────────────────────

#[test]
fn bt26_061_digivolves_from_a_glowing_dawn_level_2_for_free() {
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
fn bt26_061_alt_digivolve_needs_the_glowing_dawn_trait() {
    let mut r = setup();
    r.game.enter_main_phase();
    // A red Lv.2 without [Glowing Dawn]: neither the purple circle nor the
    // [Glowing Dawn] path applies.
    let host = r.place_on_field(0, "EGG", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    let action = encode_digivolve(0, host.index as u16);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 0.0);
}

// ─── Inherited [When Attacking] [Once Per Turn] ─────────────────────────────

#[test]
fn bt26_061_inherited_when_attacking_draws_then_trashes_one() {
    let mut r = setup();
    stack_deck_top(&mut r, 0, &["DECKTOP"]);
    let m = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    push_hand(&mut r, 0, "GD");
    r.attack_player(m, 1, false);
    let v = r.pending_selection_view().expect("trash 1 hand card");
    assert!(!v.is_optional, "mandatory");
    assert_eq!(r.hand_size(0), 2, "<Draw 1> first");
    pick_hand(&mut r, 0, "GD");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["DECKTOP".to_string()]);
    assert!(trash_ids(&r, 0).contains(&"GD".to_string()));
}

#[test]
fn bt26_061_inherited_is_once_per_turn() {
    let mut r = setup();
    let m = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    fire(&mut r, EffectTiming::WhenAttacking, m);
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    let hand = r.hand_size(0);
    let deck = r.deck_size(0);
    fire(&mut r, EffectTiming::WhenAttacking, m);
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck, "no second draw this turn");
    assert_eq!(r.hand_size(0), hand);
    r.pass_turn();
    r.pass_turn();
    let deck = r.deck_size(0);
    fire(&mut r, EffectTiming::WhenAttacking, m);
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck - 1, "lockout clears next turn");
}

#[test]
fn bt26_061_face_up_chiropmon_has_no_when_attacking() {
    let mut r = setup();
    let m = r.place_on_field(0, CARD_ID, Some(0));
    let deck = r.deck_size(0);
    fire(&mut r, EffectTiming::WhenAttacking, m);
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck, "inherited only");
    assert!(r.game.pending_selection.is_none());
}
