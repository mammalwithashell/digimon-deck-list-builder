//! Phase 7 Task 7 — behavioral end-to-end replacement test.
//!
//! This test covers the full fire-site → replacement-dispatch → optional
//! selection → accept → post-commit observer flow using the native `<Barrier>`
//! keyword attached to a Digimon in battle. It complements the dispatcher-
//! level unit tests in `dispatcher_core.rs` + `dispatcher_guard.rs` and the
//! per-fire-site route tests in `route_replacements.rs` by exercising the
//! whole pipeline the way a real RL loop would.
//!
//! **Deferred scope:** the original Task 7 plan called for a "layered
//! Barriers" scenario combining a Cherubimon-style Digimon's printed Barrier
//! with a Tamer aura granting Barrier to all own Digimon. The aura-grant
//! path requires a Tamer-scoped modifier that installs a replacement on
//! another permanent, which is not yet covered by Phase 7 v1 (the passive-
//! modifier registry-side replacement install is Task 5 scope, but tamer-
//! aura-grant of keyword-sourced replacements is beyond it). The single-
//! Barrier scenario below is the canonical end-to-end proof; the layered
//! case will land once aura-granted keywords are wired (likely Phase 9 or
//! part of the Tamer-card effect work).

use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;

fn card_with_keywords(id: &str, keywords: Vec<Keyword>) -> digimon_engine::CardData {
    let mut c = make_test_card(id, id);
    c.keywords = keywords;
    c
}

/// Full-pipeline end-to-end: a Barrier-keyworded Digimon is attacked by a
/// higher-DP opponent. Battle resolution fires `WhenWouldBeDeleted` with
/// cause=Battle; the defender's printed `<Barrier>` installs an optional
/// `PendingSelection::Replacement`. On accept, the Barrier process trashes
/// the top of the defender's security and cancels the deletion; the defender
/// stays on the field and `OnDeletion` observers do NOT fire.
///
/// Verifies:
/// - PendingSelection::Replacement installs with `REPLACEMENT_ACCEPT` in
///   `valid_action_ids` (optional-replacement contract).
/// - Accept path trashes exactly one card from the top of the owner's security.
/// - The defender is still on the field (battle-deletion prevented).
/// - Security -= 1 and trash += 1 end state.
#[test]
fn ts_olympos_cherubimon_barrier_battle_end_to_end() {
    // Attacker on P1 — high DP, plain card with no replacements.
    let mut attacker_card = make_test_card("BIG_ATTACKER", "Big Attacker");
    attacker_card.dp = Some(10000);

    // Defender on P0 — Cherubimon-flavored Digimon with printed <Barrier>.
    // Lower DP so battle resolution would delete it without the Barrier.
    let mut cherubimon = card_with_keywords("CHERUBIMON", vec![Keyword::Barrier]);
    cherubimon.dp = Some(3000);

    let mut r = DebugRunner::builder()
        .add_card(attacker_card)
        .add_card(make_test_card("SEC", "Security"))
        .add_card(cherubimon)
        .security(0, &["SEC", "SEC"])
        .start();

    let attacker = r.place_on_field(1, "BIG_ATTACKER", Some(0));
    let defender = r.place_on_field(0, "CHERUBIMON", Some(0));

    let security_before = r.game.player(0).security.len();
    let trash_before = r.game.player(0).trash.len();
    assert_eq!(security_before, 2);
    assert_eq!(trash_before, 0);
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(r.battle_area_size(1), 1);

    // Initiate the attack. Battle resolution dispatches WhenWouldBeDeleted
    // with cause=Battle on the defender; Barrier installs an optional
    // replacement prompt for P0 (the defender's controller).
    let _ = r.attack_digimon(attacker, defender, false);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("Barrier should install a PendingSelection::Replacement on would-be-deleted");
    assert!(
        sel.valid_action_ids.contains(&REPLACEMENT_ACCEPT),
        "optional replacement prompt must offer REPLACEMENT_ACCEPT"
    );
    assert!(
        sel.is_optional,
        "Barrier is a 'may' effect — decline must always be available"
    );
    assert_eq!(
        sel.selecting_player, 0,
        "defender's controller gets the Barrier prompt"
    );

    // Accept the Barrier replacement.
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Barrier replacement");

    // Post-accept assertions:
    // 1. Defender survived (Barrier cancelled the deletion).
    assert_eq!(
        r.battle_area_size(0),
        1,
        "Cherubimon survives — Barrier cancels the battle-deletion"
    );
    // 2. Owner's security lost exactly one card to the Barrier cost.
    assert_eq!(
        r.game.player(0).security.len(),
        security_before - 1,
        "Barrier should trash one card from the top of the owner's security"
    );
    assert_eq!(
        r.game.player(0).trash.len(),
        trash_before + 1,
        "that card lands in the owner's trash"
    );
    // 3. Pending selection cleared and replacement_depth back to 0 (hygiene).
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.replacement_depth, 0);
}
