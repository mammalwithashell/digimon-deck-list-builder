//! EX13-066 Sistermon Noir (Awakened) — DUAL card (Digimon + Option).
//! Digimon face: Lv.4 White/Black, DP 6000, Puppet, Champion, Virus (+ Data via
//! Rule). No play cost (reached only by digivolving / Arts digivolving).
//! Option face: White, use cost 5.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-066.md`;
//! per-card JSON `code/digimon-engine/cards/ex13/EX13-066.json`)
//!
//! DIGIMON FACE
//!   [Digivolve] [Sistermon Noir]/[Sistermon Ciel]: Cost 1
//!   [Digivolve] Lv.3 w/[Huckmon] in text: Cost 3
//!   ＜Decode ([Sistermon Noir]/[Sistermon Ciel])＞
//!   [When Digivolving] Delete 1 of your opponent's Digimon with a play cost of
//!   4 or less.
//!   (Rule) Also treated as Name: [Sistermon Ciel (Awakened)] and Trait: [Data]
//!   Attribute.
//! OPTION FACE
//!   [Main] You may play 1 play cost 4 or lower card with [Sistermon] in its
//!   name from your hand or trash without paying the cost. Then, to 1 of your
//!   opponent's Digimon, ＜De-Digivolve 1＞ for each of your Digimon.
//!   ＜Arts Digivolve＞
//!
//! # DCGO C# reference
//! None at the submodule (no EX13_066.cs at b9a0638cd). Printed text +
//! general_rule.pdf (16-35 <Decode>, <De-Digivolve>) govern.
//!
//! # Pattern rows
//! - G-DSL-DUAL-PER-FACE-EFFECTS, G-DSL-ARTS-DIGIVOLVE
//! - Two-name <Decode> replacement (BT25-025 idiom, name_in)
//! - [When Digivolving] mandatory delete by play cost (BT23-077 idiom)
//! - also_treated_as name (BT23-077 idiom)
//! - A-union hand-or-trash free play + formula-magnitude <De-Digivolve N>

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::action::space::{encode_attack, PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "EX13-066";

fn digimon(id: &str, name: &str, level: u8, cost: u16, dp: i32, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.play_cost = cost;
    c.dp = Some(dp);
    c.colors = vec![color];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-066 YAML loads from the embedded pack")
        .add_card(digimon("SIS-NOIR", "Sistermon Noir", 4, 4, 4000, CardColor::White))
        .add_card(digimon("SIS-CIEL", "Sistermon Ciel", 4, 4, 4000, CardColor::White))
        .add_card(digimon("SIS-BLANC", "Sistermon Blanc", 3, 3, 3000, CardColor::White))
        .add_card(digimon("SIS-BIG", "Sistermon Grande", 5, 5, 6000, CardColor::White))
        .add_card({
            let mut c = digimon("HUCK-L3", "Huckmon", 3, 3, 3000, CardColor::Red);
            c.effect_text = String::new();
            c
        })
        .add_card({
            let mut c = digimon("HUCK-TEXT-L3", "Textmon", 3, 3, 3000, CardColor::Red);
            c.effect_text = "Treated as [Huckmon] for digivolution.".to_string();
            c
        })
        .add_card(digimon("PLAIN-L3", "Plainmon", 3, 3, 3000, CardColor::Red))
        .add_card(digimon("WHITE-ALLY", "White Ally", 4, 4, 5000, CardColor::White))
        .add_card(digimon("OPP-C4", "Opp Four", 4, 4, 5000, CardColor::Blue))
        .add_card(digimon("OPP-C5", "Opp Five", 4, 5, 5000, CardColor::Blue))
        .add_card(digimon("OPP-L3", "Opp L3", 3, 3, 3000, CardColor::Blue))
        .add_card(digimon("OPP-L4", "Opp L4", 4, 5, 5000, CardColor::Blue))
        .add_card(digimon("OPP-L5", "Opp L5", 5, 7, 7000, CardColor::Blue))
}

fn field_ids(r: &DebugRunner, player: u8) -> Vec<String> {
    r.game
        .player(player)
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_pos(r: &DebugRunner, player: u8, id: &str) -> usize {
    r.game
        .player(player)
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} in hand"))
}

fn trash_pos(r: &DebugRunner, player: u8, id: &str) -> usize {
    r.game
        .player(player)
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} in trash"))
}

// ─── Section 1 — structural ──────────────────────────────────────────────────

#[test]
fn ex13_066_is_dual_with_both_faces() {
    let r = builder().start();
    let compiled = r.compiled_card(CARD_ID).expect("compiled EX13-066");
    assert_eq!(compiled.kind, CompiledCardKind::Dual);
    assert!(compiled.also_treated_as.iter().any(|n| n == "Sistermon Ciel (Awakened)"));
    let dual = compiled.dual.as_ref().unwrap();
    assert_eq!(dual.digimon.level, 4);
    assert_eq!(dual.digimon.dp, 6000);
    assert_eq!(dual.option.use_cost, 5);
    // Digimon face: <Decode> replacement + [When Digivolving] delete.
    assert_eq!(dual.digimon.effects.len(), 2);
    let wd = dual
        .digimon
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause");
    assert!(wd.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!wd.optional, "the delete is mandatory");
    assert_eq!(dual.option.effects.len(), 1);
    assert!(dual.option.keywords.iter().any(|k| k == "ArtsDigivolve"));
}

// ─── Digivolution conditions ─────────────────────────────────────────────────

fn digivolve_runner(base: &str) -> (DebugRunner, usize) {
    let mut r = builder().hand(0, &[CARD_ID]).memory(5).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, base, Some(0)).index as usize;
    (r, slot)
}

#[test]
fn ex13_066_digivolves_from_sistermon_noir_for_1() {
    let (mut r, slot) = digivolve_runner("SIS-NOIR");
    let mem = r.game.memory;
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(field_ids(&r, 0)[slot], CARD_ID);
    assert_eq!(mem - r.game.memory, 1);
}

#[test]
fn ex13_066_digivolves_from_sistermon_ciel_for_1() {
    let (mut r, slot) = digivolve_runner("SIS-CIEL");
    let mem = r.game.memory;
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(field_ids(&r, 0)[slot], CARD_ID);
    assert_eq!(mem - r.game.memory, 1);
}

#[test]
fn ex13_066_digivolves_from_huckmon_text_lv3_for_3() {
    let (mut r, slot) = digivolve_runner("HUCK-TEXT-L3");
    let mem = r.game.memory;
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(field_ids(&r, 0)[slot], CARD_ID);
    assert_eq!(mem - r.game.memory, 3);
}

#[test]
fn ex13_066_cannot_digivolve_from_plain_lv3() {
    let (mut r, slot) = digivolve_runner("PLAIN-L3");
    assert!(!r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(field_ids(&r, 0)[slot], "PLAIN-L3");
}

#[test]
fn ex13_066_cannot_digivolve_from_sistermon_blanc() {
    let (mut r, slot) = digivolve_runner("SIS-BLANC");
    assert!(!r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(field_ids(&r, 0)[slot], "SIS-BLANC");
}

// ─── [When Digivolving] delete play cost <= 4 ────────────────────────────────

#[test]
fn ex13_066_when_digivolving_deletes_cost_4_opponent_digimon() {
    let (mut r, slot) = digivolve_runner("SIS-NOIR");
    let c4 = r.place_on_field(1, "OPP-C4", Some(0));
    let c5 = r.place_on_field(1, "OPP-C5", Some(0));
    r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    let view = r.pending_selection_view().expect("delete pick");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!view.is_optional, "mandatory delete");
    assert!(view.valid_action_ids.contains(&encode_attack(0, c4.index as u16)));
    assert!(
        !view.valid_action_ids.contains(&encode_attack(0, c5.index as u16)),
        "play cost 5 is not 4 or less"
    );
    r.execute_action(0, encode_attack(0, c4.index as u16)).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP-C5".to_string()]);
}

#[test]
fn ex13_066_when_digivolving_spares_cost_5_opponent_digimon() {
    let (mut r, slot) = digivolve_runner("SIS-NOIR");
    r.place_on_field(1, "OPP-C5", Some(0));
    r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP-C5".to_string()]);
}

// ─── <Decode ([Sistermon Noir]/[Sistermon Ciel])> ───────────────────────────

fn accept_all(r: &mut DebugRunner) {
    while let Some(v) = r.pending_selection_view() {
        let a = v.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        r.execute_action(v.selecting_player, a).unwrap();
    }
}

#[test]
fn ex13_066_decode_plays_sistermon_ciel_source() {
    let mut r = builder().memory(5).start();
    let me = r.place_stack(0, &["SIS-CIEL", CARD_ID]);
    r.game.return_to_hand_from_effect(me, 1);
    let view = r.pending_selection_view().expect("<Decode> window");
    assert!(view.is_optional);
    accept_all(&mut r);
    assert!(field_ids(&r, 0).contains(&"SIS-CIEL".to_string()));
}

#[test]
fn ex13_066_decode_plays_sistermon_noir_source() {
    let mut r = builder().memory(5).start();
    let me = r.place_stack(0, &["SIS-NOIR", CARD_ID]);
    r.game.return_to_hand_from_effect(me, 1);
    accept_all(&mut r);
    assert!(field_ids(&r, 0).contains(&"SIS-NOIR".to_string()));
}

#[test]
fn ex13_066_decode_ignores_non_matching_source() {
    let mut r = builder().memory(5).start();
    let me = r.place_stack(0, &["HUCK-TEXT-L3", CARD_ID]);
    r.game.return_to_hand_from_effect(me, 1);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"HUCK-TEXT-L3".to_string()));
}

#[test]
fn ex13_066_decode_does_not_fire_on_battle_deletion() {
    let mut r = builder().memory(5).start();
    let me = r.place_stack(0, &["SIS-NOIR", CARD_ID]);
    r.game.delete_permanent_with_cause(me, ReplacementCause::Battle);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"SIS-NOIR".to_string()));
}

// ─── Option [Main] ───────────────────────────────────────────────────────────

fn option_runner(hand: &[&str], trash: &[&str]) -> DebugRunner {
    let mut r = builder().hand(0, hand).memory(8).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, "WHITE-ALLY", Some(0));
    for id in trash {
        r.inject_trash(0, id);
    }
    r
}

#[test]
fn ex13_066_option_plays_then_de_digivolves_per_ally() {
    let mut r = option_runner(&[CARD_ID, "SIS-CIEL"], &[]);
    let opp = r.place_stack(1, &["OPP-L3", "OPP-L4", "OPP-L5"]);
    let idx = hand_pos(&r, 0, CARD_ID);
    assert_eq!(r.game.play_option_from_hand(0, idx), OptionPlayResult::Pending);
    let view = r.pending_selection_view().expect("union pick");
    assert!(matches!(view.kind, SelectionKind::UnionZone { .. }));
    assert!(view.is_optional);
    let ciel = PLAY_HAND_START + hand_pos(&r, 0, "SIS-CIEL") as u16;
    r.execute_action(0, ciel).unwrap();
    assert!(field_ids(&r, 0).contains(&"SIS-CIEL".to_string()));
    let view = r.pending_selection_view().expect("De-Digivolve target");
    assert_eq!(view.kind, SelectionKind::OppField);
    r.execute_action(0, encode_attack(0, opp.index as u16)).unwrap();
    // 2 own Digimon → De-Digivolve 2: L5 and L4 trashed, L3 remains.
    let perm = &r.game.player(1).battle_area[opp.index as usize];
    assert_eq!(perm.stack_size(), 1);
    assert_eq!(perm.top_card().card_id(&r.game.card_data), "OPP-L3");
}

#[test]
fn ex13_066_option_declined_play_de_digivolves_once() {
    let mut r = option_runner(&[CARD_ID, "SIS-CIEL"], &[]);
    let opp = r.place_stack(1, &["OPP-L3", "OPP-L4", "OPP-L5"]);
    let idx = hand_pos(&r, 0, CARD_ID);
    assert_eq!(r.game.play_option_from_hand(0, idx), OptionPlayResult::Pending);
    r.execute_action(0, PASS).unwrap();
    let view = r.pending_selection_view().expect("De-Digivolve target");
    assert_eq!(view.kind, SelectionKind::OppField);
    r.execute_action(0, encode_attack(0, opp.index as u16)).unwrap();
    let perm = &r.game.player(1).battle_area[opp.index as usize];
    assert_eq!(perm.stack_size(), 2, "1 own Digimon → De-Digivolve 1");
    assert_eq!(perm.top_card().card_id(&r.game.card_data), "OPP-L4");
}

#[test]
fn ex13_066_option_plays_sistermon_from_trash() {
    let mut r = option_runner(&[CARD_ID], &["SIS-NOIR"]);
    let idx = hand_pos(&r, 0, CARD_ID);
    assert_eq!(r.game.play_option_from_hand(0, idx), OptionPlayResult::Pending);
    let noir = TRASH_EFFECT_START + trash_pos(&r, 0, "SIS-NOIR") as u16;
    let view = r.pending_selection_view().expect("union pick");
    assert!(view.valid_action_ids.contains(&noir));
    r.execute_action(0, noir).unwrap();
    assert!(field_ids(&r, 0).contains(&"SIS-NOIR".to_string()));
}

#[test]
fn ex13_066_option_excludes_cost_5_sistermon() {
    let mut r = option_runner(&[CARD_ID, "SIS-BIG"], &[]);
    let idx = hand_pos(&r, 0, CARD_ID);
    let big = PLAY_HAND_START + hand_pos(&r, 0, "SIS-BIG") as u16 - 1; // after the Option leaves hand
    // Nothing to do (no eligible card, no opp Digimon, no Arts base) → the
    // Option may resolve straight through; either way SIS-BIG is never offered.
    let _ = r.game.play_option_from_hand(0, idx);
    if let Some(view) = r.pending_selection_view() {
        if matches!(view.kind, SelectionKind::UnionZone { .. }) {
            assert!(!view.valid_action_ids.contains(&big));
        }
    }
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"SIS-BIG".to_string()));
}

// ─── <Arts Digivolve> ────────────────────────────────────────────────────────

#[test]
fn ex13_066_arts_digivolves_onto_sistermon_ciel_instead_of_trashing() {
    let mut r = builder().hand(0, &[CARD_ID]).memory(8).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let base = r.place_on_field(0, "SIS-CIEL", Some(0));
    let idx = hand_pos(&r, 0, CARD_ID);
    assert_eq!(r.game.play_option_from_hand(0, idx), OptionPlayResult::Pending);
    let arts_action = encode_attack(0, base.index as u16);
    loop {
        let v = r.pending_selection_view().expect("selection parked before Arts");
        if v.kind == SelectionKind::OwnField && v.is_optional && v.valid_action_ids.contains(&arts_action) {
            break;
        }
        let a = if v.is_optional { PASS } else { v.valid_action_ids[0] };
        r.execute_action(v.selecting_player, a).unwrap();
    }
    let trash_before = r.trash_size(0);
    let mem = r.game.memory;
    r.execute_action(0, arts_action).unwrap();
    let perm = &r.game.player(0).battle_area[base.index as usize];
    assert_eq!(perm.top_card().card_id(&r.game.card_data), CARD_ID);
    assert_eq!(r.trash_size(0), trash_before, "Arts replaces the trash");
    assert_eq!(r.game.memory, mem, "without paying the cost");
}
