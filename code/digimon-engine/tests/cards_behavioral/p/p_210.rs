//! P-210 Hiroko Sagisaka — Tamer, Red, Cost 3. Traits: TS.
//!
//! # Card text (official Bandai DB — data/card_bundles/P-210.md)
//!
//! [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//! [On Play] You may return 1 Digimon card with the [TS] trait from your trash
//! to the hand.
//! [Security] Play this card without paying the cost.
//!
//! (Not a reprint of BT24-083 / BT26-088 — different text.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Red/P_210.cs
//!   - OnStartMainPhase: Gain1MemoryTamerOpponentDigimonEffect.
//!   - OnEnterFieldAnyone: mandatory trigger; SelectCardEffect over trash
//!     (IsDigimon && HasTSTraits), canNoSelect: true, Mode.AddHand.
//!   - SecuritySkill: PlaySelfTamerSecurityEffect.
//!
//! # Patterns
//! - start_of_your_main_phase + opponent-has-Digimon gate + gain_memory
//! - on_play optional select_trash → add_to_hand_from_trash
//! - play_from_security (Tamer)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_engine::action::space::{PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "P-210";

fn digimon(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn ts_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Red];
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.traits = vec!["TS".to_string()];
    c
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-210 compiles")
        .add_card(digimon("TS-DIGI", &["TS"]))
        .add_card(digimon("PLAIN-DIGI", &["Beast"]))
        .add_card(ts_tamer("TS-TAMER"))
        .add_card(digimon("OPP", &[]))
        .add_card(digimon("FILLER", &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
}

fn push_trash(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .trash
        .push(CardSource::new(idx, p, next));
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, h: PermanentHandle) {
    r.game.enqueue_triggered(timing, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

fn ids(cards: &[CardSource], r: &DebugRunner) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

// ─── [Start of Your Main Phase] ──────────────────────────────────────────────

#[test]
fn p_210_somp_gains_memory_when_opponent_has_digimon() {
    let mut r = builder().memory(3).start();
    r.set_first_player(0);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP", Some(0));
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, h);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem + 1);
}

#[test]
fn p_210_somp_no_memory_without_opponent_digimon() {
    let mut r = builder().memory(3).start();
    r.set_first_player(0);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "TS-TAMER", Some(0)); // a Tamer is not a Digimon
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, h);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem);
}

// ─── [On Play] ───────────────────────────────────────────────────────────────

#[test]
fn p_210_on_play_returns_chosen_ts_digimon_from_trash() {
    let mut r = builder().memory(5).start();
    r.set_first_player(0);
    push_trash(&mut r, 0, "PLAIN-DIGI");
    push_trash(&mut r, 0, "TS-TAMER");
    push_trash(&mut r, 0, "TS-DIGI");
    let h = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, h);
    let v = r.pending_selection_view().expect("trash pick");
    assert!(r.pending_is_optional(), "canNoSelect: true (PASS declines)");
    let picks: Vec<u16> = v
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        picks,
        vec![TRASH_EFFECT_START + 2],
        "only the [TS] Digimon card (not the [TS] Tamer, not the non-TS Digimon)"
    );
    r.execute_action(0, TRASH_EFFECT_START + 2).unwrap();
    let _ = r.auto_resolve();
    let hand = ids(&r.game.players[0].hand, &r);
    assert!(hand.contains(&"TS-DIGI".to_string()));
    let trash = ids(&r.game.players[0].trash, &r);
    assert!(!trash.contains(&"TS-DIGI".to_string()));
    assert_eq!(trash.len(), 2);
}

#[test]
fn p_210_on_play_may_decline() {
    let mut r = builder().memory(5).start();
    r.set_first_player(0);
    push_trash(&mut r, 0, "TS-DIGI");
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let hand_before = r.game.players[0].hand.len();
    fire(&mut r, EffectTiming::OnPlay, h);
    r.execute_action(0, PASS).expect("decline");
    let _ = r.auto_resolve();
    assert_eq!(r.game.players[0].hand.len(), hand_before);
    assert_eq!(ids(&r.game.players[0].trash, &r), vec!["TS-DIGI"]);
}

#[test]
fn p_210_on_play_no_eligible_card_no_prompt() {
    let mut r = builder().memory(5).start();
    r.set_first_player(0);
    push_trash(&mut r, 0, "PLAIN-DIGI");
    let h = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, h);
    let offered: Vec<u16> = r
        .pending_selection_view()
        .map(|v| v.valid_action_ids.into_iter().filter(|&a| a != PASS).collect())
        .unwrap_or_default();
    assert!(offered.is_empty(), "no [TS] Digimon in trash → nothing to pick");
}

#[test]
fn p_210_played_from_hand_fires_on_play() {
    let mut r = builder().hand(0, &[CARD_ID]).memory(5).start();
    r.set_first_player(0);
    push_trash(&mut r, 0, "TS-DIGI");
    r.play(0, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert!(ids(&r.game.players[0].hand, &r).contains(&"TS-DIGI".to_string()));
}

// ─── [Security] ──────────────────────────────────────────────────────────────

#[test]
fn p_210_security_plays_itself_for_free() {
    let mut r = builder().security(0, &[CARD_ID]).memory(5).start();
    r.set_first_player(0);
    let attacker = r.place_on_field(1, "OPP", Some(0));
    let mem = r.memory();
    r.attack_player(attacker, 0, false);
    let _ = r.auto_resolve();
    r.game.drain_effect_queue();
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(r.memory(), mem, "played without paying the cost");
}
