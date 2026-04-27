//! Phase F Task 3 — `Keyword::Execute` auto-install behavioral tests.
//!
//! `<Execute>` is a Trigger-type keyword printed on certain Digimon
//! (RULES_CONTEXT 16-37 / DCGO `Execute.cs`). At end of the controller's
//! turn the player MAY attack with this Digimon — including unsuspended
//! Digimon — and when the attack ends the carrier is deleted by its own
//! effect. Optional at the trigger; declining (PASS at the may dialog)
//! leaves the carrier intact.
//!
//! Auto-install shape (Phase F §F1):
//!   - `EndOfYourTurn` triggered effect (NOT `.optional()` — see below).
//!     Body unconditionally grants `MayAttack` + `CanAttackUnsuspended`
//!     modifiers on self with `Expiry::EndOfTurn` so the EOT-action
//!     phase parks (via `has_end_of_turn_keywords`) and the §4.6 mask
//!     emits attack bits — including bits against unsuspended
//!     defenders.
//!   - `EndOfAttack` observer gated on `pa.attacker == self`. When the
//!     attack ends the carrier is self-deleted via
//!     `delete_permanent_with_cause(..., ReplacementCause::OwnEffect)`
//!     so the cause is faithfully attributed to the keyword's effect
//!     and the wider engine treats it as such (cf. Retaliation's same
//!     cause-labeling).
//!
//! DCGO behavior reference (`Execute.cs:18-87`):
//!   1. `OnEndYourTurn` activation parks an "accept Execute?" prompt.
//!   2. On accept: `CanAttackTargetDefendingPermanentClass` is added to
//!      `selectedPermanent.UntilEndAttackEffects` → during this attack
//!      window the defender filter `!defender.IsSuspended` lets us hit
//!      unsuspended Digimon.
//!   3. `SelectAttackEffect` runs (player picks a target).
//!   4. `OnEndAttack` observer queues a `DeleteSelfEffect`.
//!
//! The Rust mapping uses modifiers (`MayAttack` +
//! `CanAttackUnsuspended`) instead of DCGO's per-attack defender
//! filter — both produce the same observable outcome via the existing
//! end-of-turn-attack mask emitter (§4.6 in `action/mask.rs`).
//!
//! Where the "may" lives. RULES_CONTEXT 16-37 marks Execute Optional;
//! DCGO surfaces this as an OnEndYourTurn accept dialog. The Rust
//! engine's effect-queue drainer auto-fires single optional triggers
//! WITHOUT a dialog, and nesting an inner may-pick parks AFTER
//! `Game::end_turn`'s `has_end_of_turn_keywords` check has already
//! decided whether to park the EOT phase — so the modifier grant
//! wouldn't yet exist when `end_turn` checks. The Rust port pushes
//! the may decision down to the EOT-action phase itself, where PASS
//! is the standard exit. PASS at EOT-action = decline; the
//! EndOfAttack self-delete observer's `pa.attacker == me` gate keeps
//! the carrier alive when no attack initiates. Modifiers expire
//! `Expiry::EndOfTurn` so they clear cleanly on rotation.
//!
//! Observable parity with DCGO is preserved: accept-and-attack runs
//! identically (modifiers, attack, self-delete); decline produces an
//! identical end state (no attack, no self-delete, no lingering
//! modifiers next turn).

use digimon_engine::action::space::{encode_attack, ATTACK_START, PASS, TARGETS_PER_ATTACKER};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{GamePhase, Keyword, ModifierType};

use super::helpers::plain_digimon;

fn execute_card(id: &str, dp: i32) -> CardData {
    let mut c = plain_digimon(id);
    c.dp = Some(dp);
    c.keywords = vec![Keyword::Execute];
    c
}

// ─── Test 1: attack unsuspended → self-delete ───────────────────────────────

/// P0 has an Execute carrier (`EXEC`). P1 has a plain UNSUSPENDED
/// defender (`DEF`). End P0's turn — the `EndOfYourTurn` body grants
/// `MayAttack` + `CanAttackUnsuspended` on `EXEC` and `end_turn` parks
/// the phase in `EndOfTurnAction`. Attack `DEF` (unsuspended). After
/// the attack resolves, `EXEC` is deleted (cause = OwnEffect) and now
/// lives in P0's trash.
///
/// Optionality note: the printed "may" is satisfied at the
/// `EndOfTurnAction` phase — the player may PASS to decline (see
/// Test 2). DCGO arrives at the same observable outcome via an
/// optional OnEndYourTurn dialog; the Rust port routes through the
/// existing EOT-action PASS exit because the engine's effect-queue
/// drainer auto-fires single optional triggers without a dialog and
/// nesting an inner may-pick parks AFTER `end_turn`'s park check.
#[test]
fn execute_attack_unsuspended_then_self_delete() {
    let mut r = DebugRunner::builder()
        .add_card(execute_card("EXEC", 5000))
        .add_card({
            // Lower-DP defender so EXEC wins the battle and survives the
            // combat resolution — proving the self-delete comes from the
            // EndOfAttack observer, not from losing combat.
            let mut c = plain_digimon("DEF");
            c.dp = Some(2000);
            c
        })
        .start();

    // Place EXEC on P0's field (turn_played=0 so it's eligible to attack
    // immediately via the granted MayAttack), DEF on P1's field
    // unsuspended.
    let exec = r.place_on_field(0, "EXEC", Some(0));
    let _def = r.place_on_field(1, "DEF", Some(0));

    // Sanity: defender starts unsuspended (place_on_field default).
    assert!(
        !r.game.players[1].battle_area[0].is_suspended,
        "DEF should start unsuspended"
    );

    // Make sure it's P0's turn.
    assert_eq!(r.game.turn_player(), 0);

    // End the turn — fires EndOfYourTurn observers, which grant
    // MayAttack + CanAttackUnsuspended unconditionally for the carrier.
    r.game.end_turn();

    // After fire_end_of_your_turn drains, the modifiers should be on
    // EXEC and the phase should have parked in EndOfTurnAction
    // (because MayAttack on a can-attack permanent is one of the gates
    // `has_end_of_turn_keywords` checks).
    assert!(
        r.game.modifiers.has(exec, ModifierType::MayAttack),
        "MayAttack must be granted by Execute's EndOfYourTurn body"
    );
    assert!(
        r.game
            .modifiers
            .has(exec, ModifierType::CanAttackUnsuspended),
        "CanAttackUnsuspended must be granted by Execute's EndOfYourTurn body"
    );
    assert_eq!(
        r.game.current_phase,
        GamePhase::EndOfTurnAction,
        "phase must park in EndOfTurnAction so the player can spend the granted attack"
    );

    // Execute the attack on the unsuspended DEF.
    let result = r.attack_digimon(exec, _def, /*vortex=*/ false);

    // EXEC won the DP comparison (5000 > 2000), so DEF is gone.
    assert_eq!(
        r.game.players[1].battle_area.len(),
        0,
        "DEF should be deleted by combat (EXEC wins DP comparison)"
    );
    let _ = result;

    // EXEC self-deletes via its EndOfAttack observer (cause = OwnEffect),
    // regardless of the combat outcome — verified here by EXEC ending
    // up in P0's trash.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "EXEC should self-delete after the attack ends"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EXEC"),
        "EXEC should land in P0's trash via the EndOfAttack self-delete"
    );
}

// ─── Test 2: PASS at EOT-action declines → no self-delete ───────────────────

/// Same setup as Test 1, but the player PASSes the EndOfTurnAction
/// phase instead of attacking. The EndOfAttack self-delete observer
/// fires only when `pa.attacker == me`, so a declined attack leaves
/// the carrier on field. Modifiers expire on turn rotation via
/// `Expiry::EndOfTurn`.
#[test]
fn execute_no_self_delete_when_attack_declined() {
    let mut r = DebugRunner::builder()
        .add_card(execute_card("EXEC", 5000))
        .add_card({
            let mut c = plain_digimon("DEF");
            c.dp = Some(2000);
            c
        })
        .start();

    let exec = r.place_on_field(0, "EXEC", Some(0));
    let _def = r.place_on_field(1, "DEF", Some(0));

    r.game.end_turn();

    // Phase parks in EOT-action with the granted modifiers live.
    assert_eq!(r.game.current_phase, GamePhase::EndOfTurnAction);
    assert!(r.game.modifiers.has(exec, ModifierType::MayAttack));

    // Decline at EOT-action — this is the printed "may" decision:
    // the player chooses NOT to spend the granted attack.
    r.game.pass_end_of_turn_action();

    // Turn rotated; EOT modifiers expired.
    assert_ne!(
        r.game.current_phase,
        GamePhase::EndOfTurnAction,
        "PASS at EOT-action must rotate the turn out of EOT phase"
    );
    assert!(
        !r.game.modifiers.has(exec, ModifierType::MayAttack),
        "EndOfTurn-expiring modifiers must clear after rotation"
    );
    assert!(
        !r.game
            .modifiers
            .has(exec, ModifierType::CanAttackUnsuspended),
        "EndOfTurn-expiring modifiers must clear after rotation"
    );

    // EXEC remains on field — no attack happened, so the
    // EndOfAttack self-delete observer never fires.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "EXEC must remain on field when the player declines the granted attack"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "EXEC",
        "EXEC must still be the top card on its slot"
    );
    assert!(
        !r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EXEC"),
        "EXEC must not appear in trash — no attack happened"
    );
}

// ─── Test 3: attack mask exposes unsuspended-target bit after grant ─────────

/// Verify the action mask actually surfaces an attack-against-
/// unsuspended bit when the Execute carrier is the attacker. The
/// default end-of-turn-attack mask emits unsuspended bits ONLY when
/// the attacker has `CanAttackUnsuspended` (or `Raid`); plain
/// `MayAttack` without the unsuspended grant would only mask
/// suspended targets. This test pins the unsuspended-target widening
/// to Execute's modifier bundle.
#[test]
fn execute_attack_target_filter_includes_unsuspended() {
    let mut r = DebugRunner::builder()
        .add_card(execute_card("EXEC", 5000))
        .add_card({
            let mut c = plain_digimon("DEF");
            c.dp = Some(2000);
            c
        })
        .start();

    let exec = r.place_on_field(0, "EXEC", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    assert!(
        !r.game.players[1].battle_area[0].is_suspended,
        "DEF must be unsuspended for this test to be meaningful"
    );

    r.game.end_turn();

    // Phase must be EndOfTurnAction with the modifiers live.
    assert_eq!(r.game.current_phase, GamePhase::EndOfTurnAction);
    assert!(r
        .game
        .modifiers
        .has(exec, ModifierType::CanAttackUnsuspended));

    // Build the mask for P0 and verify the bit for (attacker=EXEC,
    // target=DEF) is set despite DEF being unsuspended.
    let mask = digimon_engine::build_action_mask(&r.game, 0);
    let bit = encode_attack(exec.index as u16, def.index as u16) as usize;
    assert!(
        bit >= ATTACK_START as usize
            && bit < ATTACK_START as usize + (TARGETS_PER_ATTACKER as usize) * 14,
        "attack bit must be in the §4.6 attack range"
    );
    assert_eq!(
        mask[bit], 1.0,
        "EOT-action mask must expose the attack-bit against the unsuspended DEF \
         when EXEC carries MayAttack + CanAttackUnsuspended"
    );

    // Sanity: PASS is also legal in EndOfTurnAction (player may decline
    // to use the granted attack — though declining means the
    // EndOfAttack self-delete won't trigger, leaving EXEC on field).
    assert_eq!(
        mask[PASS as usize], 1.0,
        "PASS must remain legal in EndOfTurnAction"
    );
}
