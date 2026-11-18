//! EX11-005 Yaamon — Digi-Egg, Lv.2, Purple. Traits: Lesser, LIBERATOR.
//!
//! # Card text (card image prints "Lv.2"; official Bandai DB bundle `data/card_bundles/EX11-005.md`)
//!
//! Inherited Effect: [Start of Your Main Phase] This Digimon may digivolve
//! into a [Dark Dragon] or [Evil Dragon] trait Digimon card in the trash with
//! the digivolution cost reduced by 1. If this effect digivolved, trash 2
//! cards in your hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX11/Purple/EX11_005.cs —
//! `DigivolveIntoHandOrTrashCard(payCost, reduce 1, isHand: false)`, then
//! `successProcess` discards min(2, hand) (mandatory) only after a real digivolve.

#![allow(dead_code, unused_imports)]

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "EX11-005";

fn digimon(id: &str, name: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(3000 + 1000 * level as i32);
    c.play_cost = 4;
    c.colors = vec![CardColor::Purple];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// A Lv.4 purple Digimon digivolving from a Lv.3 purple for `cost`.
fn evo(id: &str, name: &str, traits: &[&str], cost: u8) -> CardData {
    let mut c = digimon(id, name, 4, traits);
    c.evo_costs = vec![EvoCost {
        card_color: 6,
        level: 3,
        memory_cost: cost.into(),
    }];
    c
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-005 YAML loads from the embedded DSL pack")
        .add_card(digimon("CARRIER", "Carrier", 3, &[]))
        .add_card(evo("DARK-DRAGON", "Dark Dragon Mon", &["Dark Dragon"], 3))
        .add_card(evo("EVIL-DRAGON", "Evil Dragon Mon", &["Evil Dragon"], 3))
        .add_card(evo("OTHER", "Other Mon", &["Reptile"], 3))
        .add_card(make_test_card("PAD", "Pad"))
        .add_card(make_test_card("H1", "H1"))
        .add_card(make_test_card("H2", "H2"))
        .add_card(make_test_card("H3", "H3"))
}

fn runner_with(trash: &[&str], hand: &[&str]) -> DebugRunner {
    let mut runner = builder()
        .hand(0, hand)
        .deck(0, &["PAD", "PAD", "PAD"])
        .deck(1, &["PAD", "PAD"])
        .memory(10)
        .start();
    for id in trash {
        runner.inject_trash(0, id);
    }
    runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.set_memory(5);
    runner
}

fn top_id(runner: &DebugRunner) -> String {
    runner.game.players[0].battle_area[0]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

/// Digivolve into a [Dark Dragon] trash card at cost 3-1 = 2, then trash 2.
#[test]
fn ex11_005_digivolves_from_trash_with_cost_reduced_by_1_then_trashes_2() {
    let mut runner = runner_with(&["DARK-DRAGON"], &["H1", "H2", "H3"]);
    let before = runner.game.memory;
    runner.game.enter_main_phase();

    let view = runner.pending_selection_view().expect("trash pick pending");
    assert!(runner.pending_is_optional(), "'may' is declinable");
    let pick = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("a trash card is offered");
    runner
        .execute_action(view.selecting_player, pick)
        .expect("pick");
    // Two mandatory discards follow the digivolve.
    for _ in 0..2 {
        let v = runner.pending_selection_view().expect("discard pick pending");
        assert!(!v.valid_action_ids.contains(&PASS), "discards are mandatory");
        runner
            .execute_action(v.selecting_player, v.valid_action_ids[0])
            .expect("discard");
    }
    let _ = runner.auto_resolve();

    assert_eq!(top_id(&runner), "DARK-DRAGON");
    assert_eq!(runner.game.memory, before - 2, "printed cost 3 reduced by 1");
    // 3 in hand + the digivolution bonus draw (official Q&A: "You perform the
    // digivolution bonus draw and then trash 2 cards in your hand") - 2 trashed.
    assert_eq!(runner.game.players[0].hand.len(), 2, "3 + 1 draw - 2 trashed");
}

#[test]
fn ex11_005_evil_dragon_trait_is_also_eligible_other_traits_are_not() {
    let mut runner = runner_with(&["EVIL-DRAGON", "OTHER"], &["H1", "H2"]);
    runner.game.enter_main_phase();
    let view = runner.pending_selection_view().expect("trash pick pending");
    let offered = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .count();
    assert_eq!(offered, 1, "only the [Evil Dragon] card is a candidate");
}

#[test]
fn ex11_005_declining_leaves_everything_untouched_and_trashes_nothing() {
    let mut runner = runner_with(&["DARK-DRAGON"], &["H1", "H2", "H3"]);
    let before = runner.game.memory;
    runner.game.enter_main_phase();
    runner
        .execute_action(0, PASS)
        .expect("decline the digivolve");
    let _ = runner.auto_resolve();
    assert_eq!(top_id(&runner), "CARRIER");
    assert_eq!(runner.game.memory, before);
    assert_eq!(
        runner.game.players[0].hand.len(),
        3,
        "no hand trash without a digivolve"
    );
}

/// No [Dark Dragon]/[Evil Dragon] Digimon in the trash: nothing to resolve.
#[test]
fn ex11_005_no_offer_without_a_matching_trash_card() {
    let mut runner = runner_with(&["OTHER"], &["H1", "H2"]);
    runner.game.enter_main_phase();
    let only_pass = match runner.pending_selection_view() {
        None => true,
        Some(v) => v.valid_action_ids.iter().all(|&a| a == PASS),
    };
    assert!(only_pass, "no candidate -> no real choice");
    assert_eq!(runner.game.players[0].hand.len(), 2);
}
