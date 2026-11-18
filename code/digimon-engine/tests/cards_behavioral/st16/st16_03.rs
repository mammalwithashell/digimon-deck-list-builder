//! ST16-03 Gabumon — Lv.3 Purple, cost 3, 1000 DP.
//!
//! Printed text (data/card_bundles/ST16-03.md, official Bandai DB):
//!
//! ```text
//! [Digivolve] [Tsunomon]: Cost 0
//! [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//! Inherited: [When Attacking][Once Per Turn] <Draw 1>. Then, trash 1 card in
//!   your hand.
//! ```
//!
//! Official Q&A: Digimon in breeding areas aren't checked by the opening effect.
//! DCGO: DCGO/Assets/Scripts/CardEffect/ST16/Purple/ST16_03.cs. Boards are
//! synthetic.

#![allow(dead_code, unused_imports)]

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c
}

fn base() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("ST16-03")
        .expect("ST16-03 compiles into the embedded pack")
        .add_card(digimon("T-OPP", "Opp Digimon", 3, 2000))
        .add_card(digimon("T-HOST", "Host", 4, 4000))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
}

#[test]
fn st16_03_has_tsunomon_alt_path() {
    let runner = base().memory(3).start();
    let compiled = runner.compiled_card("ST16-03").expect("compiled");
    assert_eq!(compiled.alt_paths.len(), 1, "[Digivolve] [Tsunomon]: Cost 0");
    assert_eq!(compiled.cost, Some(3));
}

#[test]
fn st16_03_start_of_main_gains_memory_when_opponent_has_a_digimon() {
    let mut runner = base().memory(3).start();
    runner.place_on_field(0, "ST16-03", Some(0));
    runner.place_on_field(1, "T-OPP", None);
    let before = runner.game.memory;
    runner.game.enter_main_phase();
    runner.auto_resolve().expect("resolve");
    assert_eq!(runner.game.memory, before + 1, "+1 memory with an opposing Digimon");
}

#[test]
fn st16_03_start_of_main_does_nothing_without_an_opposing_digimon() {
    let mut runner = base().memory(3).start();
    runner.place_on_field(0, "ST16-03", Some(0));
    let before = runner.game.memory;
    runner.game.enter_main_phase();
    runner.auto_resolve().expect("resolve");
    assert_eq!(runner.game.memory, before, "no opposing Digimon: no memory");
}

/// Official Q&A: a Digimon in the opponent's breeding area is not checked.
#[test]
fn st16_03_start_of_main_ignores_the_opponents_breeding_area() {
    let mut runner = base().memory(3).start();
    runner.place_on_field(0, "ST16-03", Some(0));
    runner.place_in_breeding(1, "T-OPP");
    let before = runner.game.memory;
    runner.game.enter_main_phase();
    runner.auto_resolve().expect("resolve");
    assert_eq!(runner.game.memory, before, "breeding-area Digimon do not count");
}

/// Inherited [When Attacking][OPT]: when the host attacks, draw 1 then trash 1.
#[test]
fn st16_03_inherited_when_attacking_draws_then_trashes() {
    let mut runner = base()
        .hand(0, &["FILL"])
        .security(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    let host = runner.place_stack(0, &["ST16-03", "T-HOST"]);
    runner.game.enter_main_phase();
    runner.auto_resolve().expect("main-phase triggers");
    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);
    let trash_before = runner.trash_size(0);
    let _ = runner.attack_player(host, 1, false);
    // <Draw 1> happened; the trash-1 pick is mandatory.
    let view = runner
        .pending_selection_view()
        .expect("the trash-1-card pick is parked");
    assert!(!view.is_optional, "the discard is mandatory");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("trash a card");
    runner.auto_resolve().expect("finish the attack");
    assert_eq!(runner.deck_size(0), deck_before - 1, "drew exactly one card");
    assert_eq!(runner.hand_size(0), hand_before, "+1 drawn, -1 trashed");
    assert_eq!(runner.trash_size(0), trash_before + 1, "one card trashed");
}
