//! BT7-081 Bokomon: card image, official bundle and DCGO BT7_081.cs.
//!
//! [On Play] Reveal the top 5 cards of your deck. Add 1 card with [Hybrid] or
//! [Ten Warriors] in its traits and 1 Tamer card among them to your hand.
//! Place the remaining cards at the bottom of your deck in any order.
//! [Your Turn][Once Per Turn] When one of your Tamers digivolves, gain 2 memory.
//! Official Q&A: a lone matching card is still added.
//!
//! Every board here is synthetic; no recorded game data is used.

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::action::{
    build_action_mask,
    space::{PASS, PLAY_HAND_START, SEL_REVEAL_START},
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, CostDelta, PlaySource};
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT7-081";

fn with_traits(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn tamer(id: &str, traits: &[&str]) -> CardData {
    let mut card = with_traits(id, traits);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT7-081 must be implemented")
        .add_card(with_traits("HYBRID", &["Hybrid"]))
        .add_card(with_traits("WARRIOR", &["Ten Warriors"]))
        .add_card(with_traits("SPACELESS", &["TenWarriors"]))
        .add_card(tamer("TAMER", &[]))
        .add_card(tamer("TAMER-2", &[]))
        .add_card(tamer("HYBRID-TAMER", &["Hybrid"]))
        .add_card(make_test_card("PLAIN", "Plain"))
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
fn bt7_081_reveals_five_and_adds_one_hybrid_or_ten_warriors_card_and_one_tamer() {
    // Deck is bottom-first: TAIL stays unrevealed; the top 5 are
    // TAMER, HYBRID, WARRIOR, PLAIN, PLAIN.
    let mut runner = play(&["TAIL", "PLAIN", "PLAIN", "WARRIOR", "HYBRID", "TAMER"]);
    assert_eq!(runner.game.revealed_cards.len(), 5);
    assert_bucket(&runner, 0, &["HYBRID", "WARRIOR"]);
    pick(&mut runner, "WARRIOR");
    assert_bucket(&runner, 1, &["TAMER"]);
    pick(&mut runner, "TAMER");
    finish_order(&mut runner, false);
    assert_eq!(
        zone_ids(&runner.game.players[0].hand, &runner),
        ["WARRIOR", "TAMER"]
    );
    assert_eq!(runner.deck_size(0), 4);
    assert_eq!(
        zone_ids(&runner.game.players[0].deck, &runner).last().map(String::as_str),
        Some("TAIL"),
        "the unrevealed card stays on top of the returned cards"
    );
}

#[test]
fn bt7_081_a_lone_matching_card_is_still_added() {
    // Official Q&A: only a [Hybrid]/[Ten Warriors] card revealed.
    let mut runner = play(&["TAIL", "PLAIN", "PLAIN", "PLAIN", "PLAIN", "HYBRID"]);
    assert_bucket(&runner, 0, &["HYBRID"]);
    pick(&mut runner, "HYBRID");
    finish_order(&mut runner, false);
    assert_eq!(zone_ids(&runner.game.players[0].hand, &runner), ["HYBRID"]);
    assert_eq!(runner.deck_size(0), 5);

    // Only a Tamer revealed.
    let mut runner = play(&["TAIL", "PLAIN", "PLAIN", "PLAIN", "PLAIN", "TAMER"]);
    assert_bucket(&runner, 1, &["TAMER"]);
    pick(&mut runner, "TAMER");
    finish_order(&mut runner, false);
    assert_eq!(zone_ids(&runner.game.players[0].hand, &runner), ["TAMER"]);
    assert_eq!(runner.deck_size(0), 5);
}

#[test]
fn bt7_081_trait_filter_is_hybrid_or_ten_warriors_only() {
    let mut runner = play(&["TAIL", "PLAIN", "TAMER", "TAMER-2", "SPACELESS", "PLAIN"]);
    // [TenWarriors] without the space is not the printed trait.
    assert_bucket(&runner, 1, &["TAMER", "TAMER-2"]);
    pick(&mut runner, "TAMER-2");
    finish_order(&mut runner, false);
    assert_eq!(zone_ids(&runner.game.players[0].hand, &runner), ["TAMER-2"]);
}

#[test]
fn bt7_081_a_hybrid_tamer_fills_one_slot_only() {
    let mut runner = play(&["TAIL", "PLAIN", "PLAIN", "PLAIN", "TAMER", "HYBRID-TAMER"]);
    assert_bucket(&runner, 0, &["HYBRID-TAMER"]);
    pick(&mut runner, "HYBRID-TAMER");
    assert_bucket(&runner, 1, &["TAMER"]);
    pick(&mut runner, "TAMER");
    finish_order(&mut runner, false);
    assert_eq!(
        zone_ids(&runner.game.players[0].hand, &runner),
        ["HYBRID-TAMER", "TAMER"]
    );

    // Alone, it fills the first slot and is not offered again.
    let mut runner = play(&["TAIL", "PLAIN", "PLAIN", "PLAIN", "PLAIN", "HYBRID-TAMER"]);
    assert_bucket(&runner, 0, &["HYBRID-TAMER"]);
    pick(&mut runner, "HYBRID-TAMER");
    finish_order(&mut runner, false);
    assert_eq!(zone_ids(&runner.game.players[0].hand, &runner), ["HYBRID-TAMER"]);

    // With a plain Hybrid card present the Tamer may take the Tamer slot.
    let mut runner = play(&["TAIL", "PLAIN", "PLAIN", "PLAIN", "HYBRID", "HYBRID-TAMER"]);
    assert_bucket(&runner, 0, &["HYBRID", "HYBRID-TAMER"]);
    pick(&mut runner, "HYBRID");
    assert_bucket(&runner, 1, &["HYBRID-TAMER"]);
    pick(&mut runner, "HYBRID-TAMER");
    finish_order(&mut runner, false);
    assert_eq!(
        zone_ids(&runner.game.players[0].hand, &runner),
        ["HYBRID", "HYBRID-TAMER"]
    );
}

#[test]
fn bt7_081_remainder_order_is_a_player_choice_under_the_unrevealed_cards() {
    for reverse in [false, true] {
        let mut runner = play(&["TAIL", "PLAIN", "PLAIN", "PLAIN", "PLAIN", "PLAIN"]);
        let mut expected = zone_ids(&runner.game.revealed_cards, &runner);
        assert_eq!(expected.len(), 5);
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
fn bt7_081_short_and_empty_decks_resolve_without_inventing_cards() {
    let mut runner = play(&["HYBRID", "TAMER"]);
    assert_bucket(&runner, 0, &["HYBRID"]);
    pick(&mut runner, "HYBRID");
    assert_bucket(&runner, 1, &["TAMER"]);
    pick(&mut runner, "TAMER");
    finish_order(&mut runner, false);
    assert_eq!(runner.hand_size(0), 2);
    assert_eq!(runner.deck_size(0), 0);

    let empty = play(&[]);
    assert!(empty.pending_selection().is_none());
    assert_eq!(empty.hand_size(0), 0);
}

// ---- [Your Turn][Once Per Turn] When one of your Tamers digivolves ----------

/// Bokomon and a Tamer on the field; the controller's hand holds Digimon
/// cards the Tamer can be digivolved into by an effect.
fn digivolve_runner() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut runner = base()
        .add_card(make_test_card("EVO-A", "Evolution A"))
        .add_card(make_test_card("EVO-B", "Evolution B"))
        .hand(0, &["EVO-A", "EVO-B"])
        .hand(1, &["EVO-A"])
        .deck(0, &["TAIL"; 8])
        .memory(3)
        .start();
    runner.skip_mulligan();
    runner.set_first_player(0);
    runner.place_on_field(0, ID, Some(0));
    let tamer = runner.place_on_field(0, "TAMER", Some(0));
    (runner, tamer)
}

fn effect_digivolve(
    runner: &mut DebugRunner,
    player: u8,
    target: digimon_engine::permanent::PermanentHandle,
) -> bool {
    runner.game.effect_initiated_digivolve_ignore_requirements(
        player,
        0,
        target,
        CostDelta::Free,
        PlaySource::ByEffect,
    )
}

#[test]
fn bt7_081_tamer_digivolving_on_your_turn_gains_two_memory() {
    let (mut runner, tamer) = digivolve_runner();
    assert!(effect_digivolve(&mut runner, 0, tamer));
    runner.auto_resolve().ok();
    assert_eq!(runner.memory(), 5);
}

#[test]
fn bt7_081_is_once_per_turn() {
    let (mut runner, tamer) = digivolve_runner();
    let second = runner.place_on_field(0, "TAMER-2", Some(0));
    assert!(effect_digivolve(&mut runner, 0, tamer));
    runner.auto_resolve().ok();
    assert_eq!(runner.memory(), 5);
    assert!(effect_digivolve(&mut runner, 0, second));
    runner.auto_resolve().ok();
    assert_eq!(runner.memory(), 5, "the second Tamer digivolution gains nothing");
}

#[test]
fn bt7_081_a_digimon_digivolving_does_not_count() {
    let (mut runner, _tamer) = digivolve_runner();
    let digimon = runner.place_on_field(0, "PLAIN", Some(0));
    assert!(effect_digivolve(&mut runner, 0, digimon));
    runner.auto_resolve().ok();
    assert_eq!(runner.memory(), 3);
}

#[test]
fn bt7_081_does_not_trigger_on_the_opponents_turn() {
    let (mut runner, tamer) = digivolve_runner();
    runner.set_first_player(1);
    assert!(effect_digivolve(&mut runner, 0, tamer));
    runner.auto_resolve().ok();
    assert_eq!(runner.memory(), 3);
}

#[test]
fn bt7_081_ignores_the_opponents_tamer_digivolving() {
    let (mut runner, _tamer) = digivolve_runner();
    let opp_tamer = runner.place_on_field(1, "TAMER", Some(0));
    assert!(effect_digivolve(&mut runner, 1, opp_tamer));
    runner.auto_resolve().ok();
    assert_eq!(runner.memory(), 3);
}

#[test]
fn bt7_081_needs_to_be_on_the_battle_area() {
    let mut runner = base()
        .add_card(make_test_card("EVO-A", "Evolution A"))
        .hand(0, &["EVO-A", ID])
        .deck(0, &["TAIL"; 8])
        .memory(3)
        .start();
    runner.skip_mulligan();
    runner.set_first_player(0);
    let tamer = runner.place_on_field(0, "TAMER", Some(0));
    assert!(effect_digivolve(&mut runner, 0, tamer));
    runner.auto_resolve().ok();
    assert_eq!(runner.memory(), 3, "Bokomon is still in the hand");
}

#[test]
fn bt7_081_trigger_qualifier_reads_the_digivolved_from_card_not_the_resulting_top_card() {
    // A Tamer that digivolves becomes a Digimon permanent, so the qualifier
    // must test the card it digivolved FROM (`event_target_digivolved_from_kind`),
    // never the resulting top card (`event_target_kind`).
    let runner = base().start();
    let card = runner.compiled_card(ID).expect("compiled card");
    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnDigivolve) => {
                Some(t)
            }
            _ => None,
        })
        .expect("BT7-081 must have a digivolve observer");
    assert!(clause.once_per_turn);
    assert!(!clause.optional, "gain 2 memory is mandatory");
    let qualifier = clause.trigger_if.as_ref().expect("trigger qualifier");
    assert!(qualifier
        .all_of
        .iter()
        .any(|p| p.event_target_digivolved_from_kind == Some(CompiledCardKind::Tamer)));
    assert!(qualifier.all_of.iter().all(|p| p.event_target_kind.is_none()));
}
