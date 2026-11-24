//! Source: `data/card_bundles/BT27-035.md` (printed text and official Q&A).
//!
//! <Reboot> <Blocker> <Evade>
//! [When Digivolving] [When Attacking] [Once Per Turn] You may activate 1
//! [Main] effect on this card's Option side.
//! [When Digivolving] [When Attacking] If your opponent's hand has 8 or more
//! cards, by returning cards without looking to the bottom of the deck so that
//! it has 7 left, return their top security card to the bottom of the deck.
//! Option side: <Use Req. ([DATA SQUAD] trait)> [Main] Return 1 of your
//! opponent's Digimon to the hand. Then, if their hand has 8 or more cards, 1
//! of your Digimon may unsuspend. <Arts Digivolve>
//! Q7682/83: the activated [Main] is an Option card effect, not Option use.
//! Q7685: both triggers are ordered by the player. Q7688/89: the hand cards are
//! picked without looking; the security card is not revealed.

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::PASS;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectSourceKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-035";

fn colored(mut c: digimon_engine::card_data::CardData, color: CardColor) -> digimon_engine::card_data::CardData {
    c.colors = vec![color];
    c
}

fn runner(opp_hand: usize) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(colored(mon("BLUE6", "Blue", 6, 9, &[]), CardColor::Blue))
        .add_card(colored(mon("BLACK6", "Black", 6, 9, &[]), CardColor::Black))
        .add_card(colored(mon("SQ6", "Squad", 6, 9, &["DATA SQUAD"]), CardColor::Red))
        .add_card(colored(mon("RED6", "Red", 6, 9, &[]), CardColor::Red))
        .add_card(colored(mon("RED5", "Red5", 5, 7, &[]), CardColor::Red))
        .add_card(tamer("T", "Squad Tamer", &["DATA SQUAD"]))
        .add_card(mon("L4", "Four", 4, 5, &[]))
        .add_card(mon("SEC-A", "SecA", 3, 3, &[]))
        .add_card(mon("SEC-B", "SecB", 3, 3, &[]))
        .add_card(mon("F", "Fill", 3, 3, &[]))
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .security(1, &["SEC-A", "SEC-B"])
        .hand(0, &[CARD])
        .hand(1, &vec!["F"; opp_hand])
        .memory(10)
        .start()
}

fn do_action(r: &mut DebugRunner, a: u16) {
    let p = r.pending_selection_view().unwrap().selecting_player;
    assert_eq!(build_action_mask(&r.game, p)[a as usize], 1.0);
    r.execute_action(p, a).unwrap();
}

fn accept_offer(r: &mut DebugRunner) {
    let v = r.pending_selection_view().unwrap();
    assert!(v.is_optional, "{}", v.prompt);
    let a = *v.valid_action_ids.iter().find(|a| **a != PASS).unwrap();
    do_action(r, a);
}

fn is_order(r: &DebugRunner) -> bool {
    r.pending_selection_view().is_some_and(|v| v.kind == SelectionKind::TriggerOrder)
}

fn order_reduction_first(r: &mut DebugRunner, reduction_first: bool) {
    let v = r.pending_selection_view().unwrap();
    assert_eq!(v.kind, SelectionKind::TriggerOrder, "Q7685: player orders both triggers");
    let c = v.effect_choices.unwrap();
    assert_eq!(c.len(), 2);
    do_action(r, c[usize::from(reduction_first)].action_id);
}

fn blind(r: &mut DebugRunner, n: usize) -> Vec<CardHandle> {
    let mut handles = vec![];
    for _ in 0..n {
        let v = r.pending_selection_view().unwrap();
        assert!(matches!(v.kind, SelectionKind::CountCappedMultiSelect { .. }), "{:?}", v.kind);
        assert_eq!(v.selecting_player, 0);
        assert_eq!(v.zone_owner, Some(1));
        if let Some(c) = &v.card_choice {
            assert!(c.rows.iter().all(|row| row.card_id.is_none()), "without looking");
        }
        let a = *v.valid_action_ids.iter().rfind(|a| **a != PASS).unwrap();
        let index = (a - digimon_engine::action::space::PLAY_HAND_START) as usize;
        handles.push(r.game.players[1].hand[index].handle());
        do_action(r, a);
    }
    if r.pending_selection_view()
        .is_some_and(|v| matches!(v.kind, SelectionKind::CountCappedMultiSelect { .. }))
    {
        do_action(r, PASS);
    }
    handles
}

fn decline_optionals(r: &mut DebugRunner) {
    for _ in 0..10 {
        let Some(v) = r.pending_selection_view() else { return };
        if v.is_optional {
            do_action(r, PASS);
        } else if v.kind == SelectionKind::TriggerOrder {
            do_action(r, v.effect_choices.unwrap()[0].action_id);
        } else {
            panic!("unexpected mandatory prompt: {}", v.prompt);
        }
    }
}

fn evolve_card(r: &mut DebugRunner) -> PermanentHandle {
    let b = r.place_on_field(0, "BLUE6", Some(0));
    assert!(digivolve_hand(r, CARD, b));
    b
}

fn find(r: &DebugRunner, p: u8, id: &str) -> PermanentHandle {
    let i = field_ids(r, p).iter().position(|x| x == id).expect("on field");
    PermanentHandle { player: p, index: i as u8 }
}

#[test]
fn printed_keywords_are_reboot_blocker_evade() {
    let mut r = runner(0);
    let h = r.place_stack(0, &["BLUE6", CARD]);
    for k in [Keyword::Reboot, Keyword::Blocker, Keyword::Evade] {
        assert!(r.game.has_keyword(h, k), "{k:?}");
    }
}

#[test]
fn digivolves_for_5_from_blue_black_or_data_squad_level_6_only() {
    for base in ["BLUE6", "BLACK6", "SQ6"] {
        let mut r = runner(0);
        let b = r.place_on_field(0, base, Some(0));
        let mem = r.memory();
        assert!(digivolve_hand(&mut r, CARD, b), "{base}");
        decline_optionals(&mut r);
        assert_eq!(top_id(&r, b), CARD);
        assert_eq!(mem - r.memory(), 5, "{base}");
    }
    for base in ["RED6", "RED5"] {
        let mut r = runner(0);
        let b = r.place_on_field(0, base, Some(0));
        assert!(!digivolve_hand(&mut r, CARD, b), "{base}");
    }
}

#[test]
fn seven_cards_offers_only_the_option_main_and_declining_changes_nothing() {
    let mut r = runner(7);
    r.place_on_field(1, "L4", Some(0));
    evolve_card(&mut r);
    assert!(!is_order(&r), "the hand-size clause does not trigger at 7");
    let v = r.pending_selection_view().expect("may activate the Option side [Main]");
    assert!(v.is_optional);
    do_action(&mut r, PASS);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(1), 7);
    assert_eq!(r.game.players[1].security.len(), 2);
    assert_eq!(field_ids(&r, 1), vec!["L4"]);
}

#[test]
fn eight_or_nine_cards_blind_returns_down_to_seven_then_bottom_decks_top_security() {
    for n in [8usize, 9] {
        let mut r = runner(n);
        let top = r.game.players[1].security.last().unwrap().handle();
        let deck = r.deck_size(1);
        evolve_card(&mut r);
        order_reduction_first(&mut r, true);
        accept_offer(&mut r);
        let chosen = blind(&mut r, n - 7);
        assert_eq!(r.hand_size(1), 7, "{n}");
        assert_eq!(r.deck_size(1), deck + n - 7 + 1);
        assert_eq!(r.game.players[1].deck[0].handle(), top, "security under the returned cards");
        for (i, c) in chosen.iter().rev().enumerate() {
            assert_eq!(r.game.players[1].deck[i + 1].handle(), *c);
        }
        assert_eq!(r.game.players[1].security.len(), 1);
        assert!(r.game.revealed_cards.is_empty(), "Q7689: never revealed");
        decline_optionals(&mut r);
    }
}

#[test]
fn declining_the_reduction_keeps_hand_and_security() {
    let mut r = runner(9);
    evolve_card(&mut r);
    order_reduction_first(&mut r, true);
    do_action(&mut r, PASS);
    decline_optionals(&mut r);
    assert_eq!(r.hand_size(1), 9);
    assert_eq!(r.game.players[1].security.len(), 2);
}

#[test]
fn option_main_bounce_to_eight_then_may_unsuspend_one_of_your_digimon() {
    let mut r = runner(7);
    r.place_on_field(1, "L4", Some(0));
    let mine = r.place_on_field(0, "RED5", Some(0));
    r.game.players[0].battle_area[mine.index as usize].is_suspended = true;
    evolve_card(&mut r);
    accept_offer(&mut r);
    let v = r.pending_selection_view().expect("bounce target");
    assert_eq!(v.source_kind, EffectSourceKind::Option, "Q7682: an Option card effect");
    r.choose_prompt_card("L4", None).unwrap();
    assert_eq!(r.hand_size(1), 8);
    let v = r.pending_selection_view().expect("may unsuspend");
    assert!(v.is_optional);
    assert_eq!(candidates(&r), vec!["RED5"], "only suspended Digimon of yours");
    r.choose_prompt_card("RED5", None).unwrap();
    decline_optionals(&mut r);
    assert!(!r.game.players[0].battle_area[find(&r, 0, "RED5").index as usize].is_suspended);
    assert_eq!(field_ids(&r, 0).iter().filter(|x| *x == CARD).count(), 1, "the Option side was not used");
}

#[test]
fn option_main_bounce_to_seven_offers_no_unsuspend() {
    let mut r = runner(6);
    r.place_on_field(1, "L4", Some(0));
    let mine = r.place_on_field(0, "RED5", Some(0));
    r.game.players[0].battle_area[mine.index as usize].is_suspended = true;
    evolve_card(&mut r);
    accept_offer(&mut r);
    r.choose_prompt_card("L4", None).unwrap();
    assert_eq!(r.hand_size(1), 7);
    assert!(r.pending_selection().is_none());
    assert!(r.game.players[0].battle_area[find(&r, 0, "RED5").index as usize].is_suspended);
}

#[test]
fn resolving_the_option_main_first_counts_the_bounced_card_for_the_reduction() {
    let mut r = runner(8);
    r.place_on_field(1, "L4", Some(0));
    evolve_card(&mut r);
    order_reduction_first(&mut r, false);
    accept_offer(&mut r);
    r.choose_prompt_card("L4", None).unwrap();
    decline_all_optional_until_blind(&mut r);
    blind(&mut r, 2);
    assert_eq!(r.hand_size(1), 7, "bounced to 9, then 2 returned");
    assert_eq!(r.game.players[1].security.len(), 1);
}

fn decline_all_optional_until_blind(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("hand-size clause");
    assert!(v.is_optional);
    accept_offer(r);
}

#[test]
fn when_attacking_triggers_both_and_the_option_main_is_once_per_turn() {
    let mut r = runner(9);
    r.place_on_field(1, "L4", Some(0));
    evolve_card(&mut r);
    order_reduction_first(&mut r, true);
    do_action(&mut r, PASS);
    accept_offer(&mut r);
    r.choose_prompt_card("L4", None).unwrap();
    assert_eq!(r.hand_size(1), 10);
    let h = find(&r, 0, CARD);
    r.game.players[0].battle_area[h.index as usize].is_suspended = false;
    attack_security(&mut r, h.index);
    assert!(!is_order(&r), "the Option-side [Main] was used this turn");
    accept_offer(&mut r);
    blind(&mut r, 3);
    assert_eq!(r.hand_size(1), 7);
    assert_eq!(r.game.players[1].security.len(), 0, "1 bottom-decked, then the attack checks the other");
}

#[test]
fn using_the_option_side_needs_data_squad_pays_6_and_offers_arts_digivolve() {
    let mut r = runner(7);
    r.place_on_field(1, "L4", Some(0));
    let _ = r.game.play_option_from_hand(0, hand_index(&r, 0, CARD));
    assert!(r.pending_selection().is_none() && hand_ids(&r, 0) == vec![CARD], "no [DATA SQUAD]");

    let mut r = runner(7);
    r.place_on_field(0, "T", None);
    let mine = r.place_on_field(0, "RED5", Some(0));
    r.game.players[0].battle_area[mine.index as usize].is_suspended = true;
    r.place_on_field(1, "L4", Some(0));
    let mem = r.memory();
    let _ = r.game.play_option_from_hand(0, hand_index(&r, 0, CARD));
    r.choose_prompt_card("L4", None).unwrap();
    assert_eq!(mem - r.memory(), 6);
    assert_eq!(r.hand_size(1), 8);
    let v = r.pending_selection_view().expect("8 cards: may unsuspend");
    assert!(v.is_optional);
    r.choose_prompt_card("RED5", None).unwrap();
    assert!(!r.game.players[0].battle_area[find(&r, 0, "RED5").index as usize].is_suspended);
}

mod burst {
    use crate::burst_digivolve_common::*;
    use digimon_engine::enums::CardColor;

    const SPEC: BurstSpec = BurstSpec {
        card: "BT27-035",
        host_name: "MirageGaogamon",
        host_color: CardColor::Blue,
        tamer_name: "Thomas H. Norstein",
    };

    #[test]
    fn burst_returns_thomas_costs_zero_draws_one() {
        burst_and_check(SPEC, Host::BurstOnly);
    }

    #[test]
    fn burst_needs_thomas() {
        assert_not_offered_without_tamer(SPEC);
    }

    #[test]
    fn burst_top_card_trashed_at_turn_end() {
        assert_turn_end_trash(SPEC, Host::BurstOnly);
    }

    #[test]
    fn burst_vs_normal_route_is_a_player_choice() {
        assert_route_choice(SPEC);
    }

    #[test]
    fn effects_cannot_burst() {
        assert_effect_cannot_burst(SPEC);
    }

    #[test]
    fn breeding_burst_trashes_top_at_turn_end() {
        assert_breeding_burst_turn_end_trash(SPEC);
    }
}
