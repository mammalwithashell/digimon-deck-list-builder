//! Source: `data/card_bundles/BT27-031.md` (printed text and official Q&A).
//!
//! <Decode (Lv.5 w/[Aquatic]/[DATA SQUAD] trait)>
//! [On Play] [When Digivolving] [Counter] [Once Per Turn] By placing 1 of your
//! other Digimon or Tamers as this Digimon's bottom digivolution card, return
//! 1 of your opponent's Digimon with as many digivolution cards as this
//! Digimon or fewer to the bottom of the deck.
//! [All Turns] [Once Per Turn] When effects add to this Digimon's digivolution
//! cards, you may play 1 play cost 5 or lower blue card from its digivolution
//! cards without paying the cost.
//! (Rule) Trait: Has [Aquatic] Type.
//! Q&A: only 1 [Counter] effect can be activated per attack.

use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, GamePhase};
use digimon_engine::permanent::PermanentHandle;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-031";
const ACTIVATE: &str = "You may activate";

fn colored(mut c: CardData, color: CardColor) -> CardData {
    c.colors = vec![color];
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(colored(mon("B5", "Blue5", 5, 7, &[]), CardColor::Blue))
        .add_card(colored(mon("SQ5", "Squad5", 5, 7, &["DATA SQUAD"]), CardColor::Red))
        .add_card(colored(mon("AQ5", "Aqua5", 5, 7, &["Aquatic"]), CardColor::Red))
        .add_card(colored(mon("G5", "Green5", 5, 7, &[]), CardColor::Green))
        .add_card(colored(mon("ALLY", "Ally", 3, 3, &[]), CardColor::Blue))
        .add_card(colored(mon("RED3", "Red3", 3, 3, &[]), CardColor::Red))
        .add_card(colored(tamer("TAM", "Tam", &[]), CardColor::Blue))
        .add_card(mon("O3", "Op3", 3, 3, &[]))
        .add_card(mon("O4", "Op4", 4, 5, &[]))
        .add_card(mon("O5", "Op5", 5, 7, &[]))
        .add_card(mon("O6", "Op6", 6, 12, &[]))
        .add_card(mon("F", "Fill", 3, 4, &[]))
        .add_card(wall())
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .hand(0, &[CARD])
        .memory(10)
        .start()
}

fn activate_prompt(r: &DebugRunner) -> bool {
    r.pending_selection_view()
        .is_some_and(|v| v.prompt.starts_with(ACTIVATE) || v.prompt.starts_with("May accept replacement"))
}

fn jumbo(r: &DebugRunner) -> PermanentHandle {
    let i = field_ids(r, 0).iter().position(|x| x == CARD).expect("JumboGamemon on field");
    PermanentHandle { player: 0, index: i as u8 }
}

fn accept_offer(r: &mut DebugRunner) {
    assert!(activate_prompt(r), "the effect is optional");
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().unwrap();
}

fn pick_from(r: &mut DebugRunner, mut want: Vec<&str>, id: &str) {
    let mut c = candidates(r);
    c.sort();
    want.sort();
    assert_eq!(c, want, "{:?}", r.pending_selection_view().map(|v| v.prompt));
    r.choose_prompt_card(id, None).unwrap();
}

fn jumbo_over(r: &mut DebugRunner, base: &str) -> PermanentHandle {
    let b = r.place_on_field(0, base, Some(0));
    assert!(digivolve_hand(r, CARD, b));
    b
}

fn deck_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids_of(r, &r.game.players[p as usize].deck)
}

#[test]
fn printed_routes_and_aquatic_rule_trait() {
    for base in ["B5", "SQ5"] {
        let mut r = runner();
        let b = r.place_on_field(0, base, Some(0));
        let mem = r.memory();
        assert!(digivolve_hand(&mut r, CARD, b), "{base}");
        assert_eq!(mem - r.memory(), 4, "{base}");
    }
    let mut r = runner();
    let b = r.place_on_field(0, "G5", Some(0));
    assert!(!digivolve_hand(&mut r, CARD, b));
    let h = r.place_on_field(0, CARD, Some(0));
    let data = r.game.card_data_for_handle(r.top_card(h)).unwrap();
    assert!(data.traits.iter().any(|t| t == "Aquatic"));
}

#[test]
fn when_digivolving_places_another_digimon_then_bottom_decks_one_with_as_many_or_fewer_sources() {
    let mut r = runner();
    r.place_on_field(0, "ALLY", Some(0));
    r.place_on_field(1, "O3", Some(0));
    let mid = r.place_stack(1, &["O3", "O4", "O5"]);
    r.place_stack(1, &["O3", "O4", "O5", "O6"]);
    jumbo_over(&mut r, "B5");
    accept_offer(&mut r);
    pick_from(&mut r, vec!["ALLY"], "ALLY");
    assert_eq!(source_ids(&r, jumbo(&r)), vec!["ALLY", "B5"], "placed as the bottom card");
    pick_from(&mut r, vec!["O3", "O5"], "O5");
    assert_eq!(field_ids(&r, 1), vec!["O3", "O6"]);
    let _ = mid;
    assert_eq!(deck_ids(&r, 1)[0], "O5", "bottom of the deck");
    assert_eq!(trash_ids(&r, 1), vec!["O3", "O4"], "its digivolution cards are trashed");
}

#[test]
fn a_tamer_may_be_placed() {
    let mut r = runner();
    r.place_on_field(0, "TAM", Some(0));
    r.place_on_field(1, "O3", Some(0));
    jumbo_over(&mut r, "B5");
    accept_offer(&mut r);
    pick_from(&mut r, vec!["TAM"], "TAM");
    assert_eq!(source_ids(&r, jumbo(&r)), vec!["TAM", "B5"]);
    r.choose_prompt_card("O3", None).unwrap();
    assert!(field_ids(&r, 1).is_empty());
}

#[test]
fn declining_places_nothing_and_without_another_permanent_it_cannot_activate() {
    let mut r = runner();
    r.place_on_field(0, "ALLY", Some(0));
    r.place_on_field(1, "O3", Some(0));
    jumbo_over(&mut r, "B5");
    r.decline_optional_trigger().unwrap();
    assert_eq!(field_ids(&r, 0), vec!["ALLY", CARD]);
    assert_eq!(source_ids(&r, jumbo(&r)), vec!["B5"]);
    assert_eq!(field_ids(&r, 1), vec!["O3"]);

    let mut r = runner();
    r.place_on_field(1, "O3", Some(0));
    jumbo_over(&mut r, "B5");
    assert!(r.pending_selection().is_none(), "no other Digimon or Tamer to place");
}

#[test]
fn with_no_eligible_opponent_digimon_the_placement_still_happens() {
    let mut r = runner();
    r.place_on_field(0, "RED3", Some(0));
    r.place_stack(1, &["O3", "O4", "O5", "O6"]);
    jumbo_over(&mut r, "B5");
    accept_offer(&mut r);
    pick_from(&mut r, vec!["RED3"], "RED3");
    assert_eq!(source_ids(&r, jumbo(&r)), vec!["RED3", "B5"]);
    assert!(r.pending_selection().is_none(), "3 sources > 2, and RED3 is not blue");
    assert_eq!(field_ids(&r, 1), vec!["O6"]);
}

#[test]
fn counter_is_once_per_turn_across_two_attacks() {
    let mut r = runner();
    let me = r.place_stack(0, &["B5", CARD]);
    r.place_on_field(0, "RED3", Some(0));
    r.place_on_field(0, "TAM", Some(0));
    r.pass_turn();
    let first = r.place_on_field(1, "O5", Some(0));
    r.place_on_field(1, "O6", Some(0));
    r.attack_player(first, 0, false);
    let counter = encode_attack(0, me.index as u16);
    r.execute_action(0, counter).unwrap();
    if activate_prompt(&r) {
        accept_offer(&mut r);
    }
    pick_from(&mut r, vec!["RED3", "TAM"], "RED3");
    pick_from(&mut r, vec!["O5", "O6"], "O5");
    while let Some(v) = r.pending_selection_view() {
        assert!(v.is_optional, "{}", v.prompt);
        r.decline_prompt().unwrap();
    }
    let second = PermanentHandle { player: 1, index: field_ids(&r, 1).iter().position(|x| x == "O6").unwrap() as u8 };
    r.set_phase(GamePhase::Main);
    r.game.players[1].battle_area[second.index as usize].is_suspended = false;
    r.attack_player(second, 0, false);
    let me = jumbo(&r);
    let counter = encode_attack(0, me.index as u16);
    assert!(
        r.pending_selection_view().map_or(true, |v| !v.valid_action_ids.contains(&counter)),
        "[Once Per Turn]: no second [Counter] this turn"
    );
}

#[test]
fn counter_in_a_real_attack_bottom_decks_the_attacker() {
    let mut r = runner();
    let me = r.place_stack(0, &["B5", CARD]);
    r.place_on_field(0, "RED3", Some(0));
    r.pass_turn();
    let atk = r.place_on_field(1, "O5", Some(0));
    r.attack_player(atk, 0, false);
    assert_eq!(r.current_phase(), GamePhase::CounterTiming);
    let v = r.pending_selection_view().expect("counter prompt");
    assert_eq!(v.selecting_player, 0);
    assert!(v.is_optional);
    let counter = encode_attack(0, me.index as u16);
    assert!(v.valid_action_ids.contains(&counter));
    r.execute_action(0, counter).unwrap();
    if activate_prompt(&r) {
        accept_offer(&mut r);
    }
    pick_from(&mut r, vec!["RED3"], "RED3");
    pick_from(&mut r, vec!["O5"], "O5");
    assert!(field_ids(&r, 1).is_empty(), "the attacker left the field");
    assert_eq!(deck_ids(&r, 1)[0], "O5");
    assert_eq!(r.security_count(0), 0);
}

#[test]
fn effect_added_sources_may_play_a_blue_cost_5_or_lower_card_from_them() {
    let mut r = runner();
    r.place_on_field(0, "ALLY", Some(0));
    let me = jumbo_over(&mut r, "B5");
    accept_offer(&mut r);
    pick_from(&mut r, vec!["ALLY"], "ALLY");
    let v = r.pending_selection_view().expect("free play from sources");
    assert!(v.is_optional);
    assert!(v.prompt.contains("play cost 5 or lower blue"));
    pick_from(&mut r, vec!["ALLY"], "ALLY");
    let mut f = field_ids(&r, 0);
    f.sort();
    assert_eq!(f, vec!["ALLY", CARD]);
    assert_eq!(source_ids(&r, jumbo(&r)), vec!["B5"]);
}

#[test]
fn the_source_play_is_optional_and_filters_colour_and_cost() {
    let mut r = runner();
    r.place_on_field(0, "ALLY", Some(0));
    let me = jumbo_over(&mut r, "B5");
    accept_offer(&mut r);
    pick_from(&mut r, vec!["ALLY"], "ALLY");
    r.decline_prompt().unwrap();
    assert_eq!(source_ids(&r, jumbo(&r)), vec!["ALLY", "B5"]);

    let mut r = runner();
    r.place_on_field(0, "RED3", Some(0));
    jumbo_over(&mut r, "B5");
    accept_offer(&mut r);
    pick_from(&mut r, vec!["RED3"], "RED3");
    assert!(r.pending_selection().is_none(), "B5 costs 7, RED3 is red");
}

#[test]
fn the_source_play_is_once_per_turn() {
    let mut r = runner();
    r.place_on_field(0, "ALLY", Some(0));
    let me = jumbo_over(&mut r, "B5");
    accept_offer(&mut r);
    pick_from(&mut r, vec!["ALLY"], "ALLY");
    pick_from(&mut r, vec!["ALLY"], "ALLY");
    let again = r.place_on_field(0, "ALLY", Some(0));
    let me = jumbo(&r);
    let card = r.top_card(me);
    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(&mut r.game, card, Some(me), 0);
        ctx.place_permanent_as_bottom_sources(again, me);
    }
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none(), "once per turn");
}

#[test]
fn decode_plays_a_lv5_aquatic_or_data_squad_source_when_removed_by_an_effect() {
    let mut r = runner();
    let me = r.place_stack(0, &["B5", "AQ5", "SQ5", CARD]);
    let opp = r.place_on_field(1, "O6", Some(0));
    opponent_bounces(&mut r, opp, me);
    accept_offer(&mut r);
    pick_from(&mut r, vec!["AQ5", "SQ5"], "AQ5");
    assert_eq!(field_ids(&r, 0), vec!["AQ5"]);
    assert!(hand_ids(&r, 0).contains(&CARD.to_string()));
}

#[test]
fn decode_may_be_declined_and_does_not_apply_in_battle() {
    let mut r = runner();
    let me = r.place_stack(0, &["B5", "AQ5", CARD]);
    let opp = r.place_on_field(1, "O6", Some(0));
    opponent_bounces(&mut r, opp, me);
    assert!(activate_prompt(&r));
    r.decline_optional_trigger().unwrap();
    assert!(field_ids(&r, 0).is_empty());

    let mut r = runner();
    r.place_stack(0, &["AQ5", CARD]);
    lose_battle(&mut r, 0);
    while let Some(v) = r.pending_selection_view() {
        assert!(!v.prompt.contains("Decode") && !v.prompt.contains("Aquatic"), "{}", v.prompt);
        let a = if v.valid_action_ids.contains(&PASS) { PASS } else { v.valid_action_ids[0] };
        r.execute_action(v.selecting_player, a).unwrap();
    }
    assert!(field_ids(&r, 0).is_empty());
}
