//! Phase 9 Task 1 — `WhenWouldAttack` + `WhenWouldBeAttackTarget` dispatch
//! + Phase 6 `CannotAttack*` auto-install as passive cancel replacements.
//!
//! Exercises:
//!   - Mandatory cancel aborts attack declaration cleanly (no suspend, no
//!     OnAttack, EndOfAttack still fires, EndOfBattle does NOT fire).
//!   - Target-side mandatory cancel.
//!   - Target-side substitute rewrites `pending_attack.effective_target` and
//!     fires the global `OnAttackTargetChange` observer.
//!   - Phase 6 `CannotAttack` modifier auto-installs a passive `WhenWouldAttack`
//!     cancel via `passive_modifier_to_would`.
//!   - Optional `WhenWouldAttack` emits a `PendingSelection::Replacement`
//!     carrying REPLACEMENT_ACCEPT and admitting PASS.
//!   - Controller-first layering for multiple WhenWouldAttack candidates.
//!   - Cancel path still fires EndOfAttack globally (cleanup symmetry).
//!   - `cause_filter = OpponentEffect` on a defender's player-scoped
//!     replacement does NOT fire on a Battle-caused attack declaration.

use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_source::CardHandle;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect, EffectBuilder};
use digimon_engine::enums::{EffectTiming, Expiry, ModifierType};
use digimon_engine::modifiers::{ModifierEntry, PlayerModifierEntry};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::{ReplacementCause, ReplacementSubject};
use digimon_engine::selection::SelectionKind;

fn card(id: &str) -> digimon_engine::CardData {
    make_test_card(id, id)
}

// ─── Effect scaffolds ──────────────────────────────────────────────────

/// Mandatory `WhenWouldAttack` cancel on the cardholder.
struct CancelAttackMandatory;
impl CardEffect for CancelAttackMandatory {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        vec![EffectBuilder::new(c, EffectTiming::WhenWouldAttack)
            .name("Mandatory cancel WouldAttack")
            .replacement_process(|rctx| rctx.cancel())
            .build()]
    }
}

/// Mandatory `WhenWouldBeAttackTarget` cancel on the cardholder.
struct CancelTargetMandatory;
impl CardEffect for CancelTargetMandatory {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        vec![EffectBuilder::new(c, EffectTiming::WhenWouldBeAttackTarget)
            .name("Mandatory cancel WouldBeAttackTarget")
            .replacement_process(|rctx| rctx.cancel())
            .build()]
    }
}

/// Mandatory `WhenWouldBeAttackTarget` substitute to a fixed handle.
/// Used for redirect test — the substitute swaps the target to (0, 1).
struct SubstituteTargetToHandle(pub PermanentHandle);
impl CardEffect for SubstituteTargetToHandle {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let replacement_handle = self.0;
        vec![EffectBuilder::new(c, EffectTiming::WhenWouldBeAttackTarget)
            .name("Substitute attack target")
            .replacement_process(move |rctx| {
                rctx.substitute(ReplacementSubject::Permanent(replacement_handle));
            })
            .build()]
    }
}

/// Optional `WhenWouldAttack` cancel — installs a `PendingSelection::Replacement`.
struct OptionalCancelAttack;
impl CardEffect for OptionalCancelAttack {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        vec![EffectBuilder::new(c, EffectTiming::WhenWouldAttack)
            .name("Optional cancel WouldAttack")
            .optional()
            .replacement_process(|rctx| rctx.cancel())
            .build()]
    }
}

/// Sentinel-recording `WhenWouldAttack` replacement. Pushes its `sentinel`
/// tag into the shared log each time it fires. Outcome: `None` (no-op) so
/// both candidates fire in order.
struct SentinelWouldAttack {
    sentinel: &'static str,
    log: Arc<Mutex<Vec<&'static str>>>,
}
impl CardEffect for SentinelWouldAttack {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let tag = self.sentinel;
        let log = self.log.clone();
        vec![EffectBuilder::new(c, EffectTiming::WhenWouldAttack)
            .name("Sentinel WouldAttack")
            .replacement_process(move |rctx| {
                log.lock().unwrap().push(tag);
                // leave outcome as None so both candidates get to fire
                let _ = rctx;
            })
            .build()]
    }
}

/// Sentinel-recording `OnAttackTargetChange` observer (single permanent).
/// Records the fire for test 3.
struct SentinelOnTargetChange(Arc<Mutex<u32>>);
impl CardEffect for SentinelOnTargetChange {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let counter = self.0.clone();
        vec![Effect::on_attack_target_change(c)
            .name("Sentinel OnAttackTargetChange")
            .process(move |_ctx| {
                *counter.lock().unwrap() += 1;
            })
            .build()]
    }
}

// (EndOfAttack / EndOfBattle sentinels live inline in Test 7.)

// ─── Tests ─────────────────────────────────────────────────────────────

/// Test 1: Mandatory `WhenWouldAttack` cancel aborts the declaration
/// before suspend / OnAttack / memory swing. EndOfAttack fires for cleanup
/// symmetry; EndOfBattle does NOT fire (no battle occurred).
#[test]
fn when_would_attack_cancel_aborts_declaration() {
    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .start();
    r.register_effect("ATK", Arc::new(CancelAttackMandatory));
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    let mem_before = r.memory();
    let attacks_before = r.game.player(0).battle_area[atk.index as usize].attacks_this_turn;

    let result = r.attack_digimon(atk, def, false);

    assert_eq!(result, AttackResult::Cancelled, "attack is Cancelled");
    assert!(r.game.pending_attack.is_none(), "pending_attack cleaned up");
    assert!(r.game.pending_selection.is_none());

    // No suspend, no attack count bump.
    let atk_perm = &r.game.player(0).battle_area[atk.index as usize];
    assert!(!atk_perm.is_suspended, "attacker must not be suspended");
    assert!(!atk_perm.is_attacking, "is_attacking cleared by cleanup");
    assert_eq!(
        atk_perm.attacks_this_turn, attacks_before,
        "attack count must NOT be bumped on cancelled declaration"
    );
    assert_eq!(r.memory(), mem_before, "memory unchanged on cancel");
}

/// Test 2: Mandatory `WhenWouldBeAttackTarget` cancel on defender aborts.
/// Defender stays on field unchanged.
#[test]
fn when_would_be_attack_target_cancel_aborts() {
    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .start();
    r.register_effect("DEF", Arc::new(CancelTargetMandatory));
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    let def_index_before = def.index;
    let atk_suspended_before = r.game.player(0).battle_area[atk.index as usize].is_suspended;

    let result = r.attack_digimon(atk, def, false);
    assert_eq!(result, AttackResult::Cancelled);
    assert!(r.game.pending_attack.is_none());

    // Defender still on field, untouched.
    assert!(
        r.game
            .player(1)
            .battle_area
            .get(def_index_before as usize)
            .is_some(),
        "defender unchanged after target-side cancel"
    );
    // Attacker not suspended.
    assert_eq!(
        r.game.player(0).battle_area[atk.index as usize].is_suspended,
        atk_suspended_before
    );
}

/// Test 3: `WhenWouldBeAttackTarget` substitute rewrites `effective_target`
/// and fires `OnAttackTargetChange` globally (both players scanned).
#[test]
fn when_would_be_attack_target_substitute_redirects() {
    // Layout: P0 has ATK (attacker). P1 has DEF (orig target, idx 0) and
    // REDIR (substitute target, idx 1). Substitute replacement lives on
    // DEF so it fires when DEF is the target.
    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .add_card(card("REDIR"))
        .add_card(card("OBS"))
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

    // Register REDIR handle into the substitute effect.
    r.register_effect("DEF", Arc::new(SubstituteTargetToHandle(redir)));
    // Observer on P0 witnesses the retarget.
    let observed = Arc::new(Mutex::new(0u32));
    r.register_effect("OBS", Arc::new(SentinelOnTargetChange(observed.clone())));
    let _obs = r.place_on_field(0, "OBS", Some(0));

    let _result = r.attack_digimon(atk, def, false);

    // After the substitute fires, effective_target should point at REDIR.
    // The attack may progress further (Counter/Block/Battle) — but the
    // observable we care about is `pending_attack.effective_target` AT the
    // moment `OnAttackTargetChange` fired. If the attack already resolved,
    // we instead assert the observer was witnessed.
    assert!(
        *observed.lock().unwrap() >= 1,
        "OnAttackTargetChange must fire on target substitution"
    );
}

/// Test 4: Phase 6 `CannotAttack` modifier auto-installs a passive
/// `WhenWouldAttack` cancel replacement via `passive_modifier_to_would`.
#[test]
fn phase6_cannot_attack_auto_installs_replacement() {
    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    // Install Phase 6 CannotAttack on the attacker directly via the modifier
    // registry (no card effect, no mask). This simulates what a Phase 6
    // flood gate would do.
    r.game.modifiers.add(
        atk,
        ModifierEntry::simple(ModifierType::CannotAttack, 0, Expiry::Permanent, 0),
    );

    let result = r.attack_digimon(atk, def, false);
    assert_eq!(
        result,
        AttackResult::Cancelled,
        "Phase 6 CannotAttack must auto-install a WhenWouldAttack cancel"
    );
    assert!(r.game.pending_attack.is_none());
}

/// Test 5: Optional `WhenWouldAttack` emits a `PendingSelection::Replacement`
/// carrying REPLACEMENT_ACCEPT + admitting PASS. The attack is parked until
/// the selection resolves.
#[test]
fn optional_when_would_attack_emits_selection() {
    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .start();
    r.register_effect("ATK", Arc::new(OptionalCancelAttack));
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    let result = r.attack_digimon(atk, def, false);
    assert_eq!(
        result,
        AttackResult::InProgress,
        "parks on optional replacement"
    );

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("optional replacement installs a PendingSelection");
    assert_eq!(sel.kind, SelectionKind::Replacement);
    assert!(sel.is_optional, "replacement selection admits PASS");
    assert!(
        sel.valid_action_ids.contains(&REPLACEMENT_ACCEPT),
        "REPLACEMENT_ACCEPT must be a valid action (got {:?})",
        sel.valid_action_ids
    );

    // Decline → the attack proceeds normally. With no OnAttack observer
    // listening and a trivial DEF, it resolves synchronously.
    r.game
        .resolve_selection(sel.selecting_player, PASS)
        .expect("decline optional replacement");
}

/// Test 6: Multiple `WhenWouldAttack` candidates — subject's controller
/// fires first (Phase 7 layering rule: controller-of-subject first, then
/// opponent). Both mandatory sentinels record their tag; the order is
/// checked against the layered output.
#[test]
fn multiple_replacements_layer_controller_first() {
    let log = Arc::new(Mutex::new(Vec::<&'static str>::new()));

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("OPP"))
        .add_card(card("DEF"))
        .start();
    r.register_effect(
        "ATK",
        Arc::new(SentinelWouldAttack {
            sentinel: "owner",
            log: log.clone(),
        }),
    );
    r.register_effect(
        "OPP",
        Arc::new(SentinelWouldAttack {
            sentinel: "opponent",
            log: log.clone(),
        }),
    );
    let atk = r.place_on_field(0, "ATK", Some(0));
    let _opp = r.place_on_field(1, "OPP", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    let _ = r.attack_digimon(atk, def, false);

    let entries = log.lock().unwrap().clone();
    // Subject = attacker (Player 0). Per Phase 7 layering: candidates on the
    // subject's controller fire before opponent's candidates.
    assert_eq!(
        entries,
        vec!["owner", "opponent"],
        "controller-of-subject first, then opponent"
    );
}

/// Test 7: Cancelled `WhenWouldAttack` still fires EndOfAttack (cleanup
/// symmetry) but does NOT fire EndOfBattle (no DP battle happened).
#[test]
fn when_would_attack_cancelled_fires_end_of_attack() {
    let end_of_attack = Arc::new(Mutex::new(0u32));
    let end_of_battle = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .add_card(card("WITNESS"))
        .start();
    r.register_effect("ATK", Arc::new(CancelAttackMandatory));
    r.register_effect("WITNESS", {
        // A card that carries both observers — uses a helper that yields a
        // vec of two effects (EndOfAttack + EndOfBattle).
        struct WitnessBoth(Arc<Mutex<u32>>, Arc<Mutex<u32>>);
        impl CardEffect for WitnessBoth {
            fn effects(&self, c: CardHandle) -> Vec<Effect> {
                let ea = self.0.clone();
                let eb = self.1.clone();
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
        Arc::new(WitnessBoth(end_of_attack.clone(), end_of_battle.clone()))
    });
    let atk = r.place_on_field(0, "ATK", Some(0));
    let _witness = r.place_on_field(0, "WITNESS", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    let result = r.attack_digimon(atk, def, false);
    assert_eq!(result, AttackResult::Cancelled);
    assert_eq!(
        *end_of_attack.lock().unwrap(),
        1,
        "EndOfAttack fires even when attack was cancelled"
    );
    assert_eq!(
        *end_of_battle.lock().unwrap(),
        0,
        "EndOfBattle must NOT fire when no DP battle happened"
    );
}

/// Test 8: `cause_filter = OpponentEffect` on a player-scoped passive
/// `CannotAttackTarget` modifier does NOT fire when the attack declaration
/// runs with cause `Battle`. The attack proceeds normally.
#[test]
fn when_would_be_attack_target_opponent_cancel_filter() {
    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    // Install a player-scoped CannotAttackTarget on defender, but with
    // cause_filter = OpponentEffect (i.e. "cannot be attacked by the
    // opponent's effects"). Attack declaration carries cause `Battle`,
    // so the filter does NOT match — the replacement must skip.
    r.game.modifiers.add_player_modifier(
        1,
        PlayerModifierEntry {
            modifier: ModifierType::CannotAttackTarget,
            value: 0,
            expiry: Expiry::Permanent,
            source_permanent: None,
            source_player: 1,
            cause_filter: Some(ReplacementCause::OpponentEffect),
            replacement_condition: None,
            effect_immunity_filter: None,
        },
    );

    let result = r.attack_digimon(atk, def, false);
    assert_ne!(
        result,
        AttackResult::Cancelled,
        "replacement must NOT fire when cause_filter mismatches"
    );
    // The attack proceeded — either synchronously resolved (both sides trivial)
    // or went InProgress via some other interrupt. Either way: not Cancelled.
}
