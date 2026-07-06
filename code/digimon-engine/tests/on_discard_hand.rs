//! Gap 2 — `OnDiscardHand` trigger + `played_by_effect` condition
//! [G-ENGINE-ON-DISCARD-HAND].
//!
//! Drivers:
//! - ST16-14 Matt Ishida — "[All Turns] When one of your effects trashes a
//!   card in your hand, by suspending this Tamer, gain 1 memory." The trash
//!   must be caused by the CONTROLLER'S OWN effect (not a draw, mulligan, rule
//!   discard, or the opponent's effect).
//! - BT25-080 Witchmon / BT25-084 Titamon — inherited "[All Turns] When your
//!   hand is trashed from, delete 1 of your opponent's Digimon" (gated on the
//!   trashed card being in the observer's own hand).
//! - BT25-080's main clause tail: "if played by an effect, delete 1 opp Digimon"
//!   → `played_by_effect` predicate reading `PlaySource::ByEffect`.
//!
//! DCGO semantics (CardEffectCommons.CanTriggerOnTrashHand + DiscardHands):
//! - The event carries a CardEffect (the causing effect). A non-null CardEffect
//!   means the discard was caused by an EFFECT — draws / mulligans / rule
//!   discards do NOT set one → the trigger never fires for them.
//! - A multi-card discard is ONE batch event (DiscardHands collects the whole
//!   list, then fires OnDiscardHand once), NOT one event per card.
//! - The trashing player (whose hand lost cards) and the causing effect's
//!   controller are both exposed so a card can gate "your hand" and "your own
//!   effect".
//!
//! These tests drive the ENGINE dispatch with hand-written recording effects.
//! They do NOT author the driver cards.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect, EffectBuilder};
use digimon_engine::enums::{CardKind, CostDelta, EffectTiming, PlayerId, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use std::sync::{Arc, Mutex};

fn digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(3000);
    c
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c
}

// ─── A board-wide OnDiscardHand observer that records each fire ──────────────
//
// Gates like Matt Ishida (ST16-14): fires only when the trashing player is the
// observer's controller (`discard_hand_player == you`) AND, when `own_only`
// is set, the causing effect belongs to the observer's controller.
struct DiscardHandObserver {
    own_only: bool,
    fired: Arc<Mutex<Vec<PlayerId>>>,
}
impl CardEffect for DiscardHandObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let own_only = self.own_only;
        let fired = self.fired.clone();
        vec![EffectBuilder::new(card, EffectTiming::OnDiscardHand)
            .name("board-wide: when your hand is trashed from")
            .condition(move |ctx| {
                // "your hand": the trashing player is the observer.
                let Some(trashing) = ctx.discard_hand_player() else {
                    return false;
                };
                if trashing != ctx.player() {
                    return false;
                }
                if own_only && !ctx.discard_caused_by_own_effect() {
                    return false;
                }
                true
            })
            .process(move |ctx| {
                fired.lock().unwrap().push(ctx.player);
            })
            .build()]
    }
}

// ─── An effect that, when it fires OnPlay, trashes N cards from a hand ───────
//
// `trash_opponent` selects whose hand to trash. This is the causing EFFECT;
// its controller is the player that plays it.
struct DiscardTrigger {
    count: usize,
    trash_opponent: bool,
}
impl CardEffect for DiscardTrigger {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let count = self.count;
        let trash_opponent = self.trash_opponent;
        vec![Effect::on_play(card)
            .name("effect that trashes cards from a hand")
            .process(move |ctx| {
                let target = if trash_opponent {
                    ctx.opponent_id()
                } else {
                    ctx.player
                };
                for _ in 0..count {
                    // Always trash hand index 0 (indices shift as cards leave).
                    ctx.trash_from_hand_by_index(target, 0);
                }
            })
            .build()]
    }
}

// ─── Positive: own effect trashes own hand → fires ──────────────────────────

#[test]
fn fires_when_own_effect_trashes_own_hand() {
    let fired = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(tamer("OBS", "Observer"))
        .add_card(digimon("TRIG", "SelfDiscarder"))
        .add_card(digimon("H", "HandFiller"))
        .add_card(digimon("F", "F"))
        .hand(0, &["H"]) // player 0 has 1 hand card to lose
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(DiscardHandObserver { own_only: true, fired: fired.clone() }));
    r.register_effect("TRIG", Arc::new(DiscardTrigger { count: 1, trash_opponent: false }));
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;

    let _obs = r.place_on_field(0, "OBS", Some(0));
    let trig = r.place_on_field(0, "TRIG", Some(0));
    r.game.fire_on_play(0, trig.index as usize);
    r.auto_resolve().ok();

    assert_eq!(
        fired.lock().unwrap().as_slice(),
        &[0],
        "OnDiscardHand fires once for player 0 when player 0's OWN effect trashes its OWN hand"
    );
}

// ─── Negative: OPPONENT's effect trashes your hand → own-only must NOT fire ──

#[test]
fn does_not_fire_for_own_only_when_opponent_effect_trashes_your_hand() {
    let fired = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(tamer("OBS", "Observer"))
        .add_card(digimon("TRIG-OPP", "OppDiscarder"))
        .add_card(digimon("H", "HandFiller"))
        .add_card(digimon("F", "F"))
        .hand(0, &["H"]) // player 0's hand is the victim
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(DiscardHandObserver { own_only: true, fired: fired.clone() }));
    r.register_effect("TRIG-OPP", Arc::new(DiscardTrigger { count: 1, trash_opponent: true }));
    r.game.turn_count = 1;
    r.game.turn_player_idx = 1;

    // Observer on player 0. Player 1 plays an effect trashing player 0's hand.
    let _obs = r.place_on_field(0, "OBS", Some(0));
    let trig = r.place_on_field(1, "TRIG-OPP", Some(0));
    r.game.fire_on_play(1, trig.index as usize);
    r.auto_resolve().ok();

    assert!(
        fired.lock().unwrap().is_empty(),
        "own-only observer must NOT fire when the OPPONENT's effect trashed your hand \
         (event_caused_by_own_effect == false)"
    );
}

// ─── The 'your hand' gate (not own-only) DOES fire when your own hand is ─────
// trashed by any effect (BT25-080/084 shape: gated on the trashed card's owner).

#[test]
fn hand_owner_gate_fires_when_your_hand_trashed_by_opponent_effect() {
    let fired = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(tamer("OBS", "Observer"))
        .add_card(digimon("TRIG-OPP", "OppDiscarder"))
        .add_card(digimon("H", "HandFiller"))
        .add_card(digimon("F", "F"))
        .hand(0, &["H"])
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(DiscardHandObserver { own_only: false, fired: fired.clone() }));
    r.register_effect("TRIG-OPP", Arc::new(DiscardTrigger { count: 1, trash_opponent: true }));
    r.game.turn_count = 1;
    r.game.turn_player_idx = 1;

    let _obs = r.place_on_field(0, "OBS", Some(0));
    let trig = r.place_on_field(1, "TRIG-OPP", Some(0));
    r.game.fire_on_play(1, trig.index as usize);
    r.auto_resolve().ok();

    assert_eq!(
        fired.lock().unwrap().as_slice(),
        &[0],
        "BT25-080/084 shape (gate on the trashed card's owner) fires when YOUR hand is \
         trashed even by the opponent's effect"
    );
}

// ─── Negative: opponent's hand trashed → your 'your hand' gate must NOT fire ─

#[test]
fn does_not_fire_when_opponents_hand_is_trashed() {
    let fired = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(tamer("OBS", "Observer"))
        .add_card(digimon("TRIG-OPP", "OppDiscarder"))
        .add_card(digimon("H", "HandFiller"))
        .add_card(digimon("F", "F"))
        .hand(1, &["H"]) // player 1's hand is the victim
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(DiscardHandObserver { own_only: false, fired: fired.clone() }));
    r.register_effect("TRIG-OPP", Arc::new(DiscardTrigger { count: 1, trash_opponent: true }));
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;

    // Observer on player 0; player 0's own effect trashes the OPPONENT (1)'s hand.
    let _obs = r.place_on_field(0, "OBS", Some(0));
    let trig = r.place_on_field(0, "TRIG-OPP", Some(0));
    r.game.fire_on_play(0, trig.index as usize);
    r.auto_resolve().ok();

    assert!(
        fired.lock().unwrap().is_empty(),
        "the 'your hand' gate must NOT fire when the OPPONENT's hand was trashed"
    );
}

// ─── Batch semantics: a multi-card discard fires ONE event, not one per card ─

#[test]
fn multi_card_discard_fires_one_batch_event() {
    let fired = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(tamer("OBS", "Observer"))
        .add_card(digimon("TRIG3", "TripleDiscarder"))
        .add_card(digimon("H", "HandFiller"))
        .add_card(digimon("F", "F"))
        .hand(0, &["H", "H", "H"]) // 3 hand cards to lose in one effect
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(DiscardHandObserver { own_only: false, fired: fired.clone() }));
    r.register_effect("TRIG3", Arc::new(DiscardTrigger { count: 3, trash_opponent: false }));
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;

    let _obs = r.place_on_field(0, "OBS", Some(0));
    let trig = r.place_on_field(0, "TRIG3", Some(0));
    r.game.fire_on_play(0, trig.index as usize);
    r.auto_resolve().ok();

    assert_eq!(
        fired.lock().unwrap().len(),
        1,
        "a single effect trashing 3 hand cards fires ONE OnDiscardHand batch event \
         (DCGO DiscardHands collects the batch then fires once), not 3"
    );
}

// ─── Negative: mulligan / redraw does NOT fire (hand → deck, never trash) ────

#[test]
fn does_not_fire_on_mulligan_redraw() {
    let fired = Arc::new(Mutex::new(Vec::new()));
    // Build without starting → stays in Mulligan.
    let mut r = DebugRunner::builder()
        .add_card(tamer("OBS", "Observer"))
        .add_card(digimon("F", "F"))
        .deck(0, &["F"; 20])
        .deck(1, &["F"; 20])
        .build();
    r.register_effect("OBS", Arc::new(DiscardHandObserver { own_only: false, fired: fired.clone() }));
    // Place the observer so it could react if the redraw wrongly fired the event.
    let _obs = r.place_on_field(0, "OBS", Some(0));

    // Player 0 mulligans (redraw hand → deck); player 1 keeps.
    r.mulligan_decide(false).ok();
    r.mulligan_decide(true).ok();
    r.auto_resolve().ok();

    assert!(
        fired.lock().unwrap().is_empty(),
        "mulligan / redraw moves hand → deck (never trash) → OnDiscardHand must not fire"
    );
}

// ─── played_by_effect: BT25-080 main-clause tail ─────────────────────────────
//
// An OnPlay effect that records whether the permanent it lives on was played
// by an effect (PlaySource::ByEffect) vs by hand (PlaySource::ByHand).

struct PlayedByEffectRecorder {
    saw_by_effect: Arc<Mutex<Vec<bool>>>,
}
impl CardEffect for PlayedByEffectRecorder {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let saw = self.saw_by_effect.clone();
        vec![Effect::on_play(card)
            .name("record played_by_effect at OnPlay")
            .process(move |ctx| {
                saw.lock().unwrap().push(ctx.played_by_effect());
            })
            .build()]
    }
}

#[test]
fn played_by_effect_is_true_when_played_by_an_effect() {
    let saw = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(digimon("SUBJECT", "Subject"))
        .add_card(digimon("F", "F"))
        .hand(0, &["SUBJECT"])
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(10)
        .start();
    r.register_effect("SUBJECT", Arc::new(PlayedByEffectRecorder { saw_by_effect: saw.clone() }));
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;

    // Play SUBJECT from hand via an EFFECT-driven play (PlaySource::ByEffect).
    r.game
        .play_from_hand_with_cost(0, 0, CostDelta::Reduce(0), PlaySource::ByEffect);
    r.auto_resolve().ok();

    assert_eq!(
        saw.lock().unwrap().as_slice(),
        &[true],
        "played_by_effect must be TRUE when the OnPlay fired for an effect-driven play"
    );
}

#[test]
fn played_by_effect_is_false_for_a_normal_hand_play() {
    let saw = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(digimon("SUBJECT", "Subject"))
        .add_card(digimon("F", "F"))
        .hand(0, &["SUBJECT"])
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(10)
        .start();
    r.register_effect("SUBJECT", Arc::new(PlayedByEffectRecorder { saw_by_effect: saw.clone() }));
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;

    // Normal hand play (the player action → PlaySource::ByHand).
    r.play(0, 0);
    r.auto_resolve().ok();

    assert_eq!(
        saw.lock().unwrap().as_slice(),
        &[false],
        "played_by_effect must be FALSE for a normal player hand-play (PlaySource::ByHand)"
    );
}
