#![allow(unused_imports)]
use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, Keyword, ModifierType};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

use super::ex12_059::dramon_runner;
use super::me_machine_puppet_big::*;
use super::support::{
    digimon, field_contains, hand_contains, hand_index, push_to_trash, select_hand_card,
    source_count, trash_contains, DebugRunner,
};

// ═════════════════════════ Chaosdramon (EX12-060) ═════════════════════════

#[test]
fn ex12_060_printed_keywords_dna_and_assembly() {
    let mut r = dramon_runner("EX12-060");
    let chaos = r.place_on_field(0, "EX12-060", Some(0));
    assert!(r.game.has_keyword(chaos, Keyword::Piercing));
    assert!(r.game.has_keyword(chaos, Keyword::Fragment(2)));
    assert!(r.game.has_keyword(chaos, Keyword::Engage));
    assert!(r.game.has_keyword(chaos, Keyword::SecurityAttackPlus(1)));
    let card = r.compiled_card("EX12-060").unwrap();
    assert!(card.alt_paths.iter().any(|p| p.kind == CompiledAltPathKind::DnaDigivolve));
    assert!(card.alt_paths.iter().any(|p| p.kind == CompiledAltPathKind::Assembly));
}

#[test]
fn ex12_060_de_digivolves_all_then_deletes_two_by_play_cost_after_placing_two() {
    let mut r = dramon_runner("EX12-060");
    push_to_trash(&mut r, 0, "ME-B");
    let chaos = r.place_on_field(0, "EX12-060", Some(0));
    r.place_on_field(1, "CHEAP-A", Some(0));
    r.place_on_field(1, "CHEAP-B", Some(1));
    r.place_on_field(1, "OPP-LV5", Some(2));
    r.fire_on_play(0, chaos.index as usize);
    for _ in 0..2 {
        let view = r.pending_selection_view().expect("placement pick");
        let a = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        r.execute_action(0, a).expect("place a card");
    }
    // Two digivolution cards under Chaosdramon: Digimon with play cost <= 2 are deleted (2 of them).
    let _ = r.auto_resolve();
    assert_eq!(source_count(&r, 0, chaos.index as usize), 2);
    assert!(!field_contains(&r, 1, "CHEAP-A"));
    assert!(!field_contains(&r, 1, "CHEAP-B"));
    assert!(field_contains(&r, 1, "OPP-LV5"), "play cost 7 > 2 is untouched");
}

#[test]
fn ex12_060_declining_the_placement_deletes_nothing() {
    let mut r = dramon_runner("EX12-060");
    push_to_trash(&mut r, 0, "ME-B");
    let chaos = r.place_on_field(0, "EX12-060", Some(0));
    r.place_on_field(1, "CHEAP-A", Some(0));
    r.fire_on_play(0, chaos.index as usize);
    let view = r.pending_selection_view().expect("first placement pick");
    r.execute_action(0, PASS).expect("decline the placement");
    let _ = view;
    let _ = r.auto_resolve();
    assert!(field_contains(&r, 1, "CHEAP-A"), "no placement, no deletion");
    assert_eq!(source_count(&r, 0, chaos.index as usize), 0);
}

