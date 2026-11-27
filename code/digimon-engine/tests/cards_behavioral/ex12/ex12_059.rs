#![allow(unused_imports)]
use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, Keyword, ModifierType};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

use super::me_machine_puppet_big::*;
use super::support::{
    digimon, field_contains, hand_contains, hand_index, push_to_trash, select_hand_card,
    source_count, trash_contains, DebugRunner,
};

// ═════════════════════════ Machinedramon (EX12-059) ═════════════════════════

pub(super) fn dramon_runner(card: &str) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(card)
        .unwrap_or_else(|e| panic!("{card} YAML loads: {e}"))
        .add_card(me("OPP-LV3", CardColor::Red, 3, 3, &["Beast"]))
        .add_card(me("OPP-LV4", CardColor::Red, 4, 5, &["Beast"]))
        .add_card(me("OPP-LV5", CardColor::Red, 5, 7, &["Beast"]))
        .add_card(me("OPP-BASE", CardColor::Red, 3, 3, &["Beast"]))
        .add_card(me("ME-A", CardColor::Black, 4, 5, &["ME"]))
        .add_card(me("ME-B", CardColor::Black, 5, 7, &["Machine"]))
        .add_card(me("TOO-BIG", CardColor::Black, 6, 9, &["ME"]))
        .add_card(me("CHEAP-A", CardColor::Red, 3, 2, &["Beast"]))
        .add_card(me("CHEAP-B", CardColor::Red, 3, 1, &["Beast"]))
        .add_card(me("FILL", CardColor::Black, 3, 3, &["Beast"]))
        .hand(0, &["ME-A", "TOO-BIG"])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .start()
}

#[test]
fn ex12_059_de_digivolves_3_then_places_two_cards_and_locks_stack_trashing() {
    let mut r = dramon_runner("EX12-059");
    push_to_trash(&mut r, 0, "ME-B");
    let dramon = r.place_on_field(0, "EX12-059", Some(0));
    let mine = r.place_on_field(0, "FILL", Some(1));
    let opp = r.place_stack(1, &["OPP-BASE", "OPP-LV3", "OPP-LV4", "OPP-LV5"]); // bottom -> top
    let sources_before = source_count(&r, 0, dramon.index as usize);
    r.fire_on_play(0, dramon.index as usize);
    // <De-Digivolve 3> 1 of your opponent's Digimon (the only one: auto/mandatory pick)
    if r.pending_kind() == Some(SelectionKind::OppField) {
        let view = r.pending_selection_view().unwrap();
        let a = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        r.execute_action(0, a).expect("pick the de-digivolve target");
    }
    // <De-Digivolve 3> is "up to 3": answer the §16-12-1 count declaration with 3.
    r.declare_max_count().expect("de-digivolve count declaration");
    // Two placements: ME-A from the hand, ME-B from the trash (TOO-BIG is level 6).
    for _ in 0..2 {
        assert!(r.pending_selection_view().is_some(), "expected a placement pick");
        let view = r.pending_selection_view().unwrap();
        let a = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        r.execute_action(0, a).expect("place a card");
    }
    let _ = r.auto_resolve();
    assert_eq!(source_count(&r, 0, dramon.index as usize), sources_before + 2);
    for perm in [dramon, mine] {
        assert!(
            r.modifiers().has(perm, ModifierType::ImmuneFromStackTrashing),
            "opponent effects can't trash any of your Digimon's stacked cards"
        );
    }
    assert!(hand_contains(&r, 0, "TOO-BIG"), "level 6 can not be placed");
    // <De-Digivolve 3> trashes the level 5 and level 4 cards and stops at the level 3 card
    // ("You can't trash past level 3 cards").
    assert_eq!(
        r.game.players[1].battle_area[0].top_card().card_id(&r.game.card_data),
        "OPP-LV3"
    );
    assert!(trash_contains(&r, 1, "OPP-LV5"));
    assert!(trash_contains(&r, 1, "OPP-LV4"));
    let _ = opp;
}

#[test]
fn ex12_059_declining_the_placement_installs_no_protection() {
    let mut r = dramon_runner("EX12-059");
    let dramon = r.place_on_field(0, "EX12-059", Some(0));
    r.place_on_field(1, "OPP-LV3", Some(0));
    r.fire_on_play(0, dramon.index as usize);
    if r.pending_kind() == Some(SelectionKind::OppField) {
        let view = r.pending_selection_view().unwrap();
        let a = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        r.execute_action(0, a).expect("pick the de-digivolve target");
    }
    // <De-Digivolve 3> is "up to 3": answer the §16-12-1 count declaration with 3.
    r.declare_max_count().expect("de-digivolve count declaration");
    // Only ONE qualifying card (ME-A) in hand+trash: the placement needs 2, so it never prompts.
    let _ = r.auto_resolve();
    assert!(!r.modifiers().has(dramon, ModifierType::ImmuneFromStackTrashing));
}

#[test]
fn ex12_059_keywords_and_ace_overflow_are_printed() {
    let mut r = dramon_runner("EX12-059");
    let dramon = r.place_on_field(0, "EX12-059", Some(0));
    assert!(r.game.has_keyword(dramon, Keyword::Reboot));
    assert!(r.game.has_keyword(dramon, Keyword::Fragment(2)));
    assert!(r.game.has_keyword(dramon, Keyword::BlastDigivolve));
}

