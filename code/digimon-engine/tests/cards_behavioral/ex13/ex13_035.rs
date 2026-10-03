//! EX13-035 KingEtemon — Digimon, Lv.6, Yellow, DP 13000, Play Cost 13.
//! Traits: Puppet. Form: Mega. Attribute: Virus.
//! Digivolve: Yellow Lv.5 / 5, Black Lv.5 / 5; Lv.5 w/[Sukamon]/[Etemon] in
//! name: Cost 4.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-035.md`;
//! per-card JSON `code/digimon-engine/cards/ex13/EX13-035.json`)
//!
//! [On Play] [When Digivolving] You may play up to 2 Digimon cards with
//! [Chuumon], [Sukamon] or [Etemon] in their names and up to 6 total play cost
//! from your hand or trash without paying the costs. By returning 10 such cards
//! from your trash to the bottom of the deck, add 6 to the play cost maximum.
//! [All Turns] While there are 3 or more Digimon with [Sukamon] or [Etemon] in
//! their names, give all of your opponent's Digimon ＜Security A. -1＞ (This
//! Digimon checks 1 fewer security card.) and -3000 DP.
//!
//! Official Q&A: "If activated, first choose if you are going to return 10
//! specified cards from your trash to the bottom of your deck. If you do, this
//! effect's play cost maximum is increased by 6. Afterwards, you can play up to
//! 6 total play cost worth of specified cards from your hand or trash without
//! paying the costs."
//!
//! # DCGO C# reference
//! None at the submodule (no EX13_035.cs at b9a0638cd). Printed text, the
//! official Q&A, and general_rule.pdf govern.
//!
//! # Pattern rows
//! - A-union hand-or-trash free play, x2 under a running play-cost budget
//!   (binding_play_cost + subtract formula)
//! - optional trash-return cost gating a budget increase (EffectChoice +
//!   count-capped multi-select, BT23-057 idiom)
//! - H-aura over opponent Digimon (dp_modifier + security_attack) gated by a
//!   both-fields count ("there are" = either battle area, BT24-051 idiom)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, ModifierType, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-035";

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
    let mut b = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-035 YAML loads from the embedded pack")
        .add_card(digimon("CHUU", "Chuumon", 3, 3, 2000, CardColor::Yellow))
        .add_card(digimon("SUKA", "Sukamon", 4, 3, 4000, CardColor::Yellow))
        .add_card(digimon("ETE", "Etemon", 5, 7, 7000, CardColor::Yellow))
        .add_card(digimon("ETE-5", "Etemon Five", 5, 5, 7000, CardColor::Yellow))
        .add_card(digimon("PLAIN", "Plainmon", 4, 3, 4000, CardColor::Yellow))
        .add_card(digimon("YEL-L5", "Yellow Five", 5, 7, 7000, CardColor::Yellow))
        .add_card(digimon("BLK-L5", "Black Five", 5, 7, 7000, CardColor::Black))
        .add_card(digimon("RED-L5", "Red Five", 5, 7, 7000, CardColor::Red))
        .add_card(digimon("OPP", "Opp", 5, 7, 9000, CardColor::Blue))
        .add_card(digimon("OPP-SUKA", "Sukamon", 4, 3, 4000, CardColor::Blue))
        .add_card(digimon("FILL", "Filler", 3, 3, 3000, CardColor::Blue));
    // Ten distinct trash fodder cards that are "such cards" (Sukamon-named).
    for i in 0..10 {
        b = b.add_card(digimon(&format!("SUKA-T{i}"), "Sukamon", 4, 3, 4000, CardColor::Yellow));
    }
    b
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

fn hand_action(r: &DebugRunner, id: &str) -> u16 {
    PLAY_HAND_START + hand_pos(r, 0, id) as u16
}

/// Play KingEtemon from hand (memory 20) in Main phase.
fn play_king(hand: &[&str], trash: &[&str]) -> DebugRunner {
    let mut all = vec![CARD_ID];
    all.extend_from_slice(hand);
    let mut r = builder().hand(0, &all).deck(0, &["FILL"; 5]).memory(20).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    for id in trash {
        r.inject_trash(0, id);
    }
    let idx = hand_pos(&r, 0, CARD_ID);
    r.play(0, idx).expect("play KingEtemon");
    r
}

fn ten_fodder() -> Vec<String> {
    (0..10).map(|i| format!("SUKA-T{i}")).collect()
}

// ─── Section 1 — structural ──────────────────────────────────────────────────

#[test]
fn ex13_035_structure() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled EX13-035");
    assert_eq!(c.kind, CompiledCardKind::Digimon);
    assert_eq!(c.effects.len(), 2, "[OP][WD] play clause + [All Turns] aura");
    let trig = c
        .effects
        .iter()
        .find_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause");
    assert!(trig.when.contains(&CompiledTiming::OnPlay));
    assert!(trig.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!trig.once_per_turn);
    assert!(c
        .effects
        .iter()
        .any(|e| matches!(e, CompiledClause::Declarative(CompiledDeclarativeClause::Aura { .. }))));
}

// ─── Section 2/3 — [On Play] up to 2, 6 total play cost ─────────────────────

#[test]
fn ex13_035_on_play_plays_two_named_digimon_within_six() {
    let mut r = play_king(&["CHUU", "SUKA"], &[]);
    let view = r.pending_selection_view().expect("first free-play pick");
    assert!(matches!(view.kind, SelectionKind::UnionZone { .. }));
    assert!(view.is_optional, "'You may play'");
    r.execute_action(0, hand_action(&r, "CHUU")).unwrap();
    let view = r.pending_selection_view().expect("second free-play pick");
    assert!(matches!(view.kind, SelectionKind::UnionZone { .. }));
    r.execute_action(0, hand_action(&r, "SUKA")).unwrap();
    let mine = field_ids(&r, 0);
    assert!(mine.contains(&"CHUU".to_string()));
    assert!(mine.contains(&"SUKA".to_string()));
}

#[test]
fn ex13_035_on_play_excludes_cards_over_the_six_budget() {
    let mut r = play_king(&["ETE", "PLAIN", "CHUU"], &[]);
    let view = r.pending_selection_view().expect("first pick");
    assert!(view.valid_action_ids.contains(&hand_action(&r, "CHUU")));
    assert!(!view.valid_action_ids.contains(&hand_action(&r, "ETE")), "play cost 7 > 6");
    assert!(!view.valid_action_ids.contains(&hand_action(&r, "PLAIN")), "not a named card");
}

#[test]
fn ex13_035_second_pick_uses_remaining_budget() {
    // 5 + 3 > 6: after a cost-5 Etemon, a cost-3 Chuumon no longer fits.
    let mut r = play_king(&["ETE-5", "CHUU"], &[]);
    r.execute_action(0, hand_action(&r, "ETE-5")).unwrap();
    assert!(field_ids(&r, 0).contains(&"ETE-5".to_string()));
    if let Some(v) = r.pending_selection_view() {
        if matches!(v.kind, SelectionKind::UnionZone { .. }) {
            assert!(!v.valid_action_ids.contains(&hand_action(&r, "CHUU")), "only 1 budget left");
        }
    }
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"CHUU".to_string()));
}

#[test]
fn ex13_035_plays_from_trash() {
    let mut r = play_king(&[], &["SUKA"]);
    let a = TRASH_EFFECT_START + trash_pos(&r, 0, "SUKA") as u16;
    let view = r.pending_selection_view().expect("pick");
    assert!(view.valid_action_ids.contains(&a));
    r.execute_action(0, a).unwrap();
    assert!(field_ids(&r, 0).contains(&"SUKA".to_string()));
}

#[test]
fn ex13_035_play_is_declinable() {
    let mut r = play_king(&["CHUU"], &[]);
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"CHUU".to_string()));
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
}

// ─── Return-10 cost → +6 to the maximum ──────────────────────────────────────

#[test]
fn ex13_035_return_choice_absent_with_fewer_than_ten_such_cards() {
    let fodder = ten_fodder();
    let nine: Vec<&str> = fodder.iter().take(9).map(|s| s.as_str()).collect();
    let r = play_king(&["CHUU"], &nine);
    // First prompt is straight to the free-play pick, no EffectChoice.
    assert!(matches!(r.pending_kind(), Some(SelectionKind::UnionZone { .. })));
}

#[test]
fn ex13_035_returning_ten_raises_the_maximum_to_twelve() {
    let fodder = ten_fodder();
    let ten: Vec<&str> = fodder.iter().map(|s| s.as_str()).collect();
    let mut r = play_king(&["ETE", "ETE-5"], &ten);
    let choice = r.pending_selection_view().expect("return-10 choice first (Q&A)");
    assert_eq!(choice.kind, SelectionKind::EffectChoice);
    let deck_before = r.deck_size(0);
    r.execute_branch(0).expect("choose to return 10");
    // Pick all 10 from the trash.
    while matches!(r.pending_kind(), Some(SelectionKind::CountCappedMultiSelect { .. })) {
        let v = r.pending_selection_view().unwrap();
        let a = v.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        r.execute_action(0, a).unwrap();
    }
    assert_eq!(r.deck_size(0), deck_before + 10, "10 returned to the deck bottom");
    assert_eq!(r.trash_size(0), 0);
    // Budget 12: the cost-7 Etemon is now eligible, and a cost-5 fits after it.
    let v = r.pending_selection_view().expect("first pick");
    assert!(v.valid_action_ids.contains(&hand_action(&r, "ETE")));
    r.execute_action(0, hand_action(&r, "ETE")).unwrap();
    let v = r.pending_selection_view().expect("second pick");
    assert!(v.valid_action_ids.contains(&hand_action(&r, "ETE-5")));
    r.execute_action(0, hand_action(&r, "ETE-5")).unwrap();
    let mine = field_ids(&r, 0);
    assert!(mine.contains(&"ETE".to_string()) && mine.contains(&"ETE-5".to_string()));
}

#[test]
fn ex13_035_declining_the_return_keeps_the_six_maximum() {
    let fodder = ten_fodder();
    let ten: Vec<&str> = fodder.iter().map(|s| s.as_str()).collect();
    let mut r = play_king(&["ETE"], &ten);
    assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice));
    r.execute_branch(1).expect("decline the return");
    assert_eq!(r.trash_size(0), 10);
    let v = r.pending_selection_view().expect("first pick");
    assert!(!v.valid_action_ids.contains(&hand_action(&r, "ETE")), "cost 7 > 6");
}

// ─── [When Digivolving] ─────────────────────────────────────────────────────

#[test]
fn ex13_035_when_digivolving_offers_the_free_play() {
    let mut r = builder().hand(0, &[CARD_ID, "CHUU"]).memory(10).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, "YEL-L5", Some(0)).index as usize;
    r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    let v = r.pending_selection_view().expect("WD free-play pick");
    assert!(matches!(v.kind, SelectionKind::UnionZone { .. }));
    r.execute_action(0, hand_action(&r, "CHUU")).unwrap();
    assert!(field_ids(&r, 0).contains(&"CHUU".to_string()));
}

// ─── Digivolution routes ─────────────────────────────────────────────────────

fn digi_cost(base: &str) -> Option<i16> {
    let mut r = builder().hand(0, &[CARD_ID]).memory(10).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, base, Some(0)).index as usize;
    let mem = r.game.memory;
    if !r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand) {
        return None;
    }
    Some((mem - r.game.memory) as i16)
}

#[test]
fn ex13_035_digivolves_from_yellow_lv5_for_5() {
    assert_eq!(digi_cost("YEL-L5"), Some(5));
}

#[test]
fn ex13_035_digivolves_from_black_lv5_for_5() {
    assert_eq!(digi_cost("BLK-L5"), Some(5));
}

#[test]
fn ex13_035_cannot_digivolve_from_red_lv5() {
    assert_eq!(digi_cost("RED-L5"), None);
}

#[test]
fn ex13_035_etemon_named_lv5_offers_the_cost_4_route() {
    // A yellow Lv.5 [Etemon] satisfies both the circle (5) and the name route
    // (4) → the engine pauses for a cost choice rather than auto-paying.
    let mut r = builder().hand(0, &[CARD_ID]).memory(10).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, "ETE", Some(0)).index as usize;
    assert!(!r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    let v = r.pending_selection_view().expect("cost choice");
    assert_eq!(v.kind, SelectionKind::EffectChoice);
    let labels: Vec<String> = v
        .effect_choices
        .unwrap()
        .iter()
        .map(|c| c.label.clone())
        .collect();
    assert!(labels.iter().any(|l| l.contains('4')), "cost-4 route offered: {labels:?}");
}

// ─── [All Turns] aura ────────────────────────────────────────────────────────

#[test]
fn ex13_035_aura_active_with_three_sukamon_or_etemon() {
    let mut r = builder().start();
    r.place_on_field(0, CARD_ID, Some(0)); // KingEtemon has [Etemon] in its name
    r.place_on_field(0, "SUKA", Some(0));
    r.place_on_field(0, "ETE", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(opp), Some(9000 - 3000));
    let sa = r.game.modifiers.sum(opp, ModifierType::SecurityAttackChange)
        + r.game.dynamic_security_attack_aura_bonus(opp).unwrap_or(0);
    assert_eq!(sa, -1, "<Security A. -1>");
}

#[test]
fn ex13_035_aura_inactive_with_two() {
    let mut r = builder().start();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "SUKA", Some(0));
    r.place_on_field(0, "CHUU", Some(0)); // Chuumon doesn't count
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(opp), Some(9000));
    let sa = r.game.modifiers.sum(opp, ModifierType::SecurityAttackChange)
        + r.game.dynamic_security_attack_aura_bonus(opp).unwrap_or(0);
    assert_eq!(sa, 0);
}

#[test]
fn ex13_035_aura_counts_digimon_on_both_fields() {
    // "While there are" (no "your") counts both battle areas.
    let mut r = builder().start();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "SUKA", Some(0));
    let opp_suka = r.place_on_field(1, "OPP-SUKA", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(opp_suka), Some(4000 - 3000));
}

#[test]
fn ex13_035_aura_does_not_touch_own_digimon() {
    let mut r = builder().start();
    r.place_on_field(0, CARD_ID, Some(0));
    let suka = r.place_on_field(0, "SUKA", Some(0));
    r.place_on_field(0, "ETE", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(suka), Some(4000));
}
