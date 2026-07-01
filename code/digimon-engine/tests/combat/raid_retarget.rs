//! Phase 9 Task 4 — Raid target-switch rider + PostBlock state.
//!
//! Between the Block window closing and Battle resolution, if the
//! `effective_target` has become invalid (e.g. deleted by an OnAttack
//! side effect) AND the attacker has `<Raid>` AND at least one legal
//! retarget exists on the opposing field, the state machine pauses in
//! `AttackState::PostBlock` with a selection prompting the attacker's
//! controller to pick a new target. The retarget mirrors Raid's
//! mask-layer priority: unsuspended Digimon first (ties broken by max
//! DP), then any legal opponent Digimon if no unsuspended exist.
//!
//! Without Raid, or with Raid + no retarget candidates, the attack
//! short-circuits to `Cleanup` (attack fizzles).
//!
//! Spec: docs/superpowers/specs/2026-04-21-combat-interrupt-completion-design.md
//! §4.2 + §9.1. Plan: Task 4 (lines 412-473).

use std::sync::{Arc, Mutex};

use digimon_engine::action::space::encode_attack;
use digimon_engine::card_source::CardHandle;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{Expiry, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{AttackTarget, SelectionKind};
use digimon_engine::AttackTargetChangeReason;

fn card(id: &str) -> digimon_engine::CardData {
    make_test_card(id, id)
}

// ─── Effect scaffolds ──────────────────────────────────────────────────

/// OnAttack closure on the attacker that deletes a specific target handle.
/// Used to synthesize "original target leaves during the attack" for the
/// PostBlock check. The victim handle is captured at construction time.
struct DeleteOnAttack(PermanentHandle);
impl CardEffect for DeleteOnAttack {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let victim = self.0;
        vec![Effect::on_attack(c)
            .name("Delete victim OnAttack")
            .process(move |ctx| {
                ctx.delete_permanent(victim);
            })
            .build()]
    }
}

/// Sentinel `OnAttackTargetChange` observer that records each fire's
/// post-change `effective_target` into a shared log.
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
    old_target: AttackTarget,
    new_target: AttackTarget,
    reason: AttackTargetChangeReason,
}

struct WitnessTargetChangePayload(Arc<Mutex<Vec<TargetChangeSnapshot>>>);
impl CardEffect for WitnessTargetChangePayload {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let log = self.0.clone();
        vec![Effect::on_attack_target_change(c)
            .name("Witness raid target change payload")
            .process(move |ctx| {
                if let Some(change) = ctx.attack_target_change() {
                    log.lock().unwrap().push(TargetChangeSnapshot {
                        old_target: change.old_target,
                        new_target: change.new_target,
                        reason: change.reason,
                    });
                }
            })
            .build()]
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────

/// Test 1: Raid attacker retargets when the original target leaves mid-attack.
///
/// Setup: two opposing Digimon — DEF at slot 0, VICTIM at slot 1. Attacker
/// declares an attack against VICTIM (slot 1); attacker's OnAttack deletes
/// VICTIM. After deletion only DEF remains at slot 0, so VICTIM's handle
/// (index=1) goes out-of-range → truly invalid.
///
/// At PostBlock, effective_target is invalid AND a legal retarget exists
/// (DEF at slot 0). Assert: attack paused on an opposing-field selection
/// whose candidates include DEF.
#[test]
fn raid_opens_optional_target_switch_when_attacking_player() {
    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("LOW"))
        .add_card(card("HIGH"))
        .add_card(card("SEC"))
        .security(1, &["SEC"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let _low = r.place_on_field(1, "LOW", Some(0));
    let high = r.place_on_field(1, "HIGH", Some(0));
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Raid, Expiry::Permanent, 0);
    r.game.modifiers.add(
        high,
        ModifierEntry::simple(ModifierType::ChangeDp, 3000, Expiry::Permanent, 1),
    );

    let result = r.attack_player(atk, 1, false);
    assert_eq!(result, AttackResult::InProgress);
    let prompt = r
        .pending_selection()
        .expect("Raid should offer an optional target switch before security");
    assert_eq!(prompt.kind, SelectionKind::OppField);
    assert_eq!(prompt.selecting_player, 0);
    assert!(prompt.is_optional, "Raid target switch is printed as 'may'");
    assert_eq!(prompt.valid_action_ids, vec![encode_attack(0, 1)]);
}

#[test]
fn raid_switch_prompt_clones_faithfully() {
    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("LOW"))
        .add_card(card("HIGH"))
        .add_card(card("SEC"))
        .security(1, &["SEC"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let _low = r.place_on_field(1, "LOW", Some(0));
    let high = r.place_on_field(1, "HIGH", Some(0));
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Raid, Expiry::Permanent, 0);
    r.game.modifiers.add(
        high,
        ModifierEntry::simple(ModifierType::ChangeDp, 3000, Expiry::Permanent, 1),
    );
    let security_before = r.game.players[1].security.len();
    let switch_action = encode_attack(0, high.index as u16);

    let result = r.attack_player(atk, 1, false);
    assert_eq!(result, AttackResult::InProgress);
    assert!(
        r.game.pending_selection_resume.is_some(),
        "Raid switch prompt must park a data frame for clone-faithful prompts"
    );

    let mut clone = r.game.clone();
    clone
        .resolve_selection(0, switch_action)
        .expect("clone resolves Raid switch");
    assert!(
        clone.pending_attack.is_none(),
        "clone should continue the switched attack to completion"
    );
    assert_eq!(
        clone.players[1].security.len(),
        security_before,
        "clone should switch to the Digimon target instead of checking security"
    );

    assert!(
        r.game.pending_selection.is_some(),
        "resolving the clone must leave the original at the Raid switch prompt"
    );
    assert_eq!(
        r.game.players[1].security.len(),
        security_before,
        "resolving the clone must not mutate original security"
    );

    r.game
        .resolve_selection(0, switch_action)
        .expect("original resolves Raid switch");
    assert_eq!(
        r.game.players[1].security.len(),
        clone.players[1].security.len(),
        "original and clone replay the Raid switch identically"
    );
}

#[test]
fn raid_attacker_retargets_when_original_target_leaves() {
    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .add_card(card("VICTIM"))
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let _def = r.place_on_field(1, "DEF", Some(0));
    let victim = r.place_on_field(1, "VICTIM", Some(0));
    assert_eq!(
        victim,
        PermanentHandle {
            player: 1,
            index: 1
        }
    );

    // Attacker has Raid; OnAttack deletes the declared target (VICTIM).
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Raid, Expiry::Permanent, 0);
    r.register_effect("ATK", Arc::new(DeleteOnAttack(victim)));

    let result = r.attack_digimon(atk, victim, false);
    assert_eq!(
        result,
        AttackResult::InProgress,
        "Raid retarget installs a PendingSelection; attack pauses"
    );

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("Raid target switch installs a selection");
    assert_eq!(
        sel.selecting_player, 0,
        "attacker's controller picks the new target"
    );
    assert_eq!(sel.kind, SelectionKind::OppField);
    assert!(sel.is_optional, "Raid target switch is printed as 'may'");
    // DEF is the only surviving opposing Digimon — at slot 0.
    // Candidates are encoded as encode_attack(0, target_index).
    assert_eq!(
        sel.valid_action_ids,
        vec![encode_attack(0, 0)],
        "only the surviving opposing Digimon (DEF @ slot 0) is a legal retarget"
    );
}

#[test]
fn raid_retarget_prompt_clones_faithfully() {
    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .add_card(card("VICTIM"))
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let _def = r.place_on_field(1, "DEF", Some(0));
    let victim = r.place_on_field(1, "VICTIM", Some(0));
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Raid, Expiry::Permanent, 0);
    r.register_effect("ATK", Arc::new(DeleteOnAttack(victim)));

    let result = r.attack_digimon(atk, victim, false);
    assert_eq!(result, AttackResult::InProgress);
    assert!(
        r.game.pending_selection_resume.is_some(),
        "Raid retarget prompt must park a data frame for clone-faithful prompts"
    );
    let retarget_action = encode_attack(0, 0);

    let mut clone = r.game.clone();
    clone
        .resolve_selection(0, retarget_action)
        .expect("clone resolves Raid retarget");
    assert!(
        clone.pending_attack.is_none(),
        "clone should continue the retargeted attack to completion"
    );

    assert!(
        r.game.pending_selection.is_some(),
        "resolving the clone must leave the original at the Raid retarget prompt"
    );
    assert_eq!(
        r.game.players[1].battle_area.len(),
        1,
        "resolving the clone must not mutate the original surviving defender"
    );

    r.game
        .resolve_selection(0, retarget_action)
        .expect("original resolves Raid retarget");
    assert_eq!(
        r.game.players[1].battle_area.len(),
        clone.players[1].battle_area.len(),
        "original and clone replay the Raid retarget identically"
    );
}

/// Test 2: Raid attacker fizzles when no retarget candidates exist.
///
/// Attacker has `<Raid>`; OnAttack deletes the only defender; the opposing
/// field is now empty. PostBlock sees invalid target + no candidates and
/// transitions straight to Cleanup.
#[test]
fn raid_retarget_fizzle_when_no_candidates() {
    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Raid, Expiry::Permanent, 0);
    r.register_effect("ATK", Arc::new(DeleteOnAttack(def)));

    let result = r.attack_digimon(atk, def, false);
    // No retarget candidates → attack fizzles. Cleanup runs terminally;
    // the attacker never resolved a battle, so the result reflects the
    // trivially-cleaned-up state. We accept either SecurityCheckSurvived
    // (no security reveal ran) or AttackerWins / Invalid depending on
    // Cleanup routing — the load-bearing guarantees are:
    //   * no pending selection installed
    //   * pending_attack cleaned up
    assert!(
        r.game.pending_selection.is_none(),
        "no retarget selection installed when no candidates"
    );
    assert!(r.game.pending_attack.is_none(), "attack cleaned up");
    // Result must not be InProgress — the attack terminated this tick.
    assert_ne!(
        result,
        AttackResult::InProgress,
        "attack must not leave state machine mid-flight"
    );
}

/// Test 3: A non-Raid attacker does NOT retarget; the attack resolves
/// against the (now-invalid) target, producing the existing "trivial
/// connect" semantics (AttackerWins per `resolve_pending_battle`).
///
/// Setup mirrors Test 1 (DEF @ slot 0, VICTIM @ slot 1) but WITHOUT Raid.
#[test]
fn non_raid_attacker_fizzles_without_retarget() {
    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .add_card(card("VICTIM"))
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let _def = r.place_on_field(1, "DEF", Some(0));
    let victim = r.place_on_field(1, "VICTIM", Some(0));
    assert_eq!(
        victim,
        PermanentHandle {
            player: 1,
            index: 1
        }
    );

    // NO Raid keyword on attacker. OnAttack still deletes the declared target.
    r.register_effect("ATK", Arc::new(DeleteOnAttack(victim)));

    let _result = r.attack_digimon(atk, victim, false);

    // No PostBlock retarget — no selection installed; attack ran through
    // to Battle (invalid target → trivial AttackerWins) then Cleanup.
    assert!(
        r.game.pending_selection.is_none(),
        "non-Raid attacker must NOT install a retarget selection"
    );
    assert!(r.game.pending_attack.is_none(), "attack cleaned up");
    // DEF survives — never targeted and no retarget fired.
    assert_eq!(
        r.battle_area_size(1),
        1,
        "DEF survives; only VICTIM was removed"
    );
}

/// Test 4: Raid retarget fires `OnAttackTargetChange` on resolution.
///
/// After the retarget selection is resolved with the new target, the
/// global `OnAttackTargetChange` observer fan-out fires exactly as for
/// block-redirect and `ctx.redirect_attack` (shared entry point:
/// `apply_attack_target_substitution`).
#[test]
fn raid_retarget_fires_on_attack_target_change() {
    let witness_log: Arc<Mutex<Vec<AttackTarget>>> = Arc::new(Mutex::new(Vec::new()));

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .add_card(card("VICTIM"))
        .add_card(card("WITNESS"))
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let _def = r.place_on_field(1, "DEF", Some(0));
    let victim = r.place_on_field(1, "VICTIM", Some(0));
    assert_eq!(
        victim,
        PermanentHandle {
            player: 1,
            index: 1
        }
    );

    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Raid, Expiry::Permanent, 0);
    r.register_effect("ATK", Arc::new(DeleteOnAttack(victim)));

    // Install the witness AFTER the deletion will fire — plant it on P0's
    // side (observer fan-out covers both players).
    r.register_effect(
        "WITNESS",
        Arc::new(WitnessTargetChange(witness_log.clone())),
    );
    let _witness = r.place_on_field(0, "WITNESS", Some(0));

    let _result = r.attack_digimon(atk, victim, false);

    // Retarget selection is pending — resolve it by picking DEF (slot 0).
    let retarget_action = encode_attack(0, 0);
    r.game
        .resolve_selection(0, retarget_action)
        .expect("attacker's controller resolves Raid retarget");

    let def_after_retarget = PermanentHandle {
        player: 1,
        index: 0,
    };
    let log = witness_log.lock().unwrap().clone();
    assert!(
        log.iter()
            .any(|t| matches!(t, AttackTarget::Digimon(h) if *h == def_after_retarget)),
        "OnAttackTargetChange must fire with new target == DEF @ slot 0 (saw {:?})",
        log
    );
}

#[test]
fn raid_retarget_payload_carries_old_new_and_raid_reason() {
    let payload_log: Arc<Mutex<Vec<TargetChangeSnapshot>>> = Arc::new(Mutex::new(Vec::new()));

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .add_card(card("VICTIM"))
        .add_card(card("WITNESS"))
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let _def = r.place_on_field(1, "DEF", Some(0));
    let victim = r.place_on_field(1, "VICTIM", Some(0));

    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Raid, Expiry::Permanent, 0);
    r.register_effect("ATK", Arc::new(DeleteOnAttack(victim)));
    r.register_effect(
        "WITNESS",
        Arc::new(WitnessTargetChangePayload(payload_log.clone())),
    );
    let _witness = r.place_on_field(0, "WITNESS", Some(0));

    let _result = r.attack_digimon(atk, victim, false);

    let retarget_action = encode_attack(0, 0);
    r.game
        .resolve_selection(0, retarget_action)
        .expect("attacker's controller resolves Raid retarget");

    let def_after_retarget = PermanentHandle {
        player: 1,
        index: 0,
    };
    assert_eq!(
        payload_log.lock().unwrap().as_slice(),
        &[TargetChangeSnapshot {
            old_target: AttackTarget::Digimon(victim),
            new_target: AttackTarget::Digimon(def_after_retarget),
            reason: AttackTargetChangeReason::Raid,
        }]
    );
}

#[test]
fn raid_retarget_respects_attacker_cannot_switch_target() {
    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .add_card(card("VICTIM"))
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let _def = r.place_on_field(1, "DEF", Some(0));
    let victim = r.place_on_field(1, "VICTIM", Some(0));

    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Raid, Expiry::Permanent, 0);
    r.game.modifiers.add(
        atk,
        ModifierEntry::simple(
            ModifierType::CanNotSwitchAttackTarget,
            0,
            Expiry::EndOfTurn,
            0,
        ),
    );
    r.register_effect("ATK", Arc::new(DeleteOnAttack(victim)));

    let result = r.attack_digimon(atk, victim, false);

    assert_ne!(
        result,
        AttackResult::InProgress,
        "Raid must fizzle instead of offering a forbidden target switch"
    );
    assert!(r.game.pending_selection.is_none());
    assert!(r.game.pending_attack.is_none());
}

#[test]
fn raid_retarget_respects_cannot_be_redirected_candidate() {
    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("FIXED"))
        .add_card(card("VICTIM"))
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let fixed = r.place_on_field(1, "FIXED", Some(0));
    let victim = r.place_on_field(1, "VICTIM", Some(0));

    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Raid, Expiry::Permanent, 0);
    r.game.modifiers.add(
        fixed,
        ModifierEntry::simple(
            ModifierType::CannotBeRedirectedAsAttackTarget,
            0,
            Expiry::EndOfTurn,
            1,
        ),
    );
    r.register_effect("ATK", Arc::new(DeleteOnAttack(victim)));

    let result = r.attack_digimon(atk, victim, false);

    assert_ne!(
        result,
        AttackResult::InProgress,
        "Raid must not offer a fixed-target Digimon as a retarget candidate"
    );
    assert!(r.game.pending_selection.is_none());
    assert!(r.game.pending_attack.is_none());
}
