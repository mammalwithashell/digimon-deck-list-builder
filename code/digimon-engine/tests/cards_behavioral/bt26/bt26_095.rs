//! BT26-095 Makoto Kuonji — Tamer, Purple, Cost 3. Traits: Glowing Dawn, BEATBREAK.
//!
//! [Start of Your Main Phase] By placing 1 [BEATBREAK] trait card from your
//! hand face down under this Tamer, <Draw 1> and gain 1 memory.
//! [All Turns] When any Digimon are deleted, by suspending this Tamer,
//! <Draw 1> and trash 1 card in your hand. After, place 1 [BEATBREAK] trait
//! non-Digi-Egg card from your trash face down under this Tamer.
//! [Security] Play this card without paying the cost.
//! Official Q&A: the card is placed on the bottom of the cards under the Tamer.
//!
//! DCGO: BT26/Purple/BT26_095.cs.
//! - SOMP (OnStartMainPhase, not optional, skippable): canNoSelect:true hand
//!   pick over EqualsTraits("BEATBREAK") → AddDigivolutionCardsBottom(face
//!   down) → Draw 1 → AddMemory 1 (only when a card was placed).
//! - [All Turns] (OnDestroyedAnyone, optional): any Digimon deleted (either
//!   player's); CanActivate = suspend cost payable. Suspend → Draw 1 → if hand
//!   non-empty a mandatory Discard pick → if the trash holds a BEATBREAK
//!   non-Digi-Egg card a mandatory pick → AddDigivolutionCardsBottom(face down).
//! - Security: PlaySelfTamerSecurityEffect.
//!
//! Patterns: SOMP optional-cost stash (Kyo BT26-089 / Keenan BT26-094 idiom);
//! on_any_deletion observer + suspend_self activation cost; trash →
//! place_as_bottom_source face down; §15-5-2 one trigger per deletion event.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "BT26-095";

fn setup() -> DebugRunner {
    let mut egg = digimon("BBEGG", "BB Egg", CardColor::Purple, 2, 0, &["BEATBREAK"]);
    egg.card_kind = CardKind::DigiEgg;
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-095")
        .add_card(filler("FILLER"))
        .add_card(tamer("SECT", "SecTamer", CardColor::Red, &[]))
        .add_card(digimon(
            "BB",
            "Beatbreak",
            CardColor::Purple,
            3,
            3,
            &["BEATBREAK"],
        ))
        .add_card(tamer("BBT", "BB Tamer", CardColor::Purple, &["BEATBREAK"]))
        .add_card(egg)
        .add_card(digimon("PLAIN", "Plain", CardColor::Purple, 3, 3, &[]))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("DECKTOP", "DeckTop", CardColor::Red, 3, 3, &[]))
        .deck(0, &["FILLER", "FILLER", "DECKTOP"])
        .deck(1, &["FILLER"; 6])
        .security(0, &["SECT", "SECT", "SECT"])
        .security(1, &["SECT", "SECT", "SECT"])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn delete(r: &mut DebugRunner, victims: Vec<PermanentHandle>) {
    r.game
        .delete_permanents_batch(victims, ReplacementCause::OpponentEffect);
}

// ─── [Start of Your Main Phase] ─────────────────────────────────────────────

#[test]
fn bt26_095_somp_places_beatbreak_card_bottom_face_down_draws_and_gains_memory() {
    let mut r = setup();
    let k = r.place_stack(0, &["FILLER", CARD_ID]);
    push_hand(&mut r, 0, "BB");
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, k);
    let v = r.pending_selection_view().expect("optional hand pick");
    assert!(v.is_optional, "the placement is the optional cost");
    pick_hand(&mut r, 0, "BB");
    let _ = r.auto_resolve();
    assert_eq!(
        source_ids(&r, k),
        vec!["BB".to_string(), "FILLER".to_string()],
        "placed at the BOTTOM of the cards under the Tamer (official Q&A)"
    );
    assert_eq!(face_down_flags(&r, k)[0], true, "face down");
    assert_eq!(hand_ids(&r, 0), vec!["DECKTOP".to_string()], "<Draw 1>");
    assert_eq!(r.memory(), mem + 1, "gain 1 memory");
}

#[test]
fn bt26_095_somp_fires_at_the_start_of_your_main_phase() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "BB");
    r.pass_turn(); // → P1
    r.pass_turn(); // → P0: unsuspend, draw, breeding, main → SOMP
    let v = r
        .pending_selection_view()
        .expect("SOMP offers the BEATBREAK placement");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "BB");
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, k), vec!["BB".to_string()]);
}

#[test]
fn bt26_095_somp_declined_does_nothing() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "BB");
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, k);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, k), 0);
    assert_eq!(hand_ids(&r, 0), vec!["BB".to_string()], "no draw");
    assert_eq!(r.memory(), mem, "no memory");
}

#[test]
fn bt26_095_somp_needs_a_beatbreak_card_in_hand() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "PLAIN");
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, k);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(sources(&r, k), 0);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.memory(), mem);
}

#[test]
fn bt26_095_somp_offers_only_beatbreak_hand_cards() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "PLAIN");
    push_hand(&mut r, 0, "BB");
    fire(&mut r, EffectTiming::StartOfYourMainPhase, k);
    let picks = non_pass(&r);
    assert_eq!(
        picks,
        vec![digimon_engine::action::space::PLAY_HAND_START + 1],
        "only the [BEATBREAK] card is offered"
    );
}

// ─── [All Turns] When any Digimon are deleted ───────────────────────────────

#[test]
fn bt26_095_opponent_digimon_deletion_suspends_draws_trashes_and_stashes() {
    let mut r = setup();
    let k = r.place_stack(0, &["FILLER", CARD_ID]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    push_hand(&mut r, 0, "PLAIN");
    push_trash(&mut r, 0, "BB");
    delete(&mut r, vec![opp]);
    r.accept_optional_trigger()
        .expect("Makoto triggers on any Digimon deletion");
    assert!(suspended(&r, k), "suspend cost paid");
    assert_eq!(
        hand_ids(&r, 0),
        vec!["PLAIN".to_string(), "DECKTOP".to_string()],
        "<Draw 1>"
    );
    let v = r.pending_selection_view().expect("trash 1 hand card");
    assert!(!v.is_optional, "the hand trash is mandatory");
    pick_hand(&mut r, 0, "PLAIN");
    let v = r.pending_selection_view().expect("trash → under-Tamer pick");
    assert!(!v.is_optional, "the placement is mandatory");
    pick_trash(&mut r, 0, "BB");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["DECKTOP".to_string()]);
    assert!(trash_ids(&r, 0).contains(&"PLAIN".to_string()));
    assert_eq!(
        source_ids(&r, k),
        vec!["BB".to_string(), "FILLER".to_string()],
        "placed at the bottom"
    );
    assert_eq!(face_down_flags(&r, k)[0], true, "face down");
}

#[test]
fn bt26_095_own_digimon_deletion_also_triggers() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    let mine = r.place_on_field(0, "PLAIN", Some(0));
    push_trash(&mut r, 0, "BB");
    delete(&mut r, vec![mine]);
    r.accept_optional_trigger().expect("own deletion triggers too");
    // Hand: the drawn DECKTOP only — it is trashed.
    pick_hand(&mut r, 0, "DECKTOP");
    pick_trash(&mut r, 0, "BB");
    let _ = r.auto_resolve();
    assert!(suspended(&r, k));
    assert_eq!(source_ids(&r, k), vec!["BB".to_string()]);
    assert_eq!(r.hand_size(0), 0);
}

#[test]
fn bt26_095_deletion_trigger_is_declinable() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    push_trash(&mut r, 0, "BB");
    delete(&mut r, vec![opp]);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert!(!suspended(&r, k));
    assert_eq!(r.hand_size(0), 0, "no draw");
    assert_eq!(sources(&r, k), 0);
}

#[test]
fn bt26_095_suspended_makoto_cannot_pay_the_cost() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    r.game.players[0].battle_area[k.index as usize].is_suspended = true;
    let opp = r.place_on_field(1, "OPP", Some(0));
    push_trash(&mut r, 0, "BB");
    delete(&mut r, vec![opp]);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(sources(&r, k), 0);
}

#[test]
fn bt26_095_tamer_deletion_does_not_trigger() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    let t = r.place_on_field(1, "SECT", Some(0));
    push_trash(&mut r, 0, "BB");
    delete(&mut r, vec![t]);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none(), "a Tamer is not a Digimon");
    assert!(!suspended(&r, k));
    assert_eq!(r.hand_size(0), 0);
}

#[test]
fn bt26_095_two_simultaneous_deletions_trigger_once() {
    // §15-5-2: one trigger condition met twice at the same time triggers once.
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(1, "OPP", Some(0));
    let b = r.place_on_field(1, "OPP", Some(0));
    push_trash(&mut r, 0, "BB");
    push_trash(&mut r, 0, "BB");
    delete(&mut r, vec![a, b]);
    r.accept_optional_trigger().expect("Makoto triggers");
    pick_hand(&mut r, 0, "DECKTOP");
    pick_trash(&mut r, 0, "BB");
    assert!(
        r.pending_selection().is_none(),
        "no second activation / prompt for the same deletion event"
    );
    assert!(suspended(&r, k));
    assert_eq!(sources(&r, k), 1);
}

#[test]
fn bt26_095_placement_excludes_digi_eggs_and_non_beatbreak_cards() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    push_trash(&mut r, 0, "BBEGG");
    push_trash(&mut r, 0, "PLAIN");
    push_trash(&mut r, 0, "BBT");
    delete(&mut r, vec![opp]);
    r.accept_optional_trigger().expect("trigger");
    pick_hand(&mut r, 0, "DECKTOP");
    // Trash now: BBEGG, PLAIN, BBT, DECKTOP — only the BEATBREAK Tamer
    // (a non-Digi-Egg BEATBREAK card) is eligible.
    let bbt = r.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "BBT")
        .unwrap();
    assert_eq!(
        non_pass(&r),
        vec![digimon_engine::action::space::TRASH_EFFECT_START + bbt as u16],
        "only the non-Digi-Egg [BEATBREAK] card is offered"
    );
    pick_trash(&mut r, 0, "BBT");
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, k), vec!["BBT".to_string()]);
}

#[test]
fn bt26_095_draw_and_trash_still_happen_without_a_beatbreak_card_in_trash() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    push_hand(&mut r, 0, "PLAIN");
    push_trash(&mut r, 0, "BBEGG");
    delete(&mut r, vec![opp]);
    r.accept_optional_trigger().expect("trigger");
    pick_hand(&mut r, 0, "PLAIN");
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert!(suspended(&r, k));
    assert_eq!(hand_ids(&r, 0), vec!["DECKTOP".to_string()]);
    assert_eq!(sources(&r, k), 0, "nothing eligible to place");
}

#[test]
fn bt26_095_the_card_trashed_from_hand_may_be_the_one_placed() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    push_hand(&mut r, 0, "BB");
    delete(&mut r, vec![opp]);
    r.accept_optional_trigger().expect("trigger");
    pick_hand(&mut r, 0, "BB");
    pick_trash(&mut r, 0, "BB");
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, k), vec!["BB".to_string()]);
    assert_eq!(hand_ids(&r, 0), vec!["DECKTOP".to_string()]);
}

// ─── [Security] ─────────────────────────────────────────────────────────────

#[test]
fn bt26_095_security_effect_plays_itself() {
    let mut r = setup();
    r.game.players[0].security.clear();
    let idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .unwrap();
    let next = r.game.next_card_index();
    r.game.players[0]
        .security
        .push(digimon_engine::card_source::CardSource::new(idx, 0, next));
    r.pass_turn(); // → P1
    let atk = r.place_on_field(1, "OPP", Some(0));
    r.attack_player(atk, 0, false);
    let _ = r.auto_resolve();
    assert!(
        field_ids(&r, 0).contains(&CARD_ID.to_string()),
        "[Security] Play this card without paying the cost"
    );
}
