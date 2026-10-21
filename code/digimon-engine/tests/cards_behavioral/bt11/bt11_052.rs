//! BT11-052 Tyrannomon — Digimon, Lv.4, Green/Red, DP 5000, Play Cost 5.
//! Trait line (card image): Champion | Data | Dinosaur.
//!
//! # Card text (card image + official Bandai DB bundle — authoritative)
//!
//! Digivolve: Green Lv.3 / Cost 3 · Red Lv.3 / Cost 3
//! [On Play][When Digivolving] You may play 1 Tamer card with a play cost of 3
//!   or less from your hand without paying the cost.
//! Inherited: [Your Turn] While you have a Tamer in play, this Digimon gets
//!   +2000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT11/Green/BT11_052.cs
//! - OnEnterFieldAnyone ×2 (On Play / When Digivolving), optional:
//!   SelectHandEffect over IsTamer && GetCostItself <= 3, canNoSelect: true,
//!   PlayPermanentCards(payCost: false)
//! - ChangeSelfDPStaticEffect(+2000, isInheritedEffect: true,
//!   condition: IsOwnerTurn && own Tamer on the field)
//!
//! # Patterns this test covers
//! - B2 optional free Tamer play from hand (play-cost cap)
//! - D2 inherited [Your Turn] conditional DP aura (own Tamer present)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT11-052";

fn digimon(id: &str, level: u8, color: CardColor, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = cost;
    c.colors = vec![color];
    c
}

fn tamer(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.play_cost = cost;
    c.colors = vec![CardColor::Green];
    c.level = None;
    c.dp = None;
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT11-052 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(tamer("TAMER-3", 3))
        .add_card(tamer("TAMER-4", 4))
        .add_card(digimon("DIGI-3", 3, CardColor::Green, 3))
        .add_card(digimon("TOP-5", 5, CardColor::Green, 7))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

// ─── Section 1 — Structural ─────────────────────────────────────────────────

#[test]
fn bt11_052_printed_metadata() {
    let r = base().start();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert_eq!(card.name, "Tyrannomon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(5000));
    assert_eq!(card.cost, Some(5));
}

#[test]
fn bt11_052_clause_shapes() {
    let r = base().start();
    let card = r.compiled_card(CARD_ID).unwrap();
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    assert_eq!(
        triggered[0].when,
        vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving]
    );
    assert_eq!(triggered[0].scope, CompiledScope::FaceUp);
    assert!(!triggered[0].once_per_turn);
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope: CompiledScope::Inherited,
            dp_modifier: Some(2000),
            ..
        })
    )), "inherited +2000 DP aura");
}

// ─── Section 2 — Digivolution circles ───────────────────────────────────────

fn try_digivolve_over(base_id: &str) -> (bool, i16, DebugRunner) {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, base_id, Some(0));
    let mem_before = r.game.memory;
    let ok = r.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand);
    let delta = r.game.memory - mem_before;
    (ok, delta, r)
}

#[test]
fn bt11_052_digivolves_from_green_lv3_for_3() {
    let (ok, d, _) = try_digivolve_over("DIGI-3");
    assert!(ok);
    assert_eq!(d, -3);
}

#[test]
fn bt11_052_digivolves_from_red_lv3_for_3() {
    let mut red = digimon("RED-3", 3, CardColor::Red, 3);
    let mut r = base().add_card(red).hand(0, &[CARD_ID]).memory(10).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, "RED-3", Some(0));
    assert!(r.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand));
    assert_eq!(r.game.memory, 7);
}

#[test]
fn bt11_052_no_digivolve_from_blue_lv3() {
    let blue = digimon("BLUE-3", 3, CardColor::Blue, 3);
    let mut r = base().add_card(blue).hand(0, &[CARD_ID]).memory(10).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, "BLUE-3", Some(0));
    assert!(!r.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand));
    assert_eq!(r.game.memory, 10);
}

// ─── Section 3 — [On Play][When Digivolving] free Tamer play ────────────────

#[test]
fn bt11_052_on_play_plays_cost3_tamer_free() {
    let mut r = base().hand(0, &[CARD_ID, "TAMER-3"]).memory(5).start();
    r.play(0, 0).expect("Tyrannomon played");
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    assert!(r.pending_is_optional(), "'You may play'");
    let view = r.pending_selection_view().unwrap();
    assert_eq!(view.valid_action_ids.len(), 1, "only the cost-3 Tamer qualifies");
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    r.auto_resolve().ok();
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string(), "TAMER-3".to_string()]);
    assert_eq!(r.game.memory, 0, "the Tamer was played without paying its cost");
}

#[test]
fn bt11_052_on_play_cost4_tamer_and_digimon_not_offered() {
    let mut r = base()
        .hand(0, &[CARD_ID, "TAMER-4", "DIGI-3"])
        .memory(5)
        .start();
    r.play(0, 0).expect("Tyrannomon played");
    assert!(
        r.pending_selection().is_none(),
        "no Tamer with play cost 3 or less in hand → nothing to choose"
    );
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
    assert_eq!(hand_ids(&r), vec!["TAMER-4".to_string(), "DIGI-3".to_string()]);
}

#[test]
fn bt11_052_on_play_is_declinable() {
    let mut r = base().hand(0, &[CARD_ID, "TAMER-3"]).memory(5).start();
    r.play(0, 0).expect("Tyrannomon played");
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().ok();
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
    assert_eq!(hand_ids(&r), vec!["TAMER-3".to_string()]);
}

#[test]
fn bt11_052_when_digivolving_plays_cost3_tamer_free() {
    let mut r = base().hand(0, &[CARD_ID, "TAMER-3"]).memory(10).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, "DIGI-3", Some(0));
    assert!(r.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand));
    r.game.drain_effect_queue();
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    let view = r.pending_selection_view().unwrap();
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    r.auto_resolve().ok();
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string(), "TAMER-3".to_string()]);
    assert_eq!(r.game.memory, 7, "only the digivolve cost was paid");
}

// ─── Section 4 — Inherited [Your Turn] +2000 DP while you have a Tamer ───────

/// TOP-5 with Tyrannomon as a digivolution card.
fn stack_with_tyrannomon(r: &mut DebugRunner) -> PermanentHandle {
    let h = r.place_on_field(0, "TOP-5", Some(0));
    r.push_source(h, CARD_ID);
    h
}

#[test]
fn bt11_052_inherited_your_turn_with_tamer_gets_2000() {
    let mut r = base().start();
    let h = stack_with_tyrannomon(&mut r);
    r.place_on_field(0, "TAMER-3", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(h), Some(5000));
}

#[test]
fn bt11_052_inherited_without_tamer_no_bonus() {
    let mut r = base().start();
    let h = stack_with_tyrannomon(&mut r);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(h), Some(3000));
}

#[test]
fn bt11_052_inherited_opponent_tamer_does_not_count() {
    let mut r = base().start();
    let h = stack_with_tyrannomon(&mut r);
    r.place_on_field(1, "TAMER-3", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(h), Some(3000), "'you have a Tamer' = your own Tamer");
}

#[test]
fn bt11_052_inherited_not_on_opponents_turn() {
    let mut r = base().start();
    let h = stack_with_tyrannomon(&mut r);
    r.place_on_field(0, "TAMER-3", Some(0));
    r.end_turn();
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(h), Some(3000), "[Your Turn] only");
}
