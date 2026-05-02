//! Phase 2f3 Task 1 — selection-callback `(controller, override)` persistence.
//!
//! When a `select_*` method on `EffectContext` parks a `PendingSelection`, the
//! callback closure rebuilds an `EffectContext` to hand to the user-supplied
//! callback. Pre-Phase 2f3 it built that ctx via `EffectContext::new(game,
//! source_card, source_permanent, selecting_player)` — collapsing controller
//! and selecting_player into one field.
//!
//! For `AsSelectingPlayer { of: opponent, body: [..] }` to work, the callback
//! must rebuild ctx with `player = controller` (the original effect controller)
//! AND `override_selecting_player = Some(selecting_player_at_install_time)`.
//! Otherwise nested `select_*`s in the body (and the parked dsl_outer_tail)
//! lose the override and route back to `self.player`.
//!
//! This test exercises the new constructor `EffectContext::new_with_override`
//! by installing a P0-as-P1 hand selection and asserting that — after
//! resolution — the callback's inner `ctx.player` is still 0 and
//! `ctx.override_selecting_player()` is still `Some(1)`.

use std::sync::{Arc, Mutex};

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;

#[test]
fn select_hand_callback_with_override_preserves_controller_and_override() {
    // P0 is the controller. Override = P1 (the selecting player).
    // Source = a card in P0's hand. P1 also has a card in hand so
    // `select_hand(P1, ...)` has a non-empty candidate list.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST", "Source"))
        .hand(0, &["TST"])
        .hand(1, &["TST"])
        .start();

    let src_handle = runner.game.players[0].hand[0].handle();

    let observed: Arc<Mutex<Option<(u8, Option<u8>)>>> = Arc::new(Mutex::new(None));
    let observed_cb = observed.clone();

    {
        let mut ctx = EffectContext::new_with_override(
            &mut runner.game,
            src_handle,
            None,    // source_permanent
            0,       // controller (P0)
            Some(1), // override_selecting_player (P1)
        );
        ctx.select_hand(
            1, // of_player — P1's hand is the candidate zone
            "Pick",
            false, // is_optional
            |_g, _i| true,
            move |inner, _picked| {
                let mut slot = observed_cb.lock().unwrap();
                *slot = Some((inner.player, inner.override_selecting_player()));
            },
        );
    }

    // Pending selection should be parked with selecting_player = 1 (override).
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("select_hand should have parked a PendingSelection");
    assert_eq!(
        pending.selecting_player, 1,
        "PendingSelection.selecting_player must reflect the override (P1)"
    );
    let action = pending.valid_action_ids[0];

    runner
        .game
        .resolve_selection(1, action)
        .expect("P1 should be allowed to resolve the parked selection");

    let observed = *observed.lock().unwrap();
    assert_eq!(
        observed,
        Some((0, Some(1))),
        "callback ctx must keep controller=0 and override=Some(1)"
    );
}
