//! BT16-070 Sethmon — Digimon, Lv.4, Purple/Red, DP 5000, Cost 5 (Dark Animal,
//! Armor Form).
//!
//! Printed (official Bandai DB bundle + DCGO BT16_070.cs):
//! - Evo box: Lv.3 Purple / cost 3; "Digivolve [Veemon]: Cost 2".
//! - <Armor Purge>.
//! - [When Digivolving] [When Attacking] You may choose 1 of your Digimon.
//!   Delete it and 1 of your opponent's Digimon with as much or less DP as it.
//!
//! Regression (hunt cluster alt-digivolve-xros-req-cards): no YAML spec, so the
//! [Veemon] cost-2 route lived only in `xros_req` — a non-Purple-Lv.3 [Veemon]
//! had its digivolve masked entirely and a Purple Lv.3 [Veemon] got no
//! cost-choice prompt. Synthetic boards only.

use crate::alt_digivolve_support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::action::{build_action_mask, space::{encode_digivolve, PASS}};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{GamePhase, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT16-070";

// ─── Digivolution routes ──────────────────────────────────────────────────────

#[test]
fn bt16_070_purple_lv3_circle_costs_3() {
    assert_single_route(ID, base("P3", "Purple Rookie", 3, &[PURPLE], &["Beast"]), 3);
}

#[test]
fn bt16_070_veemon_of_any_colour_or_level_costs_2() {
    assert_single_route(ID, base("V2", "Veemon", 2, &[BLUE], &["Dragon"]), 2);
    assert_single_route(ID, base("V3", "Veemon", 3, &[BLUE], &["Dragon"]), 2);
}

#[test]
fn bt16_070_purple_lv3_veemon_prompts_cost_2_vs_3() {
    assert_cost_choice(ID, base("PV3", "Veemon", 3, &[PURPLE], &["Dragon"]), 2, 3);
}

#[test]
fn bt16_070_other_bases_have_no_route() {
    assert_not_offered(ID, base("B3", "Blue Rookie", 3, &[BLUE], &["Beast"]));
    assert_not_offered(ID, base("VX", "ExVeemon", 3, &[BLUE], &["Dragon"]));
    assert_not_offered(ID, base("P2", "Purple In-Training", 2, &[PURPLE], &["Beast"]));
}

#[test]
fn bt16_070_has_armor_purge_and_both_delete_timings() {
    let (r, slot) = runner(ID, base("V2", "Veemon", 2, &[BLUE], &["Dragon"]));
    let _ = slot;
    let c = r.compiled_card(ID).expect("compiled");
    let clause = c
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => Some(t),
            _ => None,
        })
        .expect("the delete clause");
    assert!(clause.when.contains(&CompiledTiming::WhenAttacking));
    assert!(clause.optional, "\"You may\"");
    // Armor Purge reaches a field Sethmon.
    let mut r = r;
    let h = r.place_on_field(0, ID, Some(1));
    assert!(r.game.has_keyword(h, Keyword::ArmorPurge));
}

// ─── [When Digivolving]: choose own Digimon; delete it + opp Digimon ≤ its DP ─

fn dp_card(id: &str, name: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut c = base(id, name, 3, &[BLUE], &["Beast"]);
    c.dp = Some(dp);
    c
}

fn wd_runner(opp_dps: &[i32]) -> (DebugRunner, usize, Vec<PermanentHandle>) {
    let mut filler = make_test_card("FILL", "Filler");
    filler.level = Some(3);
    let mut b = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT16-070 production YAML loads")
        .add_card(base("VEE", "Veemon", 2, &[BLUE], &["Dragon"]))
        .add_card(dp_card("ALLY", "Ally", 4000))
        .add_card(filler);
    for (i, dp) in opp_dps.iter().enumerate() {
        b = b.add_card(dp_card(&format!("OPP{i}"), &format!("Opp {i}"), *dp));
    }
    let mut r = b
        .hand(0, &[ID])
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, "VEE", Some(0)).index as usize;
    r.place_on_field(0, "ALLY", Some(0));
    let opps = (0..opp_dps.len())
        .map(|i| r.place_on_field(1, &format!("OPP{i}"), Some(0)))
        .collect();
    (r, slot, opps)
}

fn digivolve(r: &mut DebugRunner, slot: usize) {
    let a = encode_digivolve(0, slot as u16);
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.0);
    r.game.decode_action(a, 0);
}

fn alive(r: &DebugRunner, player: u8, id: &str) -> bool {
    r.game.players[player as usize]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == id)
}

#[test]
fn bt16_070_deletes_the_chosen_digimon_and_an_opponent_digimon_with_dp_at_most_its_dp() {
    // ALLY has 4000: only OPP0 (3000) qualifies; OPP1 (5000) / OPP2 (9000) do not.
    let (mut r, slot, _opps) = wd_runner(&[3000, 5000, 9000]);
    digivolve(&mut r, slot);
    let own = r.pending_selection_view().expect("own pick parks");
    assert_eq!(own.kind, SelectionKind::OwnField);
    assert!(own.is_optional, "\"You may choose\"");
    let mut ids: Vec<u16> = own.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    ids.sort();
    assert_eq!(ids.len(), 2, "any of your Digimon: Sethmon and the ally");
    r.execute_action(0, ids[1]).expect("choose the ally (slot 1)");

    let opp = r.pending_selection_view().expect("opponent pick parks");
    assert!(!opp.is_optional, "the opponent pick is mandatory once a target exists");
    let picks: Vec<u16> = opp.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(picks.len(), 1, "only DP <= 4000: {picks:?}");
    r.execute_action(0, picks[0]).expect("choose the opponent Digimon");

    assert!(!alive(&r, 0, "ALLY"), "the chosen Digimon is deleted");
    assert!(!alive(&r, 1, "OPP0"), "and the opponent's Digimon too");
    assert!(alive(&r, 1, "OPP1") && alive(&r, 1, "OPP2"));
    assert!(alive(&r, 0, ID), "Sethmon itself remains");
}

#[test]
fn bt16_070_choosing_a_stronger_digimon_widens_the_opponent_pool() {
    let (mut r, slot, _opps) = wd_runner(&[3000, 5000, 9000]);
    digivolve(&mut r, slot);
    let own = r.pending_selection_view().unwrap();
    let mut ids: Vec<u16> = own.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    ids.sort();
    // Slot 0 is the freshly digivolved Sethmon (DP 5000): OPP0 and OPP1 qualify.
    r.execute_action(0, ids[0]).expect("choose Sethmon itself");
    let opp = r.pending_selection_view().expect("opponent pick");
    let picks: Vec<u16> = opp.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(picks.len(), 2, "DP <= 5000 (Sethmon's own DP): {picks:?}");
}

#[test]
fn bt16_070_with_no_opponent_digimon_in_range_only_the_chosen_digimon_is_deleted() {
    let (mut r, slot, _opps) = wd_runner(&[9000]);
    digivolve(&mut r, slot);
    let own = r.pending_selection_view().unwrap();
    let mut ids: Vec<u16> = own.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    ids.sort();
    r.execute_action(0, ids[1]).expect("choose the ally");
    assert!(r.pending_selection().is_none(), "no opponent Digimon with DP <= 4000");
    assert!(!alive(&r, 0, "ALLY"), "the chosen Digimon is still deleted");
    assert!(alive(&r, 1, "OPP0"));
}

#[test]
fn bt16_070_declining_the_choice_deletes_nothing() {
    let (mut r, slot, _opps) = wd_runner(&[3000]);
    digivolve(&mut r, slot);
    r.execute_action(0, PASS).expect("decline");
    assert!(r.pending_selection().is_none());
    assert!(alive(&r, 0, "ALLY"));
    assert!(alive(&r, 1, "OPP0"));
}
