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

// ═════════════════════════ HiAndromon (EX12-058) ═════════════════════════

#[test]
fn ex12_058_reveal_plays_a_cost_7_or_lower_card_and_trashes_the_rest() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-058")
        .expect("EX12-058 YAML loads")
        .add_card(me("MACH-7", CardColor::Black, 5, 7, &["Machine"]))
        .add_card(me("CYB-8", CardColor::Black, 5, 8, &["Cyborg"]))
        .add_card(me("PLAIN", CardColor::Black, 3, 3, &["Beast"]))
        .add_card(me("FILL", CardColor::Black, 3, 3, &["Beast"]))
        .hand(0, &["EX12-058"])
        .deck(0, &deck_top(&["CYB-8", "MACH-7", "PLAIN"]))
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    let slot = hand_index(&r, 0, "EX12-058");
    assert!(r.play(0, slot).is_some());
    assert_eq!(reveal_offered(&r), vec!["MACH-7".to_string()], "cost 8 is too high");
    reveal_pick(&mut r, "MACH-7");
    let _ = r.auto_resolve();
    assert!(field_contains(&r, 0, "MACH-7"));
    assert!(trash_contains(&r, 0, "CYB-8"));
    assert!(trash_contains(&r, 0, "PLAIN"));
}

#[test]
fn ex12_058_gives_all_your_me_digimon_alliance_and_reboot() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-058")
        .expect("EX12-058 YAML loads")
        .add_card(me("ME-4", CardColor::Black, 4, 5, &["ME"]))
        .add_card(me("BEAST-4", CardColor::Black, 4, 5, &["Beast"]))
        .start();
    let hi = r.place_on_field(0, "EX12-058", Some(0));
    let ally = r.place_on_field(0, "ME-4", Some(1));
    let beast = r.place_on_field(0, "BEAST-4", Some(2));
    r.game.tick_declarative_effects();
    for kw in [Keyword::Alliance, Keyword::Reboot] {
        assert!(r.game.has_keyword(hi, kw), "HiAndromon is itself an [ME] Digimon");
        assert!(r.game.has_keyword(ally, kw));
        assert!(!r.game.has_keyword(beast, kw));
    }
}

