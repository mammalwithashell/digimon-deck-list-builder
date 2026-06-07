//! LM-020 Quantumon — Digimon, Yellow/Green, Lv.6, cost 13, DP 13000.
//!
//! # Card text (card image / cards.json)
//!   [When Digivolving] By placing 1 Digimon on top of its owner's security stack,
//!     reveal all of your opponent's security cards, and place 1 card among them
//!     on top of your opponent's deck. Shuffle the rest and return them to the
//!     security stack.
//!   [Start of Opponent's Turn] Declare 1 card category. Then, reveal the top card
//!     of your opponent's deck. If that card is of the declared category, this
//!     Digimon isn't affected by the effects of that card category for the turn.
//!     Return the revealed card to the top or the bottom of your opponent's deck.
//!
//! These tests cover clause 2 (the [Start of Opponent's Turn] declare-category
//! conditional immunity, which underpins judge-quiz Q18).

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectSourceKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn pad(id: &str) -> digimon_engine::card_data::CardData {
    make_test_card(id, id)
}

/// A Tamer test card (for the "declared Digimon but revealed Tamer" mismatch case).
fn tamer(id: &str) -> digimon_engine::card_data::CardData {
    let mut d = make_test_card(id, id);
    d.card_kind = CardKind::Tamer;
    d.level = None;
    d.dp = None;
    d.colors = vec![CardColor::Yellow];
    d
}

/// LM-020 on player 0's field; player 1's deck top is `top_card_id`.
fn runner_with_deck_top(top_card_id: &str, top_is_tamer: bool) -> (DebugRunner, PermanentHandle) {
    let mut b = DebugRunner::builder()
        .dsl_card("LM-020")
        .expect("LM-020 Quantumon must load from the embedded DSL pack")
        .add_card(pad("PAD"));
    b = if top_is_tamer {
        b.add_card(tamer(top_card_id))
    } else {
        b.add_card(pad(top_card_id))
    };
    // deck top = Vec end, so the revealed card is the LAST id.
    let mut r = b
        .deck(1, &["PAD", "PAD", top_card_id])
        .deck(0, &["PAD", "PAD", "PAD"])
        .memory(0)
        .start();
    let quantumon = r.place_on_field(0, "LM-020", Some(0));
    (r, quantumon)
}

#[test]
fn lm_020_loads_from_dsl() {
    let r = runner_with_deck_top("DGM", false).0;
    assert!(r.compiled_card("LM-020").is_some());
}

/// Declare Digimon + reveal a Digimon ⇒ Quantumon becomes immune to Digimon
/// effects from ANY controller (own included) for the turn.
#[test]
fn lm_020_declare_digimon_matching_reveal_grants_self_inclusive_immunity() {
    let (mut r, quantumon) = runner_with_deck_top("DGM", false);

    r.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::StartOfOpponentsTurn,
        TriggerSource::Permanent(quantumon),
    );
    r.game.drain_effect_queue();

    // First choice: declare a category (0 = Digimon).
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::EffectChoice),
        "declare-category prompt"
    );
    r.execute_branch(0).expect("declare Digimon");

    // Second choice: where to return the revealed card (0 = top).
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::EffectChoice),
        "return-position prompt"
    );
    r.execute_branch(0).expect("return revealed to deck top");

    // Quantumon is now unaffected by Digimon effects from ANY controller —
    // including its OWN (controller 0). This is what blocks its own
    // <Blast Digivolve> for the turn (judge-quiz Q18).
    assert!(
        r.game
            .permanent_is_unaffected_by_effect(quantumon, 0, EffectSourceKind::Digimon),
        "immune to OWN Digimon effects"
    );
    assert!(
        r.game
            .permanent_is_unaffected_by_effect(quantumon, 1, EffectSourceKind::Digimon),
        "immune to opponent Digimon effects"
    );
    // Not a blanket immunity — Tamer/Option effects still affect it.
    assert!(
        !r.game
            .permanent_is_unaffected_by_effect(quantumon, 1, EffectSourceKind::Tamer),
        "Tamer effects are NOT covered by a Digimon declaration"
    );
}

/// Declare Digimon but the revealed top card is a Tamer ⇒ no match ⇒ NO immunity.
#[test]
fn lm_020_declare_digimon_mismatched_reveal_grants_no_immunity() {
    let (mut r, quantumon) = runner_with_deck_top("TMR", true);

    r.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::StartOfOpponentsTurn,
        TriggerSource::Permanent(quantumon),
    );
    r.game.drain_effect_queue();

    assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice));
    r.execute_branch(0).expect("declare Digimon");
    // Return-position prompt still appears (the card is returned regardless).
    assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice));
    r.execute_branch(1).expect("return revealed to deck bottom");

    assert!(
        !r.game
            .permanent_is_unaffected_by_effect(quantumon, 0, EffectSourceKind::Digimon),
        "no immunity when the revealed card's category does not match the declaration"
    );
}

/// Clause 1 [When Digivolving]: cost places 1 of your Digimon on top of your
/// security; then 1 opponent security card is sent to the top of their deck and
/// the rest are shuffled back.
#[test]
fn lm_020_when_digivolving_places_cost_and_moves_one_opponent_security_to_deck() {
    let mut r = DebugRunner::builder()
        .dsl_card("LM-020")
        .expect("LM-020 loads")
        .add_card(pad("ALLY"))
        .add_card(pad("SEC"))
        .add_card(pad("PAD"))
        // Opponent (player 1) holds 3 security cards + a deck.
        .security(1, &["SEC", "SEC", "SEC"])
        .deck(1, &["PAD", "PAD"])
        .memory(0)
        .start();
    // ALLY at slot 0 (the cost pick auto_resolve takes), Quantumon at slot 1.
    let _ally = r.place_on_field(0, "ALLY", Some(0));
    let quantumon = r.place_on_field(0, "LM-020", Some(1));

    let opp_sec_before = r.game.players[1].security.len();
    let opp_deck_before = r.game.players[1].deck.len();
    let own_sec_before = r.game.players[0].security.len();

    // Fire [When Digivolving]; auto-resolve all picks.
    r.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(quantumon),
    );
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();

    // Cost: one of your Digimon was placed on top of your security.
    assert_eq!(
        r.game.players[0].security.len(),
        own_sec_before + 1,
        "cost: 1 of your Digimon placed on top of your security stack"
    );
    // Effect: exactly 1 opponent security card moved to the top of their deck.
    assert_eq!(
        r.game.players[1].security.len(),
        opp_sec_before - 1,
        "1 opponent security card left the security stack"
    );
    assert_eq!(
        r.game.players[1].deck.len(),
        opp_deck_before + 1,
        "the removed security card was placed on top of the opponent's deck"
    );
}
