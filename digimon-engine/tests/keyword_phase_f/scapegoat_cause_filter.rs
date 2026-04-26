//! Phase F Task 1 — Scapegoat outer-dialog suppression via the
//! cause-aware `replacement_condition` substrate.
//!
//! Phase E §E2 left a known UX divergence: the outer "may" dialog parks
//! unconditionally for `WhenWouldBeDeleted` triggers, even when the
//! deletion cause is `OwnEffect` (RULES_CONTEXT 16-31 says Scapegoat
//! ignores own-effect deletions) and even when the controller has no
//! other own permanents to substitute (mirrors DCGO `CanActivateScapegoat`'s
//! `HasMatchConditionPermanent` gate). Task 1 closes this by moving the
//! cause / candidate filter UPSTREAM into `collect_candidates` via the new
//! `EffectBuilder::replacement_condition` cause-aware filter.
//!
//! Test matrix:
//!   1. OwnEffect cause + a viable substitute → no outer dialog parks,
//!      carrier is deleted.
//!   2. OpponentEffect cause + no viable substitute → no outer dialog
//!      parks, carrier is deleted.
//!   3. OpponentEffect cause + a viable substitute → outer dialog parks
//!      (regression for the happy path; PASS still works).

use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;
use digimon_engine::replacement::ReplacementCause;

use super::helpers::plain_digimon;

fn scapegoat_card(id: &str) -> CardData {
    let mut c = plain_digimon(id);
    c.keywords = vec![Keyword::Scapegoat];
    c
}

// ─── Test 1: OwnEffect cause must skip the outer accept dialog ──────────────

/// `<Scapegoat>` ignores own-effect deletions per RULES_CONTEXT 16-31.
/// With a viable ALLY substitute on the field and an OwnEffect-caused
/// deletion of the SCAP carrier, the outer "may" dialog must NOT park —
/// the cause-aware candidate filter should drop the candidate before
/// `install_optional_selection` runs.
#[test]
fn scapegoat_skips_outer_dialog_on_own_effect_cause() {
    let mut r = DebugRunner::builder()
        .add_card(scapegoat_card("SCAP"))
        .add_card(plain_digimon("ALLY"))
        .start();

    let scap = r.place_on_field(0, "SCAP", None);
    let _ally = r.place_on_field(0, "ALLY", None);

    r.game
        .delete_permanent_with_cause(scap, ReplacementCause::OwnEffect);

    assert!(
        r.game.pending_selection.is_none(),
        "OwnEffect-caused deletion of a Scapegoat carrier must NOT park the \
         outer accept dialog (RULES_CONTEXT 16-31). Cause-aware \
         replacement_condition should drop the candidate before \
         install_optional_selection."
    );

    // Carrier is gone, ally survives.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "ALLY should survive — only SCAP was deleted"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "ALLY",
        "ALLY should be the surviving permanent"
    );
    assert_eq!(
        r.game.players[0].trash.len(),
        1,
        "SCAP should be in the trash"
    );
    assert_eq!(
        r.game.players[0].trash[0].card_id(&r.game.card_data),
        "SCAP",
        "trashed card should be SCAP"
    );
}

// ─── Test 2: no viable substitute → no outer accept dialog ──────────────────

/// DCGO `CanActivateScapegoat`'s `HasMatchConditionPermanent` gate:
/// when the controller has no other own permanents, the keyword is
/// inactive — the outer "may" dialog must NOT park even when the cause
/// would otherwise be admissible.
#[test]
fn scapegoat_skips_outer_dialog_when_no_other_candidate() {
    let mut r = DebugRunner::builder()
        .add_card(scapegoat_card("SCAP"))
        .start();

    let scap = r.place_on_field(0, "SCAP", None);

    r.game
        .delete_permanent_with_cause(scap, ReplacementCause::OpponentEffect);

    assert!(
        r.game.pending_selection.is_none(),
        "Scapegoat with no other own permanents must NOT park the outer \
         accept dialog (mirrors DCGO HasMatchConditionPermanent gate)."
    );

    assert!(
        r.game.players[0].battle_area.is_empty(),
        "SCAP should be deleted when no other permanents are available"
    );
    assert_eq!(
        r.game.players[0].trash.len(),
        1,
        "SCAP should be in the trash"
    );
    assert_eq!(
        r.game.players[0].trash[0].card_id(&r.game.card_data),
        "SCAP",
        "trashed card should be SCAP"
    );
}

// ─── Test 3: happy path — OpponentEffect + viable substitute parks dialog ───

/// Regression for the happy path: opponent-effect deletion with a viable
/// substitute → outer optional accept dialog parks normally. PASS leaves
/// the original deletion to proceed (Phase E behavior, preserved).
#[test]
fn scapegoat_parks_outer_dialog_on_opponent_effect_with_candidate() {
    let mut r = DebugRunner::builder()
        .add_card(scapegoat_card("SCAP"))
        .add_card(plain_digimon("ALLY"))
        .start();

    let scap = r.place_on_field(0, "SCAP", None);
    let _ally = r.place_on_field(0, "ALLY", None);

    r.game
        .delete_permanent_with_cause(scap, ReplacementCause::OpponentEffect);

    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Scapegoat outer accept dialog must be parked");
        assert!(
            pending.is_optional,
            "Scapegoat is optional ('may'); outer dialog must accept PASS"
        );
        assert_eq!(
            pending.valid_action_ids,
            vec![REPLACEMENT_ACCEPT],
            "outer accept dialog must list exactly [REPLACEMENT_ACCEPT]; \
             PASS is admitted via is_optional",
        );
        assert_eq!(pending.selecting_player, 0);
    }

    // Decline path — original deletion proceeds.
    r.game.resolve_selection(0, PASS).expect("decline Scapegoat");

    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "ALLY should survive after declining Scapegoat"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "ALLY",
        "ALLY is the surviving permanent"
    );
    assert_eq!(
        r.game.players[0].trash.len(),
        1,
        "SCAP should be in trash after PASS"
    );
}
