//! BT16-001 Poromon — Digi-Egg Lv.2, Red, cost 0.
//!
//! Printed text (data/card_bundles/BT16-001.md, official Bandai DB):
//!
//! ```text
//! Inherited: [When Attacking][Once Per Turn] If this Digimon has 2 or more
//!   colors, delete 1 of your opponent's Digimon with 2000 DP or less.
//! ```
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/BT16/Red/BT16_001.cs. Synthetic boards.

#![allow(dead_code, unused_imports)]

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};

fn digimon(id: &str, dp: i32, colors: Vec<CardColor>) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c.colors = colors;
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT16-001")
        .expect("BT16-001 compiles into the embedded pack")
        .add_card(digimon("T-HOST2", 6000, vec![CardColor::Red, CardColor::Yellow]))
        .add_card(digimon("T-HOST1", 6000, vec![CardColor::Red]))
        .add_card(digimon("T-SMALL", 2000, vec![CardColor::Blue]))
        .add_card(digimon("T-BIG", 2100, vec![CardColor::Blue]))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL", "FILL"])
        .memory(5)
}

fn on_field(runner: &DebugRunner, p: usize, id: &str) -> bool {
    runner.game.players[p]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == id)
}

/// 2+ colours: delete one opposing Digimon with 2000 DP or less (mandatory pick,
/// only the <= 2000 DP Digimon is offered).
#[test]
fn bt16_001_two_colour_host_deletes_a_small_opposing_digimon_when_attacking() {
    let mut runner = base().start();
    let host = runner.place_stack(0, &["BT16-001", "T-HOST2"]);
    runner.place_on_field(1, "T-SMALL", None);
    runner.place_on_field(1, "T-BIG", None);
    runner.game.enter_main_phase();
    let _ = runner.attack_player(host, 1, false);
    let view = runner.pending_selection_view().expect("delete target parked");
    assert!(!view.is_optional, "the delete pick is mandatory once it triggers");
    assert_eq!(view.valid_action_ids.len(), 1, "only the 2000-DP Digimon is offered");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("pick it");
    runner.auto_resolve().expect("finish");
    assert!(!on_field(&runner, 1, "T-SMALL"), "the 2000-DP Digimon was deleted");
    assert!(on_field(&runner, 1, "T-BIG"), "the 2100-DP Digimon was not");
}

/// A single-coloured host does not trigger the effect.
#[test]
fn bt16_001_single_colour_host_does_nothing() {
    let mut runner = base().start();
    let host = runner.place_stack(0, &["BT16-001", "T-HOST1"]);
    runner.place_on_field(1, "T-SMALL", None);
    runner.game.enter_main_phase();
    let _ = runner.attack_player(host, 1, false);
    runner.auto_resolve().expect("finish");
    assert!(on_field(&runner, 1, "T-SMALL"), "one colour: no delete");
}
