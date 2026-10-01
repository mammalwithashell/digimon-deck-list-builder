//! EX13-072 Kota Domoto — Tamer, Black, Cost 4. Trait: Chronicle.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-072.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-072.md` agrees)
//!
//! ```text
//! [Start of Your Main Phase] By trashing 1 [Chronicle] trait card from your
//! hand, ＜Draw 1＞ (Draw 1 card from your deck.) and gain 1 memory.
//! [Your Turn] When one of your [Chronicle] trait Digimon attacks, by
//! suspending this Tamer, you may use 1 [X Antibody] or 1 Option card with the
//! [Chronicle] trait from your hand with the cost reduced by 1.
//!
//! Security Effect:
//! [Security] Play this card without paying the cost.
//! ```
//! (The corpus JSON files the [Security] line under `inherited_effect_
//! description_eng`; the official bundle prints it as the Security Effect.)
//!
//! "1 [X Antibody]" names the card [X Antibody] (BT9-109, an Option card) —
//! square brackets around a bare name are a card-name reference; the trait
//! form is spelled "with the [X] trait".
//!
//! # DCGO C# reference
//! None at b9a0638cd (no `EX13_072.cs`). Closest DCGO-backed siblings:
//! BT18-092 (Start-of-Main trash-1-from-hand → Draw 1 + 1 memory) and
//! EX12-066 (ally-attack trigger, suspend this Tamer, use an Option from hand
//! with the cost reduced).
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - [Start of Your Main Phase] optional hand-trash cost → draw + memory.
//! - [Your Turn] ally-attack observer, suspend-self activation cost, use an
//!   Option from hand with cost reduction.
//! - [Security] play self free.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

use super::chronicle_support::*;

const CARD_ID: &str = "EX13-072";

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-072 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("FILL", CardColor::Black, 3, 1000, &[]))
        .add_card(digimon("DRAWN", CardColor::Black, 3, 1000, &[]))
        .add_card(digimon("CHRON-CARD", CardColor::Yellow, 3, 1000, &["Chronicle"]))
        .add_card(digimon("PLAIN-CARD", CardColor::Yellow, 3, 1000, &["Beast"]))
        .add_card(digimon("CHRON-ATK", CardColor::Black, 4, 6000, &["Chronicle"]))
        .add_card(digimon("XA-ATK", CardColor::Black, 4, 6000, &["X Antibody"]))
        .add_card(digimon("XA-DIGI-CARD", CardColor::Black, 4, 6000, &["X Antibody"]))
        .add_card(option("XAB", "X Antibody", CardColor::Black, 3, &[]))
        .add_card(option("CHRON-OPT", "Chronicle Option", CardColor::Black, 3, &["Chronicle"]))
        .add_card(option("PLAIN-OPT", "Plain Option", CardColor::Black, 3, &[]))
        .add_card(option("XAB-PROTO", "X Antibody Proto Form", CardColor::Black, 3, &[]))
}

fn triggered(runner: &DebugRunner) -> Vec<CompiledTriggeredClause> {
    runner
        .compiled_card(CARD_ID)
        .expect("compiled")
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_072_structure_matches_printed_text() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.traits, vec!["Chronicle"]);
    let t = triggered(&runner);
    assert_eq!(t.len(), 3);
    assert!(t.iter().all(|c| c.scope == CompiledScope::FaceUp));
    let som = t
        .iter()
        .find(|c| c.when == vec![CompiledTiming::StartOfYourMainPhase])
        .expect("[Start of Your Main Phase]");
    assert!(som.optional, "\"By trashing ...\" is a may-pay");
    assert!(!som.once_per_turn);
    let atk = t
        .iter()
        .find(|c| c.when == vec![CompiledTiming::OnAllyAttack])
        .expect("[Your Turn] ally-attack clause");
    assert!(atk.optional, "\"you may use\"");
    assert!(!atk.once_per_turn, "no [Once Per Turn] printed");
    assert!(t.iter().any(|c| c.when == vec![CompiledTiming::OnSecurity]));
}

// ─── [Start of Your Main Phase] ──────────────────────────────────────────────

fn som_setup(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder()
        .hand(0, hand)
        .deck(0, &["FILL", "FILL", "DRAWN"])
        .memory(2)
        .start();
    runner.skip_mulligan();
    let kota = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, kota)
}

#[test]
fn ex13_072_start_of_main_trashes_chronicle_card_draws_and_gains_memory() {
    let (mut runner, kota) = som_setup(&["CHRON-CARD", "PLAIN-CARD"]);
    let memory_before = runner.memory();
    fire(&mut runner, EffectTiming::StartOfYourMainPhase, kota);
    let view = runner.pending_selection_view().expect("trash-cost prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(view.is_optional, "the cost is declinable");
    assert!(hand_offered(&runner, 0, "CHRON-CARD"));
    assert!(!hand_offered(&runner, 0, "PLAIN-CARD"), "only [Chronicle] trait cards pay");
    pick_hand(&mut runner, 0, "CHRON-CARD");
    let _ = runner.auto_resolve();
    assert!(trash_ids(&runner, 0).contains(&"CHRON-CARD".to_string()));
    let hand = hand_ids(&runner, 0);
    assert!(hand.contains(&"DRAWN".to_string()), "Draw 1");
    assert_eq!(hand.len(), 2, "PLAIN-CARD + drawn card");
    assert_eq!(runner.memory(), memory_before + 1, "gain 1 memory");
}

#[test]
fn ex13_072_start_of_main_declined_does_nothing() {
    let (mut runner, kota) = som_setup(&["CHRON-CARD"]);
    let memory_before = runner.memory();
    fire(&mut runner, EffectTiming::StartOfYourMainPhase, kota);
    decline(&mut runner);
    let _ = runner.auto_resolve();
    assert_eq!(hand_ids(&runner, 0), vec!["CHRON-CARD"]);
    assert_eq!(runner.memory(), memory_before);
    assert_eq!(runner.deck_size(0), 3, "no draw");
}

#[test]
fn ex13_072_start_of_main_without_a_chronicle_card_gives_nothing() {
    let (mut runner, kota) = som_setup(&["PLAIN-CARD"]);
    let memory_before = runner.memory();
    fire(&mut runner, EffectTiming::StartOfYourMainPhase, kota);
    decline_all(&mut runner);
    assert_eq!(hand_ids(&runner, 0), vec!["PLAIN-CARD"]);
    assert_eq!(runner.memory(), memory_before, "unpaid cost → no memory");
    assert_eq!(runner.deck_size(0), 3, "unpaid cost → no draw");
}

#[test]
fn ex13_072_start_of_main_fires_from_the_turn_flow() {
    let mut runner = builder()
        .hand(0, &["CHRON-CARD"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(0)
        .start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    next_turn(&mut runner);
    assert_eq!(runner.turn_player(), 1);
    runner.game.memory = -3;
    runner.end_turn();
    // Back on our turn: the start-of-main prompt appears in the turn flow.
    for _ in 0..8 {
        match runner.pending_selection_view() {
            Some(v) if v.kind == SelectionKind::Hand && v.selecting_player == 0 => break,
            Some(v) => {
                runner
                    .execute_action(v.selecting_player, digimon_engine::action::space::PASS)
                    .expect("pass unrelated prompt");
            }
            None => panic!("Kota's [Start of Your Main Phase] prompt never appeared"),
        }
    }
    assert!(hand_offered(&runner, 0, "CHRON-CARD"));
}

// ─── [Your Turn] Chronicle ally attacks → use an Option ─────────────────────

fn attack_setup(
    attacker: &str,
    hand: &[&str],
    memory: i16,
) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = builder()
        .hand(0, hand)
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(memory)
        .start();
    runner.skip_mulligan();
    let kota = runner.place_on_field(0, CARD_ID, Some(0));
    let atk = runner.place_on_field(0, attacker, Some(0));
    (runner, kota, atk)
}

#[test]
fn ex13_072_chronicle_attack_suspends_kota_to_use_chronicle_option_cost_minus_1() {
    let (mut runner, kota, atk) = attack_setup("CHRON-ATK", &["CHRON-OPT"], 5);
    runner.attack_player(atk, 1, false);
    let view = runner.pending_selection_view().expect("Kota's optional trigger");
    assert!(view.is_optional);
    accept(&mut runner);
    assert!(suspended(&runner, kota), "suspend cost paid");
    let view = runner.pending_selection_view().expect("Option pick");
    assert_eq!(view.kind, SelectionKind::Hand);
    pick_hand(&mut runner, 0, "CHRON-OPT");
    assert!(trash_ids(&runner, 0).contains(&"CHRON-OPT".to_string()), "Option used");
    assert_eq!(runner.memory(), 3, "cost 3 reduced by 1 → paid 2");
}

#[test]
fn ex13_072_can_use_the_x_antibody_option_by_name() {
    let (mut runner, _kota, atk) = attack_setup("CHRON-ATK", &["XAB"], 5);
    runner.attack_player(atk, 1, false);
    accept(&mut runner);
    assert!(hand_offered(&runner, 0, "XAB"), "[X Antibody] (by name) is usable");
    pick_hand(&mut runner, 0, "XAB");
    assert!(trash_ids(&runner, 0).contains(&"XAB".to_string()));
    assert_eq!(runner.memory(), 3);
}

#[test]
fn ex13_072_offers_only_x_antibody_or_chronicle_options() {
    let (mut runner, _kota, atk) = attack_setup(
        "CHRON-ATK",
        &["CHRON-OPT", "XAB", "PLAIN-OPT", "XAB-PROTO", "XA-DIGI-CARD", "CHRON-CARD"],
        5,
    );
    runner.attack_player(atk, 1, false);
    accept(&mut runner);
    assert!(hand_offered(&runner, 0, "CHRON-OPT"));
    assert!(hand_offered(&runner, 0, "XAB"));
    assert!(!hand_offered(&runner, 0, "PLAIN-OPT"), "trait-less Option not offered");
    assert!(!hand_offered(&runner, 0, "XAB-PROTO"), "exact name [X Antibody] only");
    assert!(!hand_offered(&runner, 0, "XA-DIGI-CARD"), "a Digimon card is not usable");
    assert!(!hand_offered(&runner, 0, "CHRON-CARD"), "a [Chronicle] Digimon card is not an Option");
}

#[test]
fn ex13_072_declining_keeps_kota_unsuspended() {
    let (mut runner, kota, atk) = attack_setup("CHRON-ATK", &["CHRON-OPT"], 5);
    runner.attack_player(atk, 1, false);
    decline(&mut runner);
    assert!(!suspended(&runner, kota));
    assert!(hand_ids(&runner, 0).contains(&"CHRON-OPT".to_string()));
    assert_eq!(runner.memory(), 5);
}

#[test]
fn ex13_072_non_chronicle_attacker_does_not_trigger() {
    let (mut runner, kota, atk) = attack_setup("XA-ATK", &["CHRON-OPT"], 5);
    runner.attack_player(atk, 1, false);
    assert_ne!(runner.pending_kind(), Some(SelectionKind::Hand));
    decline_all(&mut runner);
    assert!(!suspended(&runner, kota), "only [Chronicle] trait attackers trigger");
    assert!(hand_ids(&runner, 0).contains(&"CHRON-OPT".to_string()));
}

#[test]
fn ex13_072_suspended_kota_cannot_pay() {
    let (mut runner, kota, atk) = attack_setup("CHRON-ATK", &["CHRON-OPT"], 5);
    set_suspended(&mut runner, kota, true);
    runner.attack_player(atk, 1, false);
    decline_all(&mut runner);
    assert!(hand_ids(&runner, 0).contains(&"CHRON-OPT".to_string()), "no Option used");
    assert_eq!(runner.memory(), 5);
}

#[test]
fn ex13_072_opponents_chronicle_attack_does_not_trigger() {
    let mut runner = builder()
        .hand(0, &["CHRON-OPT"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .security(0, &["FILL", "FILL", "FILL"])
        .memory(3)
        .start();
    runner.skip_mulligan();
    let kota = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_atk = runner.place_on_field(1, "CHRON-ATK", Some(0));
    next_turn(&mut runner);
    assert_eq!(runner.turn_player(), 1);
    runner.attack_player(opp_atk, 0, false);
    assert_ne!(runner.pending_kind(), Some(SelectionKind::Hand));
    decline_all(&mut runner);
    assert!(!suspended(&runner, kota));
    assert!(hand_ids(&runner, 0).contains(&"CHRON-OPT".to_string()));
}

// ─── [Security] ──────────────────────────────────────────────────────────────

#[test]
fn ex13_072_security_plays_kota_without_paying_the_cost() {
    let mut runner = builder()
        .security(1, &[CARD_ID])
        .memory(0)
        .start();
    runner.skip_mulligan();
    let atk = runner.place_on_field(0, "XA-ATK", Some(0));
    runner.attack_player(atk, 1, false);
    let _ = runner.auto_resolve();
    assert!(field_has(&runner, 1, CARD_ID), "played from security");
    assert_eq!(runner.security_count(1), 0);
    assert_eq!(runner.memory(), 0, "without paying the cost");
    let kota_kind = runner.game.players[1]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .map(|p| p.top_card().card_kind(&runner.game.card_data));
    assert_eq!(kota_kind, Some(CardKind::Tamer));
}
