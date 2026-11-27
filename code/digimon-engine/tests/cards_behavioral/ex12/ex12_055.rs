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

// ═════════════════════════ Andromon (EX12-055) ═════════════════════════

fn andromon_runner(top: &[&'static str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX12-055")
        .expect("EX12-055 YAML loads")
        .add_card(me("ME-3", CardColor::Black, 3, 3, &["ME"]))
        .add_card(me("MACH-5", CardColor::Black, 4, 5, &["Machine"]))
        .add_card(me("CYB-6", CardColor::Black, 5, 6, &["Cyborg"]))
        .add_card(me("PLAIN", CardColor::Black, 3, 3, &["Beast"]))
        .add_card(me("FILL", CardColor::Black, 3, 3, &["Beast"]))
        .hand(0, &["EX12-055"])
        .deck(0, &deck_top(top))
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start()
}

#[test]
fn ex12_055_on_play_may_play_a_cost_5_or_lower_machine_cyborg_me_card_free_and_trashes_the_rest() {
    let mut r = andromon_runner(&["ME-3", "CYB-6", "PLAIN"]);
    let slot = hand_index(&r, 0, "EX12-055");
    let memory_before = r.memory();
    assert!(r.play(0, slot).is_some());
    // CYB-6 costs 6 (> 5) and PLAIN is not [Machine]/[Cyborg]/[ME].
    assert_eq!(reveal_offered(&r), vec!["ME-3".to_string()]);
    reveal_pick(&mut r, "ME-3");
    let _ = r.auto_resolve();
    assert!(field_contains(&r, 0, "ME-3"));
    assert_eq!(memory_before - r.memory(), 7, "only Andromon's own cost 7 is paid");
    assert!(trash_contains(&r, 0, "CYB-6"));
    assert!(trash_contains(&r, 0, "PLAIN"));
}

#[test]
fn ex12_055_on_play_the_play_is_optional_and_the_rest_is_trashed_anyway() {
    let mut r = andromon_runner(&["MACH-5", "PLAIN", "CYB-6"]);
    let slot = hand_index(&r, 0, "EX12-055");
    assert!(r.play(0, slot).is_some());
    assert_eq!(reveal_offered(&r), vec!["MACH-5".to_string()]);
    r.execute_action(0, PASS).expect("decline the free play");
    let _ = r.auto_resolve();
    assert!(!field_contains(&r, 0, "MACH-5"));
    assert!(trash_contains(&r, 0, "MACH-5"));
    assert!(trash_contains(&r, 0, "PLAIN"));
    assert!(trash_contains(&r, 0, "CYB-6"));
}

#[test]
fn ex12_055_printed_digivolve_lines_and_clause_shape() {
    let r = andromon_runner(&["PLAIN", "PLAIN", "PLAIN"]);
    let card = r.compiled_card("EX12-055").unwrap();
    assert!(card.alt_paths.iter().any(|p| p.kind == CompiledAltPathKind::DnaDigivolve));
    assert!(
        card.alt_paths.iter().filter(|p| p.kind == CompiledAltPathKind::Digivolve).count() >= 2,
        "black Lv.4 / 4 and Lv.4 w/[Machine]/[ME] / 3"
    );
    let clauses = triggered_clauses(&r, "EX12-055");
    assert!(clauses.contains(&(CompiledScope::FaceUp, vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving])));
    assert!(clauses.contains(&(CompiledScope::FaceUp, vec![CompiledTiming::Counter])));
    assert!(clauses.contains(&(CompiledScope::Inherited, vec![CompiledTiming::OnOpponentAttack])));
}

