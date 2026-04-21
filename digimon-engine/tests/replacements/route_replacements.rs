//! Phase 7 Task 4 — per-route Would* fire-site wiring tests.
//!
//! Exercises the return-to-hand, return-to-deck, trash-by-effect,
//! place-in-security, draw, and de-digivolve Would* replacements end-to-end
//! through their respective fire-sites in `game_actions.rs` and
//! `effect_context/mod.rs`.
//!
//! Task 4 does NOT wire `WhenWouldLoseSecurity` — see the Task 4 spec note in
//! `drive_security_resolution`: reinstating the revealed card back onto the
//! security stack requires non-trivial security-phase replumbing that is best
//! revisited in Task 6 (native keyword parsing) or Task 7 (end-to-end).

use std::sync::{Arc, Mutex};

use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{
    CardSourceRef, EffectTiming, Expiry, ModifierType, StackPosition, Zone,
};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementSubject;

// ─── Inline test-effect helpers ───────────────────────────────────────

/// Mandatory cancel replacement for any single Would* timing, parameterised
/// at construction. Used across tests to test cancel semantics.
struct CancelOn(EffectTiming);
impl CardEffect for CancelOn {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let builder = match self.0 {
            EffectTiming::WhenWouldBeReturnedToHand => {
                Effect::when_would_be_returned_to_hand(card)
            }
            EffectTiming::WhenWouldBeReturnedToDeck => {
                Effect::when_would_be_returned_to_deck(card)
            }
            EffectTiming::WhenWouldBeTrashed => Effect::when_would_be_trashed(card),
            EffectTiming::WhenWouldBeDeDigivolved => {
                Effect::when_would_be_de_digivolved(card)
            }
            EffectTiming::WhenWouldDraw => Effect::when_would_draw(card),
            EffectTiming::WhenWouldPlaceInSecurity => {
                Effect::when_would_place_in_security(card)
            }
            _ => panic!("CancelOn: unsupported timing {:?}", self.0),
        };
        vec![builder
            .name("Cancel always")
            .replacement_process(|rctx| {
                rctx.cancel();
            })
            .build()]
    }
}

/// Mandatory redirect-to-Trash replacement for `WhenWouldBeReturnedToHand`.
struct RedirectReturnToHandToTrash;
impl CardEffect for RedirectReturnToHandToTrash {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_returned_to_hand(card)
            .name("Redirect return-to-hand to Trash")
            .replacement_process(|rctx| {
                rctx.redirect_to(Zone::Trash);
            })
            .build()]
    }
}

/// Mandatory redirect-to-Trash replacement for `WhenWouldPlaceInSecurity`.
struct RedirectPlaceToTrash;
impl CardEffect for RedirectPlaceToTrash {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_place_in_security(card)
            .name("Redirect place-in-security to Trash")
            .replacement_process(|rctx| {
                rctx.redirect_to(Zone::Trash);
            })
            .build()]
    }
}

/// Mandatory substitute-to-(0,1) replacement for `WhenWouldBeDeDigivolved`.
/// Only substitutes when the current subject is this card (self-scope guard).
struct DeDigivolveSubstituteTo01;
impl CardEffect for DeDigivolveSubstituteTo01 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_de_digivolved(card)
            .name("De-Digi substitute to (0, 1)")
            .replacement_process(|rctx| {
                let me = rctx.effect.source_permanent;
                if let ReplacementSubject::Permanent(subject) = rctx.subject {
                    if Some(subject) == me {
                        rctx.substitute(ReplacementSubject::Permanent(PermanentHandle {
                            player: 0,
                            index: 1,
                        }));
                    }
                }
            })
            .build()]
    }
}

/// `WhenWouldBeDeDigivolved` replacement that calls `rctx.handled()` after
/// doing internal bookkeeping. No real state is mutated — just the "handled"
/// outcome, which the fire-site treats as "do nothing, return 0".
struct DeDigivolveCustomHandled;
impl CardEffect for DeDigivolveCustomHandled {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_de_digivolved(card)
            .name("De-Digi custom handled")
            .replacement_process(|rctx| {
                rctx.handled();
            })
            .build()]
    }
}

/// `WhenWouldDraw` replacement with a sentinel: records whether it was ever
/// invoked, then cancels. Used to check flood-gate-before-replacement order.
struct DrawCancelWithSentinel(Arc<Mutex<bool>>);
impl CardEffect for DrawCancelWithSentinel {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![Effect::when_would_draw(card)
            .name("Draw cancel sentinel")
            .replacement_process(move |rctx| {
                *slot.lock().unwrap() = true;
                rctx.cancel();
            })
            .build()]
    }
}

fn card(id: &str) -> digimon_engine::CardData {
    make_test_card(id, id)
}

// ─── Tests: return-to-hand ────────────────────────────────────────────

/// Cancel-on-WhenWouldBeReturnedToHand keeps the permanent on the field.
#[test]
fn would_be_returned_to_hand_cancel_keeps_permanent_on_field() {
    let mut r = DebugRunner::builder().add_card(card("TARGET")).start();
    r.register_effect(
        "TARGET",
        Arc::new(CancelOn(EffectTiming::WhenWouldBeReturnedToHand)),
    );
    let handle = r.place_on_field(0, "TARGET", Some(0));
    let hand_before = r.game.player(0).hand.len();

    let result = r.game.return_to_hand(handle);

    assert!(result.is_none(), "cancel → return_to_hand returns None");
    assert_eq!(r.battle_area_size(0), 1, "permanent still on field");
    assert_eq!(
        r.game.player(0).hand.len(),
        hand_before,
        "hand size unchanged"
    );
}

/// Redirect-to-Trash on return-to-hand should delete the permanent instead
/// (fires via `delete_permanent_with_cause`).
#[test]
fn would_be_returned_to_hand_redirect_to_trash_deletes_instead() {
    let mut r = DebugRunner::builder().add_card(card("TARGET")).start();
    r.register_effect("TARGET", Arc::new(RedirectReturnToHandToTrash));
    let handle = r.place_on_field(0, "TARGET", Some(0));
    let hand_before = r.game.player(0).hand.len();
    let trash_before = r.game.player(0).trash.len();

    let result = r.game.return_to_hand(handle);

    assert!(
        result.is_none(),
        "redirect-to-trash → return_to_hand returns None"
    );
    assert_eq!(
        r.battle_area_size(0),
        0,
        "permanent is gone from the field (deleted)"
    );
    assert_eq!(
        r.game.player(0).hand.len(),
        hand_before,
        "hand size unchanged by redirect-to-trash path"
    );
    assert_eq!(
        r.game.player(0).trash.len(),
        trash_before + 1,
        "trash grew by 1 (the top card was routed to trash)"
    );
}

// ─── Tests: return-to-deck ────────────────────────────────────────────

/// Cancel-on-WhenWouldBeReturnedToDeck keeps the permanent on the field.
#[test]
fn would_be_returned_to_deck_cancel_keeps_permanent_on_field() {
    let mut r = DebugRunner::builder().add_card(card("TARGET")).start();
    r.register_effect(
        "TARGET",
        Arc::new(CancelOn(EffectTiming::WhenWouldBeReturnedToDeck)),
    );
    let handle = r.place_on_field(0, "TARGET", Some(0));
    let deck_before = r.game.player(0).deck.len();

    let ok = r.game.return_to_deck(handle, StackPosition::Bottom);

    assert!(!ok, "cancel → return_to_deck returns false");
    assert_eq!(r.battle_area_size(0), 1, "permanent still on field");
    assert_eq!(
        r.game.player(0).deck.len(),
        deck_before,
        "deck size unchanged"
    );
}

// ─── Tests: draw ──────────────────────────────────────────────────────

/// Cancel-on-WhenWouldDraw prevents the draw from committing.
///
/// The replacement is sourced from a permanent on the battle area; its
/// controller is also the drawing player, so the fire-site's collected
/// candidates include the replacement.
#[test]
fn would_draw_cancel_prevents_card_movement() {
    let mut r = DebugRunner::builder()
        .add_card(card("GATE"))
        .add_card(card("FILLER"))
        .deck(0, &["FILLER"; 5])
        .start();
    r.register_effect("GATE", Arc::new(CancelOn(EffectTiming::WhenWouldDraw)));
    let gate = r.place_on_field(0, "GATE", Some(0));
    let hand_before = r.game.player(0).hand.len();
    let deck_before = r.game.player(0).deck.len();

    // Call ctx.draw on behalf of player 0. Source context: the GATE permanent.
    let drawn = {
        let mut ctx =
            EffectContext::new(&mut r.game, CardHandle(999), Some(gate), 0);
        ctx.draw(0, 1)
    };

    assert_eq!(drawn, 0, "cancel → draw returns 0");
    assert_eq!(r.game.player(0).hand.len(), hand_before, "hand unchanged");
    assert_eq!(r.game.player(0).deck.len(), deck_before, "deck unchanged");
}

/// `CannotDrawByEffect` flood gate fires BEFORE the WhenWouldDraw replacement
/// window. A sentinel on the replacement confirms the process was never run.
#[test]
fn would_draw_still_respects_flood_gate_cannot_draw_by_effect() {
    let sentinel = Arc::new(Mutex::new(false));

    let mut r = DebugRunner::builder()
        .add_card(card("GATE"))
        .add_card(card("FILLER"))
        .deck(0, &["FILLER"; 5])
        .start();
    r.register_effect(
        "GATE",
        Arc::new(DrawCancelWithSentinel(sentinel.clone())),
    );
    let gate = r.place_on_field(0, "GATE", Some(0));

    // Install CannotDrawByEffect on player 0.
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotDrawByEffect,
            0,
            Expiry::Permanent,
            None,
            1,
        ),
    );

    let drawn = {
        let mut ctx =
            EffectContext::new(&mut r.game, CardHandle(999), Some(gate), 0);
        ctx.draw(0, 1)
    };

    assert_eq!(drawn, 0, "flood gate suppresses the draw");
    assert_eq!(
        *sentinel.lock().unwrap(),
        false,
        "replacement process must not be invoked when the flood gate fires first"
    );
}

// ─── Tests: place-in-security ─────────────────────────────────────────

/// Redirect-to-Trash on WhenWouldPlaceInSecurity: the hand card lands in
/// trash instead of the security stack.
#[test]
fn would_place_in_security_redirect_to_trash() {
    let mut r = DebugRunner::builder()
        .add_card(card("GATE"))
        .add_card(card("PAYLOAD"))
        .start();
    r.register_effect("GATE", Arc::new(RedirectPlaceToTrash));
    let _gate = r.place_on_field(1, "GATE", Some(0));

    // Push a PAYLOAD card into player 0's hand.
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "PAYLOAD")
        .expect("PAYLOAD card registered");
    let next_idx = r.game.next_card_index();
    let mut cs = CardSource::new(data_idx, 0, next_idx);
    cs.card_index = next_idx;
    r.game.players[0].hand.push(cs);

    let sec_before = r.game.player(0).security.len();
    let trash_before = r.game.player(0).trash.len();

    let ok = r.game.place_on_security(
        0,
        CardSourceRef::Hand(0, 0),
        StackPosition::Top,
        false,
    );

    assert!(!ok, "redirect-to-trash → place_on_security returns false");
    assert_eq!(
        r.game.player(0).security.len(),
        sec_before,
        "security stack unchanged"
    );
    assert_eq!(
        r.game.player(0).trash.len(),
        trash_before + 1,
        "trash grew by 1 (payload redirected)"
    );
    assert_eq!(
        r.game.player(0).hand.len(),
        0,
        "payload was removed from hand"
    );
}

// ─── Tests: trash-from-hand by effect ─────────────────────────────────

/// Cancel-on-WhenWouldBeTrashed keeps the hand card in the player's hand.
#[test]
fn would_be_trashed_from_hand_cancel_keeps_card_in_hand() {
    let mut r = DebugRunner::builder()
        .add_card(card("GATE"))
        .add_card(card("HANDCARD"))
        .start();
    r.register_effect("GATE", Arc::new(CancelOn(EffectTiming::WhenWouldBeTrashed)));
    let gate = r.place_on_field(0, "GATE", Some(0));

    // Push a HANDCARD into player 0's hand.
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "HANDCARD")
        .expect("HANDCARD card registered");
    let next_idx = r.game.next_card_index();
    let mut cs = CardSource::new(data_idx, 0, next_idx);
    cs.card_index = next_idx;
    r.game.players[0].hand.push(cs);

    let hand_before = r.game.player(0).hand.len();
    let trash_before = r.game.player(0).trash.len();

    let result = {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(999), Some(gate), 0);
        ctx.trash_from_hand_by_index(0, 0)
    };

    assert!(result.is_none(), "cancel → trash_from_hand_by_index returns None");
    assert_eq!(
        r.game.player(0).hand.len(),
        hand_before,
        "hand size unchanged"
    );
    assert_eq!(
        r.game.player(0).trash.len(),
        trash_before,
        "trash unchanged"
    );
}

// ─── Tests: de-digivolve ──────────────────────────────────────────────

/// Cancel-on-WhenWouldBeDeDigivolved prevents any popping of the stack.
#[test]
fn would_be_de_digivolved_cancel_prevents_pops() {
    let mut r = DebugRunner::builder()
        .add_card(card("TARGET"))
        .start();
    r.register_effect(
        "TARGET",
        Arc::new(CancelOn(EffectTiming::WhenWouldBeDeDigivolved)),
    );
    let target = r.place_on_field(0, "TARGET", Some(0));

    // Manually stack two more sources onto the target's permanent so
    // there's material to de-digivolve.
    {
        let perm = &mut r.game.players[0].battle_area[target.index as usize];
        let data_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TARGET")
            .unwrap();
        for _ in 0..2 {
            let idx = perm.card_sources.len() as u16 + 1000;
            let mut cs = CardSource::new(data_idx, 0, idx);
            cs.card_index = idx;
            perm.card_sources.push(cs);
        }
    }
    let stack_size_before =
        r.game.players[0].battle_area[target.index as usize].stack_size();

    let popped = {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(999), Some(target), 0);
        ctx.de_digivolve(target, None, Some(2))
    };

    assert_eq!(popped, 0, "cancel → de_digivolve returns 0");
    assert_eq!(
        r.game.players[0].battle_area[target.index as usize].stack_size(),
        stack_size_before,
        "stack unchanged"
    );
}

/// Substitute targets another permanent: A holds the replacement and
/// substitutes to B; the pops land on B, not A.
#[test]
fn would_be_de_digivolved_substitute_targets_other_permanent() {
    let mut r = DebugRunner::builder()
        .add_card(card("SUBBER"))
        .add_card(card("VICTIM"))
        .start();
    r.register_effect("SUBBER", Arc::new(DeDigivolveSubstituteTo01));
    let subber = r.place_on_field(0, "SUBBER", Some(0));
    let victim = r.place_on_field(0, "VICTIM", Some(0));
    assert_eq!(victim, PermanentHandle { player: 0, index: 1 });

    // Stack an extra source onto VICTIM so it has material to de-digivolve.
    {
        let data_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "VICTIM")
            .unwrap();
        let perm = &mut r.game.players[0].battle_area[victim.index as usize];
        let idx = 9999;
        let mut cs = CardSource::new(data_idx, 0, idx);
        cs.card_index = idx;
        perm.card_sources.push(cs);
    }
    let subber_stack_before =
        r.game.players[0].battle_area[subber.index as usize].stack_size();
    let victim_stack_before =
        r.game.players[0].battle_area[victim.index as usize].stack_size();
    assert_eq!(victim_stack_before, 2);

    let popped = {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(999), Some(subber), 0);
        ctx.de_digivolve(subber, None, Some(1))
    };

    assert_eq!(popped, 1, "one pop should land on the substituted target");
    assert_eq!(
        r.game.players[0].battle_area[subber.index as usize].stack_size(),
        subber_stack_before,
        "SUBBER stack unchanged (substitute retargeted)"
    );
    assert_eq!(
        r.game.players[0].battle_area[victim.index as usize].stack_size(),
        victim_stack_before - 1,
        "VICTIM stack popped by 1"
    );
}

/// `rctx.handled()` on WhenWouldBeDeDigivolved returns 0 and leaves the
/// target's stack untouched.
#[test]
fn would_be_de_digivolved_custom_handled_returns_zero() {
    let mut r = DebugRunner::builder().add_card(card("TARGET")).start();
    r.register_effect("TARGET", Arc::new(DeDigivolveCustomHandled));
    let target = r.place_on_field(0, "TARGET", Some(0));

    // Stack an extra source so there's material to de-digi.
    {
        let perm = &mut r.game.players[0].battle_area[target.index as usize];
        let data_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TARGET")
            .unwrap();
        let idx = perm.card_sources.len() as u16 + 1000;
        let mut cs = CardSource::new(data_idx, 0, idx);
        cs.card_index = idx;
        perm.card_sources.push(cs);
    }
    let stack_before =
        r.game.players[0].battle_area[target.index as usize].stack_size();

    let popped = {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(999), Some(target), 0);
        ctx.de_digivolve(target, None, Some(1))
    };

    assert_eq!(popped, 0, "handled → de_digivolve returns 0");
    assert_eq!(
        r.game.players[0].battle_area[target.index as usize].stack_size(),
        stack_before,
        "stack unchanged under custom-handled"
    );
}

