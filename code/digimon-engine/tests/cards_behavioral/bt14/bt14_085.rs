//! BT14-085 Mimi Tachikawa — Tamer, Green, Cost 3.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT14-085.md)
//! [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with
//! [Vegetation], [Plant] or [Fairy] in any of its traits among them to the
//! hand. Return the rest to the bottom of the deck.
//! [Your Turn] When effects suspend Digimon, by suspending this Tamer, gain
//! 1 memory.
//! [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT14/Green/BT14_085.cs
//!   On Play: SimplifiedRevealDeckTopCardsAndSelect(3, Plant/Fairy Digimon →
//!   hand, max 1), rest → deck bottom. OnTappedAnyone: owner's turn, ANY
//!   player's battle-area Digimon, IsByEffect, isOptional true, needs this
//!   Tamer suspendable → suspend self, +1 memory (no OPT). Security: play self.
//!
//! # Patterns
//! - real play from hand → On Play reveal-3 / choose 1 / remainder to bottom
//! - optional suspend-self activation cost observer (accept / decline)
//! - "Digimon" = either player's; attack-suspend, opponent-turn, already-
//!   suspended negatives; no OPT (re-arms after unsuspend)
//! - [Security] play self

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS, PLAY_HAND_START, SECURITY_TARGET};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT14-085";

fn digimon(id: &str, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c.traits = traits.iter().map(|s| s.to_string()).collect();
    c
}

struct SuspendByEffect {
    target: PermanentHandle,
}
impl CardEffect for SuspendByEffect {
    fn effects(&self, card: digimon_engine::card_source::CardHandle) -> Vec<Effect> {
        let target = self.target;
        vec![Effect::when_attacking(card)
            .name("suspend by effect")
            .process(move |ctx| {
                ctx.suspend(target);
            })
            .build()]
    }
}

fn runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT14-085 in pack")
        .add_card(digimon("VEG", 6000, &["Vegetation"]))
        .add_card(digimon("FAIRY", 2000, &["Fairy"]))
        .add_card(digimon("BEAST", 3000, &["Beast"]))
        .add_card(digimon("ALLY", 3000, &[]))
        .add_card(make_test_card("SRC", "Src"))
        .add_card(digimon("FILLER", 3000, &[]))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .security(1, &["FILLER"; 5])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

/// Replace P0's deck; `top_first[0]` becomes the top card.
fn set_deck(r: &mut DebugRunner, top_first: &[&str]) {
    let mut deck = Vec::new();
    for id in top_first.iter().rev() {
        let di = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap();
        let ni = r.game.next_card_index();
        deck.push(CardSource::new(di, 0, ni));
    }
    r.game.players[0].deck = deck;
}

fn deck_ids(r: &DebugRunner) -> Vec<String> {
    // top-first
    r.game.players[0]
        .deck
        .iter()
        .rev()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_has(r: &DebugRunner, id: &str) -> bool {
    r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == id)
}

fn play_mimi(r: &mut DebugRunner) {
    let idx = r.add_to_hand(0, CARD_ID);
    r.game.enter_main_phase();
    r.game.set_memory(5);
    let act = PLAY_HAND_START + idx as u16;
    assert_eq!(build_action_mask(&r.game, 0)[act as usize], 1.0);
    r.game.decode_action(act, 0);
}

fn suspend_via_effect(r: &mut DebugRunner, player: PlayerId, target: PermanentHandle) {
    let src = r.place_on_field(player, "SRC", None);
    r.register_effect("SRC", Arc::new(SuspendByEffect { target }));
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(src));
    r.game.drain_effect_queue();
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

// ─── Structural ─────────────────────────────────────────────────────────────

#[test]
fn bt14_085_structure() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, "Mimi Tachikawa");
    let t: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 3);
    assert!(t[0].when.contains(&CompiledTiming::OnPlay));
    assert!(t[1].when.contains(&CompiledTiming::OnSuspend));
    assert!(t[1].optional, "'by suspending this Tamer' is optional");
    assert!(!t[1].once_per_turn, "no [Once Per Turn] printed");
    assert!(t[2].when.contains(&CompiledTiming::OnSecurity));
}

// ─── [On Play] ──────────────────────────────────────────────────────────────

#[test]
fn bt14_085_on_play_adds_one_matching_digimon_rest_to_bottom() {
    let mut r = runner();
    set_deck(&mut r, &["BEAST", "VEG", "FAIRY", "FILLER", "FILLER"]);
    play_mimi(&mut r);
    // First prompt: the reveal pick — both VEG (Vegetation, any DP) and
    // FAIRY are eligible; BEAST is not.
    let v = r.pending_selection_view().expect("reveal pick");
    let picks: Vec<u16> = v.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(picks.len(), 2, "exactly the 2 matching cards are offered: {:?}", v.valid_action_ids);
    r.execute_action(0, picks[0]).unwrap();
    let _ = r.auto_resolve();
    let added_veg = hand_has(&r, "VEG");
    let added_fairy = hand_has(&r, "FAIRY");
    assert!(added_veg ^ added_fairy, "exactly one matching card added");
    assert!(!hand_has(&r, "BEAST"));
    let deck = deck_ids(&r);
    assert_eq!(deck.len(), 4);
    assert_eq!(&deck[0..2], &["FILLER".to_string(), "FILLER".to_string()]);
    assert!(deck[2..].contains(&"BEAST".to_string()), "rest went to the bottom");
}

#[test]
fn bt14_085_on_play_no_match_all_to_bottom() {
    let mut r = runner();
    set_deck(&mut r, &["BEAST", "ALLY", "FILLER", "VEG"]);
    let hand_before = r.hand_size(0);
    play_mimi(&mut r);
    let _ = r.auto_resolve();
    // Mimi left the hand; nothing was added (VEG is 4th, not revealed).
    assert_eq!(r.hand_size(0), hand_before);
    let deck = deck_ids(&r);
    assert_eq!(deck[0], "VEG", "the unrevealed card is now on top");
    assert_eq!(deck.len(), 4);
}

// ─── [Your Turn] suspend observer ───────────────────────────────────────────

#[test]
fn bt14_085_effect_suspend_accept_suspends_mimi_gains_memory() {
    let mut r = runner();
    let mimi = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.game.set_memory(3);
    suspend_via_effect(&mut r, 0, ally);
    assert!(r.pending_is_optional(), "optional 'by suspending this Tamer'");
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(suspended(&r, mimi));
    assert_eq!(r.memory(), 4);
}

#[test]
fn bt14_085_effect_suspend_decline_no_change() {
    let mut r = runner();
    let mimi = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.game.set_memory(3);
    suspend_via_effect(&mut r, 0, ally);
    assert!(r.pending_is_optional());
    r.decline_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(!suspended(&r, mimi));
    assert_eq!(r.memory(), 3);
}

#[test]
fn bt14_085_opponent_digimon_effect_suspend_also_triggers() {
    let mut r = runner();
    let mimi = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "ALLY", Some(0));
    r.game.set_memory(3);
    suspend_via_effect(&mut r, 0, opp);
    assert!(r.pending_is_optional(), "'Digimon' covers either player's");
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(suspended(&r, mimi));
    assert_eq!(r.memory(), 4);
}

#[test]
fn bt14_085_attack_suspend_does_not_trigger() {
    let mut r = runner();
    let _mimi = r.place_on_field(0, CARD_ID, Some(0));
    let atk = r.place_on_field(0, "ALLY", Some(0));
    r.game.enter_main_phase();
    r.game.set_memory(3);
    let act = encode_attack(atk.index as u16, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[act as usize], 1.0);
    r.game.decode_action(act, 0);
    assert!(
        !r.pending_is_optional() || r.pending_selection_view().is_none(),
        "no Mimi prompt on an attack suspension"
    );
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 3);
}

#[test]
fn bt14_085_not_on_opponents_turn() {
    let mut r = runner();
    let mimi = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.game.turn_player_idx = 1;
    suspend_via_effect(&mut r, 1, ally);
    assert!(r.pending_selection_view().is_none(), "[Your Turn]");
    assert!(!suspended(&r, mimi));
}

#[test]
fn bt14_085_already_suspended_mimi_cannot_pay() {
    let mut r = runner();
    let mimi = r.place_on_field(0, CARD_ID, Some(0));
    r.game.players[0].battle_area[mimi.index as usize].is_suspended = true;
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.game.set_memory(3);
    suspend_via_effect(&mut r, 0, ally);
    assert!(r.pending_selection_view().is_none(), "cost unpayable");
    assert_eq!(r.memory(), 3);
}

#[test]
fn bt14_085_no_once_per_turn_rearms_after_unsuspend() {
    let mut r = runner();
    let mimi = r.place_on_field(0, CARD_ID, Some(0));
    let a1 = r.place_on_field(0, "ALLY", Some(0));
    let a2 = r.place_on_field(0, "ALLY", Some(0));
    r.game.set_memory(3);
    suspend_via_effect(&mut r, 0, a1);
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    r.game.players[0].battle_area[mimi.index as usize].is_suspended = false;
    suspend_via_effect(&mut r, 0, a2);
    assert!(r.pending_is_optional(), "fires again the same turn");
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 5);
}

// ─── [Security] ─────────────────────────────────────────────────────────────

#[test]
fn bt14_085_security_plays_itself() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("pack")
        .add_card(digimon("ATK", 9000, &[]))
        .add_card(digimon("FILLER", 3000, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(1, &[CARD_ID])
        .memory(5)
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();
    assert!(r.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID));
}
