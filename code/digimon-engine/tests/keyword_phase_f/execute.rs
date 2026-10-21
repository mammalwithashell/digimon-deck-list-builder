//! Phase F Task 3 — `Keyword::Execute` auto-install behavioral tests.
//!
//! Comprehensive Rules Ver. 4.3 §16-38: "<Execute> is a keyword effect. At the
//! end of your turn, the Digimon with this keyword effect may attack. At the
//! end of the attack, this Digimon is deleted. This effect also allows for
//! attacking an opponent's unsuspended Digimon." (§16-38-1). A trigger-type
//! effect at the end of your turn (§16-38-2), optional (§16-38-3); the
//! deletion at the end of the attack is pending processing (§16-38-4).
//!
//! Auto-install shape (2026-10-18, G-ENGINE-ENGAGE-ATTACK-AFTER-END-OF-TURN-TRIGGERS):
//!   - `EndOfYourTurn` trigger whose processing is one optional attack
//!     declaration decision inside the end-of-turn pool, over the player and
//!     every opponent's Digimon (suspended or not), PASS = decline. No
//!     `MayAttack` / `CanAttackUnsuspended` grant, no `EndOfTurnAction`
//!     window.
//!   - `EndOfAttack` observer that deletes the carrier
//!     (`ReplacementCause::OwnEffect`) at the end of the attack <Execute>
//!     declared, and only that one.
//!
//! DCGO behavior reference (`CardEffectCommons/KeyWordEffects/Execute.cs`):
//!   1. `OnEndYourTurn` activation (optional).
//!   2. `CanAttackTargetDefendingPermanentClass` added to
//!      `UntilEndAttackEffects` → this attack may target unsuspended Digimon.
//!   3. `SelectAttackEffect` runs (player picks a target).
//!   4. `OnEndAttack` `DeleteSelfEffect`.

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
/// defender (`DEF`). End P0's turn: <Execute>'s trigger offers its attack
/// as its own decision (§16-38-2). Attack `DEF` (unsuspended, §16-38-1).
/// After the attack resolves, `EXEC` is deleted (cause = OwnEffect) and now
/// lives in P0's trash.
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

    let exec = r.place_on_field(0, "EXEC", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    assert!(
        !r.game.players[1].battle_area[0].is_suspended,
        "DEF should start unsuspended"
    );
    assert_eq!(r.game.turn_player(), 0);

    r.game.end_turn();

    // §16-38-2: the attack is offered inside the end-of-turn pool; no
    // `MayAttack` window is granted.
    assert!(
        !r.game.modifiers.has(exec, ModifierType::MayAttack),
        "no MayAttack grant: Execute offers its attack itself"
    );
    let sel = r.pending_selection().expect("<Execute>'s attack decision");
    assert!(sel.is_optional, "§16-38-3: optional");
    let attack = encode_attack(exec.index as u16, def.index as u16);
    assert!(sel.valid_action_ids.contains(&attack), "{sel:?}");

    r.execute_action(0, attack).expect("attack the unsuspended DEF");

    // EXEC won the DP comparison (5000 > 2000), so DEF is gone.
    assert_eq!(
        r.game.players[1].battle_area.len(),
        0,
        "DEF should be deleted by combat (EXEC wins DP comparison)"
    );

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
    assert_eq!(r.game.turn_player(), 1, "then the turn passed");
}

// ─── Test 2: declining → no self-delete ─────────────────────────────────────

/// Same setup as Test 1, but the player declines <Execute>'s attack
/// decision (§16-38-3). No attack, so the EndOfAttack self-delete never
/// applies and the carrier stays on the field; the turn passes.
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

    let _exec = r.place_on_field(0, "EXEC", Some(0));
    let _def = r.place_on_field(1, "DEF", Some(0));

    r.game.end_turn();
    assert!(r.pending_selection().is_some(), "<Execute>'s decision");

    // Decline — the printed "may" decision.
    r.decline_optional_trigger().expect("decline <Execute>");

    assert_ne!(r.game.current_phase, GamePhase::EndOfTurnAction);
    assert_eq!(r.game.turn_player(), 1, "the turn passed");

    // EXEC remains on field — no attack happened.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "EXEC must remain on field when the player declines the attack"
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

// ─── Test 3: the decision offers the unsuspended target ─────────────────────

/// §16-38-1 "This effect also allows for attacking an opponent's unsuspended
/// Digimon": <Execute>'s decision offers the attack on the unsuspended DEF
/// (an ordinary attack would not, §11-2-7-1), as well as the player.
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
    assert!(r.pending_selection().is_some(), "<Execute>'s decision");

    let mask = digimon_engine::build_action_mask(&r.game, 0);
    let bit = encode_attack(exec.index as u16, def.index as u16) as usize;
    assert!(
        bit >= ATTACK_START as usize
            && bit < ATTACK_START as usize + (TARGETS_PER_ATTACKER as usize) * 14,
        "attack bit must be in the §4.6 attack range"
    );
    assert_eq!(
        mask[bit], 1.0,
        "the decision must offer the attack against the unsuspended DEF"
    );
    assert_eq!(mask[PASS as usize], 1.0, "PASS declines <Execute>");
}

// ─── Test 4 (task_69f10a66): a MAIN-phase attack must NOT self-delete ───────

/// §16-38 scopes the self-delete to "the end of THAT attack" — the attack
/// Execute itself granted at end of turn. DCGO registers the delete inside
/// `ExecuteProcess` (`UntilEndAttackEffects`), so an ordinary Main-phase
/// attack by a Digimon that merely HAS `<Execute>` survives its attack.
/// Regression pin for the phase gate on the `EndOfAttack` observer
/// (exam witness: EX12-036-effect3 — a granted-Execute carrier attacked
/// security in its Main phase and was wrongly self-deleted, cascading a
/// spurious `<Decode>` window).
#[test]
fn execute_main_phase_attack_does_not_self_delete() {
    let mut r = DebugRunner::builder()
        .add_card(execute_card("EXEC", 5000))
        .add_card({
            let mut c = plain_digimon("DEF");
            c.dp = Some(2000);
            c
        })
        .start();

    let exec = r.place_on_field(0, "EXEC", Some(0));
    // Suspended defender — an ordinary Main-phase attack target.
    let def = r.place_on_field(1, "DEF", Some(0));
    if let Some(perm) = r.game.player_mut(1).battle_area.get_mut(def.index as usize) {
        perm.is_suspended = true;
    }

    assert_eq!(r.game.current_phase, GamePhase::Main);
    let result = r.attack_digimon(exec, def, /*vortex=*/ false);
    let _ = result;

    assert_eq!(
        r.game.players[1].battle_area.len(),
        0,
        "DEF loses the battle (5000 > 2000)"
    );
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "EXEC must SURVIVE its Main-phase attack — §16-38's self-delete \
         applies only to the end-of-turn attack Execute granted"
    );
    assert!(
        !r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EXEC"),
        "no self-delete: EXEC must not be in the trash"
    );
}
