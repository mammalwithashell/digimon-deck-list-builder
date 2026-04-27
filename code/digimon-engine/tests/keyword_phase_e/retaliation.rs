//! Phase E §E1 — `Keyword::Retaliation` auto-install behavioral tests.
//!
//! A card declaring ONLY `keywords: vec![Keyword::Retaliation]` (no
//! hand-rolled `CardEffect`) must, when self is deleted by Battle, delete
//! the opposing combatant. Mandatory; no "may" clause (RULES_CONTEXT
//! 16-12). Cause filter: `deletion_cause() == Some(ReplacementCause::Battle)`.
//!
//! Mirrors DCGO `Retaliation.cs` — fires from `OnDestroyedAnyone` with
//! `IsByBattle(hashtable)` cause filter, targets the opposing combatant
//! (`WinnerPermanents` in DCGO terms) read from the live battle state.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

use super::helpers::{digimon_with_keywords, plain_digimon};

fn retaliation_card(id: &str, dp: i32) -> CardData {
    digimon_with_keywords(id, 5, dp, vec![Keyword::Retaliation])
}

// ─── Test 1: happy path — self deleted in battle → deletes the winner ────────

/// Stack: P0[ATK 5000 DP], P1[RETAL 3000 DP, Retaliation].
/// P0 attacks RETAL → battle resolves, attacker wins → RETAL deleted
/// (cause=Battle) → Retaliation fires → ATK also deleted.
/// Both end in trash; both battle areas empty.
#[test]
fn retaliation_deletes_winner_when_self_loses_battle() {
    let atk = {
        let mut c = plain_digimon("ATK");
        c.dp = Some(5000);
        c
    };
    let retal = retaliation_card("RETAL", 3000);

    let mut r = DebugRunner::builder().add_card(atk).add_card(retal).start();

    // Pass Some(0) for turn_played_override to bypass summoning sickness
    // (turn_count == 1 after start_game(); turn_played == 0 < 1 → not fresh).
    let atk_h = r.place_on_field(0, "ATK", Some(0));
    let retal_h = r.place_on_field(1, "RETAL", Some(0));

    // Drive battle: ATK attacks RETAL. ATK wins (5000 > 3000).
    // RETAL is deleted with cause=Battle. Its Retaliation fires and
    // deletes ATK.
    r.attack_digimon(atk_h, retal_h, false);

    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "ATK should be deleted by Retaliation"
    );
    assert_eq!(
        r.game.players[1].battle_area.len(),
        0,
        "RETAL should be deleted by losing the battle"
    );
    assert_eq!(r.game.players[0].trash.len(), 1, "ATK lands in P0 trash");
    assert_eq!(r.game.players[1].trash.len(), 1, "RETAL lands in P1 trash");
    assert_eq!(
        r.game.players[0].trash[0].card_id(&r.game.card_data),
        "ATK",
        "P0 trash should contain ATK (deleted by Retaliation)"
    );
    assert_eq!(
        r.game.players[1].trash[0].card_id(&r.game.card_data),
        "RETAL",
        "P1 trash should contain RETAL (deleted by losing battle)"
    );
}

// ─── Test 2: cause gate — effect deletion does NOT fire Retaliation ──────────

/// Cause gate: when self is deleted by an opponent's effect (not Battle),
/// Retaliation does NOT fire — there is no live pending_attack.
#[test]
fn retaliation_does_not_fire_on_effect_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(retaliation_card("RETAL", 3000))
        .add_card(plain_digimon("BYSTANDER"))
        .start();

    let retal_h = r.place_on_field(0, "RETAL", None);
    let _by = r.place_on_field(1, "BYSTANDER", None);

    // Direct effect-cause deletion — no battle.
    r.game.delete_permanent_with_cause(
        retal_h,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );

    // BYSTANDER must be untouched — Retaliation did not fire.
    assert_eq!(
        r.game.players[1].battle_area.len(),
        1,
        "BYSTANDER must survive — Retaliation must not fire on OpponentEffect deletion"
    );
    assert_eq!(r.game.players[1].trash.len(), 0);
    assert_eq!(
        r.game.players[0].trash.len(),
        1,
        "RETAL is trashed normally"
    );
}

// ─── Test 3: cause gate — own-effect deletion does NOT fire Retaliation ──────

/// Cause gate: when self is deleted by its own controller's effect,
/// Retaliation does NOT fire.
#[test]
fn retaliation_does_not_fire_on_own_effect_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(retaliation_card("RETAL", 3000))
        .add_card(plain_digimon("BYSTANDER"))
        .start();

    let retal_h = r.place_on_field(0, "RETAL", None);
    let _by = r.place_on_field(1, "BYSTANDER", None);

    r.game.delete_permanent_with_cause(
        retal_h,
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );

    assert_eq!(
        r.game.players[1].battle_area.len(),
        1,
        "BYSTANDER must survive — Retaliation must not fire on OwnEffect deletion"
    );
    assert_eq!(
        r.game.players[0].trash.len(),
        1,
        "RETAL is trashed normally"
    );
}

// ─── Test 4: mutual destruction ──────────────────────────────────────────────

/// Mutual destruction: both combatants have Retaliation and tie in DP.
/// Both are deleted in battle (resolve_battle's MutualDestruction branch).
/// The defender's Retaliation fires first and tries to delete the attacker;
/// the attacker's Retaliation fires but the defender is already gone.
/// No panic; both trashes have exactly 1 card each.
#[test]
fn retaliation_handles_mutual_destruction() {
    let r1 = retaliation_card("R1", 4000);
    let r2 = retaliation_card("R2", 4000);

    let mut r = DebugRunner::builder().add_card(r1).add_card(r2).start();

    let r1_h = r.place_on_field(0, "R1", Some(0));
    let r2_h = r.place_on_field(1, "R2", Some(0));

    // Equal DP → mutual destruction. Both deleted in battle; each
    // Retaliation fires against the already-deleting opponent. Graceful
    // (no panic); final state: both battle areas empty, each trash has 1.
    r.attack_digimon(r1_h, r2_h, false);

    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "R1 deleted in mutual destruction"
    );
    assert_eq!(
        r.game.players[1].battle_area.len(),
        0,
        "R2 deleted in mutual destruction"
    );
    assert_eq!(r.game.players[0].trash.len(), 1);
    assert_eq!(r.game.players[1].trash.len(), 1);
}
