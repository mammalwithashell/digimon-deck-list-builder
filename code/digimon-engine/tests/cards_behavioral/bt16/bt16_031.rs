//! BT16-031 Gatomon — Lv.4 Yellow + Purple, cost 4, 4000 DP.
//!
//! Printed text (data/card_bundles/BT16-031.md, official Bandai DB):
//!
//! ```text
//! [Digivolve] [Salamon]: Cost 2
//! <Barrier>
//! [On Play] [When Digivolving] By trashing 1 card in your hand, return 1 level
//!   6 or lower 2-color Digimon card that's red or purple from your trash to
//!   the hand.
//! Inherited: [Your Turn] All of your opponent's Security Digimon get -3000 DP.
//! ```
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/BT16/Yellow/BT16_031.cs. Synthetic boards.

#![allow(dead_code, unused_imports)]

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

fn digimon(id: &str, level: u8, colors: Vec<CardColor>) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = colors;
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT16-031")
        .expect("BT16-031 compiles into the embedded pack")
        .add_card(digimon("T-RP5", 5, vec![CardColor::Red, CardColor::Purple]))
        .add_card(digimon("T-RB6", 6, vec![CardColor::Red, CardColor::Blue]))
        .add_card(digimon("T-PURE", 5, vec![CardColor::Purple]))
        .add_card(digimon("T-BY5", 5, vec![CardColor::Blue, CardColor::Yellow]))
        .add_card(digimon("T-RP7", 7, vec![CardColor::Red, CardColor::Purple]))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
}

fn in_hand(runner: &DebugRunner, id: &str) -> bool {
    runner.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == id)
}

#[test]
fn bt16_031_has_salamon_alt_path_and_three_effects() {
    let runner = base().memory(10).start();
    let compiled = runner.compiled_card("BT16-031").expect("compiled");
    assert_eq!(compiled.alt_paths.len(), 1, "[Digivolve] [Salamon]: Cost 2");
    assert_eq!(compiled.effects.len(), 3, "Barrier, On Play/When Digivolving, inherited aura");
}

/// By trashing 1 hand card, return a level 6 or lower, exactly-2-colour,
/// red-or-purple Digimon from the trash. The pick offers only those cards.
#[test]
fn bt16_031_on_play_trashes_a_card_to_return_a_two_colour_red_or_purple_digimon() {
    let mut runner = base().hand(0, &["BT16-031", "FILL"]).memory(10).start();
    runner.inject_trash(0, "T-RP5");
    runner.inject_trash(0, "T-RB6");
    runner.inject_trash(0, "T-PURE");
    runner.inject_trash(0, "T-BY5");
    runner.inject_trash(0, "T-RP7");
    runner.game.enter_main_phase();
    let trash_before = runner.trash_size(0);
    runner.play(0, 0);
    runner.game.drain_effect_queue();

    // The "by trashing" cost: pick the hand card (optional: you may decline).
    let view = runner.pending_selection_view().expect("trash-a-card cost parked");
    assert_eq!(view.kind, SelectionKind::Hand);
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("trash the filler");
    runner.game.drain_effect_queue();

    // The mandatory return pick: only T-RP5 and T-RB6 qualify.
    let view = runner.pending_selection_view().expect("return pick parked");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert!(!view.is_optional, "the return is mandatory once the cost is paid");
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "level<=6, exactly 2 colours, red or purple: T-RP5 and T-RB6 only"
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("return it");
    runner.auto_resolve().expect("finish");
    assert!(
        in_hand(&runner, "T-RP5") || in_hand(&runner, "T-RB6"),
        "a qualifying Digimon returned to the hand"
    );
    // filler trashed (+1), one card returned (-1): the trash net is unchanged.
    assert_eq!(runner.trash_size(0), trash_before);
}

/// Declining the "by trashing" cost does nothing.
#[test]
fn bt16_031_on_play_may_be_declined() {
    let mut runner = base().hand(0, &["BT16-031", "FILL"]).memory(10).start();
    runner.inject_trash(0, "T-RP5");
    runner.game.enter_main_phase();
    runner.play(0, 0);
    runner.game.drain_effect_queue();
    runner.decline_optional_trigger().expect("decline");
    runner.auto_resolve().expect("finish");
    assert!(!in_hand(&runner, "T-RP5"), "declined: nothing returned");
}
