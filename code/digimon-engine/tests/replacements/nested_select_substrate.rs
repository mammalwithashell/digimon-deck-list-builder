//! Phase C — substrate-level tests for the parked-replacement slot and the
//! `EffectContext::cancel_leave` / `handle_replacement` / `redirect_replacement`
//! / `substitute_replacement` outcome-setters.
//!
//! These tests do NOT exercise end-to-end replacement flows — they manually
//! install `Game.parked_replacement` and verify that the outcome-setters
//! mutate the slot correctly. End-to-end coverage lives in the per-keyword
//! test files (`nested_select_save`, `nested_select_fragment`, etc.).

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, Zone};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::{
    ParkedReplacement, ReplacementCause, ReplacementOutcome, ReplacementSubject,
};

fn fighter(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn install_parked(game: &mut digimon_engine::game::Game, target: PermanentHandle) {
    game.install_parked_replacement_for_test(ParkedReplacement {
        subject: ReplacementSubject::Permanent(target),
        cause: ReplacementCause::OpponentEffect,
        original_destination: Some(Zone::Trash),
        source_card: CardHandle(0),
        source_permanent: None,
        controller: 0,
        outcome: ReplacementOutcome::None,
    });
}

#[test]
fn cancel_leave_writes_cancelled_outcome_to_parked_slot() {
    let mut r = DebugRunner::builder().add_card(fighter("X")).start();
    let target = r.place_on_field(0, "X", None);

    install_parked(&mut r.game, target);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.cancel_leave();
    }

    let outcome = r
        .game
        .parked_replacement_outcome_for_test()
        .expect("slot still set");
    assert_eq!(
        outcome,
        ReplacementOutcome::Cancelled,
        "cancel_leave should write Cancelled outcome to parked slot"
    );
}

#[test]
fn handle_replacement_writes_custom_handled_to_parked_slot() {
    let mut r = DebugRunner::builder().add_card(fighter("X")).start();
    let target = r.place_on_field(0, "X", None);
    install_parked(&mut r.game, target);
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.handle_replacement();
    }
    let outcome = r
        .game
        .parked_replacement_outcome_for_test()
        .expect("slot still set");
    assert_eq!(
        outcome,
        ReplacementOutcome::CustomHandled,
        "handle_replacement should write CustomHandled to parked slot"
    );
}

#[test]
fn redirect_replacement_writes_redirected_outcome_to_parked_slot() {
    let mut r = DebugRunner::builder().add_card(fighter("X")).start();
    let target = r.place_on_field(0, "X", None);
    install_parked(&mut r.game, target);
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.redirect_replacement(Zone::Hand);
    }
    let outcome = r
        .game
        .parked_replacement_outcome_for_test()
        .expect("slot still set");
    assert_eq!(
        outcome,
        ReplacementOutcome::Redirected(Zone::Hand),
        "redirect_replacement(Hand) should write Redirected(Hand) to parked slot"
    );
}

#[test]
fn substitute_replacement_writes_substituted_outcome_to_parked_slot() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("X"))
        .add_card(fighter("Y"))
        .start();
    let target = r.place_on_field(0, "X", None);
    let other = r.place_on_field(0, "Y", None);
    install_parked(&mut r.game, target);
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.substitute_replacement(ReplacementSubject::Permanent(other));
    }
    let outcome = r
        .game
        .parked_replacement_outcome_for_test()
        .expect("slot still set");
    assert_eq!(
        outcome,
        ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other)),
        "substitute_replacement should write Substituted(other) to parked slot"
    );
}

use digimon_engine::effect::{CardEffect, Effect};
use std::sync::{Arc, Mutex};

#[test]
fn post_process_hook_installs_parked_replacement_when_select_called() {
    // Hand-rolled card with a WhenWouldBeDeleted replacement whose process
    // closure installs a select_own_permanent — the post-process hook should
    // see pending_selection.is_some() and install Game.parked_replacement.

    struct ParkingCard {
        installed: Arc<Mutex<bool>>,
    }
    impl CardEffect for ParkingCard {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            let installed = Arc::clone(&self.installed);
            vec![Effect::when_would_be_deleted(card)
                .name("PARK-TEST")
                .optional()
                .replacement_process(move |rctx| {
                    rctx.effect.select_own_permanent(
                        "pick anyone",
                        false,
                        |_g, _h| true,
                        move |_ctx, _picked| {
                            // Body never runs in this test — we resolve the
                            // outer accept then inspect parked_replacement
                            // before resolving the inner select.
                        },
                    );
                    *installed.lock().unwrap() = true;
                })
                .build()]
        }
    }

    let installed = Arc::new(Mutex::new(false));
    let mut r = DebugRunner::builder()
        .add_card(fighter("PARK-TEST"))
        .add_card(fighter("OTHER"))
        .start();
    r.register_effect(
        "PARK-TEST",
        Arc::new(ParkingCard {
            installed: Arc::clone(&installed),
        }),
    );
    let parker = r.place_on_field(0, "PARK-TEST", None);
    let _other = r.place_on_field(0, "OTHER", None);

    // Trigger the would-be-deleted dispatch.
    r.game.delete_permanent_with_effects(parker);

    // Outer optional-accept dialog is installed.
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("optional accept installed");
    assert_eq!(
        pending.kind,
        digimon_engine::selection::SelectionKind::Replacement
    );

    // Resolve the outer accept (REPLACEMENT_ACCEPT action).
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept ok");

    // Process closure ran (installed flag set).
    assert!(
        *installed.lock().unwrap(),
        "process closure should have run"
    );
    // Inner select_own_permanent installed a fresh PendingSelection.
    assert!(r.game.pending_selection.is_some(), "inner select installed");
    // POST-PROCESS HOOK: parked_replacement populated.
    let outcome = r.game.parked_replacement_outcome_for_test();
    assert_eq!(
        outcome,
        Some(ReplacementOutcome::None),
        "post-process hook should install Game.parked_replacement (outcome=None until callback writes)"
    );
}

#[test]
fn post_callback_hook_drains_parked_and_commits_outcome() {
    // After the inner select_* callback writes outcome via cancel_leave(),
    // the post-callback hook in resolve_generic_selection should:
    //   1. Take Game.parked_replacement
    //   2. Run commit_deferred_outcome with the parked outcome
    //   3. Leave Game.parked_replacement = None

    struct CancelOnPickCard;
    impl CardEffect for CancelOnPickCard {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![Effect::when_would_be_deleted(card)
                .name("CANCEL-ON-PICK")
                .optional()
                .replacement_process(|rctx| {
                    rctx.effect.select_own_permanent(
                        "pick anyone",
                        false,
                        |_g, _h| true,
                        |ctx, _picked| {
                            ctx.cancel_leave();
                        },
                    );
                })
                .build()]
        }
    }

    let mut r = DebugRunner::builder()
        .add_card(fighter("CANCEL-ON-PICK"))
        .add_card(fighter("OTHER"))
        .start();
    r.register_effect("CANCEL-ON-PICK", Arc::new(CancelOnPickCard));
    let parker = r.place_on_field(0, "CANCEL-ON-PICK", None);
    let _other = r.place_on_field(0, "OTHER", None);

    r.game.delete_permanent_with_effects(parker);

    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept ok");
    assert!(
        r.game.parked_replacement_outcome_for_test().is_some(),
        "parked installed after accept"
    );

    // The inner OwnField selection's first valid action ID picks the parker.
    let pending = r.game.pending_selection.as_ref().expect("inner select");
    let action = pending.valid_action_ids[0];
    let player = pending.selecting_player;
    r.game
        .resolve_selection(player, action)
        .expect("inner pick ok");

    // POST-CALLBACK HOOK should have drained parked_replacement and committed.
    assert!(
        r.game.parked_replacement_outcome_for_test().is_none(),
        "post-callback hook should clear parked_replacement after commit"
    );
    // Deletion was cancelled — parker stayed on the field.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "parker survived: deletion was cancelled by cancel_leave()"
    );
    assert!(r.game.pending_selection.is_none(), "no leftover selection");
}

#[test]
fn default_none_when_callback_skips_outcome() {
    // A replacement process that installs a select_* but the user's callback
    // never calls any outcome-setter — parked.outcome stays None, so the
    // original event proceeds normally.

    struct NoOutcomeCard;
    impl CardEffect for NoOutcomeCard {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![Effect::when_would_be_deleted(card)
                .name("NO-OUTCOME")
                .optional()
                .replacement_process(|rctx| {
                    rctx.effect.select_own_permanent(
                        "pick anyone",
                        false,
                        |_g, _h| true,
                        |_ctx, _picked| {
                            // No outcome-setter call — outcome stays None.
                        },
                    );
                })
                .build()]
        }
    }

    let mut r = DebugRunner::builder()
        .add_card(fighter("NO-OUTCOME"))
        .add_card(fighter("X"))
        .start();
    r.register_effect("NO-OUTCOME", Arc::new(NoOutcomeCard));
    let parker = r.place_on_field(0, "NO-OUTCOME", None);
    let _x = r.place_on_field(0, "X", None);

    r.game.delete_permanent_with_effects(parker);
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept");
    let pending = r.game.pending_selection.as_ref().unwrap();
    let action = pending.valid_action_ids[0];
    let player = pending.selecting_player;
    r.game.resolve_selection(player, action).expect("pick");

    // outcome was None → original deletion proceeds → parker is gone.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "parker should have been deleted (outcome=None defaults to original event)"
    );
}

#[test]
fn last_write_wins_on_multiple_outcome_setters() {
    let mut r = DebugRunner::builder().add_card(fighter("X")).start();
    let target = r.place_on_field(0, "X", None);

    install_parked(&mut r.game, target);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.cancel_leave();
        ctx.redirect_replacement(Zone::Hand);
    }

    let outcome = r
        .game
        .parked_replacement_outcome_for_test()
        .expect("slot still set");
    assert_eq!(
        outcome,
        ReplacementOutcome::Redirected(Zone::Hand),
        "last write should win"
    );
}

#[test]
#[should_panic(expected = "nested replacement park")]
fn single_outstanding_park_panics_on_double_install() {
    // Manually install parked_replacement, then trigger a second install via
    // the dispatcher post-process hook — should panic in dev builds.

    struct DoubleParkCard;
    impl CardEffect for DoubleParkCard {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![Effect::when_would_be_deleted(card)
                .name("DOUBLE-PARK")
                .optional()
                .replacement_process(|rctx| {
                    rctx.effect
                        .select_own_permanent("x", false, |_g, _h| true, |_ctx, _p| {});
                })
                .build()]
        }
    }

    let mut r = DebugRunner::builder()
        .add_card(fighter("DOUBLE-PARK"))
        .add_card(fighter("X"))
        .start();
    r.register_effect("DOUBLE-PARK", Arc::new(DoubleParkCard));
    let parker = r.place_on_field(0, "DOUBLE-PARK", None);
    let other = r.place_on_field(0, "X", None);

    // Pre-install parked_replacement so the dispatcher hook sees an existing slot.
    install_parked(&mut r.game, other);

    r.game.delete_permanent_with_effects(parker);
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    // The accept-callback runs run_candidate_inner which hits the post-process
    // hook with parked_replacement already Some(_) → debug_assert! fires.
    let _ = r.game.resolve_selection(0, REPLACEMENT_ACCEPT);
}

#[test]
fn synchronous_outcome_preserved_when_process_also_parks() {
    // A process that calls rctx.cancel() AND rctx.effect.select_*(...) should
    // preserve the synchronous Cancelled outcome as the parked default.
    // If the user's nested callback doesn't override, the deletion stays
    // cancelled (rctx.cancel was honored, not silently dropped).

    struct CancelThenParkCard;
    impl CardEffect for CancelThenParkCard {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![Effect::when_would_be_deleted(card)
                .name("CANCEL-THEN-PARK")
                .optional()
                .replacement_process(|rctx| {
                    rctx.cancel(); // synchronous outcome — preserve as parked default
                    rctx.effect.select_own_permanent(
                        "pick anyone",
                        false,
                        |_g, _h| true,
                        |_ctx, _picked| {
                            // Don't override outcome — parked default should be Cancelled
                        },
                    );
                })
                .build()]
        }
    }

    let mut r = DebugRunner::builder()
        .add_card(fighter("CANCEL-THEN-PARK"))
        .add_card(fighter("OTHER"))
        .start();
    r.register_effect("CANCEL-THEN-PARK", Arc::new(CancelThenParkCard));
    let parker = r.place_on_field(0, "CANCEL-THEN-PARK", None);
    let _other = r.place_on_field(0, "OTHER", None);

    r.game.delete_permanent_with_effects(parker);
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept");

    let pending = r.game.pending_selection.as_ref().unwrap();
    let action = pending.valid_action_ids[0];
    let player = pending.selecting_player;
    r.game.resolve_selection(player, action).expect("pick");

    // Synchronous Cancelled outcome should have been preserved as parked default.
    // The empty inner callback didn't override, so deletion should be cancelled.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "synchronous rctx.cancel() should be preserved as parked outcome default; \
         empty nested callback shouldn't drop it"
    );
}

#[test]
fn cancel_leave_outside_parked_scope_panics_in_dev() {
    let mut r = DebugRunner::builder().add_card(fighter("X")).start();
    let _ = r.place_on_field(0, "X", None);

    // No parked_replacement installed — ctx.cancel_leave() should panic in dev.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.cancel_leave();
    }));
    assert!(
        result.is_err(),
        "cancel_leave outside parked scope should debug_assert!"
    );
}
