//! BT26-065 Falcomon — Lv.3 Purple, Avian/DATA SQUAD.
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 [Keenan Crier] or card
//! with the [DATA SQUAD] trait and 1 purple card with [Ravemon] in its name or
//! [Avian] or [Bird] in any of its traits among them to the hand. Return the
//! rest to the bottom of the deck.
//! Inherited: [When Attacking] [Once Per Turn] <Draw 1> and trash 1 card in
//! your hand.
//!
//! DCGO: BT26/Purple/BT26_065.cs (two ordered AddHand buckets, mandatory).

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-065";

fn setup(deck_top: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut deck: Vec<&str> = vec!["FILLER"; 4];
    deck.extend(deck_top.iter().rev());
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-065 in embedded DSL pack")
        .add_card(filler("FILLER"))
        // Green [DATA SQUAD] — bucket 1 only (not purple).
        .add_card(digimon(
            "DS-GREEN",
            "Lalamon",
            CardColor::Green,
            3,
            3,
            &["Vegetation", "DATA SQUAD"],
        ))
        // Purple Mysterious Bird/[DATA SQUAD] — legal for BOTH buckets.
        .add_card(digimon(
            "DS-BIRD",
            "Crowmon",
            CardColor::Purple,
            5,
            7,
            &["Mysterious Bird", "DATA SQUAD"],
        ))
        // Purple Avian, not DATA SQUAD — bucket 2 only.
        .add_card(digimon(
            "AVIAN",
            "Hawkmon",
            CardColor::Purple,
            3,
            3,
            &["Avian"],
        ))
        // Purple named Ravemon, no traits — bucket 2 only (name match).
        .add_card(digimon("RAVE", "Ravemon", CardColor::Purple, 6, 12, &[]))
        // Green Avian — fails bucket 2's purple gate.
        .add_card(digimon(
            "GREEN-AVIAN",
            "Biyomon",
            CardColor::Green,
            3,
            3,
            &["Avian"],
        ))
        .deck(0, &deck)
        .deck(1, &["FILLER"; 5])
        .memory(3)
        .start();
    r.set_first_player(0);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    (r, h)
}

#[test]
fn bt26_065_adds_one_per_bucket_and_bottoms_the_rest() {
    let (mut r, h) = setup(&["DS-GREEN", "AVIAN", "FILLER"]);
    r.fire_on_play(0, h.index as usize);
    pick_reveal(&mut r, 0, "DS-GREEN");
    pick_reveal(&mut r, 0, "AVIAN");
    let _ = r.auto_resolve();

    let hand = hand_ids(&r, 0);
    assert!(
        hand.contains(&"DS-GREEN".to_string()) && hand.contains(&"AVIAN".to_string()),
        "{hand:?}"
    );
    assert_eq!(hand.len(), 2);
    // Remainder (FILLER) went to the BOTTOM (index 0).
    assert_eq!(deck_ids(&r, 0)[0], "FILLER");
    assert_eq!(r.deck_size(0), 5);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn bt26_065_first_bucket_is_mandatory_when_a_candidate_exists() {
    let (mut r, h) = setup(&["DS-GREEN", "FILLER", "FILLER"]);
    r.fire_on_play(0, h.index as usize);
    let v = r.pending_selection_view().expect("bucket 1 prompt");
    assert!(
        !v.is_optional,
        "'Add 1' is not a 'may' — no PASS while a candidate exists"
    );
}

#[test]
fn bt26_065_same_card_cannot_fill_both_buckets() {
    // Only DS-BIRD is legal for either bucket: after it fills bucket 1 there is
    // no second candidate, so exactly 1 card is added.
    let (mut r, h) = setup(&["DS-BIRD", "FILLER", "FILLER"]);
    r.fire_on_play(0, h.index as usize);
    pick_reveal(&mut r, 0, "DS-BIRD");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["DS-BIRD".to_string()]);
    assert_eq!(r.deck_size(0), 6);
}

#[test]
fn bt26_065_second_bucket_requires_purple() {
    let (mut r, h) = setup(&["GREEN-AVIAN", "FILLER", "FILLER"]);
    r.fire_on_play(0, h.index as usize);
    let _ = r.auto_resolve();
    assert!(
        hand_ids(&r, 0).is_empty(),
        "a green Avian matches neither bucket"
    );
    assert_eq!(r.deck_size(0), 7);
}

#[test]
fn bt26_065_second_bucket_accepts_ravemon_by_name() {
    let (mut r, h) = setup(&["RAVE", "DS-GREEN", "FILLER"]);
    r.fire_on_play(0, h.index as usize);
    pick_reveal(&mut r, 0, "DS-GREEN");
    pick_reveal(&mut r, 0, "RAVE");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0).len(), 2);
}

#[test]
fn bt26_065_inherited_draws_then_trashes_one() {
    let (mut r, _) = setup(&["FILLER", "FILLER", "FILLER"]);
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    push_hand(&mut r, 0, "AVIAN");
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    // Draw happened first: hand is AVIAN + the drawn FILLER; discard is mandatory.
    let v = r.pending_selection_view().expect("discard prompt");
    assert!(!v.is_optional, "the trash is mandatory");
    assert_eq!(r.hand_size(0), 2, "drew before the discard prompt");
    pick_hand(&mut r, 0, "AVIAN");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["FILLER".to_string()]);
    assert_eq!(trash_ids(&r, 0), vec!["AVIAN".to_string()]);

    // [Once Per Turn]: a second attack this turn does nothing.
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 1);
}
