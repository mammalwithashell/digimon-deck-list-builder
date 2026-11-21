//! EX5-018 Garurumon (X Antibody) — Digimon, Lv.4, Blue+Purple, DP 6000, Cost 6
//! (Beast, X Antibody).
//!
//! Printed (official Bandai DB bundle + DCGO EX5_018.cs):
//! - Evo box: Blue Lv.3 / cost 3 OR Purple Lv.3 / cost 3; "[Digivolve]
//!   [Garurumon]: Cost 0" (exact name, any level).
//! - [When Digivolving] <Draw 2>. Then, trash 2 cards in your hand. If
//!   [Garurumon] or [X Antibody] is in this Digimon's digivolution cards, gain 1
//!   memory.
//! - Inherited: [All Turns][Once Per Turn] When this Digimon with [Garurumon] or
//!   [Omnimon] in its name would be deleted in battle, by returning 2 non-Digi-Egg
//!   cards from your trash to the bottom of the deck, prevent that deletion.
//!
//! Regression (hunt2 cluster alt-digivolve-cost-cards): no YAML spec, so the
//! cost-0 [Garurumon] route lived only in `xros_req` — a Lv.4 [Garurumon] base
//! had no legal digivolve (illegal_action) and a Blue/Purple Lv.3 [Garurumon]
//! never got DCGO's "which cost?" prompt. Synthetic boards only.

use crate::alt_digivolve_support::*;
use digimon_engine::action::{build_action_mask, space::encode_digivolve};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, GamePhase};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const ID: &str = "EX5-018";

// ─── Digivolution routes ──────────────────────────────────────────────────────

#[test]
fn ex5_018_blue_or_purple_lv3_circle_costs_3_without_prompt() {
    assert_single_route(ID, base("B3", "Blue Rookie", 3, &[BLUE], &["Beast"]), 3);
    assert_single_route(ID, base("P3", "Purple Rookie", 3, &[PURPLE], &["Beast"]), 3);
}

#[test]
fn ex5_018_garurumon_named_base_of_any_level_costs_0() {
    // The alt route has no level gate: a Lv.4 [Garurumon] reaches it and no
    // circle applies (Lv.4 is not a printed circle level).
    assert_single_route(ID, base("G4", "Garurumon", 4, &[RED], &["Beast"]), 0);
    assert_single_route(ID, base("G5", "Garurumon", 5, &[YELLOW], &["Beast"]), 0);
}

#[test]
fn ex5_018_blue_or_purple_lv3_garurumon_prompts_cost_0_vs_3() {
    assert_cost_choice(ID, base("BG3", "Garurumon", 3, &[BLUE], &["Beast"]), 0, 3);
    assert_cost_choice(ID, base("PG3", "Garurumon", 3, &[PURPLE], &["Beast"]), 0, 3);
}

#[test]
fn ex5_018_other_bases_have_no_route() {
    // [Garurumon (X Antibody)] is a different name (DCGO `CardNames.Contains`).
    assert_not_offered(
        ID,
        base("GX4", "Garurumon (X Antibody)", 4, &[BLUE], &["Beast", "X Antibody"]),
    );
    assert_not_offered(ID, base("R3", "Red Rookie", 3, &[RED], &["Beast"]));
    assert_not_offered(ID, base("B2", "Blue In-Training", 2, &[BLUE], &["Beast"]));
}

// ─── [When Digivolving] draw 2, trash 2, conditional memory ──────────────────

fn source_card(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c
}

/// EX5-018 + two filler cards in hand; `base_card` on the field with
/// `source_name` (if any) as a digivolution card beneath it.
fn wd_runner(base_card: CardData, source_name: Option<&str>) -> (DebugRunner, usize) {
    let base_id = base_card.card_id.clone();
    let mut filler = make_test_card("FILL", "Filler");
    filler.level = Some(3);
    filler.dp = Some(3000);
    let mut b = DebugRunner::builder()
        .dsl_card(ID)
        .expect("EX5-018 production YAML loads")
        .add_card(base_card)
        .add_card(filler);
    if let Some(name) = source_name {
        b = b.add_card(source_card("SRC", name));
    }
    let mut r = b
        .hand(0, &[ID, "FILL", "FILL"])
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let host = r.place_on_field(0, &base_id, Some(0));
    let slot = host.index as usize;
    if source_name.is_some() {
        r.push_source(host, "SRC");
    }
    (r, slot)
}

fn digivolve(r: &mut DebugRunner, slot: usize) {
    let a = encode_digivolve(0, slot as u16);
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.0, "digivolve offered");
    r.game.decode_action(a, 0);
}

/// Resolve the two mandatory hand picks of "trash 2 cards in your hand".
fn trash_two(r: &mut DebugRunner) {
    for n in 0..2 {
        let view = r
            .pending_selection_view()
            .unwrap_or_else(|| panic!("hand pick {n} is pending"));
        assert_eq!(view.kind, SelectionKind::Hand, "pick {n}");
        assert!(!view.is_optional, "trashing is mandatory (DCGO canNoSelect: false)");
        r.execute_action(0, view.valid_action_ids[0]).expect("trash a card");
    }
}

#[test]
fn ex5_018_when_digivolving_draws_2_and_trashes_2_from_the_cost_0_route() {
    // Base [Garurumon] Lv.3 blue: cost choice 0 vs 3, take the cost-0 option.
    let (mut r, slot) = wd_runner(base("BG3", "Garurumon", 3, &[BLUE], &["Beast"]), None);
    let hand_before = r.hand_size(0); // EX5-018 + 2 filler
    digivolve(&mut r, slot);
    let view = r.pending_selection_view().expect("cost choice");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    let zero = view
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .find(|c| c.label.to_lowercase().contains("cost 0"))
        .expect("cost 0 option")
        .action_id;
    r.execute_action(0, zero).unwrap();
    // WD: <Draw 2> happened before the first pick (on top of the digivolution
    // bonus draw of 1 and the digivolved card leaving the hand).
    let at_pick = r.hand_size(0);
    assert_eq!(at_pick, hand_before - 1 + 1 + 2, "card left, bonus draw 1, Draw 2");
    trash_two(&mut r);
    assert_eq!(r.hand_size(0), at_pick - 2, "trashed 2");
    assert_eq!(r.trash_size(0), 2);
}

#[test]
fn ex5_018_when_digivolving_gains_memory_for_a_garurumon_named_source() {
    let (mut r, slot) = wd_runner(
        base("B3", "Blue Rookie", 3, &[BLUE], &["Beast"]),
        Some("Garurumon"),
    );
    digivolve(&mut r, slot);
    let mem_before = r.memory();
    trash_two(&mut r);
    assert_eq!(r.memory(), mem_before + 1, "[Garurumon] digivolution card");
}

#[test]
fn ex5_018_when_digivolving_gains_memory_for_x_antibody_named_source() {
    let (mut r, slot) = wd_runner(
        base("B3", "Blue Rookie", 3, &[BLUE], &["Beast"]),
        Some("X Antibody"),
    );
    digivolve(&mut r, slot);
    let mem_before = r.memory();
    trash_two(&mut r);
    assert_eq!(r.memory(), mem_before + 1, "[X Antibody] digivolution card");
}

#[test]
fn ex5_018_when_digivolving_no_memory_without_a_named_source() {
    // The source's name merely CONTAINING "X Antibody" does not count: DCGO
    // `CardNames.Contains("X Antibody")` is an exact-name list lookup.
    let (mut r, slot) = wd_runner(
        base("B3", "Blue Rookie", 3, &[BLUE], &["Beast"]),
        Some("Gabumon (X Antibody)"),
    );
    digivolve(&mut r, slot);
    let mem_before = r.memory();
    trash_two(&mut r);
    assert_eq!(r.memory(), mem_before, "no [Garurumon]/[X Antibody] source: no memory");
    assert!(r.pending_selection().is_none());
}

#[test]
fn ex5_018_top_card_garurumon_name_is_not_a_digivolution_card() {
    // EX5-018 itself is named "Garurumon (X Antibody)" but the top card is not
    // one of its own digivolution cards; a plain base gives no memory.
    let (mut r, slot) = wd_runner(base("B3", "Blue Rookie", 3, &[BLUE], &["Beast"]), None);
    digivolve(&mut r, slot);
    let mem_before = r.memory();
    trash_two(&mut r);
    assert_eq!(r.memory(), mem_before);
}

// ─── Inherited [All Turns][OPT] substitute (battle deletion) ─────────────────

fn named_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c
}

fn seed_non_egg_trash(runner: &mut DebugRunner, count: usize) {
    for i in 0..count {
        let id = format!("TRASH-{i}");
        let data = named_digimon(&id, &format!("TrashCard{i}"));
        let data_index = runner.game.card_data.len();
        runner.game.card_data.push(data);
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .trash
            .push(CardSource::new(data_index, 0, card_index));
    }
}

#[test]
fn ex5_018_inherited_substitute_prevents_battle_deletion_on_garurumon_host() {
    let mut runner = DebugRunner::builder()
        .dsl_card(ID)
        .expect("EX5-018 YAML loads")
        .add_card(named_digimon("HOST", "WereGarurumon"))
        .start();
    let host = runner.place_stack(0, &[ID, "HOST"]);
    seed_non_egg_trash(&mut runner, 3);
    let deck_before = runner.game.players[0].deck.len();

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::Battle);

    let view = runner.pending_selection_view().expect("Substitute prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(view.is_optional);
    runner.execute_action(0, view.valid_action_ids[0]).expect("accept");
    for n in 0..2 {
        let view = runner
            .pending_selection_view()
            .unwrap_or_else(|| panic!("trash pick {n}"));
        runner.execute_action(0, view.valid_action_ids[0]).expect("pick");
    }
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(0), 1, "the host survives");
    assert_eq!(runner.game.players[0].deck.len(), deck_before + 2);
}

#[test]
fn ex5_018_inherited_substitute_ignores_a_host_without_garurumon_or_omnimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(ID)
        .expect("EX5-018 YAML loads")
        .add_card(named_digimon("HOST", "Greymon"))
        .start();
    let host = runner.place_stack(0, &[ID, "HOST"]);
    seed_non_egg_trash(&mut runner, 3);
    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::Battle);
    assert!(
        !matches!(runner.pending_kind(), Some(SelectionKind::Replacement)),
        "no Substitute offer on a non-Garurumon/Omnimon host"
    );
    assert_eq!(runner.battle_area_size(0), 0, "the host is deleted");
}
