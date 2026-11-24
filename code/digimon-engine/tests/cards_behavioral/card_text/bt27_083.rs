//! Source: `data/card_bundles/BT27-083.md` (printed text and official Q&A).
//!
//! <Blocker> <Retaliation>
//! [When Digivolving] Your opponent trashes any 1 card from their hand or
//! security stack.
//! [On Deletion] If your opponent's hand has 7 or fewer cards, you may play 1
//! play cost 7 or lower purple Digimon card with [Avian] or [Bird] in any of
//! its traits from your trash without paying the cost.
//! (Rule) Trait: Has [Birdkin] Type.
//! Option side: <Use Req. ([DATA SQUAD] trait)> [Main] Your opponent trashes
//! cards in their hand so that they have 7 left. If this effect didn't trash,
//! delete 1 of their Digimon. <Arts Digivolve>
//! Q&A: the Option side is treated as an Option card with the [DATA SQUAD] trait.

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-083";

fn colored(mut c: CardData, color: CardColor) -> CardData {
    c.colors = vec![color];
    c
}

fn costed(mut c: CardData, cost: u16) -> CardData {
    c.play_cost = cost;
    c
}

fn runner(opp_hand: usize) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(colored(mon("PUR5", "Purple", 5, 7, &[]), CardColor::Purple))
        .add_card(colored(mon("GRN5", "Green", 5, 7, &[]), CardColor::Green))
        .add_card(colored(mon("SQ5", "Squad", 5, 7, &["DATA SQUAD"]), CardColor::Red))
        .add_card(colored(mon("CROW", "Crowmon", 5, 7, &[]), CardColor::Red))
        .add_card(colored(costed(mon("AV7", "Av", 5, 7, &["Avian"]), 7), CardColor::Purple))
        .add_card(colored(costed(mon("BIRD", "Bd", 4, 5, &["Birdkin"]), 5), CardColor::Purple))
        .add_card(colored(costed(mon("AV8", "Av8", 6, 8, &["Avian"]), 8), CardColor::Purple))
        .add_card(colored(costed(mon("GAV", "GAv", 4, 5, &["Avian"]), 5), CardColor::Green))
        .add_card(colored(costed(mon("PNO", "Plain", 4, 5, &["Beast"]), 5), CardColor::Purple))
        .add_card(tamer("T", "Squad Tamer", &["DATA SQUAD"]))
        .add_card(mon("L3", "Three", 3, 3, &[]))
        .add_card(mon("H1", "HandA", 3, 3, &[]))
        .add_card(mon("SEC", "Sec", 3, 3, &[]))
        .add_card(mon("F", "Fill", 3, 3, &[]))
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .security(1, &["SEC", "SEC"])
        .hand(0, &[CARD])
        .hand(1, &vec!["F"; opp_hand])
        .memory(10)
        .start();
    for id in ["AV7", "BIRD", "AV8", "GAV", "PNO"] {
        r.inject_trash(0, id);
    }
    r
}

fn pick(r: &mut DebugRunner, a: u16) {
    let p = r.pending_selection_view().unwrap().selecting_player;
    assert_eq!(build_action_mask(&r.game, p)[a as usize], 1.0);
    r.execute_action(p, a).unwrap();
}

fn pick_first(r: &mut DebugRunner) {
    let a = *r.pending_selection_view().unwrap().valid_action_ids.iter().find(|a| **a != PASS).unwrap();
    pick(r, a);
}

fn opponent_chooses(r: &mut DebugRunner, security: bool) {
    let v = r.pending_selection_view().expect("hand or security choice");
    assert_eq!(v.kind, SelectionKind::EffectChoice);
    assert_eq!(v.selecting_player, 1, "the opponent chooses");
    assert!(!v.is_optional);
    let c = v.effect_choices.unwrap();
    assert_eq!(c.len(), 2);
    pick(r, c[usize::from(security)].action_id);
}

fn opponent_deletes(r: &mut DebugRunner, target: PermanentHandle) {
    use digimon_engine::effect_context::EffectContext;
    let src = r.place_on_field(1, "L3", Some(0));
    let card = r.top_card(src);
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, card, Some(src), 1);
        ctx.delete_permanent(target);
    }
    r.game.set_effect_source_player_for_test(None);
    r.game.drain_effect_queue();
}

#[test]
fn printed_keywords_and_birdkin_rule_trait() {
    let mut r = runner(0);
    let h = r.place_stack(0, &["PUR5", CARD]);
    for k in [Keyword::Blocker, Keyword::Retaliation] {
        assert!(r.game.has_keyword(h, k), "{k:?}");
    }
    assert!(printed(CARD).traits.iter().any(|t| t == "Birdkin"));
}

#[test]
fn digivolves_for_3_from_purple_level_5_crowmon_or_data_squad_level_5() {
    for base in ["PUR5", "CROW", "SQ5"] {
        let mut r = runner(0);
        let b = r.place_on_field(0, base, Some(0));
        let mem = r.memory();
        assert!(digivolve_hand(&mut r, CARD, b), "{base}");
        while r.pending_selection().is_some() {
            pick_first(&mut r);
        }
        assert_eq!(top_id(&r, b), CARD);
        assert_eq!(mem - r.memory(), 3, "{base}");
    }
    let mut r = runner(0);
    let b = r.place_on_field(0, "GRN5", Some(0));
    assert!(!digivolve_hand(&mut r, CARD, b));
}

#[test]
fn when_digivolving_opponent_trashes_a_hand_card() {
    let mut r = runner(0);
    r.add_to_hand(1, "H1");
    r.add_to_hand(1, "F");
    let b = r.place_on_field(0, "PUR5", Some(0));
    assert!(digivolve_hand(&mut r, CARD, b));
    opponent_chooses(&mut r, false);
    let v = r.pending_selection_view().unwrap();
    assert_eq!(v.selecting_player, 1);
    assert!(!v.is_optional);
    r.choose_prompt_card("H1", None).unwrap();
    assert_eq!(hand_ids(&r, 1), vec!["F"]);
    assert!(trash_ids(&r, 1).contains(&"H1".to_string()));
    assert_eq!(r.game.players[1].security.len(), 2);
}

#[test]
fn when_digivolving_opponent_may_trash_a_security_card_instead() {
    let mut r = runner(3);
    let b = r.place_on_field(0, "PUR5", Some(0));
    assert!(digivolve_hand(&mut r, CARD, b));
    opponent_chooses(&mut r, true);
    assert_eq!(r.pending_selection_view().unwrap().selecting_player, 1);
    pick_first(&mut r);
    assert_eq!(r.game.players[1].security.len(), 1);
    assert_eq!(r.hand_size(1), 3);
    assert!(trash_ids(&r, 1).contains(&"SEC".to_string()));
}

#[test]
fn when_digivolving_with_empty_hand_trashes_security_without_a_choice() {
    let mut r = runner(0);
    let b = r.place_on_field(0, "PUR5", Some(0));
    assert!(digivolve_hand(&mut r, CARD, b));
    let v = r.pending_selection_view().unwrap();
    assert_ne!(v.kind, SelectionKind::EffectChoice);
    pick_first(&mut r);
    assert_eq!(r.game.players[1].security.len(), 1);
}

#[test]
fn on_deletion_at_7_may_play_a_purple_avian_or_bird_cost_7_or_lower_for_free() {
    let mut r = runner(7);
    let h = r.place_stack(0, &["PUR5", CARD]);
    let mem = r.memory();
    opponent_deletes(&mut r, h);
    let v = r.pending_selection_view().expect("may play from trash");
    assert!(v.is_optional && v.selecting_player == 0);
    let mut c = candidates(&r);
    c.sort();
    assert_eq!(c, vec!["AV7", "BIRD"], "purple, cost 7 or lower, [Avian]/[Bird] in a trait");
    r.choose_prompt_card("BIRD", None).unwrap();
    assert!(field_ids(&r, 0).contains(&"BIRD".to_string()));
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[test]
fn on_deletion_may_decline_and_does_nothing_at_8_cards() {
    let mut r = runner(7);
    let h = r.place_stack(0, &["PUR5", CARD]);
    opponent_deletes(&mut r, h);
    pick(&mut r, PASS);
    assert!(!field_ids(&r, 0).contains(&"AV7".to_string()));

    let mut r = runner(8);
    let h = r.place_stack(0, &["PUR5", CARD]);
    opponent_deletes(&mut r, h);
    assert!(r.pending_selection().is_none(), "8 cards: no play");
    assert!(trash_ids(&r, 0).contains(&"AV7".to_string()));
}

#[test]
fn option_main_trims_the_opponent_hand_to_7_and_does_not_delete() {
    let mut r = runner(9);
    r.place_on_field(0, "T", None);
    r.place_on_field(1, "L3", Some(0));
    let mem = r.memory();
    let _ = r.game.play_option_from_hand(0, hand_index(&r, 0, CARD));
    let v = r.pending_selection_view().expect("opponent picks discards");
    assert_eq!(v.selecting_player, 1, "the opponent chooses what to trash");
    while r.pending_selection_view().is_some_and(|v| v.selecting_player == 1) {
        pick_first(&mut r);
    }
    assert_eq!(r.hand_size(1), 7);
    assert_eq!(mem - r.memory(), 5);
    assert!(r.pending_selection_view().is_none_or(|v| !v.prompt.contains("Delete")), "it trashed");
    assert_eq!(field_ids(&r, 1), vec!["L3"]);
}

#[test]
fn option_main_at_7_or_fewer_deletes_one_opponent_digimon() {
    for n in [7usize, 3] {
        let mut r = runner(n);
        r.place_on_field(0, "T", None);
        r.place_on_field(1, "L3", Some(0));
        let _ = r.game.play_option_from_hand(0, hand_index(&r, 0, CARD));
        let v = r.pending_selection_view().expect("mandatory delete");
        assert_eq!(v.selecting_player, 0);
        assert!(!v.is_optional);
        r.choose_prompt_card("L3", None).unwrap();
        assert!(field_ids(&r, 1).is_empty(), "{n}");
        assert_eq!(r.hand_size(1), n);
    }
}

#[test]
fn using_the_option_side_needs_data_squad() {
    let mut r = runner(9);
    r.place_on_field(1, "L3", Some(0));
    let _ = r.game.play_option_from_hand(0, hand_index(&r, 0, CARD));
    assert!(r.pending_selection().is_none() && hand_ids(&r, 0) == vec![CARD]);
}
