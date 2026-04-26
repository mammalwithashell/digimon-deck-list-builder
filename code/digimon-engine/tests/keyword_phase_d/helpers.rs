//! Shared test helpers for Phase D keyword behavioral tests.

use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::debug_runner::DebugRunner;

/// Drain an optional outer-accept replacement dialog if one is currently
/// parked on `r.game.pending_selection`.
///
/// Used by the neighbor-deletion self-scope tests for optional keywords
/// (Fragment, ArmorPurge, …): the candidate-walk in
/// `replacement::try_replace_inner` may install an outer accept dialog for
/// the carrier's effect even when the deletion subject is a neighbor — the
/// body's self-scope guard then no-ops once the dialog is drained.
///
/// Asserts the parked selection has the expected outer-accept shape
/// (`valid_action_ids == [REPLACEMENT_ACCEPT]` and `is_optional == true`,
/// which implicitly admits `PASS` as the decline action), then resolves with
/// `action` — pass `REPLACEMENT_ACCEPT` to accept, `PASS` to decline.
///
/// Returns `true` if a dialog was drained, `false` if no selection was
/// pending.
pub fn drain_outer_accept_dialog(r: &mut DebugRunner, action: u16) -> bool {
    assert_outer_accept_dialog_shape(r);
    let player = r
        .game
        .pending_selection
        .as_ref()
        .expect("dialog presence checked above")
        .selecting_player;
    assert!(
        action == REPLACEMENT_ACCEPT || action == PASS,
        "drain_outer_accept_dialog: action must be REPLACEMENT_ACCEPT or PASS; \
         got {}",
        action,
    );
    r.game
        .resolve_selection(player, action)
        .expect("drain outer accept dialog");
    true
}

/// Assert the parked `pending_selection` is an outer optional-accept dialog
/// with exactly one valid action (`REPLACEMENT_ACCEPT`) and `is_optional`
/// true. PASS is admitted implicitly via the optional flag — it is NOT
/// listed in `valid_action_ids`. See `effect_queue::resolve_generic_selection`
/// `is_pass && !sel.is_optional` gate.
///
/// Panics if no selection is pending or the shape doesn't match.
pub fn assert_outer_accept_dialog_shape(r: &DebugRunner) {
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("expected an outer accept dialog to be parked");
    let valid: Vec<u16> = pending.valid_action_ids.iter().copied().collect();
    assert_eq!(
        valid,
        vec![REPLACEMENT_ACCEPT],
        "outer accept dialog must list exactly [REPLACEMENT_ACCEPT]; got {:?}",
        valid,
    );
    assert!(
        pending.is_optional,
        "outer accept dialog must have is_optional=true so PASS declines",
    );
}
