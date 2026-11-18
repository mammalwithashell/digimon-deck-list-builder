//! RB1-005 Gammamon: card image, official bundle and DCGO RB1_005.cs.
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 card with [Gammamon]
//! in its text and 1 [Hiro Amanokawa] among them to the hand. Place the rest at
//! the bottom of the deck in any order.
//! Inherited: [Your Turn] While this Digimon has [Gammamon] in its text, it
//! gets +2000 DP (official Q&A: only the TOP card of the stack is read).
//!
//! Every board here is synthetic; no recorded game data is used.

use digimon_engine::action::{
    build_action_mask,
    space::{PASS, PLAY_HAND_START, SEL_REVEAL_START},
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardKind;
use digimon_engine::selection::SelectionKind;

const ID: &str = "RB1-005";

fn gammamon_in_name(id: &str) -> CardData {
    make_test_card(id, "Gammamon Variant")
}

fn gammamon_in_effect_text(id: &str) -> CardData {
    let mut card = make_test_card(id, "Some Other Digimon");
    card.effect_text = "[On Play] If you have [Gammamon], gain 1 memory.".into();
    card
}

fn gammamon_in_traits(id: &str) -> CardData {
    let mut card = make_test_card(id, "Trait Carrier");
    card.traits = vec!["Gammamon".into()];
    card
}

fn hiro(id: &str, name: &str, effect_text: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.effect_text = effect_text.into();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("RB1-005 must be implemented")
        .add_card(gammamon_in_name("GAM-NAME"))
        .add_card(gammamon_in_effect_text("GAM-TEXT"))
        .add_card(gammamon_in_traits("GAM-TRAIT"))
        .add_card(hiro("HIRO", "Hiro Amanokawa", ""))
        .add_card(hiro("HIRO-GAM", "Hiro Amanokawa", "Mentions [Gammamon]."))
        .add_card(hiro("HIRO-JR", "Hiro Amanokawa Jr", ""))
        .add_card(make_test_card("FILLER", "Filler"))
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

fn finish_order(runner: &mut DebugRunner, reverse: bool) {
    while let Some(view) = runner.pending_selection_view() {
        let SelectionKind::OrderedPermutation { remaining } = view.kind else {
            panic!("only remainder ordering expected, got {:?}", view.kind);
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
    assert!(runner.game.revealed_cards.is_empty());
}

fn play(deck: &[&str]) -> DebugRunner {
    let mut runner = base().hand(0, &[ID]).deck(0, deck).start();
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
fn rb1_005_reveals_three_and_adds_one_gammamon_text_card_and_one_hiro() {
    // Deck is listed bottom-first: TAIL stays unrevealed; the top 3 are
    // HIRO, GAM-NAME, FILLER.
    let mut runner = play(&["TAIL", "FILLER", "GAM-NAME", "HIRO"]);
    assert_eq!(runner.game.revealed_cards.len(), 3);
    assert_bucket(&runner, 0, &["GAM-NAME"]);
    pick(&mut runner, "GAM-NAME");
    assert_bucket(&runner, 1, &["HIRO"]);
    pick(&mut runner, "HIRO");
    finish_order(&mut runner, false);
    assert_eq!(
        zone_ids(&runner.game.players[0].hand, &runner),
        ["GAM-NAME", "HIRO"]
    );
    assert_eq!(zone_ids(&runner.game.players[0].deck, &runner), ["FILLER", "TAIL"]);
}

#[test]
fn rb1_005_gammamon_in_text_matches_name_effect_text_and_traits() {
    // "[Gammamon] in its text" is the broad whole-card scan: a name, an
    // effect-text mention or a trait each qualify and are offered as separate
    // choices.
    let mut runner = play(&["TAIL", "GAM-TRAIT", "GAM-TEXT", "GAM-NAME"]);
    assert_bucket(&runner, 0, &["GAM-NAME", "GAM-TEXT", "GAM-TRAIT"]);
    pick(&mut runner, "GAM-TEXT");
    // No Hiro revealed: no second prompt, only the remainder ordering.
    finish_order(&mut runner, false);
    assert_eq!(zone_ids(&runner.game.players[0].hand, &runner), ["GAM-TEXT"]);
    assert_eq!(runner.deck_size(0), 3);
}

#[test]
fn rb1_005_lone_hiro_is_still_added_when_no_gammamon_card_is_revealed() {
    let mut runner = play(&["TAIL", "FILLER", "FILLER", "HIRO"]);
    assert_bucket(&runner, 1, &["HIRO"]);
    pick(&mut runner, "HIRO");
    finish_order(&mut runner, false);
    assert_eq!(zone_ids(&runner.game.players[0].hand, &runner), ["HIRO"]);
    assert_eq!(runner.deck_size(0), 3);
}

#[test]
fn rb1_005_hiro_must_match_the_name_exactly() {
    let mut runner = play(&["TAIL", "FILLER", "HIRO-JR", "GAM-NAME"]);
    assert_bucket(&runner, 0, &["GAM-NAME"]);
    pick(&mut runner, "GAM-NAME");
    finish_order(&mut runner, false);
    assert_eq!(zone_ids(&runner.game.players[0].hand, &runner), ["GAM-NAME"]);
    assert_eq!(runner.deck_size(0), 3);
}

#[test]
fn rb1_005_one_hiro_with_gammamon_text_cannot_fill_both_slots() {
    let mut runner = play(&["TAIL", "FILLER", "HIRO", "HIRO-GAM"]);
    assert_bucket(&runner, 0, &["HIRO-GAM"]);
    pick(&mut runner, "HIRO-GAM");
    // The Gammamon-text Hiro is gone from the pool; the plain Hiro fills the
    // second slot.
    assert_bucket(&runner, 1, &["HIRO"]);
    pick(&mut runner, "HIRO");
    finish_order(&mut runner, false);
    assert_eq!(
        zone_ids(&runner.game.players[0].hand, &runner),
        ["HIRO-GAM", "HIRO"]
    );

    // A lone Gammamon-text Hiro fills one slot only.
    let mut runner = play(&["TAIL", "FILLER", "FILLER", "HIRO-GAM"]);
    assert_bucket(&runner, 0, &["HIRO-GAM"]);
    pick(&mut runner, "HIRO-GAM");
    finish_order(&mut runner, false);
    assert_eq!(zone_ids(&runner.game.players[0].hand, &runner), ["HIRO-GAM"]);
    assert_eq!(runner.deck_size(0), 3);
}

#[test]
fn rb1_005_hiro_with_gammamon_text_can_be_taken_for_the_hiro_slot() {
    let mut runner = play(&["TAIL", "GAM-NAME", "HIRO", "HIRO-GAM"]);
    assert_bucket(&runner, 0, &["GAM-NAME", "HIRO-GAM"]);
    pick(&mut runner, "GAM-NAME");
    assert_bucket(&runner, 1, &["HIRO", "HIRO-GAM"]);
    pick(&mut runner, "HIRO-GAM");
    finish_order(&mut runner, false);
    assert_eq!(
        zone_ids(&runner.game.players[0].hand, &runner),
        ["GAM-NAME", "HIRO-GAM"]
    );
}

#[test]
fn rb1_005_remainder_order_is_a_player_choice_and_goes_under_the_unrevealed_cards() {
    for reverse in [false, true] {
        let mut runner = play(&["TAIL", "FILLER", "GAM-NAME", "HIRO"]);
        pick(&mut runner, "GAM-NAME");
        pick(&mut runner, "HIRO");
        // Only one card remains, so there is exactly one order; use a
        // no-match reveal to see a real permutation.
        finish_order(&mut runner, reverse);
        assert_eq!(runner.deck_size(0), 2);
        assert_eq!(runner.hand_size(0), 2);
    }
    for reverse in [false, true] {
        let mut runner = play(&["TAIL", "FILLER", "FILLER", "FILLER"]);
        let mut expected = zone_ids(&runner.game.revealed_cards, &runner);
        assert_eq!(expected.len(), 3);
        if reverse {
            expected.reverse();
        }
        expected.reverse();
        expected.push("TAIL".into());
        finish_order(&mut runner, reverse);
        assert_eq!(zone_ids(&runner.game.players[0].deck, &runner), expected);
        assert_eq!(runner.hand_size(0), 0);
    }
}

#[test]
fn rb1_005_short_and_empty_decks_resolve_without_inventing_cards() {
    let mut runner = play(&["GAM-NAME", "HIRO"]);
    assert_bucket(&runner, 0, &["GAM-NAME"]);
    pick(&mut runner, "GAM-NAME");
    assert_bucket(&runner, 1, &["HIRO"]);
    pick(&mut runner, "HIRO");
    finish_order(&mut runner, false);
    assert_eq!(runner.hand_size(0), 2);
    assert_eq!(runner.deck_size(0), 0);

    let empty = play(&[]);
    assert!(empty.pending_selection().is_none());
    assert_eq!(empty.hand_size(0), 0);
}

#[test]
fn rb1_005_inherited_dp_reads_the_top_card_text_on_the_owners_turn_only() {
    let mut runner = base().deck(0, &["TAIL"; 8]).start();
    runner.skip_mulligan();
    // RB1-005 as a digivolution card under a [Gammamon]-text top card.
    let under_gammamon = runner.place_stack(0, &[ID, "GAM-NAME"]);
    let under_effect_text = runner.place_stack(0, &[ID, "GAM-TEXT"]);
    let under_trait = runner.place_stack(0, &[ID, "GAM-TRAIT"]);
    // ... under a top card with no Gammamon text at all.
    let under_filler = runner.place_stack(0, &[ID, "FILLER"]);
    // RB1-005 itself as the top card: it is not an inherited effect there and
    // its own text is never read (official Q&A).
    let face = runner.place_on_field(0, ID, Some(0));
    // An opponent's copy of the same stack follows the owner-turn rule from
    // its own controller's perspective.
    let opponent_host = runner.place_stack(1, &[ID, "GAM-NAME"]);

    for (turn, own, opp) in [(0, 4000, 2000), (1, 2000, 4000)] {
        runner.set_first_player(turn);
        assert_eq!(runner.effective_dp(under_gammamon), Some(own), "turn {turn}");
        assert_eq!(runner.effective_dp(under_effect_text), Some(own), "turn {turn}");
        assert_eq!(runner.effective_dp(under_trait), Some(own), "turn {turn}");
        assert_eq!(runner.effective_dp(under_filler), Some(2000), "turn {turn}");
        assert_eq!(runner.effective_dp(face), Some(1000), "turn {turn}");
        assert_eq!(runner.effective_dp(opponent_host), Some(opp), "turn {turn}");
    }
}
