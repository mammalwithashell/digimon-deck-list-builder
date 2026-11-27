#![allow(unused_imports)]
use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost, CompiledPredicate};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS, SEL_REVEAL_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, Keyword, PlaySource};
use digimon_engine::selection::SelectionKind;

use super::me_machine_puppet::*;
use super::support::{
    digimon, hand_contains, hand_ids, select_hand_card, trash_contains, DebugRunner,
};

// ═════════════════════════ Hagurumon (EX12-053) ═════════════════════════

fn hagurumon_runner(top: &[&str]) -> DebugRunner {
    let deck = deck_with_top(top, "FILL", 4);
    DebugRunner::builder()
        .dsl_card("EX12-053")
        .expect("EX12-053 YAML loads")
        .add_card(me("DUAL", CardColor::Black, 4, &["Machine", "ME"]))
        .add_card(me("MACH", CardColor::Black, 4, &["Machine"]))
        .add_card(me("CYB", CardColor::Black, 4, &["Cyborg"]))
        .add_card(me("ME-ONLY", CardColor::Black, 4, &["ME"]))
        .add_card(me("PLAIN", CardColor::Black, 4, &["Beast"]))
        .add_card(me("FILL", CardColor::Black, 3, &["Beast"]))
        .hand(0, &["EX12-053"])
        .deck(0, &deck)
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start()
}

fn play_hagurumon(r: &mut DebugRunner) {
    let slot = hand_slot(r, "EX12-053");
    assert!(r.play(0, slot).is_some(), "Hagurumon should be playable");
}

#[test]
fn ex12_053_on_play_adds_one_machine_cyborg_mutant_and_one_me_card() {
    let mut r = hagurumon_runner(&["DUAL", "MACH", "ME-ONLY"]);
    play_hagurumon(&mut r);
    assert_eq!(
        r.pending_kind().map(|k| format!("{k:?}")).unwrap_or_default().split(' ').next().unwrap(),
        "RevealBucket"
    );
    assert_eq!(offered_reveal_ids(&r), vec!["DUAL".to_string(), "MACH".to_string()]);
    pick_revealed(&mut r, "DUAL");
    // One card never fills both slots: DUAL is out of the [ME] bucket.
    assert_eq!(offered_reveal_ids(&r), vec!["ME-ONLY".to_string()]);
    pick_revealed(&mut r, "ME-ONLY");
    let _ = r.auto_resolve();
    let mut hand = hand_ids(&r, 0);
    hand.sort();
    assert_eq!(hand, vec!["DUAL".to_string(), "ME-ONLY".to_string()]);
    assert_eq!(deck_bottom_ids(&r, 1), vec!["MACH".to_string()], "rest to the deck bottom");
    assert_eq!(r.deck_size(0), 5);
}

#[test]
fn ex12_053_a_machine_and_me_card_fills_exactly_one_slot() {
    let mut r = hagurumon_runner(&["DUAL", "PLAIN", "FILL"]);
    play_hagurumon(&mut r);
    assert_eq!(offered_reveal_ids(&r), vec!["DUAL".to_string()]);
    pick_revealed(&mut r, "DUAL");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["DUAL".to_string()]);
    assert_eq!(r.deck_size(0), 6);
}

#[test]
fn ex12_053_cyborg_and_mutant_count_for_the_first_slot() {
    let mut r = hagurumon_runner(&["CYB", "ME-ONLY", "PLAIN"]);
    play_hagurumon(&mut r);
    assert_eq!(offered_reveal_ids(&r), vec!["CYB".to_string()]);
    pick_revealed(&mut r, "CYB");
    assert_eq!(offered_reveal_ids(&r), vec!["ME-ONLY".to_string()]);
    pick_revealed(&mut r, "ME-ONLY");
    let _ = r.auto_resolve();
    let mut hand = hand_ids(&r, 0);
    hand.sort();
    assert_eq!(hand, vec!["CYB".to_string(), "ME-ONLY".to_string()]);
}

#[test]
fn ex12_053_no_match_returns_all_three_to_the_bottom() {
    let mut r = hagurumon_runner(&["PLAIN", "FILL", "PLAIN"]);
    play_hagurumon(&mut r);
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).is_empty());
    assert_eq!(r.deck_size(0), 7);
}

#[test]
fn ex12_053_digivolves_from_a_level_2_me_digimon_for_free() {
    // Red Lv.2 [ME]: not the printed black circle, so only the `Lv.2 w/[ME]
    // trait: Cost 0` line can allow this.
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-053")
        .expect("EX12-053 YAML loads")
        .add_card(me("RED-ME-2", CardColor::Red, 2, &["ME"]))
        .add_card(me("RED-PLAIN-2", CardColor::Red, 2, &["Beast"]))
        .hand(0, &["EX12-053", "EX12-053"])
        .memory(5)
        .start();
    let me_base = r.place_on_field(0, "RED-ME-2", Some(0));
    let plain_base = r.place_on_field(0, "RED-PLAIN-2", Some(1));
    assert_me_alt_path(&r, "EX12-053", 2, 0);
    assert_eq!(digivolve_cost(&mut r, "EX12-053", me_base.index as usize), Some(0));
    r.auto_resolve().ok();
    assert_eq!(
        digivolve_cost(&mut r, "EX12-053", plain_base.index as usize),
        None,
        "a non-[ME] red Lv.2 is not a legal base"
    );
}

#[test]
fn ex12_053_inherited_grants_blocker() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-053")
        .expect("EX12-053 YAML loads")
        .add_card(me("CARRIER", CardColor::Black, 4, &["Machine"]))
        .start();
    let carrier = r.place_stack(0, &["EX12-053", "CARRIER"]);
    assert!(r.game.has_keyword(carrier, Keyword::Blocker));
}

