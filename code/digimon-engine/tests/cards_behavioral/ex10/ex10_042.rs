//! EX10-042 GulusGammamon — Digimon, Lv.4, Purple+Red, DP 5000, Cost 5
//! (Dragonkin).
//!
//! Printed (official Bandai DB bundle + DCGO EX10_042.cs):
//! - Evo box: Purple Lv.3 / cost 3 OR Red Lv.3 / cost 3; "[Digivolve]
//!   [Gammamon]: Cost 2" (exact name, any level).
//! - [On Play] [When Digivolving] Trash the top 2 cards of your deck. Then, you
//!   may place 1 Digimon card with [Gammamon] in its name from your trash as this
//!   Digimon's bottom digivolution card.
//! - [Your Turn] [Once Per Turn] When effects add to this Digimon's digivolution
//!   cards, this Digimon may digivolve into [Regulusmon] in the hand or trash with
//!   the digivolution cost reduced by 1.
//! - Inherited <Raid>.
//!
//! Regression (hunt2 cluster alt-digivolve-cost-cards): no YAML spec, so the
//! cost-2 [Gammamon] route lived only in `xros_req`: a base satisfying both
//! routes never got DCGO's cost-choice prompt (cost 2 vs 3). Synthetic boards
//! only.

use crate::alt_digivolve_support::*;
use digimon_engine::action::{build_action_mask, space::{encode_digivolve, PASS}};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, GamePhase, Keyword};
use digimon_engine::selection::SelectionKind;

const ID: &str = "EX10-042";

// ─── Digivolution routes ──────────────────────────────────────────────────────

#[test]
fn ex10_042_purple_or_red_lv3_circle_costs_3_without_prompt() {
    assert_single_route(ID, base("P3", "Purple Rookie", 3, &[PURPLE], &["Dragonkin"]), 3);
    assert_single_route(ID, base("R3", "Red Rookie", 3, &[RED], &["Dragonkin"]), 3);
}

#[test]
fn ex10_042_gammamon_named_base_of_any_level_costs_2() {
    assert_single_route(ID, base("GM2", "Gammamon", 2, &[BLUE], &["Dragonkin"]), 2);
    assert_single_route(ID, base("GM3", "Gammamon", 3, &[YELLOW], &["Dragonkin"]), 2);
}

#[test]
fn ex10_042_purple_or_red_lv3_gammamon_prompts_cost_2_vs_3() {
    assert_cost_choice(ID, base("PGM3", "Gammamon", 3, &[PURPLE], &["Dragonkin"]), 2, 3);
    assert_cost_choice(ID, base("RGM3", "Gammamon", 3, &[RED], &["Dragonkin"]), 2, 3);
}

#[test]
fn ex10_042_other_bases_have_no_route() {
    // [Gammamon] means the exact name (DCGO `EqualsCardName`).
    assert_not_offered(ID, base("SGM", "Super Gammamon", 3, &[BLUE], &["Dragonkin"]));
    assert_not_offered(ID, base("B3", "Blue Rookie", 3, &[BLUE], &["Dragonkin"]));
    assert_not_offered(ID, base("P2", "Purple In-Training", 2, &[PURPLE], &["Dragonkin"]));
}

// ─── [On Play][When Digivolving] mill 2, may tuck a [Gammamon]-name Digimon ──

fn digimon(id: &str, name: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(3000);
    c
}

/// Regulusmon (Lv.5 Purple, digivolves from a Lv.4 Purple for 4): exact name.
fn regulusmon() -> CardData {
    let mut c = digimon("REGULUS", "Regulusmon", 5);
    c.colors = vec![CardColor::Purple];
    c.evo_costs = vec![EvoCost {
        card_color: 6,
        level: 4,
        memory_cost: 4,
    }];
    c
}

/// EX10-042 in hand; a plain Purple Lv.3 base on the field; the owner's trash
/// holds `trash_ids`; `hand_extra` rides along in hand.
fn runner_with(trash_ids: &[&str], hand_extra: &[&str]) -> (DebugRunner, usize) {
    let mut filler = make_test_card("FILL", "Filler");
    filler.level = Some(3);
    filler.dp = Some(3000);
    let mut gm_option = make_test_card("GM-OPT", "Gammamon Charm");
    gm_option.card_kind = CardKind::Option;
    let mut hand = vec![ID];
    hand.extend_from_slice(hand_extra);
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("EX10-042 production YAML loads")
        .add_card(base("P3", "Purple Rookie", 3, &[PURPLE], &["Dragonkin"]))
        .add_card(filler)
        .add_card(digimon("GM-A", "Gammamon", 2))
        .add_card(digimon("GM-B", "Super Gammamon", 3))
        .add_card(gm_option)
        .add_card(regulusmon())
        .hand(0, &hand)
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(15)
        .start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    for id in trash_ids {
        r.inject_trash(0, id);
    }
    let slot = r.place_on_field(0, "P3", Some(0)).index as usize;
    (r, slot)
}

fn digivolve(r: &mut DebugRunner, slot: usize) {
    let a = encode_digivolve(0, slot as u16);
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.0, "digivolve offered");
    r.game.decode_action(a, 0);
}

#[test]
fn ex10_042_when_digivolving_mills_2_then_offers_only_gammamon_name_digimon() {
    let (mut r, slot) = runner_with(&["GM-A", "GM-B", "GM-OPT", "FILL"], &[]);
    let trash_before = r.trash_size(0);
    let deck_before = r.deck_size(0);
    digivolve(&mut r, slot);
    // The digivolution bonus draws 1 (CR 4.3), then the clause trashes 2.
    assert_eq!(r.deck_size(0), deck_before - 1 - 2, "bonus draw + the top 2 trashed");
    assert_eq!(r.trash_size(0), trash_before + 2);
    let view = r.pending_selection_view().expect("the optional tuck pick parks");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert!(view.is_optional, "\"you may\"");
    let picks: Vec<u16> = view.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(
        picks.len(),
        2,
        "[Gammamon] is a name SUBSTRING here: Gammamon + Super Gammamon, never the Option or filler: {picks:?}"
    );
}

#[test]
fn ex10_042_tuck_lands_as_the_bottom_digivolution_card() {
    let (mut r, slot) = runner_with(&["GM-A"], &[]);
    digivolve(&mut r, slot);
    let view = r.pending_selection_view().expect("tuck pick");
    let pick = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();
    let perm = &r.game.players[0].battle_area[slot];
    assert_eq!(
        perm.card_sources[0].card_id(&r.game.card_data),
        "GM-A",
        "bottom digivolution card"
    );
}

#[test]
fn ex10_042_declining_the_tuck_places_nothing() {
    let (mut r, slot) = runner_with(&["GM-A"], &[]);
    digivolve(&mut r, slot);
    r.execute_action(0, PASS).expect("decline");
    assert!(r.pending_selection().is_none());
    let perm = &r.game.players[0].battle_area[slot];
    // Stack = P3 (base) + EX10-042 only.
    assert_eq!(perm.card_sources.len(), 2);
}

// ─── [Your Turn][OPT] digivolve into [Regulusmon] (cost -1) ──────────────────

#[test]
fn ex10_042_tuck_triggers_the_regulusmon_digivolve_from_hand_with_cost_minus_1() {
    let (mut r, slot) = runner_with(&["GM-A"], &["REGULUS"]);
    digivolve(&mut r, slot);
    let view = r.pending_selection_view().expect("tuck pick");
    let pick = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();
    // The placement fires the [Your Turn] trigger: an accept/decline prompt.
    r.accept_optional_trigger().expect("accept the optional trigger");
    let view = r.pending_selection_view().expect("Regulusmon pick");
    let pick = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
    let before = r.memory();
    r.execute_action(0, pick).unwrap();
    let top = r.game.players[0].battle_area[slot].top_card().card_id(&r.game.card_data).to_string();
    assert_eq!(top, "REGULUS", "digivolved into [Regulusmon]");
    assert_eq!(before - r.memory(), 3, "printed cost 4, reduced by 1");
}

#[test]
fn ex10_042_regulusmon_digivolve_can_come_from_the_trash() {
    let (mut r, slot) = runner_with(&["GM-A", "REGULUS"], &[]);
    digivolve(&mut r, slot);
    let view = r.pending_selection_view().expect("tuck pick");
    let pick = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();
    r.accept_optional_trigger().expect("accept");
    let view = r.pending_selection_view().expect("Regulusmon pick from trash");
    let pick = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();
    assert_eq!(
        r.game.players[0].battle_area[slot].top_card().card_id(&r.game.card_data),
        "REGULUS"
    );
}

#[test]
fn ex10_042_no_regulusmon_no_prompt_after_the_tuck() {
    let (mut r, slot) = runner_with(&["GM-A"], &[]);
    digivolve(&mut r, slot);
    let view = r.pending_selection_view().expect("tuck pick");
    let pick = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();
    assert!(
        r.pending_selection().is_none(),
        "nothing to digivolve into: the [Your Turn] clause offers no prompt"
    );
}

// ─── Inherited <Raid> ─────────────────────────────────────────────────────────

#[test]
fn ex10_042_inherited_raid_on_the_host() {
    let (mut r, _) = runner_with(&[], &[]);
    let host = r.place_stack(0, &[ID, "P3"]);
    assert!(r.game.has_keyword(host, Keyword::Raid));
}

use digimon_engine::enums::CardColor;
