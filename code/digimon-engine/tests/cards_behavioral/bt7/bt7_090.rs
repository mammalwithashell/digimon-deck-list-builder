//! BT7-090 Kota Domoto: inspected printed image, official bundle, and DCGO BT7_090.cs.
//! CR 4.3 1-3-6 and 15-1-5: choose one available card for a mandatory search.
//! The printed search is for an X Antibody card of any kind, not only Digimon.

use digimon_engine::action::{
    build_action_mask,
    space::{encode_attack, PASS, PLAY_HAND_START, SECURITY_TARGET, SEL_REVEAL_START},
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT7-090";

fn card(id: &str, kind: CardKind, trait_name: Option<&str>) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    if kind != CardKind::Digimon {
        c.level = None;
        c.dp = None;
    }
    if let Some(trait_name) = trait_name {
        c.traits = vec![trait_name.into()];
    }
    c
}

fn base() -> DebugRunnerBuilder {
    let mut name_only = make_test_card("NAME-ONLY", "X Antibody");
    name_only.traits = vec!["Dragon".into()];
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.dp = Some(9000);
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT7-090 must be in the production pack")
        .add_card(card("X-DIGIMON", CardKind::Digimon, Some("X Antibody")))
        .add_card(card("X-OPTION", CardKind::Option, Some("X Antibody")))
        .add_card(card("X-TAMER", CardKind::Tamer, Some("X Antibody")))
        .add_card(card("NEAR-TRAIT", CardKind::Digimon, Some("X Antibody Variant")))
        .add_card(name_only)
        .add_card(attacker)
        .add_card(make_test_card("A", "A"))
        .add_card(make_test_card("B", "B"))
        .add_card(make_test_card("C", "C"))
        .add_card(make_test_card("DEEP", "Unrevealed card"))
        .deck(0, &["DEEP"; 12])
        .deck(1, &["DEEP"; 12])
        .memory(10)
}

fn ids_in_hand(r: &DebugRunner, owner: u8) -> Vec<String> {
    r.game.players[owner as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_owned())
        .collect()
}

fn ids_in_deck(r: &DebugRunner, owner: u8) -> Vec<String> {
    r.game.players[owner as usize]
        .deck
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_owned())
        .collect()
}

fn revealed_action(r: &DebugRunner, id: &str) -> u16 {
    SEL_REVEAL_START
        + r.game
            .revealed_cards
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == id)
            .expect("card must be in the revealed pool") as u16
}

fn assert_mandatory_prompt(r: &DebugRunner, owner: u8) -> Vec<u16> {
    let s = r.pending_selection().expect("player-visible mandatory choice");
    assert_eq!(s.selecting_player, owner);
    assert!(!s.is_optional);
    let mask = build_action_mask(&r.game, owner);
    assert_eq!(mask[PASS as usize], 0.0, "no optional decline is printed");
    for action in &s.valid_action_ids {
        assert_eq!(mask[*action as usize], 1.0);
    }
    s.valid_action_ids.clone()
}

fn play(r: &mut DebugRunner) {
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
}

/// Ordered-permutation actions index the remaining choices, not the reveal pool.
/// Select each printed ID in the requested draw order, checking every mask.
fn order_bottom(r: &mut DebugRunner, owner: u8, order: &[&str]) {
    let mut remaining: Vec<String> = r
        .game
        .revealed_cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_owned())
        .collect();
    assert_eq!(remaining.len(), order.len());
    for id in order {
        assert_eq!(
            r.pending_kind(),
            Some(SelectionKind::OrderedPermutation {
                remaining: remaining.len() as u8,
            })
        );
        let actions = assert_mandatory_prompt(r, owner);
        assert_eq!(actions.len(), remaining.len());
        let index = remaining.iter().position(|candidate| candidate == id).unwrap();
        let action = SEL_REVEAL_START + index as u16;
        assert!(actions.contains(&action));
        r.execute_action(owner, action).unwrap();
        remaining.remove(index);
    }
    assert!(r.game.revealed_cards.is_empty());
    assert!(r.pending_selection().is_none());
}

fn start_security_search() -> DebugRunner {
    let mut r = base()
        .deck(1, &["DEEP", "B", "A", "X-OPTION", "X-DIGIMON"])
        .security(1, &[ID])
        .memory(5)
        .start();
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    let action = encode_attack(attacker.index as u16, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    r
}

#[test]
fn bt7_090_mandatory_search_accepts_x_antibody_cards_of_every_kind() {
    for chosen in ["X-DIGIMON", "X-OPTION", "X-TAMER"] {
        let mut r = base()
            .hand(0, &[ID])
            .deck(0, &["DEEP", "A", "X-TAMER", "X-OPTION", "X-DIGIMON"])
            .start();
        play(&mut r);
        assert_eq!(r.memory(), 6, "normal Tamer play costs four");
        let legal = assert_mandatory_prompt(&r, 0);
        assert_eq!(legal.len(), 3);
        for id in ["X-DIGIMON", "X-OPTION", "X-TAMER"] {
            assert!(legal.contains(&revealed_action(&r, id)));
        }
        assert!(!legal.contains(&revealed_action(&r, "A")));
        assert!(r.execute_action(0, PASS).is_err());
        let chosen_action = revealed_action(&r, chosen);
        r.execute_action(0, chosen_action).unwrap();
        assert_eq!(ids_in_hand(&r, 0), vec![chosen]);
        assert!(matches!(r.pending_kind(), Some(SelectionKind::OrderedPermutation { .. })));
        r.auto_resolve().unwrap();
        assert_eq!(r.deck_size(0), 4);
        assert!(r.game.revealed_cards.is_empty());
    }
}

#[test]
fn bt7_090_search_rejects_name_only_and_near_trait_matches() {
    let mut r = base()
        .hand(0, &[ID])
        .deck(0, &["A", "NAME-ONLY", "NEAR-TRAIT", "X-DIGIMON"])
        .start();
    play(&mut r);
    assert_eq!(assert_mandatory_prompt(&r, 0), vec![revealed_action(&r, "X-DIGIMON")]);
    let mask = build_action_mask(&r.game, 0);
    for id in ["A", "NAME-ONLY", "NEAR-TRAIT"] {
        assert_eq!(mask[revealed_action(&r, id) as usize], 0.0);
    }
    r.execute_action(0, revealed_action(&r, "X-DIGIMON")).unwrap();
    order_bottom(&mut r, 0, &["NAME-ONLY", "A", "NEAR-TRAIT"]);
    assert_eq!(ids_in_hand(&r, 0), vec!["X-DIGIMON"]);
}

#[test]
fn bt7_090_reveals_exactly_four_and_does_not_search_the_fifth_card() {
    let mut r = base()
        .hand(0, &[ID])
        .deck(0, &["X-OPTION", "C", "B", "A", "X-DIGIMON"])
        .start();
    play(&mut r);
    assert_eq!(r.game.revealed_cards.len(), 4);
    assert_eq!(ids_in_deck(&r, 0), vec!["X-OPTION"]);
    assert_eq!(assert_mandatory_prompt(&r, 0).len(), 1);
    r.execute_action(0, revealed_action(&r, "X-DIGIMON")).unwrap();
    order_bottom(&mut r, 0, &["C", "A", "B"]);
    assert_eq!(ids_in_deck(&r, 0), vec!["B", "A", "C", "X-OPTION"]);
    assert_eq!(ids_in_hand(&r, 0), vec!["X-DIGIMON"]);
}

#[test]
fn bt7_090_each_bottom_order_choice_is_exposed_and_changes_draw_order() {
    for order in [["A", "B", "C"], ["C", "A", "B"]] {
        let mut r = base()
            .hand(0, &[ID])
            .deck(0, &["DEEP", "C", "B", "A", "X-DIGIMON"])
            .start();
        play(&mut r);
        r.execute_action(0, revealed_action(&r, "X-DIGIMON")).unwrap();
        order_bottom(&mut r, 0, &order);
        let mut expected: Vec<&str> = order.iter().rev().copied().collect();
        expected.push("DEEP");
        assert_eq!(ids_in_deck(&r, 0), expected);
    }
}

#[test]
fn bt7_090_duplicate_ids_remain_distinct_choices_and_only_one_is_added() {
    let mut r = base()
        .hand(0, &[ID])
        .deck(0, &["B", "A", "X-DIGIMON", "X-DIGIMON"])
        .start();
    play(&mut r);
    let legal = assert_mandatory_prompt(&r, 0);
    assert_eq!(legal, vec![SEL_REVEAL_START, SEL_REVEAL_START + 1]);
    let selected = r.game.revealed_cards[1].handle();
    let other = r.game.revealed_cards[0].handle();
    assert_ne!(selected, other);
    r.execute_action(0, legal[1]).unwrap();
    assert_eq!(r.game.players[0].hand[0].handle(), selected);
    order_bottom(&mut r, 0, &["A", "X-DIGIMON", "B"]);
    assert!(r.game.players[0].deck.iter().any(|c| c.handle() == other));
    assert_eq!(ids_in_hand(&r, 0), vec!["X-DIGIMON"]);
}

#[test]
fn bt7_090_no_matching_reveals_still_orders_all_four_to_bottom() {
    let mut r = base()
        .hand(0, &[ID])
        .deck(0, &["DEEP", "NAME-ONLY", "C", "B", "A"])
        .start();
    play(&mut r);
    assert!(ids_in_hand(&r, 0).is_empty());
    order_bottom(&mut r, 0, &["NAME-ONLY", "A", "C", "B"]);
    assert_eq!(ids_in_deck(&r, 0), vec!["B", "C", "A", "NAME-ONLY", "DEEP"]);
}

#[test]
fn bt7_090_empty_and_short_decks_do_as_much_as_possible() {
    for count in 0..=3 {
        let deck = ["C", "B", "X-DIGIMON"];
        let mut r = base().hand(0, &[ID]).deck(0, &deck[3 - count..]).start();
        play(&mut r);
        if count == 0 {
            assert!(r.pending_selection().is_none());
            assert!(ids_in_hand(&r, 0).is_empty());
        } else {
            assert_eq!(r.game.revealed_cards.len(), count);
            assert_mandatory_prompt(&r, 0);
            r.execute_action(0, revealed_action(&r, "X-DIGIMON")).unwrap();
            r.auto_resolve().unwrap();
            assert_eq!(ids_in_hand(&r, 0), vec!["X-DIGIMON"]);
            assert_eq!(r.deck_size(0), count - 1);
        }
        assert!(r.game.revealed_cards.is_empty());
        assert_eq!(r.memory(), 6);
    }
}

#[test]
fn bt7_090_reveal_selection_clone_resolves_independent_choices() {
    let mut original = base()
        .hand(0, &[ID])
        .deck(0, &["B", "A", "X-OPTION", "X-DIGIMON"])
        .start();
    play(&mut original);
    let mut fork = DebugRunner::wrap(original.game.clone());
    fork.execute_action(0, revealed_action(&fork, "X-OPTION")).unwrap();
    order_bottom(&mut fork, 0, &["X-DIGIMON", "B", "A"]);
    assert!(ids_in_hand(&original, 0).is_empty());
    assert_eq!(original.game.revealed_cards.len(), 4);
    original.execute_action(0, revealed_action(&original, "X-DIGIMON")).unwrap();
    order_bottom(&mut original, 0, &["A", "B", "X-OPTION"]);
    assert_eq!(ids_in_hand(&original, 0), vec!["X-DIGIMON"]);
    assert_eq!(ids_in_hand(&fork, 0), vec!["X-OPTION"]);
    assert_ne!(ids_in_deck(&original, 0), ids_in_deck(&fork, 0));
}

#[test]
fn bt7_090_bottom_order_clone_preserves_the_pending_permutation() {
    let mut original = base()
        .hand(0, &[ID])
        .deck(0, &["DEEP", "C", "B", "A", "X-DIGIMON"])
        .start();
    play(&mut original);
    original.execute_action(0, revealed_action(&original, "X-DIGIMON")).unwrap();
    let mut fork = DebugRunner::wrap(original.game.clone());
    order_bottom(&mut original, 0, &["A", "B", "C"]);
    order_bottom(&mut fork, 0, &["C", "B", "A"]);
    assert_eq!(ids_in_deck(&original, 0), vec!["C", "B", "A", "DEEP"]);
    assert_eq!(ids_in_deck(&fork, 0), vec!["A", "B", "C", "DEEP"]);
    assert_eq!(ids_in_hand(&original, 0), ids_in_hand(&fork, 0));
}

fn owner_turn(owner: u8, memory: i16) -> DebugRunner {
    let mut r = base().start();
    r.place_on_field(owner, ID, Some(0));
    if owner == 0 {
        r.end_turn();
    }
    r.game.set_memory(-memory);
    r.end_turn();
    assert_eq!(r.turn_player(), owner);
    r
}

#[test]
fn bt7_090_start_of_owner_turn_sets_zero_through_two_to_three_for_both_seats() {
    for owner in [0, 1] {
        for memory in [0, 1, 2] {
            let r = owner_turn(owner, memory);
            assert_eq!(r.memory(), 3, "owner {owner}, starting memory {memory}");
            assert!(r.pending_selection().is_none(), "the memory setter is mandatory");
        }
    }
}

#[test]
fn bt7_090_start_of_owner_turn_does_not_reduce_three_or_more_memory() {
    for owner in [0, 1] {
        for memory in [3, 5, 10] {
            let r = owner_turn(owner, memory);
            assert_eq!(r.memory(), memory, "owner {owner}");
        }
    }
}

#[test]
fn bt7_090_does_not_set_memory_at_the_opponents_turn_start() {
    let mut r = base().start();
    r.place_on_field(0, ID, Some(0));
    r.game.set_memory(-1);
    r.end_turn();
    assert_eq!(r.turn_player(), 1);
    assert_eq!(r.memory(), 1, "opponent has no memory setter");
    assert!(r.pending_selection().is_none());
}

#[test]
fn bt7_090_security_plays_for_free_and_owner_must_resolve_its_on_play_search() {
    let mut r = start_security_search();
    assert_eq!(r.memory(), 5);
    assert_eq!(r.security_count(1), 0);
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    let kota = r.game.players[1].battle_area[0].top_card();
    assert_eq!(kota.card_id(&r.game.card_data), ID);
    assert_eq!(kota.card_kind(&r.game.card_data), CardKind::Tamer);
    assert_eq!(kota.colors(&r.game.card_data), &[CardColor::Black]);
    assert_mandatory_prompt(&r, 1);
    let pick = revealed_action(&r, "X-OPTION");
    assert!(r.execute_action(0, pick).is_err(), "attacker cannot choose defender's search");
    r.execute_action(1, pick).unwrap();
    order_bottom(&mut r, 1, &["B", "X-DIGIMON", "A"]);
    assert_eq!(ids_in_hand(&r, 1), vec!["X-OPTION"]);
    assert!(ids_in_hand(&r, 0).is_empty());
    assert_eq!(ids_in_deck(&r, 1), vec!["A", "X-DIGIMON", "B", "DEEP"]);
    assert_eq!(r.memory(), 5, "Security play and On Play search are free");
    assert!(r.game.players[1].trash.iter().all(|c| c.card_id(&r.game.card_data) != ID));
}

#[test]
fn bt7_090_security_search_clone_keeps_owner_and_completes_attack_once() {
    let mut original = start_security_search();
    let mut fork = DebugRunner::wrap(original.game.clone());
    for r in [&mut original, &mut fork] {
        assert_mandatory_prompt(r, 1);
        r.execute_action(1, revealed_action(r, "X-DIGIMON")).unwrap();
        order_bottom(r, 1, &["X-OPTION", "A", "B"]);
        assert_eq!(r.game.players[1].battle_area.len(), 1);
        assert_eq!(r.security_count(1), 0);
        assert_eq!(r.memory(), 5);
        assert!(r.game.pending_attack.is_none());
    }
    assert_eq!(ids_in_hand(&original, 1), ids_in_hand(&fork, 1));
    assert_eq!(ids_in_deck(&original, 1), ids_in_deck(&fork, 1));
}
