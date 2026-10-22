//! BT14-085 Mimi Tachikawa — Tamer, Green, Cost 3.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT14-085.md; card image agrees)
//!
//! ```text
//! [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with
//! [Vegetation], [Plant] or [Fairy] in one of its traits among them to the
//! hand. Return the rest to the bottom of the deck.
//! [Your Turn] When an effect suspends a Digimon, by suspending this Tamer,
//! gain 1 memory.
//! Security: [Security] Play this card without paying the cost.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT14/Green/BT14_085.cs
//!   - On Play: SimplifiedRevealDeckTopCardsAndSelect(revealCount 3, one
//!     AddHand bucket maxCount 1 over Digimon with HasPlantTraits (Vegetation /
//!     Plant substring) or HasFairyTraits), remainingCardsPlace DeckBottom.
//!   - OnTappedAnyone: IsOwnerTurn, ANY battle-area Digimon (either player's),
//!     IsByEffect; isOptional true; CanActivate = CanActivateSuspendCostEffect
//!     (this Tamer can suspend). Body: suspend this Tamer → AddMemory(1). No
//!     per-turn cap (maxCountPerTurn -1).
//!   - SecuritySkill: PlaySelfTamerSecurityEffect.
//!
//! # Official Q&A (data/card_qa.json)
//! - Q2458: the [Your Turn] effect activates even when the OPPONENT's Digimon
//!   is suspended by an effect (any Digimon, any effect, on your turn).
//!
//! # Rules
//! - §15-7: "by suspending this Tamer" is an optional processing condition; it
//!   can't be chosen when the Tamer is already suspended (§15-7-4).
//! - Suspending this Tamer suspends a Tamer, not a Digimon, so it never
//!   re-triggers the clause.
//!
//! # Patterns this test covers
//! - A2 reveal-N add-1-by-trait (`select_reveal_buckets`) + remainder to bottom
//! - [Your Turn] on_suspend observer (`trigger_if: event_is_effect_initiated`,
//!   any player's Digimon) with an optional suspend-self cost; no OPT
//! - B-row Tamer [Security] play-self

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT14-085";

fn digimon(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.traits = traits.iter().map(|s| s.to_string()).collect();
    c
}

fn tamer(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Green];
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
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

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT14-085 in pack")
        .add_card(digimon("VEG", &["Vegetation"]))
        .add_card(digimon("FAIRY", &["Fairy"]))
        .add_card(digimon("BEAST", &["Beast"]))
        .add_card(tamer("PLANT-TAMER", &["Plant"]))
        .add_card(digimon("ALLY", &[]))
        .add_card({
            let mut big = digimon("BIG", &[]);
            big.dp = Some(9000);
            big
        })
        .add_card(make_test_card("SRC", "Src"))
        .add_card(digimon("FILLER", &[]))
}

/// Mimi on P0's field (no On Play), standard decks.
fn field_runner() -> (DebugRunner, PermanentHandle) {
    let mut r = builder()
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .security(0, &["FILLER"; 5])
        .security(1, &["FILLER"; 5])
        .memory(3)
        .start();
    r.set_first_player(0);
    let mimi = r.place_on_field(0, CARD_ID, Some(0));
    (r, mimi)
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

fn ids(cards: &[digimon_engine::card_source::CardSource], r: &DebugRunner) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn revealed_action(r: &DebugRunner, id: &str) -> Option<u16> {
    r.game.revealed_cards.iter().enumerate().find_map(|(idx, c)| {
        (c.card_id(&r.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
    })
}

/// Play Mimi from hand over a deck whose top 3 are `top3` (top-first).
fn play_mimi_over(top3: &[&str]) -> DebugRunner {
    // Engine convention: deck.last() is the top card.
    let mut deck: Vec<&str> = vec!["FILLER", "FILLER"];
    deck.extend(top3.iter().rev());
    let mut r = builder()
        .deck(0, &deck)
        .deck(1, &["FILLER"; 5])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    r.set_first_player(0);
    let _ = r.play(0, 0);
    r
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn bt14_085_is_a_green_cost_3_tamer() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, "Mimi Tachikawa");
    assert_eq!(card.kind, digimon_dsl::compiled::CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(3));
    assert!(card
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Green));
}

#[test]
fn bt14_085_has_on_play_your_turn_suspend_and_security_clauses() {
    let card = dsl_card_data::compiled(CARD_ID);
    let t: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(card.effects.len(), 3);
    assert_eq!(t.len(), 3);
    assert_eq!(t[0].when, vec![CompiledTiming::OnPlay]);
    assert!(!t[0].optional, "the reveal is mandatory");
    assert_eq!(t[1].when, vec![CompiledTiming::OnSuspend]);
    assert!(t[1].optional, "'by suspending this Tamer' is optional");
    assert!(!t[1].once_per_turn, "no [Once Per Turn] printed");
    assert_eq!(t[2].when, vec![CompiledTiming::OnSecurity]);
    assert_eq!(t[2].scope, CompiledScope::FaceUp);
    assert!(!t[2].optional);
}

// ─── SECTION 2 — [On Play] ─────────────────────────────────────────────────

#[test]
fn bt14_085_on_play_adds_a_matching_digimon_and_bottoms_the_rest() {
    let mut r = play_mimi_over(&["BEAST", "FAIRY", "PLANT-TAMER"]);
    let v = r.pending_selection_view().expect("reveal pick installs");
    let fairy = revealed_action(&r, "FAIRY").expect("FAIRY revealed");
    assert!(v.valid_action_ids.contains(&fairy), "[Fairy] Digimon is eligible");
    let beast = revealed_action(&r, "BEAST").expect("BEAST revealed");
    assert!(!v.valid_action_ids.contains(&beast), "no matching trait");
    let tamer = revealed_action(&r, "PLANT-TAMER").expect("tamer revealed");
    assert!(!v.valid_action_ids.contains(&tamer), "only Digimon cards");

    r.execute_action(0, fairy).unwrap();
    let _ = r.auto_resolve();

    let hand = ids(&r.game.players[0].hand, &r);
    assert!(hand.contains(&"FAIRY".to_string()), "added to hand: {hand:?}");
    assert!(!hand.contains(&"BEAST".to_string()));
    assert!(!hand.contains(&"PLANT-TAMER".to_string()));
    let deck = ids(&r.game.players[0].deck, &r);
    assert_eq!(deck.len(), 4, "2 untouched + 2 returned");
    let bottom2: Vec<&String> = deck.iter().take(2).collect();
    assert!(
        bottom2.contains(&&"BEAST".to_string()) && bottom2.contains(&&"PLANT-TAMER".to_string()),
        "the rest go to the bottom (deck[0..2]): {deck:?}"
    );
    assert!(r.game.revealed_cards.is_empty(), "nothing left revealed");
}

#[test]
fn bt14_085_on_play_vegetation_digimon_is_eligible() {
    let r = play_mimi_over(&["VEG", "BEAST", "FILLER"]);
    let v = r.pending_selection_view().expect("reveal pick installs");
    let veg = revealed_action(&r, "VEG").expect("VEG revealed");
    assert!(v.valid_action_ids.contains(&veg), "[Vegetation] Digimon is eligible");
}

#[test]
fn bt14_085_on_play_without_a_match_returns_all_three_to_the_bottom() {
    let mut r = play_mimi_over(&["BEAST", "PLANT-TAMER", "FILLER"]);
    let _ = r.auto_resolve();
    let hand = ids(&r.game.players[0].hand, &r);
    assert!(!hand.contains(&"BEAST".to_string()));
    assert!(!hand.contains(&"PLANT-TAMER".to_string()));
    let deck = ids(&r.game.players[0].deck, &r);
    assert_eq!(deck.len(), 5, "all 3 revealed cards return to the deck");
    let bottom3: Vec<&String> = deck.iter().take(3).collect();
    assert!(bottom3.contains(&&"BEAST".to_string()));
    assert!(bottom3.contains(&&"PLANT-TAMER".to_string()));
}

// ─── SECTION 3 — [Your Turn] effect-suspend → suspend Mimi → +1 memory ─────

#[test]
fn bt14_085_effect_suspend_of_own_digimon_accept_suspends_mimi_and_gains_1() {
    let (mut r, mimi) = field_runner();
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let mem = r.memory();
    suspend_via_effect(&mut r, 0, ally);
    assert!(r.pending_is_optional(), "'by suspending this Tamer' prompt");
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(suspended(&r, mimi), "Mimi pays the cost");
    assert_eq!(r.memory(), mem + 1, "gain 1 memory");
}

/// Q2458: an effect suspending the OPPONENT's Digimon also qualifies.
#[test]
fn bt14_085_effect_suspend_of_opponents_digimon_also_triggers() {
    let (mut r, mimi) = field_runner();
    let opp = r.place_on_field(1, "ALLY", Some(0));
    let mem = r.memory();
    suspend_via_effect(&mut r, 0, opp);
    assert!(r.pending_selection().is_some(), "Q2458: any Digimon");
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(suspended(&r, mimi));
    assert_eq!(r.memory(), mem + 1);
}

#[test]
fn bt14_085_declining_keeps_mimi_unsuspended_and_gains_nothing() {
    let (mut r, mimi) = field_runner();
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let mem = r.memory();
    suspend_via_effect(&mut r, 0, ally);
    r.decline_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(!suspended(&r, mimi));
    assert_eq!(r.memory(), mem);
}

#[test]
fn bt14_085_attack_suspend_does_not_trigger() {
    let (mut r, mimi) = field_runner();
    // 9000 DP survives the 4000 DP security Digimon battle.
    let ally = r.place_on_field(0, "BIG", Some(0));
    r.game.enter_main_phase();
    let mem = r.memory();
    let act = encode_attack(ally.index as u16, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[act as usize], 1.0);
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();
    assert!(suspended(&r, ally));
    assert!(!suspended(&r, mimi), "no prompt, no cost paid");
    assert_eq!(r.memory(), mem, "attacking is not 'an effect suspends'");
}

#[test]
fn bt14_085_does_not_trigger_on_opponents_turn() {
    let (mut r, mimi) = field_runner();
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.game.turn_player_idx = 1;
    suspend_via_effect(&mut r, 1, ally);
    assert!(r.pending_selection().is_none(), "[Your Turn]");
    let _ = r.auto_resolve();
    assert!(!suspended(&r, mimi));
}

/// §15-7-4: an already-suspended Mimi can't pay, so nothing is offered.
#[test]
fn bt14_085_already_suspended_mimi_is_not_offered() {
    let (mut r, mimi) = field_runner();
    r.game.players[0].battle_area[mimi.index as usize].is_suspended = true;
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let mem = r.memory();
    suspend_via_effect(&mut r, 0, ally);
    assert!(r.pending_selection().is_none(), "unpayable cost → no prompt");
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem);
}

/// No [Once Per Turn]: once Mimi is unsuspended again, a second effect
/// suspension in the same turn offers the cost again.
#[test]
fn bt14_085_has_no_once_per_turn_limit() {
    let (mut r, mimi) = field_runner();
    let a1 = r.place_on_field(0, "ALLY", Some(0));
    let a2 = r.place_on_field(0, "ALLY", Some(0));
    let mem = r.memory();
    suspend_via_effect(&mut r, 0, a1);
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem + 1);
    r.game.players[0].battle_area[mimi.index as usize].is_suspended = false;
    suspend_via_effect(&mut r, 0, a2);
    assert!(r.pending_selection().is_some(), "offered again the same turn");
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem + 2);
}

// ─── SECTION 4 — [Security] ─────────────────────────────────────────────────

#[test]
fn bt14_085_security_effect_plays_mimi_without_paying_the_cost() {
    let mut r = builder()
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(1, &[CARD_ID])
        .memory(3)
        .start();
    r.set_first_player(0);
    let attacker = r.place_on_field(0, "ALLY", Some(0));
    let mem = r.memory();
    r.attack_player(attacker, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 0);
    assert!(
        r.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID),
        "Mimi is played to P1's battle area"
    );
    assert_eq!(r.memory(), mem, "no cost paid");
}
