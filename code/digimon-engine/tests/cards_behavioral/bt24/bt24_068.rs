//! BT24-068 DemiDevimon: card image, official bundle and DCGO BT24_068.cs.
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Evil] or
//! [Fallen Angel] trait and 1 card with the [Seven Great Demon Lords] trait
//! among them to the hand. Return the rest to the bottom of the deck. Then,
//! trash 1 card in your hand.
//! Inherited: [When Attacking] [Once Per Turn] Trash the top card of both
//! players' decks.
//!
//! Every board here is synthetic; no recorded game data is used.

use digimon_engine::action::{
    build_action_mask,
    space::{PASS, PLAY_HAND_START, SEL_REVEAL_START},
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[path = "../../support/owned_card_choice.rs"]
mod physical_menu;

const ID: &str = "BT24-068";

fn with_traits(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT24-068 must be implemented")
        .add_card(with_traits("EVIL", &["Evil"]))
        .add_card(with_traits("ANGEL", &["Fallen Angel"]))
        .add_card(with_traits("LORD", &["Seven Great Demon Lords"]))
        .add_card(with_traits(
            "EVIL-LORD",
            &["Evil", "Seven Great Demon Lords"],
        ))
        .add_card(with_traits("ANGEL-LORD", &["Fallen Angel", "Seven Great Demon Lords"]))
        .add_card(with_traits("DEMON", &["Demon Lord"]))
        .add_card(make_test_card("PLAIN", "Plain"))
        .add_card(make_test_card("HAND-A", "Hand A"))
        .add_card(make_test_card("HAND-B", "Hand B"))
        .add_card(make_test_card("TAIL", "Unrevealed card"))
        .deck(1, &["TAIL"; 8])
        .memory(10)
}

fn zone_ids(cards: &[CardSource], runner: &DebugRunner) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn reveal_action(runner: &DebugRunner, id: &str) -> u16 {
    SEL_REVEAL_START
        + runner
            .game
            .revealed_cards
            .iter()
            .position(|card| card.card_id(&runner.game.card_data) == id)
            .expect("revealed card") as u16
}

fn assert_prompt(runner: &DebugRunner, expected: &[u16]) {
    let view = runner
        .pending_selection_view()
        .expect("player-visible choice");
    assert_eq!(view.selecting_player, 0);
    assert!(!view.is_optional, "printed adds and ordering are mandatory");
    let mut actual = view.valid_action_ids.clone();
    actual.sort_unstable();
    let mut expected = expected.to_vec();
    expected.sort_unstable();
    assert_eq!(actual, expected);
    assert_eq!(build_action_mask(&runner.game, 0)[PASS as usize], 0.0);
}

fn assert_bucket(runner: &DebugRunner, bucket: u8, candidates: &[&str]) {
    assert!(
        matches!(runner.pending_kind(), Some(SelectionKind::RevealBucket { bucket_index, .. }) if bucket_index == bucket),
        "expected bucket {bucket}, got {:?}",
        runner.pending_kind()
    );
    let actions: Vec<_> = candidates
        .iter()
        .map(|id| reveal_action(runner, id))
        .collect();
    assert_prompt(runner, &actions);
}

fn pick(runner: &mut DebugRunner, id: &str) {
    let action = reveal_action(runner, id);
    assert_eq!(build_action_mask(&runner.game, 0)[action as usize], 1.0);
    runner
        .execute_action(0, action)
        .expect("choose revealed card");
}

/// Resolve the remainder ordering; stops at the first non-ordering prompt.
fn finish_order(runner: &mut DebugRunner, reverse: bool) {
    while let Some(view) = runner.pending_selection_view() {
        let SelectionKind::OrderedPermutation { remaining } = view.kind else {
            return;
        };
        let actions: Vec<_> = (0..remaining)
            .map(|i| SEL_REVEAL_START + i as u16)
            .collect();
        assert_prompt(runner, &actions);
        let action = if reverse {
            *actions.last().unwrap()
        } else {
            actions[0]
        };
        runner
            .execute_action(0, action)
            .expect("choose remainder order");
    }
}

fn pick_hand(runner: &mut DebugRunner, card_id: &str) {
    let idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in hand"));
    let action = if runner
        .game
        .pending_selection
        .as_ref()
        .is_some_and(|pending| pending.card_choice.is_some())
    {
        physical_menu::zone_action(
            &runner.game,
            0,
            idx,
            digimon_engine::owned_card_choice::CardChoiceZone::Hand,
        )
    } else {
        PLAY_HAND_START + idx as u16
    };
    let view = runner.pending_selection_view().expect("hand pick pending");
    assert!(
        view.valid_action_ids.contains(&action),
        "{card_id} is not a legal hand pick (valid {:?})",
        view.valid_action_ids
    );
    runner.execute_action(0, action).expect("trash from hand");
}

fn play(hand_extra: &[&str], deck: &[&str]) -> DebugRunner {
    let mut hand = vec![ID];
    hand.extend_from_slice(hand_extra);
    let mut runner = base().hand(0, &hand).deck(0, deck).start();
    runner.skip_mulligan();
    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        1.0
    );
    runner.game.decode_action(PLAY_HAND_START, 0);
    assert_eq!(runner.memory(), 7);
    runner
}

#[test]
fn bt24_068_adds_one_evil_or_fallen_angel_and_one_demon_lord_then_trashes_one_from_hand() {
    // Top 3 revealed: LORD, ANGEL, EVIL; TAIL stays unrevealed.
    let mut runner = play(&["HAND-A"], &["TAIL", "EVIL", "ANGEL", "LORD"]);
    assert_bucket(&runner, 0, &["EVIL", "ANGEL"]);
    pick(&mut runner, "ANGEL");
    assert_bucket(&runner, 1, &["LORD"]);
    pick(&mut runner, "LORD");
    finish_order(&mut runner, false);

    // The hand trash is mandatory and offers the added cards too.
    let view = runner.pending_selection_view().expect("hand trash prompt");
    assert!(!view.is_optional, "trash 1 card in your hand is mandatory");
    assert_eq!(view.valid_action_ids.len(), 3);
    assert_eq!(build_action_mask(&runner.game, 0)[PASS as usize], 0.0);
    pick_hand(&mut runner, "ANGEL");

    assert!(runner.pending_selection().is_none());
    assert_eq!(
        zone_ids(&runner.game.players[0].hand, &runner),
        ["HAND-A", "LORD"]
    );
    assert_eq!(zone_ids(&runner.game.players[0].trash, &runner), ["ANGEL"]);
    assert_eq!(zone_ids(&runner.game.players[0].deck, &runner), ["EVIL", "TAIL"]);
}

#[test]
fn bt24_068_card_just_added_may_be_the_one_trashed() {
    let mut runner = play(&[], &["TAIL", "PLAIN", "EVIL", "LORD"]);
    pick(&mut runner, "EVIL");
    pick(&mut runner, "LORD");
    finish_order(&mut runner, false);
    pick_hand(&mut runner, "LORD");
    assert_eq!(zone_ids(&runner.game.players[0].hand, &runner), ["EVIL"]);
    assert_eq!(zone_ids(&runner.game.players[0].trash, &runner), ["LORD"]);
}

#[test]
fn bt24_068_each_trait_bucket_offers_exactly_the_printed_traits() {
    // [Evil] and [Fallen Angel] both qualify for the first slot; a lookalike
    // "Demon Lord" trait and an unrelated card qualify for neither slot.
    let mut runner = play(&[], &["TAIL", "DEMON", "PLAIN", "EVIL"]);
    assert_bucket(&runner, 0, &["EVIL"]);
    pick(&mut runner, "EVIL");
    finish_order(&mut runner, false);
    pick_hand(&mut runner, "EVIL");

    let mut runner = play(&[], &["TAIL", "ANGEL", "DEMON", "LORD"]);
    assert_bucket(&runner, 0, &["ANGEL"]);
    pick(&mut runner, "ANGEL");
    assert_bucket(&runner, 1, &["LORD"]);
}

#[test]
fn bt24_068_one_dual_trait_card_cannot_fill_both_slots() {
    let mut runner = play(&[], &["TAIL", "PLAIN", "ANGEL", "EVIL-LORD"]);
    assert_bucket(&runner, 0, &["ANGEL", "EVIL-LORD"]);
    pick(&mut runner, "EVIL-LORD");
    // It cannot also be the second pick; nothing else qualifies, so the
    // search ends after one card.
    finish_order(&mut runner, false);
    assert_eq!(zone_ids(&runner.game.players[0].hand, &runner), ["EVIL-LORD"]);
    pick_hand(&mut runner, "EVIL-LORD");
    assert_eq!(runner.hand_size(0), 0);
    assert_eq!(runner.deck_size(0), 3);
}

#[test]
fn bt24_068_dual_trait_card_may_take_the_demon_lord_slot_when_another_card_fills_the_first() {
    let mut runner = play(&[], &["TAIL", "EVIL", "ANGEL-LORD", "PLAIN"]);
    assert_bucket(&runner, 0, &["EVIL", "ANGEL-LORD"]);
    pick(&mut runner, "EVIL");
    assert_bucket(&runner, 1, &["ANGEL-LORD"]);
    pick(&mut runner, "ANGEL-LORD");
    finish_order(&mut runner, false);
    assert_eq!(
        zone_ids(&runner.game.players[0].hand, &runner),
        ["EVIL", "ANGEL-LORD"]
    );
}

#[test]
fn bt24_068_a_missing_category_does_not_stop_the_other_pick() {
    let mut runner = play(&[], &["TAIL", "PLAIN", "PLAIN", "LORD"]);
    assert_bucket(&runner, 1, &["LORD"]);
    pick(&mut runner, "LORD");
    finish_order(&mut runner, false);
    pick_hand(&mut runner, "LORD");
    assert_eq!(runner.hand_size(0), 0);
    assert_eq!(runner.deck_size(0), 3);
}

#[test]
fn bt24_068_no_match_orders_the_remainder_then_still_trashes_from_hand() {
    for reverse in [false, true] {
        let mut runner = play(&["HAND-A", "HAND-B"], &["TAIL", "PLAIN", "DEMON", "PLAIN"]);
        let mut expected = zone_ids(&runner.game.revealed_cards, &runner);
        assert_eq!(expected.len(), 3);
        if reverse {
            expected.reverse();
        }
        expected.reverse(); // deck stores the bottom first
        expected.push("TAIL".into());
        finish_order(&mut runner, reverse);
        assert_eq!(zone_ids(&runner.game.players[0].deck, &runner), expected);
        // Nothing was added, but the hand trash still happens.
        pick_hand(&mut runner, "HAND-B");
        assert_eq!(zone_ids(&runner.game.players[0].hand, &runner), ["HAND-A"]);
        assert_eq!(zone_ids(&runner.game.players[0].trash, &runner), ["HAND-B"]);
    }
}

#[test]
fn bt24_068_empty_hand_after_the_search_trashes_nothing() {
    // The played card left the hand, nothing is added and the hand is empty.
    let mut runner = play(&[], &["TAIL", "PLAIN", "PLAIN", "PLAIN"]);
    finish_order(&mut runner, false);
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.hand_size(0), 0);
    assert!(runner.game.players[0].trash.is_empty());
}

#[test]
fn bt24_068_short_and_empty_decks_resolve_without_inventing_cards() {
    let mut runner = play(&["HAND-A"], &["EVIL", "LORD"]);
    assert_bucket(&runner, 0, &["EVIL"]);
    pick(&mut runner, "EVIL");
    assert_bucket(&runner, 1, &["LORD"]);
    pick(&mut runner, "LORD");
    finish_order(&mut runner, false);
    pick_hand(&mut runner, "HAND-A");
    assert_eq!(zone_ids(&runner.game.players[0].hand, &runner), ["EVIL", "LORD"]);
    assert_eq!(runner.deck_size(0), 0);

    // Nothing to reveal: the hand trash is still mandatory.
    let mut runner = play(&["HAND-A"], &[]);
    pick_hand(&mut runner, "HAND-A");
    assert_eq!(runner.hand_size(0), 0);
}

#[test]
fn bt24_068_inherited_when_attacking_trashes_both_decks_once_per_turn() {
    let mut runner = base()
        .add_card(make_test_card("CARRIER", "Carrier"))
        .deck(0, &["PLAIN", "EVIL", "LORD", "TAIL"])
        .deck(1, &["PLAIN", "EVIL", "LORD", "TAIL"])
        .start();
    let carrier = runner.place_stack(0, &[ID, "CARRIER"]);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();
    assert!(runner.pending_selection().is_none(), "no choices to make");
    for player in [0, 1] {
        assert_eq!(
            zone_ids(&runner.game.players[player].trash, &runner),
            ["TAIL"],
            "player {player} trashes the top card of their deck"
        );
        assert_eq!(runner.deck_size(player as u8), 3);
    }

    // Once per turn: a second attack this turn does not trash again.
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();
    for player in [0, 1] {
        assert_eq!(runner.game.players[player].trash.len(), 1);
        assert_eq!(runner.deck_size(player as u8), 3);
    }
}

#[test]
fn bt24_068_attack_effect_is_inherited_only() {
    // As the top card the printed [When Attacking] is an inherited effect and
    // does not apply to the Digimon itself.
    let mut runner = base().deck(0, &["PLAIN", "TAIL"]).deck(1, &["PLAIN", "TAIL"]).start();
    let face = runner.place_on_field(0, ID, Some(0));
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(face));
    runner.game.drain_effect_queue();
    assert!(runner.game.players[0].trash.is_empty());
    assert!(runner.game.players[1].trash.is_empty());
}
