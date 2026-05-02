//! Phase F follow-up — Scapegoat substitute filter must reject Tamers.
//!
//! Printed text (RULES_CONTEXT 16-31, DCGO `Scapegoat.cs`): "When this
//! Digimon would be deleted, you may delete another of your **Digimon** to
//! prevent it." The substitute must be a Digimon — Tamers are NOT valid.
//!
//! The pre-fix implementation accepted any non-self own permanent
//! (including Tamers) at both:
//!   1. The `replacement_condition` candidate-existence gate — would let
//!      the outer dialog park even when the only other own permanent is
//!      a Tamer.
//!   2. The inner `select_own_permanent` filter — would offer a Tamer as
//!      a valid substitute pick.
//!
//! Both filters must consult `Permanent::is_tamer(&card_data)` to exclude
//! Tamer permanents.

use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;
use digimon_engine::replacement::ReplacementCause;

use super::helpers::{plain_digimon, plain_tamer};

fn scapegoat_card(id: &str) -> CardData {
    let mut c = plain_digimon(id);
    c.keywords = vec![Keyword::Scapegoat];
    c
}

// ─── Test 1: outer dialog suppressed when only other own perm is a Tamer ─────

/// Printed text restricts the substitute to "another of your **Digimon**".
/// When the only other own permanent is a Tamer, the candidate-existence
/// gate (in `replacement_condition`) must report no viable substitute and
/// the outer "may" dialog must NOT park — the carrier is deleted normally.
#[test]
fn scapegoat_skips_outer_dialog_when_only_other_perm_is_tamer() {
    let mut r = DebugRunner::builder()
        .add_card(scapegoat_card("SCAP"))
        .add_card(plain_tamer("TAMER"))
        .start();

    let scap = r.place_on_field(0, "SCAP", None);
    let _tamer = r.place_on_field(0, "TAMER", None);

    r.game
        .delete_permanent_with_cause(scap, ReplacementCause::OpponentEffect);

    assert!(
        r.game.pending_selection.is_none(),
        "Scapegoat must NOT park the outer dialog when the only other own \
         permanent is a Tamer (printed text: 'another of your Digimon')."
    );

    // Carrier is gone, Tamer survives.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "TAMER should survive — only SCAP was deleted"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "TAMER",
        "TAMER should be the surviving permanent"
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

// ─── Test 2: inner pick filter excludes Tamer when a Digimon is also present ─

/// With a Scapegoat carrier, a plain Digimon, AND a plain Tamer all on
/// player 0, the outer dialog must park (the Digimon is a valid Digimon
/// substitute). Once accepted, the inner pick must filter out the Tamer
/// — only the plain Digimon should be selectable.
///
/// Verification strategy: park the inner pick, then confirm that
/// resolving with the plain Digimon's action_id substitutes correctly
/// (carrier survives, Digimon goes to trash, Tamer untouched). Inner pick
/// `valid_action_ids` should NOT contain any action that selects the
/// Tamer permanent.
#[test]
fn scapegoat_picker_filter_excludes_tamer() {
    let mut r = DebugRunner::builder()
        .add_card(scapegoat_card("SCAP"))
        .add_card(plain_digimon("ALLY"))
        .add_card(plain_tamer("TAMER"))
        .start();

    let scap = r.place_on_field(0, "SCAP", None);
    let _ally = r.place_on_field(0, "ALLY", None);
    let _tamer = r.place_on_field(0, "TAMER", None);

    r.game
        .delete_permanent_with_cause(scap, ReplacementCause::OpponentEffect);

    // Outer optional accept dialog should park — ALLY is a valid Digimon
    // substitute even though TAMER is not.
    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Scapegoat outer accept dialog must park (ALLY is valid)");
        assert!(pending.is_optional);
        assert_eq!(pending.selecting_player, 0);
    }
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Scapegoat substitute");

    // Inner pick: must offer EXACTLY one candidate (ALLY); TAMER must be
    // excluded by the Digimon-only filter.
    let inner_action = {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Scapegoat inner pick must park");
        assert!(
            !pending.is_optional,
            "inner pick is mandatory once accepted"
        );
        assert_eq!(
            pending.valid_action_ids.len(),
            1,
            "inner pick must offer exactly one candidate (ALLY); TAMER \
             must be filtered out per printed text 'another of your Digimon'"
        );
        pending.valid_action_ids[0]
    };
    r.game
        .resolve_selection(0, inner_action)
        .expect("pick the only valid substitute (ALLY)");

    // Post-state: SCAP survives, ALLY in trash, TAMER untouched.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "SCAP and TAMER should remain after substitute"
    );
    let names: Vec<&str> = r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data))
        .collect();
    assert!(names.contains(&"SCAP"), "SCAP should survive");
    assert!(names.contains(&"TAMER"), "TAMER should be untouched");
    assert!(!names.contains(&"ALLY"), "ALLY should be the substituted");
    assert_eq!(r.game.players[0].trash.len(), 1, "ALLY should be in trash");
    assert_eq!(
        r.game.players[0].trash[0].card_id(&r.game.card_data),
        "ALLY",
        "trashed card should be ALLY (the substitute)"
    );
}
