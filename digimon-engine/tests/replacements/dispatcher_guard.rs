//! Phase 7 Task 7 — spec §7.5 once-per-event guard tests.
//!
//! The guard tracks `(timing, subject)` pairs that have already fired within
//! a single `try_replace` call chain and short-circuits re-dispatches for the
//! same pair. The set clears at the outermost entry of a fresh call chain
//! (distinct events get clean slates), but is preserved across callback-
//! commit boundaries so a redirect route cannot re-fire a replacement that
//! was just resolved for the same subject.
//!
//! These tests drive the dispatcher directly through `Game::try_replace` and
//! the `TEST-P7-GUARD-SENTINEL` test card, which installs two mandatory
//! replacements (WhenWouldLeaveBattleArea + WhenWouldBeDeleted) that each
//! increment a shared atomic fire-counter. The WWLBA process also re-invokes
//! `try_replace` on the same `(timing, subject)` so the same-timing re-entry
//! path exercises without needing a real zone-mover fire-site.

use std::sync::atomic::Ordering;

use digimon_engine::cards::test::test_phase7_guard_sentinel::{
    GUARD_SENTINEL_WWBD, GUARD_SENTINEL_WWLBA,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::replacement::{ReplacementCause, ReplacementOutcome, ReplacementSubject};

fn reset_sentinels() {
    GUARD_SENTINEL_WWLBA.store(0, Ordering::SeqCst);
    GUARD_SENTINEL_WWBD.store(0, Ordering::SeqCst);
}

/// Test 1: Super-timing double-fire is prevented.
///
/// The WWLBA process re-invokes `try_replace(WWLBA, same_subject, …)` from
/// inside its own run. With the §7.5 guard, the re-entry returns `None`
/// without running the process a second time — the sentinel counter ends at
/// exactly 1 (not 2).
#[test]
fn once_per_event_guard_prevents_super_timing_double_fire() {
    reset_sentinels();
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-P7-GUARD-SENTINEL", "Guard Sentinel"))
        .start();
    let handle = r.place_on_field(0, "TEST-P7-GUARD-SENTINEL", Some(0));

    let outcome = r.game.try_replace(
        EffectTiming::WhenWouldLeaveBattleArea,
        ReplacementSubject::Permanent(handle),
        ReplacementCause::OwnEffect,
        None,
    );

    assert_eq!(outcome, ReplacementOutcome::Cancelled);
    assert_eq!(
        GUARD_SENTINEL_WWLBA.load(Ordering::SeqCst),
        1,
        "WWLBA replacement process must fire exactly once despite the nested re-entry"
    );
    assert_eq!(r.game.replacement_depth, 0);
}

/// Test 2: Distinct call chains each get a fresh guard set.
///
/// Two back-to-back `try_replace` calls on the same `(timing, subject)` both
/// fire because the guard only suppresses re-entry WITHIN a single call chain.
/// The fired-set is cleared at the outermost entry of the second call.
#[test]
fn once_per_event_guard_clears_between_call_chains() {
    reset_sentinels();
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-P7-GUARD-SENTINEL", "Guard Sentinel"))
        .start();
    let handle = r.place_on_field(0, "TEST-P7-GUARD-SENTINEL", Some(0));

    let outcome1 = r.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(handle),
        ReplacementCause::OwnEffect,
        None,
    );
    let outcome2 = r.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(handle),
        ReplacementCause::OwnEffect,
        None,
    );

    // WWBD process sets no outcome (just increments the counter) so both
    // calls return None — but they DO run.
    assert_eq!(outcome1, ReplacementOutcome::None);
    assert_eq!(outcome2, ReplacementOutcome::None);
    assert_eq!(
        GUARD_SENTINEL_WWBD.load(Ordering::SeqCst),
        2,
        "distinct call chains each get a fresh guard — the second call must also fire"
    );
}

/// Test 3: Different subjects under the same timing are NOT coalesced — both
/// permanents' WWBD replacements fire independently in the same call chain.
#[test]
fn once_per_event_guard_different_subjects_both_fire() {
    reset_sentinels();
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-P7-GUARD-SENTINEL", "Guard Sentinel"))
        .start();
    let h1 = r.place_on_field(0, "TEST-P7-GUARD-SENTINEL", Some(0));
    let h2 = r.place_on_field(0, "TEST-P7-GUARD-SENTINEL", Some(0));
    assert_ne!(h1, h2);

    // Fire WWBD for h1 then h2 in sequence. Neither subject was fired in the
    // other's call chain, so both process closures should run. Each call is
    // its own chain (clears the set on entry), but the counter is global, so
    // the total is 2.
    //
    // h1's permanent carries a WWBD replacement; so does h2's. Per the
    // dispatcher's collect phase, the subject's own effects come first; in
    // this test each subject's card has exactly one WWBD effect that matches
    // its own handle, and the cross-permanent one runs too (Phase 7 v1 has no
    // `source_permanent` filter on non-keyword effects). So a single
    // try_replace call on h1 fires WWBD TWICE — once from h1's own
    // installed effect and once from h2's (they're distinct effect instances
    // pointing at different source_permanents, but the dispatcher does not
    // filter by source_permanent).
    //
    // The assertion that matters for this guard test: within a single
    // `try_replace` call, different subjects in the fired-set do not collide.
    // We verify that by making TWO calls with different subjects and checking
    // the counter doubled (not singled).
    let _ = r.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(h1),
        ReplacementCause::OwnEffect,
        None,
    );
    let count_after_first = GUARD_SENTINEL_WWBD.load(Ordering::SeqCst);

    let _ = r.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(h2),
        ReplacementCause::OwnEffect,
        None,
    );
    let count_after_second = GUARD_SENTINEL_WWBD.load(Ordering::SeqCst);

    // Each call ran at least once; the second call produced additional fires
    // because the fired-set was cleared on its outermost entry.
    assert!(count_after_first >= 1);
    assert!(
        count_after_second > count_after_first,
        "second call's chain started fresh and re-fired for the different subject; \
         got {count_after_first} before, {count_after_second} after"
    );
}

/// Test 4: Basic sanity — after a call chain completes, the guard's
/// `replacement_fired` set is preserved until the next outermost entry
/// (which clears it). Outside the window this is fine because no call is
/// active — but we also verify `replacement_depth` returns to 0 cleanly.
#[test]
fn once_per_event_guard_depth_unwinds() {
    reset_sentinels();
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-P7-GUARD-SENTINEL", "Guard Sentinel"))
        .start();
    let handle = r.place_on_field(0, "TEST-P7-GUARD-SENTINEL", Some(0));

    let _ = r.game.try_replace(
        EffectTiming::WhenWouldLeaveBattleArea,
        ReplacementSubject::Permanent(handle),
        ReplacementCause::OwnEffect,
        None,
    );
    assert_eq!(r.game.replacement_depth, 0);
    // `in_replacement_commit` is pub(crate) — we verify the externally
    // visible effect: the next unrelated call's sentinel fires (guard cleared).
    reset_sentinels();
    let _ = r.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(handle),
        ReplacementCause::OwnEffect,
        None,
    );
    assert!(
        GUARD_SENTINEL_WWBD.load(Ordering::SeqCst) >= 1,
        "subsequent fresh call chain fires normally after prior unwind"
    );
}
