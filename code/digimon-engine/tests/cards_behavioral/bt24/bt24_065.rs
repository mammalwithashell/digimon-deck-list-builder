//! BT24-065 Diaboromon (X Antibody) — Digimon, Lv.6, Black, DP 13000, Cost 13
//! (Unidentified, X Antibody).
//!
//! Printed (official Bandai DB bundle + DCGO BT24_065.cs):
//! - Evo box: Black Lv.5 / cost 5; "[Digivolve] [Diaboromon]: Cost 2" (exact
//!   name, any level).
//! - <Overclock ([Unidentified] Trait)> <Blocker>
//! - [When Digivolving] To 1 of your opponent's Digimon, <De-Digivolve 1> for each
//!   of your Digimon. Then, delete all of your opponent's Digimon with the highest
//!   play cost.
//! - [All Turns][Once Per Turn] When any of your Digimon with [Diaboromon] in
//!   their names would leave the battle area, you may play 1 [Diaboromon] from
//!   your hand or this Digimon's digivolution cards without paying the cost.
//!
//! Regression (hunt2 cluster alt-digivolve-cost-cards): no YAML spec, so a Lv.5
//! [Diaboromon] base could not digivolve into it for cost 2 (and a Lv.6
//! [Diaboromon] had no legal digivolve at all — illegal_action). Substrate
//! widened: `aggregate: highest_play_cost`. Synthetic boards only.

use crate::alt_digivolve_support::*;
use digimon_engine::action::{build_action_mask, space::{encode_digivolve, PASS}};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT24-065";

// ─── Digivolution routes ──────────────────────────────────────────────────────

#[test]
fn bt24_065_black_lv5_circle_costs_5_without_prompt() {
    assert_single_route(ID, base("B5", "Black Ultimate", 5, &[BLACK], &["Unidentified"]), 5);
}

#[test]
fn bt24_065_diaboromon_named_base_of_any_level_costs_2() {
    assert_single_route(ID, base("D6", "Diaboromon", 6, &[RED], &["Unidentified"]), 2);
    assert_single_route(ID, base("D5", "Diaboromon", 5, &[YELLOW], &["Unidentified"]), 2);
}

#[test]
fn bt24_065_black_lv5_diaboromon_prompts_cost_2_vs_5() {
    assert_cost_choice(ID, base("BD5", "Diaboromon", 5, &[BLACK], &["Unidentified"]), 2, 5);
}

#[test]
fn bt24_065_other_bases_have_no_route() {
    assert_not_offered(ID, base("DX", "Diaboromon (X Antibody)", 6, &[BLACK], &["Unidentified"]));
    assert_not_offered(ID, base("R5", "Red Ultimate", 5, &[RED], &["Unidentified"]));
    assert_not_offered(ID, base("B4", "Black Champion", 4, &[BLACK], &["Unidentified"]));
}

// ─── Static keywords ──────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, level: u8, play_cost: u16) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = play_cost;
    c.colors = vec![CardColor::Black];
    c
}

fn plain_runner(cards: Vec<CardData>, hand: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder().dsl_card(ID).expect("BT24-065 production YAML loads");
    for c in cards {
        b = b.add_card(c);
    }
    let mut filler = make_test_card("FILL", "Filler");
    filler.level = Some(3);
    let mut r = b
        .add_card(filler)
        .hand(0, hand)
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r
}

#[test]
fn bt24_065_has_blocker() {
    let mut r = plain_runner(vec![], &[]);
    let h = r.place_on_field(0, ID, None);
    assert!(r.game.has_keyword(h, Keyword::Blocker));
}

// ─── [When Digivolving] de-digivolve per own Digimon, then delete highest cost ──

/// P0: a Lv.5 [Diaboromon] base (red -> only the cost-2 route) + `own_extra`
/// other Digimon; P1: the given stacks (bottom -> top).
fn wd_runner(own_extra: usize, opp_stacks: &[&[&str]]) -> (DebugRunner, usize) {
    let mut cards = vec![
        base("DB5", "Diaboromon", 5, &[CardColor::Red], &["Unidentified"]),
        digimon("OWN", "Own Rookie", 3, 3),
        digimon("O-LV3", "Opp Rookie", 3, 3),
        digimon("O-LV4A", "Opp Champion A", 4, 12),
        digimon("O-LV4B", "Opp Champion B", 4, 9),
        digimon("O-LV5", "Opp Ultimate", 5, 14),
        digimon("O-LV5B", "Opp Ultimate B", 5, 14),
    ];
    // distinct low cards used as sources
    cards.push(digimon("O-LV4C", "Opp Champion C", 4, 4));
    let mut r = plain_runner(cards, &[ID]);
    let slot = r.place_on_field(0, "DB5", Some(0)).index as usize;
    for _ in 0..own_extra {
        r.place_on_field(0, "OWN", None);
    }
    for stack in opp_stacks {
        r.place_stack(1, stack);
    }
    (r, slot)
}

fn digivolve(r: &mut DebugRunner, slot: usize) {
    let a = encode_digivolve(0, slot as u16);
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.0, "digivolve offered");
    r.game.decode_action(a, 0);
}

fn opp_tops(r: &DebugRunner) -> Vec<String> {
    r.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn pick_first(r: &mut DebugRunner) {
    let view = r.pending_selection_view().expect("a selection is pending");
    let pick = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();
}

#[test]
fn bt24_065_de_digivolves_once_per_own_digimon_then_deletes_the_highest_cost_after_it() {
    // Own Digimon after the digivolve: BT24-065 + 1 extra = 2 -> De-Digivolve 1 x2.
    // Opp A: [Lv3, Lv4 (cost 4), Lv5 (cost 14)] is the highest BEFORE (14) but
    // after two de-digivolves its top is the Lv3 (cost 3). Opp B (cost 9) is
    // then the highest and is the one deleted.
    let (mut r, slot) = wd_runner(1, &[&["O-LV3", "O-LV4C", "O-LV5"], &["O-LV4B"]]);
    digivolve(&mut r, slot);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::OppField),
        "mandatory pick of 1 opponent Digimon to de-digivolve"
    );
    let view = r.pending_selection_view().unwrap();
    assert!(!view.valid_action_ids.contains(&PASS), "mandatory (canNoSelect: false)");
    // First legal target = slot 0 = Opp A.
    pick_first(&mut r);
    let _ = r.auto_resolve();
    // Opp A lost its top two cards (Lv5, Lv4) and keeps the Lv3 top; Opp B (the
    // highest play cost afterwards) was deleted.
    assert_eq!(opp_tops(&r), vec!["O-LV3".to_string()]);
}

#[test]
fn bt24_065_with_a_single_own_digimon_de_digivolves_just_once() {
    let (mut r, slot) = wd_runner(0, &[&["O-LV3", "O-LV4C", "O-LV5"]]);
    digivolve(&mut r, slot);
    pick_first(&mut r);
    let _ = r.auto_resolve();
    // De-digivolved once: top Lv5 trashed -> top O-LV4C (cost 4), the highest
    // (the only) play cost -> deleted as the sole highest.
    assert!(opp_tops(&r).is_empty(), "the only Digimon is also the highest-cost one: {:?}", opp_tops(&r));
}

#[test]
fn bt24_065_deletes_every_tied_highest_cost_digimon() {
    let (mut r, slot) = wd_runner(0, &[&["O-LV3", "O-LV5"], &["O-LV5B"], &["O-LV4B"]]);
    digivolve(&mut r, slot);
    // De-digivolve the cost-9 Digimon (it can't go below its only card).
    let view = r.pending_selection_view().expect("de-digivolve target pick");
    assert_eq!(view.kind, SelectionKind::OppField);
    let last = view.valid_action_ids.iter().copied().filter(|&a| a != PASS).last().unwrap();
    r.execute_action(0, last).unwrap();
    let _ = r.auto_resolve();
    // O-LV5 stack dropped to O-LV3 after the single de-digivolve; O-LV5B (14)
    // is now the unique highest... the cost-9 Digimon stays; the 14 is deleted.
    let tops = opp_tops(&r);
    assert!(!tops.contains(&"O-LV5B".to_string()), "the highest-cost Digimon is deleted: {tops:?}");
    assert!(tops.contains(&"O-LV4B".to_string()), "the lower-cost Digimon stays: {tops:?}");
}

#[test]
fn bt24_065_wd_with_no_opponent_digimon_does_nothing() {
    let (mut r, slot) = wd_runner(0, &[]);
    digivolve(&mut r, slot);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[0].battle_area[slot].top_card().card_id(&r.game.card_data), ID);
}

// ─── [All Turns][OPT] would leave -> play a [Diaboromon] free ──────────────────

fn leave_runner() -> DebugRunner {
    let cards = vec![
        digimon("DIA-A", "Diaboromon", 5, 12),
        digimon("DIA-B", "Arkadimon (Diaboromon)", 6, 13),
        digimon("OTHER", "Other Digimon", 4, 4),
    ];
    plain_runner(cards, &["DIA-A"])
}

#[test]
fn bt24_065_plays_a_diaboromon_from_hand_when_a_diaboromon_name_digimon_would_leave() {
    let mut r = leave_runner();
    r.place_on_field(0, ID, Some(0));
    let leaver = r.place_on_field(0, "DIA-B", None);
    let mem = r.memory();
    r.game
        .delete_permanent_with_cause(leaver, ReplacementCause::OpponentEffect);
    let view = r.pending_selection_view().expect("optional prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(view.is_optional);
    r.execute_action(0, view.valid_action_ids[0]).expect("accept");
    pick_first(&mut r);
    let _ = r.auto_resolve();
    let ids: Vec<String> = r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect();
    assert!(ids.contains(&"DIA-A".to_string()), "[Diaboromon] played from hand: {ids:?}");
    assert!(!ids.contains(&"DIA-B".to_string()), "the leaving Digimon still leaves (not a prevention)");
    assert_eq!(r.memory(), mem, "played without paying the cost");
}

#[test]
fn bt24_065_can_play_the_diaboromon_from_its_own_digivolution_cards() {
    let mut r = leave_runner();
    // BT24-065 on top of an exactly-named [Diaboromon] digivolution card.
    let host = r.place_stack(0, &["DIA-A", ID]);
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    let view = r.pending_selection_view().expect("optional prompt");
    r.execute_action(0, view.valid_action_ids[0]).expect("accept");
    pick_first(&mut r);
    let _ = r.auto_resolve();
    let ids: Vec<String> = r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect();
    assert!(ids.contains(&"DIA-A".to_string()), "[Diaboromon] played from the sources: {ids:?}");
}

#[test]
fn bt24_065_ignores_leavers_without_diaboromon_in_their_name_and_is_once_per_turn() {
    let mut r = leave_runner();
    r.place_on_field(0, ID, Some(0));
    let other = r.place_on_field(0, "OTHER", None);
    r.game
        .delete_permanent_with_cause(other, ReplacementCause::OpponentEffect);
    assert!(
        !matches!(r.pending_kind(), Some(SelectionKind::Replacement)),
        "a non-[Diaboromon] Digimon leaving does not trigger it"
    );
}
