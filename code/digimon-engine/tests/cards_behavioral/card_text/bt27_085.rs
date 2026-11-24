//! Source: `data/card_bundles/BT27-085.md` (printed text and official Q&A).
//!
//! <Piercing> <Blocker>
//! [When Digivolving] [When Attacking] You may activate 1 [Main] effect on this
//! card's Option side.
//! [All Turns] [Once Per Turn] When your opponent's hand is trashed from, this
//! Digimon may unsuspend.
//! Option side: <Use Req. ([DATA SQUAD] trait)> [Main] Your opponent trashes
//! any 1 card from their hand or security stack. Then, delete 1 of their
//! Digimon with a level as high as the number of cards in their hand or
//! higher. <Arts Digivolve>

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectSourceKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-085";

fn colored(mut c: digimon_engine::card_data::CardData, color: CardColor) -> digimon_engine::card_data::CardData {
    c.colors = vec![color];
    c
}

fn runner(opp_hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(colored(mon("PUR6", "Purple", 6, 9, &[]), CardColor::Purple))
        .add_card(colored(mon("GRN6", "Green", 6, 9, &[]), CardColor::Green))
        .add_card(colored(mon("SQ6", "Squad", 6, 9, &["DATA SQUAD"]), CardColor::Red))
        .add_card(colored(mon("RED6", "Red", 6, 9, &[]), CardColor::Red))
        .add_card(tamer("T", "Squad Tamer", &["DATA SQUAD"]))
        .add_card(mon("L3", "Three", 3, 3, &[]))
        .add_card(mon("L4", "Four", 4, 5, &[]))
        .add_card(mon("L6", "Six", 6, 9, &[]))
        .add_card(mon("H1", "HandA", 3, 3, &[]))
        .add_card(mon("H2", "HandB", 3, 3, &[]))
        .add_card(mon("SEC", "Sec", 3, 3, &[]))
        .add_card(mon("F", "Fill", 3, 3, &[]))
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .security(1, &["SEC", "SEC"])
        .hand(0, &[CARD])
        .hand(1, opp_hand)
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

/// The opponent answers the hand-or-security choice.
fn opponent_chooses(r: &mut DebugRunner, security: bool) {
    let v = r.pending_selection_view().expect("hand or security choice");
    assert_eq!(v.kind, SelectionKind::EffectChoice);
    assert_eq!(v.selecting_player, 1, "the opponent chooses");
    assert!(!v.is_optional);
    let c = v.effect_choices.unwrap();
    assert_eq!(c.len(), 2);
    do_action(r, c[usize::from(security)].action_id);
}

fn opponent_trashes_hand(r: &mut DebugRunner, id: &str) {
    let v = r.pending_selection_view().expect("hand pick");
    assert_eq!(v.selecting_player, 1);
    assert!(!v.is_optional);
    r.choose_prompt_card(id, None).unwrap();
}

fn opponent_picks_first(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("security pick");
    assert_eq!(v.selecting_player, 1);
    assert!(!v.is_optional);
    let a = *v.valid_action_ids.iter().find(|a| **a != PASS).unwrap();
    do_action(r, a);
}

fn evolve_card(r: &mut DebugRunner) -> PermanentHandle {
    let b = r.place_on_field(0, "PUR6", Some(0));
    assert!(digivolve_hand(r, CARD, b));
    b
}

fn find(r: &DebugRunner, p: u8, id: &str) -> PermanentHandle {
    let i = field_ids(r, p).iter().position(|x| x == id).expect("on field");
    PermanentHandle { player: p, index: i as u8 }
}

fn decline_optionals(r: &mut DebugRunner) {
    for _ in 0..10 {
        let Some(v) = r.pending_selection_view() else { return };
        assert!(v.is_optional, "unexpected mandatory prompt: {}", v.prompt);
        do_action(r, PASS);
    }
}

#[test]
fn printed_keywords_are_piercing_and_blocker() {
    let mut r = runner(&[]);
    let h = r.place_stack(0, &["PUR6", CARD]);
    for k in [Keyword::Piercing, Keyword::Blocker] {
        assert!(r.game.has_keyword(h, k), "{k:?}");
    }
}

#[test]
fn digivolves_for_5_from_purple_green_or_data_squad_level_6_only() {
    for base in ["PUR6", "GRN6", "SQ6"] {
        let mut r = runner(&[]);
        let b = r.place_on_field(0, base, Some(0));
        let mem = r.memory();
        assert!(digivolve_hand(&mut r, CARD, b), "{base}");
        decline_optionals(&mut r);
        assert_eq!(top_id(&r, b), CARD);
        assert_eq!(mem - r.memory(), 5, "{base}");
    }
    let mut r = runner(&[]);
    let b = r.place_on_field(0, "RED6", Some(0));
    assert!(!digivolve_hand(&mut r, CARD, b));
}

#[test]
fn when_digivolving_hand_branch_then_deletes_a_digimon_at_or_above_the_hand_size() {
    let mut r = runner(&["H1", "H2", "F", "F", "F"]);
    for id in ["L3", "L4", "L6"] {
        r.place_on_field(1, id, Some(0));
    }
    evolve_card(&mut r);
    accept_offer(&mut r);
    assert_eq!(r.pending_selection_view().unwrap().source_kind, EffectSourceKind::Option);
    opponent_chooses(&mut r, false);
    opponent_trashes_hand(&mut r, "H2");
    assert!(trash_ids(&r, 1).contains(&"H2".to_string()));
    assert_eq!(r.hand_size(1), 4);
    let v = r.pending_selection_view().expect("mandatory delete");
    assert_eq!(v.selecting_player, 0);
    assert!(!v.is_optional);
    let mut c = candidates(&r);
    c.sort();
    assert_eq!(c, vec!["L4", "L6"], "level 4 or higher with 4 cards in hand");
    r.choose_prompt_card("L4", None).unwrap();
    assert!(trash_ids(&r, 1).contains(&"L4".to_string()));
    assert_eq!(r.game.players[1].security.len(), 2);
}

#[test]
fn security_branch_trashes_a_security_card_and_keeps_the_hand() {
    let mut r = runner(&["H1", "H2", "F", "F", "F", "F", "F"]);
    r.place_on_field(1, "L6", Some(0));
    r.place_on_field(1, "L4", Some(0));
    evolve_card(&mut r);
    accept_offer(&mut r);
    opponent_chooses(&mut r, true);
    opponent_picks_first(&mut r);
    assert_eq!(r.game.players[1].security.len(), 1);
    assert_eq!(r.hand_size(1), 7);
    assert!(trash_ids(&r, 1).contains(&"SEC".to_string()));
    assert!(r.pending_selection().is_none(), "no Digimon is level 7 or higher");
    assert_eq!(field_ids(&r, 1).len(), 2);
}

#[test]
fn empty_hand_leaves_only_the_security_branch_and_empty_hand_lets_any_level_be_deleted() {
    let mut r = runner(&[]);
    r.place_on_field(1, "L3", Some(0));
    evolve_card(&mut r);
    accept_offer(&mut r);
    opponent_picks_first(&mut r);
    assert_eq!(r.game.players[1].security.len(), 1);
    assert_eq!(candidates(&r), vec!["L3"]);
    r.choose_prompt_card("L3", None).unwrap();
    assert!(field_ids(&r, 1).is_empty());
}

#[test]
fn empty_security_leaves_only_the_hand_branch() {
    let mut r = runner(&["H1"]);
    r.game.players[1].security.clear();
    evolve_card(&mut r);
    accept_offer(&mut r);
    opponent_trashes_hand(&mut r, "H1");
    assert_eq!(r.hand_size(1), 0);
}

#[test]
fn declining_the_activation_does_nothing() {
    let mut r = runner(&["H1"]);
    r.place_on_field(1, "L3", Some(0));
    evolve_card(&mut r);
    do_action(&mut r, PASS);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(1), 1);
    assert_eq!(r.game.players[1].security.len(), 2);
    assert_eq!(field_ids(&r, 1), vec!["L3"]);
}

#[test]
fn when_attacking_activates_again_and_the_hand_trash_unsuspends_it_once_per_turn() {
    let mut r = runner(&["H1", "H2", "F", "F", "F", "F", "F", "F", "F"]);
    evolve_card(&mut r);
    accept_offer(&mut r);
    opponent_chooses(&mut r, false);
    opponent_trashes_hand(&mut r, "H1");
    assert!(r.pending_selection().is_none(), "not suspended: no unsuspend offer");
    let h = find(&r, 0, CARD);
    attack_security(&mut r, h.index);
    accept_offer(&mut r);
    opponent_chooses(&mut r, false);
    opponent_trashes_hand(&mut r, "H2");
    let v = r.pending_selection_view().expect("may unsuspend");
    assert!(v.is_optional && v.selecting_player == 0);
    accept_offer(&mut r);
    assert!(!r.game.players[0].battle_area[find(&r, 0, CARD).index as usize].is_suspended);
    decline_optionals(&mut r);
    while r.pending_selection().is_some() {
        r.decline_prompt().unwrap();
    }
    no_prompt(&r);
    let h = find(&r, 0, CARD);
    attack_security(&mut r, h.index);
    accept_offer(&mut r);
    opponent_chooses(&mut r, false);
    let before = r.hand_size(1);
    opponent_picks_first(&mut r);
    assert_eq!(r.hand_size(1), before - 1, "hand card trashed");
    assert!(
        r.pending_selection_view().is_none_or(|v| !v.prompt.to_lowercase().contains("unsuspend")),
        "once per turn"
    );
}

#[test]
fn declining_unsuspend_keeps_it_suspended() {
    let mut r = runner(&["H1", "F", "F", "F", "F", "F", "F", "F", "F"]);
    let h = r.place_stack(0, &["PUR6", CARD]);
    attack_security(&mut r, h.index);
    accept_offer(&mut r);
    opponent_chooses(&mut r, false);
    opponent_trashes_hand(&mut r, "H1");
    do_action(&mut r, PASS);
    assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
}

#[test]
fn own_hand_trash_does_not_unsuspend() {
    let mut r = runner(&[]);
    let h = r.place_stack(0, &["PUR6", CARD]);
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    r.add_to_hand(0, "F");
    let i = hand_index(&r, 0, "F");
    let card = r.top_card(h);
    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(&mut r.game, card, Some(h), 0);
        let _ = ctx.trash_from_hand_by_index(0, i);
    }
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none());
    assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
}

#[test]
fn using_the_option_side_needs_data_squad_and_pays_6() {
    let mut r = runner(&["H1"]);
    r.place_on_field(1, "L3", Some(0));
    let _ = r.game.play_option_from_hand(0, hand_index(&r, 0, CARD));
    assert!(r.pending_selection().is_none() && hand_ids(&r, 0) == vec![CARD], "no [DATA SQUAD]");

    let mut r = runner(&["H1"]);
    r.place_on_field(0, "T", None);
    r.place_on_field(1, "L3", Some(0));
    let mem = r.memory();
    let _ = r.game.play_option_from_hand(0, hand_index(&r, 0, CARD));
    opponent_chooses(&mut r, false);
    opponent_trashes_hand(&mut r, "H1");
    r.choose_prompt_card("L3", None).unwrap();
    assert_eq!(mem - r.memory(), 6);
    assert!(field_ids(&r, 1).is_empty());
}

mod burst {
    use crate::burst_digivolve_common::*;
    use digimon_engine::enums::CardColor;

    const SPEC: BurstSpec = BurstSpec {
        card: "BT27-085",
        host_name: "Ravemon",
        host_color: CardColor::Purple,
        tamer_name: "Keenan Crier",
    };

    #[test]
    fn burst_returns_keenan_costs_zero_draws_one() {
        burst_and_check(SPEC, Host::BurstOnly);
    }

    #[test]
    fn burst_needs_keenan() {
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
