//! BT20-048 Dorumon: actual card image, official bundle, and DCGO BT20_048.cs.
//! Reveal ordering is a player choice under CR 4.3 15-15-3-6.

use digimon_engine::action::{
    build_action_mask,
    space::{encode_digivolve, PASS, PLAY_HAND_START, SEL_REVEAL_START},
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{
    make_test_card, make_test_egg, DebugRunner, DebugRunnerBuilder,
};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT20-048";

fn candidate(id: &str, kind: CardKind, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = kind;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    if kind != CardKind::Digimon {
        card.level = None;
        card.dp = None;
    }
    card
}

fn base() -> DebugRunnerBuilder {
    let mut builder = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT20-048 must be implemented")
        .add_card(candidate("X-DIGIMON", CardKind::Digimon, &["X Antibody"]))
        .add_card(candidate("X-OPTION", CardKind::Option, &["X Antibody"]))
        .add_card(candidate("CHRON-TAMER", CardKind::Tamer, &["Chronicle"]))
        .add_card(candidate("CHRON-OPTION", CardKind::Option, &["Chronicle"]))
        .add_card(candidate(
            "DUAL",
            CardKind::Option,
            &["X Antibody", "Chronicle"],
        ))
        .add_card(candidate(
            "CHRON-DIGIMON",
            CardKind::Digimon,
            &["Chronicle"],
        ))
        .add_card(candidate("PLAIN-TAMER", CardKind::Tamer, &[]))
        .add_card(candidate("PLAIN-OPTION", CardKind::Option, &[]))
        .add_card(make_test_card("FILLER", "X Antibody Chronicle"))
        .add_card(make_test_card("TAIL", "Unrevealed card"))
        .deck(1, &["TAIL"; 8])
        .memory(10);
    for (id, color, x) in [
        ("BLACK-EGG", CardColor::Black, false),
        ("YELLOW-EGG", CardColor::Yellow, false),
        ("BLACK-X", CardColor::Black, true),
        ("YELLOW-X", CardColor::Yellow, true),
        ("RED-X", CardColor::Red, true),
    ] {
        let mut egg = make_test_egg(id, id);
        egg.colors = vec![color];
        if x {
            egg.traits.push("X Antibody".into());
        }
        builder = builder.add_card(egg);
    }
    let mut wrong_level = candidate("BLACK-X-LV3", CardKind::Digimon, &["X Antibody"]);
    wrong_level.colors = vec![CardColor::Black];
    builder.add_card(wrong_level)
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
    assert!(!view.is_optional, "printed add and ordering are mandatory");
    let mut actual = view.valid_action_ids.clone();
    actual.sort_unstable();
    let mut expected = expected.to_vec();
    expected.sort_unstable();
    assert_eq!(actual, expected);
    for (action, value) in build_action_mask(&runner.game, 0).iter().enumerate() {
        assert_eq!(
            *value > 0.0,
            expected.contains(&(action as u16)),
            "mask action {action}"
        );
    }
    assert_eq!(build_action_mask(&runner.game, 0)[PASS as usize], 0.0);
}

fn assert_bucket(runner: &DebugRunner, bucket: u8, candidates: &[&str]) {
    assert!(
        matches!(runner.pending_kind(), Some(SelectionKind::RevealBucket { bucket_index, .. }) if bucket_index == bucket)
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
fn bt20_048_x_antibody_pick_exposes_both_digimon_and_option_without_decline() {
    for chosen in ["X-DIGIMON", "X-OPTION"] {
        let mut runner = play(&["TAIL", "CHRON-TAMER", "X-DIGIMON", "X-OPTION"]);
        assert_bucket(&runner, 0, &["X-DIGIMON", "X-OPTION"]);
        pick(&mut runner, chosen);
        assert_bucket(&runner, 1, &["CHRON-TAMER"]);
        pick(&mut runner, "CHRON-TAMER");
        finish_order(&mut runner, false);
        assert_eq!(
            zone_ids(&runner.game.players[0].hand, &runner),
            [chosen, "CHRON-TAMER"]
        );
        let other = if chosen == "X-DIGIMON" {
            "X-OPTION"
        } else {
            "X-DIGIMON"
        };
        assert_eq!(
            zone_ids(&runner.game.players[0].deck, &runner),
            [other, "TAIL"]
        );
    }
}

#[test]
fn bt20_048_chronicle_pick_exposes_tamer_and_option_as_separate_choices() {
    for chosen in ["CHRON-TAMER", "CHRON-OPTION"] {
        let mut runner = play(&["TAIL", "X-DIGIMON", "CHRON-TAMER", "CHRON-OPTION"]);
        assert_bucket(&runner, 0, &["X-DIGIMON"]);
        pick(&mut runner, "X-DIGIMON");
        assert_bucket(&runner, 1, &["CHRON-TAMER", "CHRON-OPTION"]);
        pick(&mut runner, chosen);
        finish_order(&mut runner, false);
        assert_eq!(
            zone_ids(&runner.game.players[0].hand, &runner),
            ["X-DIGIMON", chosen]
        );
        assert_eq!(runner.deck_size(0), 2);
    }
}

#[test]
fn bt20_048_one_physical_card_cannot_fill_both_buckets() {
    let mut runner = play(&["TAIL", "DUAL", "X-DIGIMON", "FILLER"]);
    assert_bucket(&runner, 0, &["DUAL", "X-DIGIMON"]);
    pick(&mut runner, "DUAL");
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::OrderedPermutation { .. })
    ));
    finish_order(&mut runner, true);
    assert_eq!(zone_ids(&runner.game.players[0].hand, &runner), ["DUAL"]);
    assert_eq!(runner.deck_size(0), 3);
}

#[test]
fn bt20_048_dual_eligible_card_remains_available_for_the_second_bucket() {
    let mut runner = play(&["TAIL", "DUAL", "X-DIGIMON", "CHRON-TAMER"]);
    assert_bucket(&runner, 0, &["DUAL", "X-DIGIMON"]);
    pick(&mut runner, "X-DIGIMON");
    assert_bucket(&runner, 1, &["DUAL", "CHRON-TAMER"]);
    pick(&mut runner, "DUAL");
    finish_order(&mut runner, false);
    assert_eq!(
        zone_ids(&runner.game.players[0].hand, &runner),
        ["X-DIGIMON", "DUAL"]
    );
}

#[test]
fn bt20_048_missing_x_antibody_bucket_does_not_skip_chronicle_pick() {
    let mut runner = play(&["TAIL", "FILLER", "CHRON-TAMER", "CHRON-OPTION"]);
    assert_bucket(&runner, 1, &["CHRON-TAMER", "CHRON-OPTION"]);
    pick(&mut runner, "CHRON-OPTION");
    finish_order(&mut runner, false);
    assert_eq!(
        zone_ids(&runner.game.players[0].hand, &runner),
        ["CHRON-OPTION"]
    );
}

#[test]
fn bt20_048_chronicle_digimon_and_nonchronicle_tamer_or_option_are_ineligible() {
    for invalid in ["CHRON-DIGIMON", "PLAIN-TAMER", "PLAIN-OPTION"] {
        let mut runner = play(&["TAIL", "X-DIGIMON", invalid, "FILLER"]);
        assert_bucket(&runner, 0, &["X-DIGIMON"]);
        pick(&mut runner, "X-DIGIMON");
        assert!(matches!(
            runner.pending_kind(),
            Some(SelectionKind::OrderedPermutation { .. })
        ));
        finish_order(&mut runner, false);
        assert_eq!(
            zone_ids(&runner.game.players[0].hand, &runner),
            ["X-DIGIMON"]
        );
        assert_eq!(runner.deck_size(0), 3);
    }
}

#[test]
fn bt20_048_no_matches_exposes_every_remainder_order_and_preserves_unrevealed_top() {
    for reverse in [false, true] {
        let mut runner = play(&["TAIL", "FILLER", "PLAIN-TAMER", "CHRON-DIGIMON"]);
        let mut expected = zone_ids(&runner.game.revealed_cards, &runner);
        assert_eq!(expected.len(), 3);
        if reverse {
            expected.reverse();
        }
        expected.reverse(); // Deck stores bottom first; first choice is drawn first of the group.
        expected.push("TAIL".into());
        finish_order(&mut runner, reverse);
        assert_eq!(zone_ids(&runner.game.players[0].deck, &runner), expected);
        assert_eq!(runner.hand_size(0), 0);
    }
}

#[test]
fn bt20_048_short_and_empty_decks_resolve_without_inventing_cards() {
    let mut runner = play(&["X-DIGIMON", "CHRON-OPTION"]);
    assert_bucket(&runner, 0, &["X-DIGIMON"]);
    pick(&mut runner, "X-DIGIMON");
    assert_bucket(&runner, 1, &["CHRON-OPTION"]);
    pick(&mut runner, "CHRON-OPTION");
    finish_order(&mut runner, false);
    assert_eq!(runner.hand_size(0), 2);
    assert_eq!(runner.deck_size(0), 0);
    let empty = play(&[]);
    assert!(empty.pending_selection().is_none());
    assert_eq!(empty.hand_size(0), 0);
}

#[test]
fn bt20_048_search_is_not_once_per_turn() {
    let mut runner = base()
        .hand(0, &[ID, ID])
        .deck(
            0,
            &[
                "X-DIGIMON",
                "CHRON-TAMER",
                "FILLER",
                "X-OPTION",
                "CHRON-OPTION",
                "FILLER",
            ],
        )
        .start();
    runner.skip_mulligan();
    for (x, chronicle) in [("X-OPTION", "CHRON-OPTION"), ("X-DIGIMON", "CHRON-TAMER")] {
        assert_eq!(
            build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
            1.0
        );
        runner.game.decode_action(PLAY_HAND_START, 0);
        assert_bucket(&runner, 0, &[x]);
        pick(&mut runner, x);
        assert_bucket(&runner, 1, &[chronicle]);
        pick(&mut runner, chronicle);
        finish_order(&mut runner, false);
    }
    assert_eq!(runner.hand_size(0), 4);
    assert_eq!(runner.memory(), 4);
}

#[test]
fn bt20_048_standard_paths_require_black_or_yellow_level_two_and_do_not_search() {
    for (egg, legal) in [
        ("BLACK-EGG", true),
        ("YELLOW-EGG", true),
        ("YELLOW-X", true),
        ("RED-X", false),
        ("BLACK-X-LV3", false),
    ] {
        let mut runner = base().hand(0, &[ID]).deck(0, &["X-DIGIMON"; 8]).start();
        runner.skip_mulligan();
        let source = runner.place_on_field(0, egg, Some(0));
        let action = encode_digivolve(0, source.index as u16);
        assert_eq!(
            build_action_mask(&runner.game, 0)[action as usize] > 0.0,
            legal,
            "{egg}"
        );
        if legal {
            runner.game.decode_action(action, 0);
            assert!(
                runner.pending_selection().is_none(),
                "{egg} has only the printed cost-1 route and no On Play search; unexpected prompt: {:?}",
                runner.pending_selection_view()
            );
            assert_eq!(runner.memory(), 9, "{egg}");
            assert_eq!(runner.deck_size(0), 7, "only the evolution bonus draw");
            assert_eq!(runner.hand_size(0), 1);
            assert_eq!(
                runner.game.players[0].battle_area[source.index as usize]
                    .top_card()
                    .card_id(&runner.game.card_data),
                ID
            );
        }
    }
}

#[test]
fn bt20_048_black_x_antibody_egg_offers_both_cost_routes() {
    for cost in [0, 1] {
        let mut runner = base().hand(0, &[ID]).deck(0, &["FILLER"; 8]).start();
        runner.skip_mulligan();
        let egg = runner.place_on_field(0, "BLACK-X", Some(0));
        let action = encode_digivolve(0, egg.index as u16);
        assert_eq!(build_action_mask(&runner.game, 0)[action as usize], 1.0);
        runner.game.decode_action(action, 0);
        let view = runner
            .pending_selection_view()
            .expect("zero or one memory route choice");
        assert_eq!(view.kind, SelectionKind::EffectChoice);
        assert_eq!(view.valid_action_ids.len(), 2);
        assert_prompt(&runner, &view.valid_action_ids);
        assert_eq!(runner.memory(), 10, "choice precedes payment");
        let action = view
            .effect_choices
            .as_ref()
            .unwrap()
            .iter()
            .find(|choice| choice.label == format!("Digivolve for cost {cost}"))
            .unwrap()
            .action_id;
        runner.execute_action(0, action).unwrap();
        assert_eq!(runner.memory(), 10 - cost);
        assert_eq!(runner.deck_size(0), 7);
        assert!(runner.pending_selection().is_none());
    }
}

#[test]
fn bt20_048_inherited_dp_applies_only_to_its_host_during_that_owners_opponents_turn() {
    let mut runner = base().deck(0, &["TAIL"; 8]).start();
    runner.skip_mulligan();
    let face = runner.place_on_field(0, ID, Some(0));
    let own_host = runner.place_stack(0, &[ID, "FILLER"]);
    let own_other = runner.place_on_field(0, "FILLER", Some(0));
    let opponent_host = runner.place_stack(1, &[ID, "FILLER"]);
    for (turn, own_dp, opponent_dp) in [(0, 2000, 4000), (1, 4000, 2000), (0, 2000, 4000)] {
        runner.set_first_player(turn);
        assert_eq!(runner.effective_dp(face), Some(1000));
        assert_eq!(runner.effective_dp(own_host), Some(own_dp));
        assert_eq!(runner.effective_dp(own_other), Some(2000));
        assert_eq!(runner.effective_dp(opponent_host), Some(opponent_dp));
    }
}
