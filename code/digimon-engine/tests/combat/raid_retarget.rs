//! `<Raid>` target switch (§16-23) and a removed attack target (§11-2-6).
//!
//! `<Raid>` is a trigger-type effect that triggers when its Digimon attacks
//! and is optional (§16-23-2 / §16-23-3): its switch is offered in the
//! declaration's trigger pool, and when the attacker's OnAttack deleted the
//! declared target first, the switch can still choose a live Digimon.
//!
//! There is no post-Block retarget "rider": it was removed (2026-10-17,
//! G-ENGINE-REMOVED-ATTACK-TARGET-SLOT-REUSED). A removed target that <Raid>
//! did not switch away from remains the attack target and the attack fails
//! (§11-2-6, §11-5-1-4-1) -- with or without <Raid>, with or without other
//! candidates.

use std::sync::{Arc, Mutex};

use digimon_engine::action::space::encode_attack;
use digimon_engine::card_source::CardHandle;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{EffectTiming, Expiry, Keyword, ModifierType};
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

/// §16-23-2: `<Raid>` is a trigger-type effect, so the attacker's `OnAttack`
/// (which deletes the declared target here) and `<Raid>` trigger together on
/// the declaration and pend in one `TriggerOrder` (§15-4-3-2). Resolve the
/// `OnAttack` first, so `<Raid>`'s switch is offered with the declared target
/// already gone — the order these tests exercise.
fn resolve_on_attack_first(r: &mut DebugRunner) {
    let sel = r.game.pending_selection.as_ref().expect("the declaration's TriggerOrder");
    assert_eq!(sel.kind, SelectionKind::TriggerOrder, "{sel:?}");
    let action = sel
        .effect_choices
        .as_ref()
        .expect("TriggerOrder branches")
        .iter()
        .find(|c| c.timing == Some(EffectTiming::OnAttack))
        .expect("the OnAttack branch")
        .action_id;
    r.game
        .resolve_selection(0, action)
        .expect("resolve the OnAttack first");
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
        "OnAttack and <Raid> pend together; attack pauses"
    );
    resolve_on_attack_first(&mut r);

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
    resolve_on_attack_first(&mut r);
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

/// Test 2: Attacker has `<Raid>`; OnAttack deletes the only defender; the
/// opposing field is now empty, so <Raid> has nothing to switch to.
///
/// §11-2-6 (Ver. 4.3): the removed Digimon remains the attack target and the
/// attack fails -- `AttackResult::Unsuccessful` (§11-5-1-4-1). Inverted
/// 2026-10-17: this used to pin the post-Block retarget rider's "fizzle"
/// (`Cancelled`); the rider is removed (§16-23-2: <Raid> triggers only when
/// the Digimon attacks).
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
    assert_eq!(
        result,
        AttackResult::Unsuccessful,
        "§11-2-6: the removed target stays the target; the attack fails"
    );
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

/// Test 3: A non-Raid attacker does NOT retarget; the removed target stays
/// the target and the attack fails (§11-2-6): DEF is not battled.
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

    // No retarget — no selection installed; Battle ends the attack as not
    // successful (§11-2-6, §11-5-1-4-1).
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
    resolve_on_attack_first(&mut r);

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
    resolve_on_attack_first(&mut r);

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

/// `[When Attacking]` sibling that logs the attack target it sees.
struct LogWhenAttacking(Arc<Mutex<Vec<String>>>);
impl CardEffect for LogWhenAttacking {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let log = self.0.clone();
        vec![Effect::when_attacking(c)
            .name("Log [When Attacking]")
            .process(move |ctx| {
                let target = ctx.game.pending_attack.as_ref().map(|pa| pa.effective_target);
                log.lock().unwrap().push(format!("sibling {target:?}"));
            })
            .build()]
    }
}

/// `OnAttackTargetChange` observer that logs its activation.
struct LogTargetChange(Arc<Mutex<Vec<String>>>);
impl CardEffect for LogTargetChange {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let log = self.0.clone();
        vec![Effect::on_attack_target_change(c)
            .name("Log target change")
            .process(move |_ctx| log.lock().unwrap().push("target change".to_string()))
            .build()]
    }
}

/// §15-8-3-2 / §15-4-5-2: `<Raid>` ordered first from the declaration pool,
/// ahead of a sibling `[When Attacking]`. Its switch triggers an
/// `OnAttackTargetChange` observer. Nothing else resolves while `<Raid>`'s
/// body (the switch pick) is resolving; once it has, the DERIVED observer
/// activates before the still-pending sibling, which then sees the switched
/// target.
#[test]
fn raid_first_derived_target_change_observer_precedes_pending_sibling() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("LOW"))
        .add_card(card("HIGH"))
        .add_card(card("WITNESS"))
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
    r.register_effect("ATK", Arc::new(LogWhenAttacking(log.clone())));
    r.register_effect("WITNESS", Arc::new(LogTargetChange(log.clone())));
    let _witness = r.place_on_field(0, "WITNESS", Some(0));

    assert_eq!(r.attack_player(atk, 1, false), AttackResult::InProgress);
    let order = r.game.pending_selection.as_ref().expect("the declaration's TriggerOrder");
    assert_eq!(order.kind, SelectionKind::TriggerOrder, "{order:?}");
    let raid = order
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .find(|c| c.keyword == Some(Keyword::Raid))
        .expect("the <Raid> branch")
        .action_id;
    r.game.resolve_selection(0, raid).expect("<Raid> first");

    let switch = r.game.pending_selection.as_ref().expect("<Raid>'s switch pick");
    assert_eq!(switch.kind, SelectionKind::OppField);
    assert!(log.lock().unwrap().is_empty(), "nothing resolved yet");
    r.game
        .resolve_selection(0, encode_attack(0, high.index as u16))
        .expect("switch to HIGH");

    assert!(
        r.game
            .pending_selection
            .as_ref()
            .is_none_or(|s| s.kind != SelectionKind::TriggerOrder),
        "the derived observer is not pooled with the pending sibling"
    );
    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "target change".to_string(),
            format!("sibling {:?}", Some(AttackTarget::Digimon(high))),
        ],
        "the derived observer resolves first; the sibling then sees the switched target"
    );
}

/// Two pooled `[When Attacking]` effects: slot 0 redirects the attack to a
/// fixed Digimon (`ctx.redirect_attack`) and then logs "tail"; slot 1 logs
/// "sibling".
struct RedirectThenTailAndSibling {
    new_target: PermanentHandle,
    log: Arc<Mutex<Vec<String>>>,
}
impl CardEffect for RedirectThenTailAndSibling {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let new_target = self.new_target;
        let tail_log = self.log.clone();
        let sibling_log = self.log.clone();
        vec![
            Effect::when_attacking(c)
                .name("Redirect, then tail")
                .process(move |ctx| {
                    ctx.redirect_attack(AttackTarget::Digimon(new_target))
                        .expect("legal redirect");
                    tail_log.lock().unwrap().push("tail".to_string());
                })
                .build(),
            Effect::when_attacking(c)
                .name("Sibling")
                .process(move |_ctx| sibling_log.lock().unwrap().push("sibling".to_string()))
                .build(),
        ]
    }
}

/// §15-8-3-2 / §15-4-5-2: a `[When Attacking]` effect that switches the
/// attack target and then does more ("tail") finishes before the
/// `OnAttackTargetChange` observer its switch triggered activates; that
/// derived observer then activates before the still-pending sibling
/// `[When Attacking]`. An inline drain at the switch would run the observer
/// and the sibling before the tail.
#[test]
fn redirect_effect_finishes_before_derived_observer_then_pending_sibling() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF-A"))
        .add_card(card("DEF-B"))
        .add_card(card("WITNESS"))
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def_a = r.place_on_field(1, "DEF-A", Some(0));
    let def_b = r.place_on_field(1, "DEF-B", Some(0));
    r.register_effect(
        "ATK",
        Arc::new(RedirectThenTailAndSibling {
            new_target: def_b,
            log: log.clone(),
        }),
    );
    r.register_effect("WITNESS", Arc::new(LogTargetChange(log.clone())));
    let _witness = r.place_on_field(0, "WITNESS", Some(0));

    assert_eq!(r.attack_digimon(atk, def_a, false), AttackResult::InProgress);
    let order = r.game.pending_selection.as_ref().expect("the declaration's TriggerOrder");
    assert_eq!(order.kind, SelectionKind::TriggerOrder, "{order:?}");
    let redirect = order
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .find(|c| c.label.starts_with("ATK slot 0"))
        .expect("the redirecting [When Attacking]")
        .action_id;
    r.game.resolve_selection(0, redirect).expect("redirect first");

    assert_eq!(
        *log.lock().unwrap(),
        vec!["tail".to_string(), "target change".to_string(), "sibling".to_string()],
    );
}

/// §16-23-3: <Raid>'s processing is optional. The attacker's OnAttack deletes
/// the declared target first; <Raid> then offers DEF and the attacker
/// DECLINES. Nothing else retargets the attack: no second (mandatory) target
/// prompt appears, the removed Digimon stays the attack target and the attack
/// fails (§11-2-6) -- DEF is not battled. Before 2026-10-17 a post-Block
/// "rider" forced a retarget here; it is removed (§16-23-2).
#[test]
fn declined_raid_switch_leaves_the_removed_target_and_the_attack_fails() {
    use digimon_engine::action::space::PASS;
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

    assert_eq!(r.attack_digimon(atk, victim, false), AttackResult::InProgress);
    resolve_on_attack_first(&mut r);
    let sel = r.game.pending_selection.as_ref().expect("the <Raid> switch");
    assert!(sel.is_optional, "§16-23-3: <Raid> is optional");
    assert_eq!(sel.valid_action_ids, vec![encode_attack(0, 0)]);

    r.game.resolve_selection(0, PASS).expect("decline <Raid>");
    assert!(
        r.game.pending_selection.is_none(),
        "no other retarget prompt: {:?}",
        r.game.pending_selection
    );
    assert!(r.game.pending_attack.is_none(), "the attack ended");
    assert_eq!(r.battle_area_size(1), 1, "DEF was not battled");
}
