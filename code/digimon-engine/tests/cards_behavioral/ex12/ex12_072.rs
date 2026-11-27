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

// ═════════════════════════ Metal Empire (EX12-072) ═════════════════════════

fn metal_empire_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("EX12-072")
        .expect("EX12-072 YAML loads")
        .add_card(me("ME-ENABLER", CardColor::Black, 4, 5, &["ME"]))
        .add_card(me("ME-LV5", CardColor::Black, 5, 7, &["ME"]))
        .add_card(me("ME-LV6", CardColor::Black, 6, 9, &["ME"]))
        .add_card(me("PLAIN-LV4", CardColor::Black, 4, 5, &["Beast"]))
        .add_card(me("BOTTOM", CardColor::Black, 3, 3, &["Beast"]))
        .add_card(me("ATTACKER", CardColor::Red, 4, 5, &["Beast"]))
}

#[test]
fn ex12_072_use_requirement_needs_an_me_field_card() {
    let mut r = metal_empire_builder().hand(0, &["EX12-072"]).memory(5).start();
    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid,
        "<Use Req. ([ME] trait)> without an [ME] Digimon/Tamer cannot ignore the colour"
    );
}

#[test]
fn ex12_072_main_adds_bottom_security_to_hand_and_places_self_face_up_bottom() {
    let mut r = metal_empire_builder()
        .hand(0, &["EX12-072"])
        .security(0, &["BOTTOM"])
        .memory(5)
        .start();
    r.place_on_field(0, "ME-ENABLER", Some(0));
    let slot = hand_index(&r, 0, "EX12-072");
    assert_ne!(r.game.play_option_from_hand(0, slot), OptionPlayResult::Invalid);
    r.auto_resolve().expect("resolve Metal Empire Main");
    assert!(hand_contains(&r, 0, "BOTTOM"));
    assert_eq!(r.game.players[0].security[0].card_id(&r.game.card_data), "EX12-072");
    let idx = r.game.players[0].security[0].card_index;
    assert!(r.game.players[0].face_up_security.contains(&idx), "placed face up");
}

#[test]
fn ex12_072_security_effect_plays_a_level_5_or_lower_me_digimon_from_hand_free() {
    let mut r = metal_empire_builder()
        .hand(1, &["ME-LV5", "ME-LV6", "PLAIN-LV4"])
        .security(1, &["EX12-072"])
        .memory(0)
        .start();
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    let memory_before = r.memory();
    r.attack_player(attacker, 1, false);
    let view = r.pending_selection_view().expect("security effect prompts");
    assert_eq!(view.kind, SelectionKind::Hand);
    // Only the level 5 [ME] Digimon qualifies (level 6 is too high; the Beast is not [ME]).
    let offered = view.valid_action_ids.iter().filter(|&&a| a != PASS).count();
    assert_eq!(offered, 1);
    select_hand_card(&mut r, 1, "ME-LV5");
    r.auto_resolve().expect("resolve Metal Empire security effect");
    assert!(field_contains(&r, 1, "ME-LV5"));
    assert_eq!(r.memory(), memory_before, "played without paying the cost");
}

#[test]
fn ex12_072_face_up_in_security_gives_me_digimon_guard() {
    let mut r = metal_empire_builder().security(0, &["EX12-072"]).start();
    let me_digimon = r.place_on_field(0, "ME-ENABLER", Some(0));
    let other = r.place_on_field(0, "PLAIN-LV4", Some(1));
    assert!(!r.game.has_keyword(me_digimon, Keyword::Guard), "face-down: no aura");
    let idx = r.game.players[0].security[0].card_index;
    r.game.players[0].face_up_security.insert(idx);
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(me_digimon, Keyword::Guard), "[ME] Digimon gain <Guard>");
    assert!(!r.game.has_keyword(other, Keyword::Guard), "non-[ME] Digimon do not");
}

