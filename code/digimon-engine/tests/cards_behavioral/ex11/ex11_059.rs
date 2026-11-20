//! EX11-059 Reina Oumi — Tamer, Yellow/Purple, cost 4. Traits: NSo, LIBERATOR.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX11-059.md)
//! [Start of Your Main Phase] [On Play] By trashing 1 [NSo] trait card from your
//! hand, <Draw 1> and gain 1 memory.
//! [All Turns] When any of your [NSo] trait Digimon are deleted, by suspending
//! this Tamer, 1 of your [NSo] trait Digimon and 1 [NSo] trait Digimon card in
//! the trash may DNA digivolve into a Digimon card with the [NSo] trait in the
//! hand.
//! Security Effect
//! [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX11/Yellow/EX11_059.cs — see the YAML header.

use digimon_engine::debug_runner::{make_test_card, make_test_dna_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

#[path = "../ex8/nso_support.rs"]
mod nso_support;
use nso_support::*;

const CARD_ID: &str = "EX11-059";

fn builder() -> DebugRunnerBuilder {
    let mut dna_result = make_test_dna_card("NSO-DNA", "Nso Dna Result", 4, 4, 0);
    dna_result.traits = vec!["NSo".to_string()];
    dna_result.colors = vec![CardColor::Purple];
    dna_result.level = Some(6);
    dna_result.dp = Some(9000);
    let mut plain_dna = make_test_dna_card("PLAIN-DNA", "Plain Dna Result", 4, 4, 0);
    plain_dna.colors = vec![CardColor::Purple];
    plain_dna.level = Some(6);
    plain_dna.dp = Some(9000);
    let mut nso_option = nso("NSO-OPTION", 3, 0, 2, CardColor::Purple);
    nso_option.card_kind = CardKind::Option;
    nso_option.level = None;
    nso_option.dp = None;
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-059 YAML loads")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(nso("N4-FIELD", 4, 4000, 4, CardColor::Purple))
        .add_card(nso("N4-FIELD-2", 4, 4000, 4, CardColor::Purple))
        .add_card(nso("N4-TRASH", 4, 4000, 4, CardColor::Purple))
        .add_card(plain("X4-TRASH", 4, 4000, CardColor::Purple))
        .add_card(nso_option)
        .add_card(nso("NSO-HAND", 3, 3000, 3, CardColor::Purple))
        .add_card(dna_result)
        .add_card(plain_dna)
        .add_card(plain("X4-FIELD", 4, 4000, CardColor::Purple))
        .add_card(plain("ATK", 4, 4000, CardColor::Blue))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 6])
        .memory(5)
}

// ─── [Start of Your Main Phase] / [On Play] ──────────────────────────────────

#[test]
fn ex11_059_on_play_trashing_an_nso_card_draws_one_and_gains_one_memory() {
    let mut r = builder().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "NSO-HAND");
    push_hand(&mut r, 0, "NSO-OPTION");
    push_hand(&mut r, 0, "X4-FIELD");
    let (mem, deck) = (r.memory(), r.game.players[0].deck.len());
    r.fire_on_play(0, me.index as usize);
    let view = r.pending_selection_view().expect("the 'by trashing' cost prompt");
    assert!(view.is_optional, "'by trashing' is declinable");
    let offered = offered_ids(&r);
    assert!(offered.contains(&"NSO-HAND".to_string()) && offered.contains(&"NSO-OPTION".to_string()), "{offered:?}");
    assert!(!offered.contains(&"X4-FIELD".to_string()), "non-[NSo] cards are not legal: {offered:?}");
    pick_hand(&mut r, 0, "NSO-HAND");
    drain_declining(&mut r);
    assert!(trash_ids(&r, 0).contains(&"NSO-HAND".to_string()));
    assert_eq!(r.game.players[0].deck.len(), deck - 1, "drew 1");
    assert_eq!(r.memory(), mem + 1, "gained 1 memory");
}

#[test]
fn ex11_059_declining_the_cost_does_nothing() {
    let mut r = builder().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "NSO-HAND");
    let (mem, deck) = (r.memory(), r.game.players[0].deck.len());
    r.fire_on_play(0, me.index as usize);
    pass(&mut r);
    assert_eq!(r.memory(), mem);
    assert_eq!(r.game.players[0].deck.len(), deck);
    assert!(hand_ids(&r, 0).contains(&"NSO-HAND".to_string()));
}

#[test]
fn ex11_059_without_an_nso_card_in_hand_the_cost_is_not_offered() {
    let mut r = builder().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "X4-FIELD");
    r.fire_on_play(0, me.index as usize);
    assert!(r.pending_selection_view().is_none());
}

// ─── [All Turns] DNA digivolve (field NSo + trash NSo -> hand NSo) ────────────

fn dna_board() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = builder().start();
    let tamer = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "N4-FIELD", Some(0));
    r.place_on_field(0, "N4-FIELD-2", Some(0));
    push_trash(&mut r, 0, "N4-TRASH");
    push_trash(&mut r, 0, "X4-TRASH");
    push_hand(&mut r, 0, "NSO-DNA");
    push_hand(&mut r, 0, "PLAIN-DNA");
    (r, tamer)
}

#[test]
fn ex11_059_when_an_nso_digimon_is_deleted_suspend_to_dna_digivolve_with_a_trash_card() {
    let (mut r, tamer) = dna_board();
    let deleted = handle_of(&r, 0, "N4-FIELD-2");
    r.game.delete_permanent_with_cause(deleted, ReplacementCause::OpponentEffect);
    // The optional "by suspending this Tamer" prompt.
    let view = r.pending_selection_view().expect("optional activation");
    assert!(view.is_optional, "'by suspending' is declinable");
    r.accept_optional_trigger().expect("accept the optional trigger");
    assert!(
        r.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "this Tamer was suspended as the cost"
    );
    let offered = offered_ids(&r);
    assert!(offered.contains(&"NSO-DNA".to_string()), "{offered:?}");
    assert!(!offered.contains(&"PLAIN-DNA".to_string()), "the result must have the [NSo] trait: {offered:?}");
    pick_hand(&mut r, 0, "NSO-DNA");
    let view = r.pending_selection_view().expect("field material");
    assert_eq!(view.kind, SelectionKind::OwnField);
    pick_field(&mut r, 0, "N4-FIELD");
    let offered = offered_ids(&r);
    assert!(offered.contains(&"N4-TRASH".to_string()), "{offered:?}");
    assert!(!offered.contains(&"X4-TRASH".to_string()), "the trash material must be [NSo]: {offered:?}");
    pick_trash(&mut r, "N4-TRASH");
    drain_declining(&mut r);
    assert!(field_ids(&r, 0).contains(&"NSO-DNA".to_string()), "DNA digivolved into the hand card");
    assert!(!trash_ids(&r, 0).contains(&"N4-TRASH".to_string()), "the trash material was consumed");
}

#[test]
fn ex11_059_the_dna_offer_may_be_declined_and_then_the_tamer_stays_unsuspended() {
    let (mut r, tamer) = dna_board();
    let deleted = handle_of(&r, 0, "N4-FIELD-2");
    r.game.delete_permanent_with_cause(deleted, ReplacementCause::OpponentEffect);
    r.decline_optional_trigger().expect("decline");
    assert!(!r.game.players[0].battle_area[tamer.index as usize].is_suspended);
    assert!(!field_ids(&r, 0).contains(&"NSO-DNA".to_string()));
}

#[test]
fn ex11_059_a_non_nso_digimon_deletion_does_not_trigger_it() {
    let mut r = builder().start();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "X4-FIELD", Some(0));
    push_hand(&mut r, 0, "NSO-DNA");
    let deleted = handle_of(&r, 0, "X4-FIELD");
    r.game.delete_permanent_with_cause(deleted, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none(), "only [NSo] Digimon deletions trigger it");
}

#[test]
fn ex11_059_does_not_trigger_on_the_opponents_nso_digimon_deletion() {
    let mut r = builder().start();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "N4-FIELD", Some(0));
    push_hand(&mut r, 0, "NSO-DNA");
    let deleted = handle_of(&r, 1, "N4-FIELD");
    r.game.delete_permanent_with_cause(deleted, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none(), "'any of YOUR [NSo] Digimon'");
}

// ─── [Security] ──────────────────────────────────────────────────────────────

#[test]
fn ex11_059_security_plays_this_tamer_without_paying_the_cost() {
    let mut r = builder().start();
    let atk = r.place_on_field(1, "ATK", Some(0));
    let idx = data_idx(&r, CARD_ID);
    let next = r.game.next_card_index();
    r.game.players[0]
        .security
        .push(digimon_engine::card_source::CardSource::new(idx, 0, next));
    r.set_first_player(1);
    r.attack_player(atk, 0, false);
    drain_declining(&mut r);
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()), "played from security");
    assert!(!trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}
