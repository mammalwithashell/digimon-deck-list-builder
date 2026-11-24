//! Source: `data/card_bundles/BT27-053.md` (printed text and official Q&A).
//!
//! When this card would be played, if your opponent has a Digimon with 11000
//! DP or more, reduce the cost by 6. <Blocker> <Fortitude>
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] This Digimon
//! may battle 1 of your opponent's Digimon.
//! [On Deletion] You may play or use 1 Tamer or Option card with the [Boss] or
//! [DATA SQUAD] trait from your hand without paying the cost.
//! Q&A: the effect immediately initiates a battle under the standard rules.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-053";

fn colored(mut c: CardData, color: CardColor) -> CardData {
    c.colors = vec![color];
    c
}

fn option(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = 4;
    c.colors = vec![CardColor::Yellow];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn runner(hand: &[&str]) -> DebugRunner {
    let mut big = mon("O11", "Op11", 6, 12, &[]);
    big.dp = Some(11000);
    let mut near = mon("O10", "Op10", 6, 12, &[]);
    near.dp = Some(10000);
    let mut mine11 = mon("M11", "Mine11", 6, 12, &[]);
    mine11.dp = Some(11000);
    let mut h = vec![CARD];
    h.extend_from_slice(hand);
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(colored(mon("G5", "Green5", 5, 7, &[]), CardColor::Green))
        .add_card(colored(mon("Y5", "Yellow5", 5, 7, &[]), CardColor::Yellow))
        .add_card(colored(mon("LEO5", "GrapLeomon", 5, 7, &[]), CardColor::Red))
        .add_card(colored(mon("SQ5", "Squad5", 5, 7, &["DATA SQUAD"]), CardColor::Red))
        .add_card(colored(mon("R5", "Red5", 5, 7, &[]), CardColor::Red))
        .add_card(colored(mon("ALLY", "Ally", 3, 3, &[]), CardColor::Yellow))
        .add_card(colored(tamer("BOSS_T", "BossTamer", &["Boss"]), CardColor::Yellow))
        .add_card(colored(tamer("SQ_T", "SquadTamer", &["DATA SQUAD"]), CardColor::Yellow))
        .add_card(colored(tamer("PLAIN_T", "PlainTamer", &[]), CardColor::Yellow))
        .add_card(colored(mon("BOSS_M", "BossMon", 3, 3, &["Boss"]), CardColor::Yellow))
        .add_card(option("SQ_O", &["DATA SQUAD"]))
        .add_card(option("PLAIN_O", &[]))
        .add_card(mon("O5", "Op5", 5, 7, &[]))
        .add_card(big)
        .add_card(near)
        .add_card(mine11)
        .add_card(mon("F", "Fill", 3, 4, &[]))
        .add_card(wall())
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .security(1, &["F"; 3])
        .hand(0, &h)
        .memory(10)
        .start()
}

fn me(r: &DebugRunner) -> PermanentHandle {
    let i = field_ids(r, 0).iter().position(|x| x == CARD).expect("BanchoLeomon on field");
    PermanentHandle { player: 0, index: i as u8 }
}

fn offered(r: &DebugRunner) -> bool {
    r.pending_selection_view().is_some_and(|v| v.prompt.starts_with("You may activate"))
}

fn accept_battle(r: &mut DebugRunner, mut want: Vec<&str>, target: &str) {
    assert!(offered(r), "the battle is optional");
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().unwrap();
    let mut c = candidates(r);
    c.sort();
    want.sort();
    assert_eq!(c, want);
    r.choose_prompt_card(target, None).unwrap();
}

fn digivolve_over(r: &mut DebugRunner, base: &str) {
    let b = r.place_on_field(0, base, Some(0));
    assert!(digivolve_hand(r, CARD, b), "{base}");
}

#[test]
fn printed_routes() {
    for (base, cost) in [("G5", 5), ("Y5", 5), ("LEO5", 4), ("SQ5", 4)] {
        let mut r = runner(&[]);
        let mem = r.memory();
        digivolve_over(&mut r, base);
        assert_eq!(mem - r.memory(), cost, "{base}");
    }
    let mut r = runner(&[]);
    let b = r.place_on_field(0, "R5", Some(0));
    assert!(!digivolve_hand(&mut r, CARD, b));
}

#[test]
fn play_cost_is_reduced_by_6_only_when_an_opponent_digimon_has_11000_dp_or_more() {
    for (opp, mine, paid) in [(Some("O11"), None, 7), (Some("O10"), None, 13), (None, Some("M11"), 13)] {
        let mut r = runner(&[]);
        if let Some(o) = opp {
            r.place_on_field(1, o, Some(0));
        }
        if let Some(m) = mine {
            r.place_on_field(0, m, Some(0));
        }
        let mem = r.memory();
        play_hand(&mut r, CARD);
        assert_eq!(mem - r.memory(), paid, "{opp:?} {mine:?}");
    }
}

#[test]
fn blocker_and_fortitude() {
    let mut r = runner(&[]);
    let h = r.place_on_field(0, CARD, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Blocker));
    assert!(r.game.has_keyword(h, Keyword::Fortitude));
}

#[test]
fn fortitude_replays_it_from_the_trash_when_deleted_with_digivolution_cards() {
    let mut r = runner(&[]);
    r.place_stack(0, &["G5", CARD]);
    lose_battle(&mut r, 0);
    assert!(offered(&r), "When Attacking");
    r.decline_optional_trigger().unwrap();
    let v = r.pending_selection_view().expect("On Deletion and Fortitude both trigger");
    let choices = v.effect_choices.clone().expect("trigger order");
    assert_eq!(choices.len(), 2);
    let on_deletion = choices.iter().find(|c| c.label.contains("mandatory")).unwrap().action_id;
    r.execute_action(0, on_deletion).unwrap();
    assert_eq!(field_ids(&r, 0), vec![CARD], "Fortitude plays it back without a prompt");
    assert!(offered(&r), "the replayed card's On Play");
    r.decline_optional_trigger().unwrap();
    assert!(r.pending_selection().is_none());
    assert!(trash_ids(&r, 0).contains(&"G5".to_string()));
}

#[test]
fn on_play_battles_a_chosen_opponent_digimon_under_standard_rules() {
    let mut r = runner(&[]);
    r.place_on_field(1, "O5", Some(0));
    r.place_on_field(1, "O10", Some(0));
    play_hand(&mut r, CARD);
    accept_battle(&mut r, vec!["O5", "O10"], "O10");
    assert_eq!(field_ids(&r, 1), vec!["O5"], "13000 beats 10000");
    assert!(trash_ids(&r, 1).contains(&"O10".to_string()));
    assert!(!r.game.players[0].battle_area[me(&r).index as usize].is_suspended, "not an attack");
    assert!(r.pending_selection().is_none());
}

#[test]
fn losing_the_battle_deletes_it_and_triggers_on_deletion() {
    let mut r = runner(&["SQ_T"]);
    r.place_on_field(1, "WALL", Some(0));
    play_hand(&mut r, CARD);
    accept_battle(&mut r, vec!["WALL"], "WALL");
    assert!(field_ids(&r, 0).is_empty());
    assert_eq!(candidates(&r), vec!["SQ_T"], "On Deletion");
    r.choose_prompt_card("SQ_T", None).unwrap();
    assert_eq!(field_ids(&r, 0), vec!["SQ_T"]);
}

#[test]
fn declining_or_no_opponent_digimon_means_no_battle() {
    let mut r = runner(&[]);
    r.place_on_field(1, "O5", Some(0));
    play_hand(&mut r, CARD);
    r.decline_optional_trigger().unwrap();
    assert_eq!(field_ids(&r, 1), vec!["O5"]);

    let mut r = runner(&[]);
    play_hand(&mut r, CARD);
    assert!(r.pending_selection().is_none());
}

#[test]
fn when_digivolving_then_when_attacking_is_once_per_turn() {
    let mut r = runner(&[]);
    r.place_on_field(1, "O5", Some(0));
    r.place_on_field(1, "ALLY", Some(0));
    digivolve_over(&mut r, "G5");
    accept_battle(&mut r, vec!["O5", "ALLY"], "O5");
    assert_eq!(field_ids(&r, 1), vec!["ALLY"]);
    let i = me(&r).index;
    attack_security(&mut r, i);
    assert!(!offered(&r), "already used this turn");
}

#[test]
fn declining_when_digivolving_leaves_when_attacking_available() {
    let mut r = runner(&[]);
    r.place_on_field(1, "O5", Some(0));
    digivolve_over(&mut r, "G5");
    r.decline_optional_trigger().unwrap();
    let i = me(&r).index;
    attack_security(&mut r, i);
    accept_battle(&mut r, vec!["O5"], "O5");
    assert!(field_ids(&r, 1).is_empty());
}

#[test]
fn when_attacking_battles_before_the_attack_continues() {
    let mut r = runner(&[]);
    let h = r.place_on_field(0, CARD, Some(0));
    r.place_on_field(1, "O5", Some(0));
    attack_security(&mut r, h.index);
    accept_battle(&mut r, vec!["O5"], "O5");
    assert!(field_ids(&r, 1).is_empty());
    assert!(trash_ids(&r, 1).contains(&"O5".to_string()));
}

#[test]
fn on_deletion_plays_a_boss_or_data_squad_tamer_free() {
    let mut r = runner(&["BOSS_T", "SQ_T", "PLAIN_T", "BOSS_M", "SQ_O", "PLAIN_O"]);
    r.place_on_field(0, CARD, Some(0));
    r.place_on_field(0, "ALLY", Some(0));
    lose_battle(&mut r, 0);
    if offered(&r) {
        r.decline_optional_trigger().unwrap();
    }
    let v = r.pending_selection_view().expect("On Deletion");
    assert!(v.is_optional);
    let mut c = candidates(&r);
    c.sort();
    assert_eq!(c, vec!["BOSS_T", "SQ_O", "SQ_T"]);
    let mem = r.memory();
    r.choose_prompt_card("BOSS_T", None).unwrap();
    assert_eq!(r.memory(), mem, "without paying the cost");
    assert!(field_ids(&r, 0).contains(&"BOSS_T".to_string()));
}

#[test]
fn on_deletion_uses_a_data_squad_option_free() {
    let mut r = runner(&["SQ_O"]);
    r.place_on_field(0, CARD, Some(0));
    r.place_on_field(0, "ALLY", Some(0));
    lose_battle(&mut r, 0);
    if offered(&r) {
        r.decline_optional_trigger().unwrap();
    }
    let mem = r.memory();
    r.choose_prompt_card("SQ_O", None).unwrap();
    while r.pending_selection().is_some() {
        r.decline_prompt().unwrap();
    }
    assert_eq!(r.memory(), mem);
    assert!(!hand_ids(&r, 0).contains(&"SQ_O".to_string()));
    assert!(trash_ids(&r, 0).contains(&"SQ_O".to_string()));
}

#[test]
fn on_deletion_may_be_declined_and_ignores_ineligible_cards() {
    let mut r = runner(&["SQ_T"]);
    r.place_on_field(0, CARD, Some(0));
    lose_battle(&mut r, 0);
    if offered(&r) {
        r.decline_optional_trigger().unwrap();
    }
    r.decline_prompt().unwrap();
    assert!(hand_ids(&r, 0).contains(&"SQ_T".to_string()));

    let mut r = runner(&["PLAIN_T", "PLAIN_O", "BOSS_M"]);
    r.place_on_field(0, CARD, Some(0));
    lose_battle(&mut r, 0);
    if offered(&r) {
        r.decline_optional_trigger().unwrap();
    }
    assert!(r.pending_selection().is_none());
}
