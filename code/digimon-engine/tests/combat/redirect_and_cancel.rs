//! Phase 9 Task 2 — `ctx.redirect_attack` / `ctx.cancel_attack` script API.
//!
//! Exercises:
//!   - Redirect from an `OnAttack` observer rewrites `effective_target` and
//!     fires `OnAttackTargetChange` globally.
//!   - Cancel from an `OnAttack` observer short-circuits the state machine
//!     to Cleanup — no Battle state, no `EndOfBattle`, `EndOfAttack` fires.
//!   - Target validation rejects non-Digimon / missing-handle redirects.
//!   - Helpers return `AttackError::NoActiveAttack` when called outside
//!     an active attack (e.g. from `OnPlay`).
//!   - Memory granted by an `OnAttack` closure is NOT applied if a
//!     `WhenWouldAttack` replacement cancels the attack before `OnAttack`
//!     fires — i.e. an earlier-phase cancel preserves the pre-declaration
//!     memory baseline.

use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::card_source::CardHandle;
use digimon_engine::combat::{AttackError, AttackResult};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect, EffectBuilder};
use digimon_engine::effect_context::AttackTargetRestriction;
use digimon_engine::enums::{EffectTiming, Expiry, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{AttackTarget, SelectionKind};
use digimon_engine::AttackTargetChangeReason;

fn card(id: &str) -> digimon_engine::CardData {
    make_test_card(id, id)
}

fn card_with_dp(id: &str, dp: i32) -> digimon_engine::CardData {
    let mut c = card(id);
    c.dp = Some(dp);
    c
}

// ─── Effect scaffolds ──────────────────────────────────────────────────

/// OnAttack closure that invokes `ctx.redirect_attack(new_target)`. The
/// new-target handle is captured at construction time.
struct RedirectOnAttack(PermanentHandle);
impl CardEffect for RedirectOnAttack {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let new_target = self.0;
        vec![Effect::on_attack(c)
            .name("Redirect from OnAttack")
            .process(move |ctx| {
                let _ = ctx.redirect_attack(AttackTarget::Digimon(new_target));
            })
            .build()]
    }
}

/// OnAttack closure that parks a player-facing redirect-target prompt.
struct SelectRedirectOnAttack;
impl CardEffect for SelectRedirectOnAttack {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        vec![Effect::on_attack(c)
            .name("Select redirect target from OnAttack")
            .process(move |ctx| {
                let _ = ctx.select_redirect_attack_target(
                    AttackTargetRestriction::PlayerOnly,
                    false,
                    "Redirect the attack to the player?",
                );
            })
            .build()]
    }
}

/// OnAttack closure that invokes `ctx.cancel_attack()`.
struct CancelOnAttack;
impl CardEffect for CancelOnAttack {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        vec![Effect::on_attack(c)
            .name("Cancel from OnAttack")
            .process(move |ctx| {
                let _ = ctx.cancel_attack();
            })
            .build()]
    }
}

/// CounterEffect closure that tries to cancel the in-flight attack and records
/// whether combat accepted or rejected the late cancellation.
struct CancelFromCounter {
    slot: Arc<Mutex<Option<Result<(), AttackError>>>>,
}
impl CardEffect for CancelFromCounter {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let slot = self.slot.clone();
        vec![Effect::on_play(c)
            .name("Late counter cancel")
            .counter()
            .timing(EffectTiming::CounterEffect)
            .process(move |ctx| {
                let result = ctx.cancel_attack();
                *slot.lock().unwrap() = Some(result);
            })
            .build()]
    }
}

/// OnAttack closure that records the result of `ctx.redirect_attack(bad)`
/// into a shared slot. Lets the test inspect the error.
struct RedirectRecordingErr {
    target: AttackTarget,
    slot: Arc<Mutex<Option<Result<(), AttackError>>>>,
}
impl CardEffect for RedirectRecordingErr {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let target = self.target;
        let slot = self.slot.clone();
        vec![Effect::on_attack(c)
            .name("Recording redirect error")
            .process(move |ctx| {
                let r = ctx.redirect_attack(target);
                *slot.lock().unwrap() = Some(r);
            })
            .build()]
    }
}

/// OnPlay closure that records the result of `ctx.redirect_attack(...)`
/// into a shared slot. Used to assert that calling outside an active
/// attack returns `AttackError::NoActiveAttack`.
struct RedirectFromOnPlay {
    target: AttackTarget,
    slot: Arc<Mutex<Option<Result<(), AttackError>>>>,
}
impl CardEffect for RedirectFromOnPlay {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let target = self.target;
        let slot = self.slot.clone();
        vec![Effect::on_play(c)
            .name("Recording redirect from OnPlay")
            .process(move |ctx| {
                let r = ctx.redirect_attack(target);
                *slot.lock().unwrap() = Some(r);
            })
            .build()]
    }
}

/// Sentinel-recording `OnAttackTargetChange` observer that also captures
/// (before, after) of `effective_target` each time it fires.
struct WitnessTargetChange(Arc<Mutex<Vec<AttackTarget>>>);
impl CardEffect for WitnessTargetChange {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let log = self.0.clone();
        vec![Effect::on_attack_target_change(c)
            .name("Witness target change")
            .process(move |ctx| {
                if let Some(pa) = ctx.game.pending_attack.as_ref() {
                    log.lock().unwrap().push(pa.effective_target);
                }
            })
            .build()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TargetChangeSnapshot {
    attacker: PermanentHandle,
    old_target: AttackTarget,
    new_target: AttackTarget,
    reason: AttackTargetChangeReason,
    controller: u8,
}

struct WitnessTargetChangePayload(Arc<Mutex<Vec<TargetChangeSnapshot>>>);
impl CardEffect for WitnessTargetChangePayload {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let log = self.0.clone();
        vec![Effect::on_attack_target_change(c)
            .name("Witness target change payload")
            .process(move |ctx| {
                if let Some(change) = ctx.attack_target_change() {
                    log.lock().unwrap().push(TargetChangeSnapshot {
                        attacker: change.attacker,
                        old_target: change.old_target,
                        new_target: change.new_target,
                        reason: change.reason,
                        controller: change.controller,
                    });
                }
            })
            .build()]
    }
}

/// EndOfAttack / EndOfBattle witness pair on one card. Each fire increments
/// its counter.
struct WitnessCleanup {
    end_of_attack: Arc<Mutex<u32>>,
    end_of_battle: Arc<Mutex<u32>>,
}
impl CardEffect for WitnessCleanup {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let ea = self.end_of_attack.clone();
        let eb = self.end_of_battle.clone();
        vec![
            Effect::end_of_attack(c)
                .name("Witness EoA")
                .process(move |_ctx| {
                    *ea.lock().unwrap() += 1;
                })
                .build(),
            Effect::end_of_battle(c)
                .name("Witness EoB")
                .process(move |_ctx| {
                    *eb.lock().unwrap() += 1;
                })
                .build(),
        ]
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────

/// Test 1: `ctx.redirect_attack` from an `OnAttack` observer rewrites
/// `effective_target` and fires `OnAttackTargetChange` — witnessed by a
/// separate observer on the opposing side.
#[test]
fn ctx_redirect_attack_rewrites_effective_target_and_fires_observer() {
    let witness_log: Arc<Mutex<Vec<AttackTarget>>> = Arc::new(Mutex::new(Vec::new()));

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .add_card(card("REDIR"))
        .add_card(card("WITNESS"))
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let redir = r.place_on_field(1, "REDIR", Some(0));
    assert_eq!(
        redir,
        PermanentHandle {
            player: 1,
            index: 1
        }
    );

    // Install the redirect on the attacker — fires OnAttack during its
    // own declaration and rewrites effective_target from DEF to REDIR.
    r.register_effect("ATK", Arc::new(RedirectOnAttack(redir)));
    // Witness on P0's side (global observer fan-out covers both players).
    r.register_effect(
        "WITNESS",
        Arc::new(WitnessTargetChange(witness_log.clone())),
    );
    let _witness = r.place_on_field(0, "WITNESS", Some(0));

    let _result = r.attack_digimon(atk, def, false);

    // OnAttackTargetChange must have fired at least once, and effective_target
    // at the time of firing was REDIR (the post-redirect value).
    let log = witness_log.lock().unwrap().clone();
    assert!(
        !log.is_empty(),
        "OnAttackTargetChange must fire on redirect_attack"
    );
    assert!(
        log.iter()
            .any(|t| matches!(t, AttackTarget::Digimon(h) if *h == redir)),
        "witness must see effective_target == REDIR post-redirect (saw {:?})",
        log
    );
}

#[test]
fn ctx_redirect_attack_payload_carries_old_new_reason_and_controller() {
    let payload_log: Arc<Mutex<Vec<TargetChangeSnapshot>>> = Arc::new(Mutex::new(Vec::new()));

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .add_card(card("REDIR"))
        .add_card(card("WITNESS"))
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let redir = r.place_on_field(1, "REDIR", Some(0));
    let source_card = r.game.players[0].battle_area[atk.index as usize]
        .top_card()
        .handle();

    r.register_effect("ATK", Arc::new(RedirectOnAttack(redir)));
    r.register_effect(
        "WITNESS",
        Arc::new(WitnessTargetChangePayload(payload_log.clone())),
    );
    let _witness = r.place_on_field(1, "WITNESS", Some(0));

    let _result = r.attack_digimon(atk, def, false);

    assert_eq!(
        payload_log.lock().unwrap().as_slice(),
        &[TargetChangeSnapshot {
            attacker: atk,
            old_target: AttackTarget::Digimon(def),
            new_target: AttackTarget::Digimon(redir),
            reason: AttackTargetChangeReason::EffectRedirect(Some(source_card)),
            controller: 0,
        }],
        "OnAttackTargetChange should expose its structured payload"
    );
}

#[test]
fn ctx_select_redirect_attack_target_prompt_clones_faithfully() {
    let mut r = DebugRunner::builder()
        .add_card(card_with_dp("ATK", 7000))
        .add_card(card_with_dp("DEF", 2000))
        .add_card(card_with_dp("SEC", 1000))
        .security(1, &["SEC"])
        .start();

    r.register_effect("ATK", Arc::new(SelectRedirectOnAttack));
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let security_before = r.game.players[1].security.len();
    let player_action = encode_attack(atk.index as u16, SECURITY_TARGET);

    assert_eq!(
        r.attack_digimon(atk, def, false),
        AttackResult::InProgress,
        "attack should park at the redirect-target prompt"
    );

    let pending = r.game.pending_selection.as_ref().expect("redirect prompt");
    assert_eq!(pending.kind, SelectionKind::Target);
    assert!(pending.valid_action_ids.contains(&player_action));
    assert!(
        r.game.pending_selection_resume.is_some(),
        "select_redirect_attack_target must park an AttackTarget data frame for clone-faithful prompts"
    );
    let selecting_player = pending.selecting_player;

    let mut clone = r.game.clone();
    clone
        .resolve_selection(selecting_player, player_action)
        .expect("clone redirect resolves");
    assert_eq!(
        clone.players[1].security.len(),
        security_before - 1,
        "clone should redirect to the player and continue into the security check"
    );

    assert!(
        r.game.pending_selection.is_some(),
        "resolving the clone must leave the original at the redirect prompt"
    );
    assert_eq!(
        r.game.players[1].security.len(),
        security_before,
        "resolving the clone must not mutate original security"
    );

    r.game
        .resolve_selection(selecting_player, player_action)
        .expect("original redirect resolves");
    assert_eq!(
        r.game.players[1].security.len(),
        clone.players[1].security.len(),
        "original and clone replay the redirect identically"
    );
}

/// Test 2: `ctx.cancel_attack` from an `OnAttack` observer short-circuits
/// the state machine to Cleanup. `EndOfAttack` fires (cleanup symmetry);
/// `EndOfBattle` does NOT (no DP battle happened).
#[test]
fn ctx_cancel_attack_short_circuits_to_cleanup() {
    let end_of_attack = Arc::new(Mutex::new(0u32));
    let end_of_battle = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .add_card(card("WITNESS"))
        .start();

    r.register_effect("ATK", Arc::new(CancelOnAttack));
    r.register_effect(
        "WITNESS",
        Arc::new(WitnessCleanup {
            end_of_attack: end_of_attack.clone(),
            end_of_battle: end_of_battle.clone(),
        }),
    );
    let atk = r.place_on_field(0, "ATK", Some(0));
    let _witness = r.place_on_field(0, "WITNESS", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    let result = r.attack_digimon(atk, def, false);
    assert_eq!(
        result,
        AttackResult::Cancelled,
        "ctx.cancel_attack() routes the terminal result to Cancelled"
    );
    assert!(
        r.game.pending_attack.is_none(),
        "pending_attack cleaned up after cancel"
    );
    assert_eq!(
        *end_of_attack.lock().unwrap(),
        1,
        "EndOfAttack fires for cleanup symmetry"
    );
    assert_eq!(
        *end_of_battle.lock().unwrap(),
        0,
        "EndOfBattle must NOT fire — no DP battle happened"
    );
}

/// Test 3: `ctx.redirect_attack` with an invalid target returns
/// `AttackError::InvalidTarget` and does NOT mutate `effective_target`.
///
/// Uses a non-existent Digimon slot handle on P1's field — `handle_valid`
/// rejects it. Same result for a Player target pointing at the attacker's
/// own controller (exercised as a second assertion with a different effect).
#[test]
fn ctx_redirect_attack_validates_target_legality() {
    let slot: Arc<Mutex<Option<Result<(), AttackError>>>> = Arc::new(Mutex::new(None));

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .start();

    // Invalid target: P1 index 99 — doesn't resolve to any permanent.
    let invalid = PermanentHandle {
        player: 1,
        index: 99,
    };
    r.register_effect(
        "ATK",
        Arc::new(RedirectRecordingErr {
            target: AttackTarget::Digimon(invalid),
            slot: slot.clone(),
        }),
    );

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    let _result = r.attack_digimon(atk, def, false);

    // The OnAttack closure must have been invoked and recorded the error.
    let recorded = slot.lock().unwrap().clone();
    assert_eq!(
        recorded,
        Some(Err(AttackError::InvalidTarget)),
        "redirect to a non-existent handle must return InvalidTarget (got {:?})",
        recorded
    );
    // Second probe: Player target pointing at the attacker's own controller
    // is rejected by `validate_attack_target`. Exercised through the same
    // ctx helper path (invalid Player(0) redirect while attacker is P0).
    let slot2: Arc<Mutex<Option<Result<(), AttackError>>>> = Arc::new(Mutex::new(None));
    let mut r2 = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .start();
    r2.register_effect(
        "ATK",
        Arc::new(RedirectRecordingErr {
            target: AttackTarget::Player(0),
            slot: slot2.clone(),
        }),
    );
    let atk2 = r2.place_on_field(0, "ATK", Some(0));
    let def2 = r2.place_on_field(1, "DEF", Some(0));
    let _ = r2.attack_digimon(atk2, def2, false);
    let recorded2 = slot2.lock().unwrap().clone();
    assert_eq!(
        recorded2,
        Some(Err(AttackError::InvalidTarget)),
        "redirect to Player(attacker's own controller) must be rejected (got {:?})",
        recorded2
    );
}

#[test]
fn ctx_redirect_attack_honors_fixed_target_modifiers() {
    let slot: Arc<Mutex<Option<Result<(), AttackError>>>> = Arc::new(Mutex::new(None));
    let witness_log: Arc<Mutex<Vec<AttackTarget>>> = Arc::new(Mutex::new(Vec::new()));

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .add_card(card("FIXED"))
        .add_card(card("WITNESS"))
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let fixed = r.place_on_field(1, "FIXED", Some(0));
    r.game.modifiers.add(
        fixed,
        ModifierEntry::simple(
            ModifierType::CannotBeRedirectedAsAttackTarget,
            0,
            Expiry::EndOfTurn,
            0,
        ),
    );

    r.register_effect(
        "ATK",
        Arc::new(RedirectRecordingErr {
            target: AttackTarget::Digimon(fixed),
            slot: slot.clone(),
        }),
    );
    r.register_effect(
        "WITNESS",
        Arc::new(WitnessTargetChange(witness_log.clone())),
    );
    let _witness = r.place_on_field(0, "WITNESS", Some(0));

    let _result = r.attack_digimon(atk, def, false);

    let recorded = slot.lock().unwrap().take().expect("redirect result");
    assert_eq!(recorded, Err(AttackError::InvalidTarget));
    assert!(
        witness_log.lock().unwrap().is_empty(),
        "rejected redirect must not fire OnAttackTargetChange"
    );
}

#[test]
fn ctx_redirect_attack_honors_cannot_switch_attacker_modifier() {
    let slot: Arc<Mutex<Option<Result<(), AttackError>>>> = Arc::new(Mutex::new(None));
    let witness_log: Arc<Mutex<Vec<AttackTarget>>> = Arc::new(Mutex::new(Vec::new()));

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .add_card(card("REDIR"))
        .add_card(card("WITNESS"))
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let redir = r.place_on_field(1, "REDIR", Some(0));
    r.game.modifiers.add(
        atk,
        ModifierEntry::simple(
            ModifierType::CanNotSwitchAttackTarget,
            0,
            Expiry::EndOfTurn,
            0,
        ),
    );

    r.register_effect(
        "ATK",
        Arc::new(RedirectRecordingErr {
            target: AttackTarget::Digimon(redir),
            slot: slot.clone(),
        }),
    );
    r.register_effect(
        "WITNESS",
        Arc::new(WitnessTargetChange(witness_log.clone())),
    );
    let _witness = r.place_on_field(0, "WITNESS", Some(0));

    let _result = r.attack_digimon(atk, def, false);

    let recorded = slot.lock().unwrap().take().expect("redirect result");
    assert_eq!(recorded, Err(AttackError::InvalidTarget));
    assert!(
        witness_log.lock().unwrap().is_empty(),
        "rejected redirect must not fire OnAttackTargetChange"
    );
}

/// Test 4: `ctx.redirect_attack` called when no attack is in flight
/// returns `AttackError::NoActiveAttack`. Exercised by invoking from an
/// `OnPlay` closure (play-from-hand, no attack).
#[test]
fn ctx_redirect_attack_invalid_without_active_attack() {
    let slot: Arc<Mutex<Option<Result<(), AttackError>>>> = Arc::new(Mutex::new(None));

    // Synthesize a target handle — doesn't need to resolve to anything
    // for the NoActiveAttack check to fire first.
    let dummy_target = PermanentHandle {
        player: 1,
        index: 0,
    };

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .start();
    r.register_effect(
        "ATK",
        Arc::new(RedirectFromOnPlay {
            target: AttackTarget::Digimon(dummy_target),
            slot: slot.clone(),
        }),
    );
    // Place ATK on field (this does NOT fire OnPlay — place_on_field
    // bypasses the play path). Manually fire OnPlay afterwards to drive
    // the closure.
    let _atk = r.place_on_field(0, "ATK", Some(0));
    let _def = r.place_on_field(1, "DEF", Some(0));
    r.fire_on_play(0, 0);

    let recorded = slot.lock().unwrap().clone();
    assert_eq!(
        recorded,
        Some(Err(AttackError::NoActiveAttack)),
        "redirect_attack outside an active attack must return NoActiveAttack (got {:?})",
        recorded
    );
}

/// Test 5: Memory-rollback semantics via early cancel.
///
/// ATK has a `WhenWouldAttack` mandatory cancel AND an `OnAttack` memory-gain
/// closure. `WhenWouldAttack` fires BEFORE `OnAttack` (spec §4.1 — the
/// replacement runs during declaration, before any observable state change).
/// A cancel at that point means `OnAttack` never fires → the memory swing
/// never happens → memory ends at the pre-declaration value.
#[test]
fn ctx_cancel_attack_memory_rollback() {
    /// A combined card that BOTH installs a mandatory `WhenWouldAttack`
    /// cancel AND grants +2 memory on attack. If the cancel fires first,
    /// OnAttack never runs → memory unchanged.
    struct EarlyCancelPlusMemoryGain;
    impl CardEffect for EarlyCancelPlusMemoryGain {
        fn effects(&self, c: CardHandle) -> Vec<Effect> {
            vec![
                EffectBuilder::new(c, EffectTiming::WhenWouldAttack)
                    .name("Cancel WhenWouldAttack")
                    .replacement_process(|rctx| rctx.cancel())
                    .build(),
                Effect::on_attack(c)
                    .name("Grant +2 memory OnAttack")
                    .process(move |ctx| {
                        ctx.gain_memory(2);
                    })
                    .build(),
            ]
        }
    }
    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .start();
    r.register_effect("ATK", Arc::new(EarlyCancelPlusMemoryGain));
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    let mem_before = r.memory();
    let result = r.attack_digimon(atk, def, false);

    assert_eq!(
        result,
        AttackResult::Cancelled,
        "WhenWouldAttack cancel aborts the declaration"
    );
    assert_eq!(
        r.memory(),
        mem_before,
        "OnAttack memory gain must NOT apply when WhenWouldAttack cancels \
         before observers fire (memory stays at pre-declaration value)"
    );
}

#[test]
fn ctx_cancel_attack_rejected_after_counter_window_opens() {
    let slot: Arc<Mutex<Option<Result<(), AttackError>>>> = Arc::new(Mutex::new(None));

    let mut r = DebugRunner::builder()
        .add_card(card_with_dp("ATK", 5000))
        .add_card(card_with_dp("COUNTER_DEF", 2000))
        .start();
    r.register_effect(
        "COUNTER_DEF",
        Arc::new(CancelFromCounter { slot: slot.clone() }),
    );

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "COUNTER_DEF", Some(0));

    let result = r.attack_digimon(atk, def, false);
    assert_eq!(result, AttackResult::InProgress);

    r.game
        .resolve_selection(1, encode_attack(0, def.index as u16))
        .expect("defender resolves field Counter ability");

    let recorded = slot.lock().unwrap().take().expect("counter cancel result");
    assert_eq!(
        recorded,
        Err(AttackError::InvalidPhase),
        "Counter-window cancellation must be rejected"
    );
    assert!(
        r.game.pending_attack.is_none(),
        "attack should continue to normal cleanup after rejected cancel"
    );
    assert_eq!(
        r.battle_area_size(1),
        0,
        "defender should be deleted by normal battle when late cancel is rejected"
    );
    assert_eq!(
        r.battle_area_size(0),
        1,
        "attacker survives the normal battle"
    );
}
