//! BT10-046 Palmon — Digimon, Lv.3, Green, DP 2000, Cost 3. Traits: Vegetation.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT10-046.md)
//!   [On Play] Reveal the top 4 cards of your deck. Add 1 card with
//!   [Vegetation] or [Plant] in one of its traits, and 1 card with [Fairy] in
//!   its traits among them to your hand. Place the rest at the bottom of your
//!   deck in any order.
//!
//! # DCGO C# reference
//!   DCGO/Assets/Scripts/CardEffect/BT10/Green/BT10_046.cs —
//!   SimplifiedRevealDeckTopCardsAndSelect(4, [HasPlantTraits, Fairy],
//!   DeckBottom, mutualConditions: true).
//!
//! # Patterns
//! - A1 reveal-N two-bucket pick (`select_reveal_buckets`, `no_duplicate_cards`)
//! - A2 remainder to deck bottom with player-chosen order (OrderedPermutation)
//! - Real play flow (`runner.play`) drives the [On Play].

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledStackPosition, CompiledStep, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::action::space::SEL_REVEAL_START;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "BT10-046";

fn digimon_with_traits(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards.iter().map(|c| c.card_id(data).to_string()).collect()
}

fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> Option<u16> {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
}

fn on_play(card: &digimon_dsl::compiled::CompiledCard) -> &CompiledTriggeredClause {
    card.effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("BT10-046 must have an [On Play] clause")
}

/// Build a runner whose P0 deck top 4 (last element = top) are `top4`, with
/// Palmon in hand and enough memory to play it.
fn runner_with_top4(extra: Vec<CardData>, top4: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT10-046 in embedded DSL pack")
        .add_card(make_test_card("BOTTOM", "Bottom"));
    for c in extra {
        b = b.add_card(c);
    }
    let mut deck = vec!["BOTTOM"];
    deck.extend_from_slice(top4);
    b.deck(0, &deck)
        .deck(1, &["BOTTOM"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start()
}

// ── Structural ───────────────────────────────────────────────────────────────

#[test]
fn bt10_046_compiles_as_green_lv3_vegetation_digimon() {
    let card = compiled(CARD_ID);
    assert_eq!(card.kind, digimon_dsl::compiled::CompiledCardKind::Digimon);
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(2000));
    assert!(card.traits.iter().any(|t| t == "Vegetation"));
}

#[test]
fn bt10_046_on_play_is_mandatory_reveal_four_two_buckets_bottom_remainder() {
    let card = compiled(CARD_ID);
    assert_eq!(card.effects.len(), 1, "only the [On Play] clause");
    let t = on_play(&card);
    assert_eq!(t.scope, CompiledScope::FaceUp);
    assert!(!t.optional, "no 'you may' — DCGO isOptional false");
    assert!(!t.once_per_turn);
    assert!(matches!(
        t.process.first(),
        Some(CompiledStep::RevealTopDeck { count: 4, .. })
    ));
    let buckets = t
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::SelectRevealBuckets {
                buckets,
                no_duplicate_cards,
                ..
            } => Some((buckets.len(), *no_duplicate_cards)),
            _ => None,
        })
        .expect("select_reveal_buckets present");
    assert_eq!(buckets, (2, true), "2 buckets, mutualConditions → no dup");
    assert_eq!(
        t.process
            .iter()
            .filter(|s| matches!(s, CompiledStep::AddToHandFromReveal { .. }))
            .count(),
        2
    );
    assert!(t.process.iter().any(|s| matches!(
        s,
        CompiledStep::PlaceRemainderOnDeck {
            position: CompiledStackPosition::Bottom,
            ..
        }
    )));
}

// ── Behavioral ───────────────────────────────────────────────────────────────

/// Positive: reveal [VEG, FAIRY, F1, F2]; pick VEG then FAIRY; the two
/// fillers return to the bottom through a player-ordered permutation.
#[test]
fn bt10_046_on_play_adds_vegetation_and_fairy_and_bottoms_rest_in_chosen_order() {
    let mut runner = runner_with_top4(
        vec![
            digimon_with_traits("VEG", &["Vegetation"]),
            digimon_with_traits("FAIRY", &["Fairy"]),
            digimon_with_traits("F1", &["Beast"]),
            digimon_with_traits("F2", &["Beast"]),
        ],
        &["F2", "F1", "FAIRY", "VEG"],
    );

    runner.play(0, 0).expect("play Palmon");

    let veg = revealed_action_for_id(&runner, "VEG").expect("VEG revealed");
    runner.execute_action(0, veg).expect("pick VEG");
    let fairy = revealed_action_for_id(&runner, "FAIRY").expect("FAIRY revealed");
    runner.execute_action(0, fairy).expect("pick FAIRY");

    // "in any order" — the remainder ordering is a real player selection.
    let view = runner
        .pending_selection_view()
        .expect("remainder ordering must surface as a selection");
    assert_eq!(
        view.kind,
        SelectionKind::OrderedPermutation { remaining: 2 },
        "bottom-in-any-order must be an OrderedPermutation over the 2 leftovers"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both leftover cards are selectable (player-chosen order)"
    );
    let _ = runner.auto_resolve();

    let hand = ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand.contains(&"VEG".to_string()), "hand={hand:?}");
    assert!(hand.contains(&"FAIRY".to_string()), "hand={hand:?}");
    assert!(!hand.iter().any(|h| h == "F1" || h == "F2"));
    let deck = ids(&runner.game.players[0].deck, &runner.game.card_data);
    assert_eq!(deck.len(), 3, "BOTTOM + 2 fillers; deck={deck:?}");
    // deck[0] is the bottom; both fillers sit beneath the original BOTTOM.
    assert!(
        deck[..2].contains(&"F1".to_string()) && deck[..2].contains(&"F2".to_string()),
        "fillers placed at the deck bottom; deck={deck:?}"
    );
    assert_eq!(deck[2], "BOTTOM");
    assert!(runner.game.revealed_cards.is_empty());
}

/// Positive: a compound [Carnivorous Plant] trait satisfies bucket 1
/// ("[Plant] in one of its traits").
#[test]
fn bt10_046_compound_plant_trait_satisfies_first_bucket() {
    let mut runner = runner_with_top4(
        vec![
            digimon_with_traits("PLANT", &["Carnivorous Plant"]),
            digimon_with_traits("F1", &["Beast"]),
            digimon_with_traits("F2", &["Beast"]),
            digimon_with_traits("F3", &["Beast"]),
        ],
        &["F3", "F2", "F1", "PLANT"],
    );
    runner.play(0, 0).expect("play Palmon");
    let plant = revealed_action_for_id(&runner, "PLANT").expect("PLANT offered");
    runner.execute_action(0, plant).expect("pick PLANT");
    let _ = runner.auto_resolve();
    let hand = ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand.contains(&"PLANT".to_string()), "hand={hand:?}");
    assert_eq!(runner.game.players[0].deck.len(), 4, "3 fillers + BOTTOM");
}

/// No-duplicate: a single card with BOTH Vegetation and Fairy can fill only
/// one bucket — it is not added twice and the other bucket has no candidate.
#[test]
fn bt10_046_dual_trait_card_fills_only_one_bucket() {
    let mut runner = runner_with_top4(
        vec![
            digimon_with_traits("BOTH", &["Vegetation", "Fairy"]),
            digimon_with_traits("F1", &["Beast"]),
            digimon_with_traits("F2", &["Beast"]),
            digimon_with_traits("F3", &["Beast"]),
        ],
        &["F3", "F2", "F1", "BOTH"],
    );
    let hand_before = runner.hand_size(0);
    runner.play(0, 0).expect("play Palmon");
    let both = revealed_action_for_id(&runner, "BOTH").expect("BOTH offered");
    runner.execute_action(0, both).expect("pick BOTH");
    // Bucket 2 must not re-offer BOTH (mutualConditions).
    if let Some(v) = runner.pending_selection_view() {
        assert!(
            !matches!(v.kind, SelectionKind::RevealBucket { .. }),
            "no second-bucket prompt may offer the already-picked card; got {:?}",
            v.kind
        );
    }
    let _ = runner.auto_resolve();
    // Palmon left hand (-1), BOTH entered (+1).
    assert_eq!(runner.hand_size(0), hand_before);
    let hand = ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert_eq!(hand.iter().filter(|h| *h == "BOTH").count(), 1);
    assert_eq!(runner.game.players[0].deck.len(), 4);
}

/// Player choice: with a Vegetation+Fairy card and a Vegetation-only card,
/// the player may route the dual card to the Fairy bucket and take both.
#[test]
fn bt10_046_dual_trait_card_can_be_routed_to_fairy_bucket() {
    let mut runner = runner_with_top4(
        vec![
            digimon_with_traits("BOTH", &["Vegetation", "Fairy"]),
            digimon_with_traits("VEG", &["Vegetation"]),
            digimon_with_traits("F1", &["Beast"]),
            digimon_with_traits("F2", &["Beast"]),
        ],
        &["F2", "F1", "VEG", "BOTH"],
    );
    runner.play(0, 0).expect("play Palmon");
    let veg = revealed_action_for_id(&runner, "VEG").expect("VEG offered");
    runner.execute_action(0, veg).expect("VEG into bucket 1");
    let both = revealed_action_for_id(&runner, "BOTH").expect("BOTH offered for Fairy");
    runner.execute_action(0, both).expect("BOTH into bucket 2");
    let _ = runner.auto_resolve();
    let hand = ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand.contains(&"VEG".to_string()) && hand.contains(&"BOTH".to_string()));
}

/// Negative: no matching traits revealed → nothing added, all 4 to bottom.
#[test]
fn bt10_046_no_match_adds_nothing_and_bottoms_all_four() {
    let mut runner = runner_with_top4(
        vec![
            digimon_with_traits("F1", &["Beast"]),
            digimon_with_traits("F2", &["Beast"]),
            digimon_with_traits("F3", &["Avian"]),
            digimon_with_traits("F4", &["Dragon"]),
        ],
        &["F4", "F3", "F2", "F1"],
    );
    let hand_before = runner.hand_size(0);
    runner.play(0, 0).expect("play Palmon");
    let _ = runner.auto_resolve();
    assert_eq!(runner.hand_size(0), hand_before - 1, "only Palmon left hand");
    let deck = ids(&runner.game.players[0].deck, &runner.game.card_data);
    assert_eq!(deck.len(), 5);
    assert_eq!(deck[4], "BOTTOM", "revealed cards went under BOTTOM; deck={deck:?}");
    assert!(runner.game.revealed_cards.is_empty());
}
