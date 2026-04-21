//! Phase 7 Task 3 — deletion fire-site wiring tests.
//!
//! Exercise `Game::delete_permanent_with_effects` /
//! `delete_permanent_with_cause` end-to-end:
//!   - `WhenWouldBeDeleted` replacements fire at the deletion fire-site and
//!     their outcomes are honored (`Cancelled`, `Redirected`, `Substituted`,
//!     `CustomHandled`).
//!   - Cancelled / redirected / handled deletions suppress `OnDeletion` and
//!     `OnAnyDeletion`.
//!   - The `ReplacementCause` inferred at the fire-site matches the runtime
//!     context (`Battle`, `SecurityCheck`, `OwnEffect`, `OpponentEffect`).
//!
//! Per the Task 3 plan, we restrict ourselves to **mandatory** replacements
//! here — optional accept/decline flows are already covered by the Task 2
//! dispatcher-core tests, and exercising them end-to-end through the
//! fire-site requires Task 6's native-keyword scenario plumbing.

use std::sync::{Arc, Mutex};

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::Zone;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::{ReplacementCause, ReplacementSubject};

// ─── Inline test-effect helpers ──────────────────────────────────────

/// `CardEffect` with a mandatory `WhenWouldBeDeleted` replacement that
/// trashes the top of the subject-controller's deck and calls
/// `handled()` — models Barrier's "trash a card from your deck to
/// prevent deletion" semantic.
struct BarrierLikeHandled;
impl CardEffect for BarrierLikeHandled {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("Barrier-like handled")
            .replacement_process(|rctx| {
                let owner = rctx.effect.player;
                let game = &mut *rctx.effect.game;
                if let Some(card) = game.players[owner as usize].deck.pop() {
                    game.players[owner as usize].trash.push(card);
                }
                rctx.handled();
            })
            .build()]
    }
}

/// `CardEffect` with a mandatory `WhenWouldBeDeleted` replacement that
/// substitutes the subject with the permanent at `(0, 1)` — but only when
/// the current subject is this card itself. The self-scoped condition
/// prevents the replacement from latching onto deletion events of the
/// substituted target (which would otherwise re-fire this substitute and
/// loop).
struct SubstituteTo01;
impl CardEffect for SubstituteTo01 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("Substitute to (0, 1)")
            .replacement_process(|rctx| {
                // Only substitute when the current subject IS the source
                // permanent (i.e. the deletion we're replacing is our own).
                // Without this self-scope guard, the replacement re-fires
                // when the substituted target is itself being deleted —
                // causing an infinite substitution loop.
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

/// Mandatory `WhenWouldBeDeleted` replacement that always cancels. Used as
/// a universal survive-the-delete probe.
struct CancelAlways;
impl CardEffect for CancelAlways {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("Cancel always")
            .replacement_process(|rctx| {
                rctx.cancel();
            })
            .build()]
    }
}

/// Sentinel card — its mandatory `WhenWouldBeDeleted` process records the
/// observed `ReplacementCause` into a shared slot, then cancels. Lets the
/// cause-inference tests read exactly what the fire-site passed in.
struct CauseSentinel(Arc<Mutex<Option<ReplacementCause>>>);
impl CardEffect for CauseSentinel {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![Effect::when_would_be_deleted(card)
            .name("Cause sentinel — cancel")
            .replacement_process(move |rctx| {
                *slot.lock().unwrap() = Some(rctx.cause);
                rctx.cancel();
            })
            .build()]
    }
}

/// Witness for `OnAnyDeletion`. Increments a shared counter each time it
/// fires (one increment per deletion, per observer permanent).
struct OnAnyDeletionCounter(Arc<Mutex<u32>>);
impl CardEffect for OnAnyDeletionCounter {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![Effect::on_any_deletion(card)
            .name("OnAnyDeletion counter")
            .process(move |_ctx| {
                *slot.lock().unwrap() += 1;
            })
            .build()]
    }
}

/// "Hand-to-delete" trigger card — its `EndOfYourTurn` process calls
/// `delete_permanent_with_effects` on the configured target. Registered via
/// `register_effect` with the controller bound so the effect's source
/// controls which `effect_source_player` gets reported.
struct DeleteTargetOnEndOfTurn(PermanentHandle);
impl CardEffect for DeleteTargetOnEndOfTurn {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let target = self.0;
        vec![Effect::end_of_your_turn(card)
            .name("Delete target at end of turn")
            .process(move |ctx| {
                ctx.game.delete_permanent_with_effects(target);
            })
            .build()]
    }
}

fn card(id: &str) -> digimon_engine::CardData {
    make_test_card(id, id)
}

// ─── Tests ─────────────────────────────────────────────────────────────

/// Test 1: Barrier-like custom-handled replacement trashes a deck card and
/// skips the actual deletion (plus suppresses `OnDeletion`).
#[test]
fn barrier_trashes_top_of_deck_and_skips_deletion() {
    let on_deletion_fired = Arc::new(Mutex::new(false));

    let mut r = DebugRunner::builder()
        .add_card(card("TARGET"))
        .add_card(card("FILLER"))
        .deck(0, &["FILLER"; 5])
        .start();
    r.register_effect("TARGET", Arc::new(BarrierLikeHandled));
    let handle = r.place_on_field(0, "TARGET", Some(0));

    // Attach an OnDeletion witness to the same permanent — install by
    // adding another effect under the same card_id. Simplest path:
    // register a composite effect implementation.
    struct Composite(Arc<Mutex<bool>>);
    impl CardEffect for Composite {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            let witness = self.0.clone();
            vec![
                Effect::when_would_be_deleted(card)
                    .name("Barrier-like handled")
                    .replacement_process(|rctx| {
                        let owner = rctx.effect.player;
                        let game = &mut *rctx.effect.game;
                        if let Some(card) = game.players[owner as usize].deck.pop() {
                            game.players[owner as usize].trash.push(card);
                        }
                        rctx.handled();
                    })
                    .build(),
                Effect::on_deletion(card)
                    .name("Witness")
                    .process(move |_ctx| {
                        *witness.lock().unwrap() = true;
                    })
                    .build(),
            ]
        }
    }
    r.register_effect("TARGET", Arc::new(Composite(on_deletion_fired.clone())));

    assert_eq!(r.game.player(0).deck.len(), 5);
    assert_eq!(r.game.player(0).trash.len(), 0);

    r.game.delete_permanent_with_effects(handle);

    assert_eq!(
        r.battle_area_size(0),
        1,
        "permanent should still be on field (handled path skips deletion)"
    );
    assert_eq!(r.game.player(0).deck.len(), 4, "top of deck was trashed");
    assert_eq!(r.game.player(0).trash.len(), 1);
    assert_eq!(
        *on_deletion_fired.lock().unwrap(),
        false,
        "OnDeletion must not fire when replacement reports CustomHandled"
    );
}

/// Test 2: Mandatory redirect-to-deck → permanent removed from field, card
/// placed on bottom of owner's deck, `OnDeletion` suppressed.
#[test]
fn evade_redirects_to_bottom_of_deck() {
    let on_deletion_fired = Arc::new(Mutex::new(false));

    // Compose redirect + on-deletion witness under one effect class.
    struct RedirectPlusWitness(Arc<Mutex<bool>>);
    impl CardEffect for RedirectPlusWitness {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            let witness = self.0.clone();
            vec![
                Effect::when_would_be_deleted(card)
                    .name("Redirect to Deck")
                    .replacement_process(|rctx| {
                        rctx.redirect_to(Zone::Deck);
                    })
                    .build(),
                Effect::on_deletion(card)
                    .name("Witness")
                    .process(move |_ctx| {
                        *witness.lock().unwrap() = true;
                    })
                    .build(),
            ]
        }
    }

    let mut r = DebugRunner::builder().add_card(card("EVADE")).start();
    r.register_effect("EVADE", Arc::new(RedirectPlusWitness(on_deletion_fired.clone())));
    let handle = r.place_on_field(0, "EVADE", Some(0));

    let deck_before = r.game.player(0).deck.len();
    r.game.delete_permanent_with_effects(handle);

    assert_eq!(
        r.battle_area_size(0),
        0,
        "permanent should leave the battle area"
    );
    assert_eq!(
        r.game.player(0).deck.len(),
        deck_before + 1,
        "redirect target: owner's deck grew by 1"
    );
    assert_eq!(
        *on_deletion_fired.lock().unwrap(),
        false,
        "OnDeletion must not fire for a redirected deletion"
    );
}

/// Test 3: Substitute redirects the deletion to another permanent (B). Task
/// 3 simplification: substituted subject is a different permanent on the
/// field. Verifies substitution dispatches to B's own deletion pass.
#[test]
fn substitute_redirects_deletion_to_other_permanent() {
    let mut r = DebugRunner::builder()
        .add_card(card("SUBBER"))
        .add_card(card("TARGET_B"))
        .start();
    r.register_effect("SUBBER", Arc::new(SubstituteTo01));
    let subber = r.place_on_field(0, "SUBBER", Some(0));
    let target_b = r.place_on_field(0, "TARGET_B", Some(0));
    assert_eq!(target_b, PermanentHandle { player: 0, index: 1 });

    r.game.delete_permanent_with_effects(subber);

    // A (the original subject) survives; B (substitute) was deleted.
    assert_eq!(
        r.battle_area_size(0),
        1,
        "one permanent remains (A survives, B was deleted via substitution)"
    );
    // The surviving permanent at index 0 is the original SUBBER.
    let top_card_idx = r.game.player(0).battle_area[0].top_card().card_index;
    assert_eq!(
        top_card_idx, subber.index as u16,
        "SUBBER should still occupy slot 0"
    );
}

/// Test 4: The substituted subject receives its own replacement pass. A
/// substitutes to B; B installs a cancel replacement of its own → B
/// survives; A remains on the field (A's deletion was dispatched to B, and
/// B's own replacement prevented B's deletion).
#[test]
fn substituted_subject_receives_own_replacement_pass() {
    let mut r = DebugRunner::builder()
        .add_card(card("SUBBER"))
        .add_card(card("CANCELER"))
        .start();
    r.register_effect("SUBBER", Arc::new(SubstituteTo01));
    r.register_effect("CANCELER", Arc::new(CancelAlways));
    let subber = r.place_on_field(0, "SUBBER", Some(0));
    let _canceler = r.place_on_field(0, "CANCELER", Some(0));

    r.game.delete_permanent_with_effects(subber);

    // Both permanents should remain on the field: A's deletion was
    // substituted onto B, then B's own cancel replacement fired.
    assert_eq!(
        r.battle_area_size(0),
        2,
        "both permanents remain — substitution + nested cancel preserves the field"
    );
}

/// Test 5: Battle-driven deletion reports `ReplacementCause::Battle`.
#[test]
fn deletion_cause_battle_is_inferred_in_resolve_battle() {
    let cause_slot: Arc<Mutex<Option<ReplacementCause>>> = Arc::new(Mutex::new(None));

    // Attacker: high DP, no replacements.
    // Defender: cause-sentinel replacement; also high DP is set via card
    // data tweak — but we need attacker > defender to trigger defender
    // deletion. We'll tweak defender card DP down directly.
    let mut attacker_card = card("BIG_ATTACKER");
    attacker_card.dp = Some(10000);
    let mut defender_card = card("SMALL_DEFENDER");
    defender_card.dp = Some(1000);

    let mut r = DebugRunner::builder()
        .add_card(attacker_card)
        .add_card(defender_card)
        .start();
    r.register_effect(
        "SMALL_DEFENDER",
        Arc::new(CauseSentinel(cause_slot.clone())),
    );
    let attacker = r.place_on_field(0, "BIG_ATTACKER", Some(0));
    let defender = r.place_on_field(1, "SMALL_DEFENDER", Some(0));

    // Bind attacker+defender via attack. Since defender has a cancel
    // replacement, it survives; the test cares about the observed cause.
    let _ = r.attack_digimon(attacker, defender, false);

    assert_eq!(
        *cause_slot.lock().unwrap(),
        Some(ReplacementCause::Battle),
        "resolve_battle should infer Battle as the deletion cause"
    );
}

/// Test 6: An effect run by the target's controller deletes the target →
/// cause is inferred as `OwnEffect`.
#[test]
fn deletion_cause_own_effect_is_inferred_when_controller_deletes_own() {
    let cause_slot: Arc<Mutex<Option<ReplacementCause>>> = Arc::new(Mutex::new(None));

    let mut r = DebugRunner::builder()
        .add_card(card("TRIGGER"))
        .add_card(card("VICTIM"))
        .start();
    r.register_effect("VICTIM", Arc::new(CauseSentinel(cause_slot.clone())));
    // Place VICTIM first so the TRIGGER effect can reference its handle.
    let victim = r.place_on_field(0, "VICTIM", Some(0));
    r.register_effect("TRIGGER", Arc::new(DeleteTargetOnEndOfTurn(victim)));
    let _trigger = r.place_on_field(0, "TRIGGER", Some(0));

    // End-of-turn fires TRIGGER's effect, whose process calls
    // delete_permanent_with_effects(victim). TRIGGER's controller is P0;
    // victim's controller is P0 → OwnEffect.
    r.end_turn();

    assert_eq!(
        *cause_slot.lock().unwrap(),
        Some(ReplacementCause::OwnEffect),
        "P0-effect deleting P0's permanent should be inferred as OwnEffect"
    );
}

/// Test 7: An effect run by the opponent's controller deletes the target →
/// cause is inferred as `OpponentEffect`.
#[test]
fn deletion_cause_opponent_effect_is_inferred_when_opponent_deletes() {
    let cause_slot: Arc<Mutex<Option<ReplacementCause>>> = Arc::new(Mutex::new(None));

    let mut r = DebugRunner::builder()
        .add_card(card("TRIGGER"))
        .add_card(card("VICTIM"))
        .start();
    r.register_effect("VICTIM", Arc::new(CauseSentinel(cause_slot.clone())));
    // Victim lives on P1.
    let victim = r.place_on_field(1, "VICTIM", Some(0));
    r.register_effect("TRIGGER", Arc::new(DeleteTargetOnEndOfTurn(victim)));
    // Trigger lives on P0; its EndOfYourTurn effect fires during P0's
    // end-of-turn and deletes the P1 victim.
    let _trigger = r.place_on_field(0, "TRIGGER", Some(0));

    r.end_turn();

    assert_eq!(
        *cause_slot.lock().unwrap(),
        Some(ReplacementCause::OpponentEffect),
        "P0-effect deleting P1's permanent should be inferred as OpponentEffect"
    );
}

/// Test 8: Deletion during a security check's battle phase reports
/// `SecurityCheck` as the cause.
///
/// Setup: P0 attacks P1's security with a weak attacker. P1's top security
/// card is a Digimon with huge DP — the attacker loses and is deleted by
/// the security Digimon. Attacker carries the cause sentinel.
#[test]
fn deletion_cause_security_check_is_inferred_during_security_reveal() {
    let cause_slot: Arc<Mutex<Option<ReplacementCause>>> = Arc::new(Mutex::new(None));

    let mut attacker_card = card("WEAK_ATTACKER");
    attacker_card.dp = Some(1000);
    let mut sec_card = card("HUGE_SEC_DIGI");
    sec_card.dp = Some(10000);

    let mut r = DebugRunner::builder()
        .add_card(attacker_card)
        .add_card(sec_card)
        .security(1, &["HUGE_SEC_DIGI"])
        .start();
    r.register_effect(
        "WEAK_ATTACKER",
        Arc::new(CauseSentinel(cause_slot.clone())),
    );
    let attacker = r.place_on_field(0, "WEAK_ATTACKER", Some(0));

    let _ = r.attack_player(attacker, 1, false);

    // The attacker's cause-sentinel fires when security's counter-battle
    // tries to delete it; at that moment `security_resolution.is_some()`,
    // so the cause should be SecurityCheck.
    assert_eq!(
        *cause_slot.lock().unwrap(),
        Some(ReplacementCause::SecurityCheck),
        "attacker deletion during security reveal should be inferred as SecurityCheck"
    );
}

/// Test 9: A cancelled deletion suppresses `OnAnyDeletion` observers.
#[test]
fn deletion_cancelled_suppresses_on_any_deletion() {
    let counter: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));

    let mut r = DebugRunner::builder()
        .add_card(card("TARGET"))
        .add_card(card("WITNESS"))
        .start();
    r.register_effect("TARGET", Arc::new(CancelAlways));
    r.register_effect("WITNESS", Arc::new(OnAnyDeletionCounter(counter.clone())));
    let target = r.place_on_field(0, "TARGET", Some(0));
    let _witness = r.place_on_field(0, "WITNESS", Some(0));

    r.game.delete_permanent_with_effects(target);

    assert_eq!(
        *counter.lock().unwrap(),
        0,
        "OnAnyDeletion must not fire for a cancelled deletion"
    );
    assert_eq!(r.battle_area_size(0), 2, "both cards still on field");
}

/// Test 10: A deletion redirected to `Hand` does not fire `OnDeletion` on
/// the target.
#[test]
fn deletion_redirected_to_hand_does_not_fire_on_deletion() {
    let on_deletion_fired = Arc::new(Mutex::new(false));

    struct RedirectHandPlusWitness(Arc<Mutex<bool>>);
    impl CardEffect for RedirectHandPlusWitness {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            let witness = self.0.clone();
            vec![
                Effect::when_would_be_deleted(card)
                    .name("Redirect to Hand")
                    .replacement_process(|rctx| {
                        rctx.redirect_to(Zone::Hand);
                    })
                    .build(),
                Effect::on_deletion(card)
                    .name("Witness")
                    .process(move |_ctx| {
                        *witness.lock().unwrap() = true;
                    })
                    .build(),
            ]
        }
    }

    let mut r = DebugRunner::builder().add_card(card("BOUNCE")).start();
    r.register_effect(
        "BOUNCE",
        Arc::new(RedirectHandPlusWitness(on_deletion_fired.clone())),
    );
    let handle = r.place_on_field(0, "BOUNCE", Some(0));
    let hand_before = r.game.player(0).hand.len();

    r.game.delete_permanent_with_effects(handle);

    assert_eq!(r.battle_area_size(0), 0, "permanent left the field");
    assert_eq!(
        r.game.player(0).hand.len(),
        hand_before + 1,
        "redirect target is hand — hand grew by 1"
    );
    assert_eq!(
        *on_deletion_fired.lock().unwrap(),
        false,
        "OnDeletion must not fire for a redirect-to-hand path"
    );
}
