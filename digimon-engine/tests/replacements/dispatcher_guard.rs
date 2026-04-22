//! Phase 7 Task 7 — spec §7.5 once-per-event guard tests.
//!
//! The guard tracks `(timing, subject)` pairs that have already fired within
//! a single `try_replace` call chain and short-circuits re-dispatches for the
//! same pair. The set clears at the outermost entry of a fresh call chain
//! (distinct events get clean slates), but is preserved across callback-
//! commit boundaries so a redirect route cannot re-fire a replacement that
//! was just resolved for the same subject.
//!
//! These scenarios drive the dispatcher directly through `Game::try_replace`
//! and the `TEST-P7-GUARD-SENTINEL` test card, which installs two mandatory
//! replacements (WhenWouldLeaveBattleArea + WhenWouldBeDeleted) that each
//! increment a shared atomic fire-counter. The WWLBA process also re-invokes
//! `try_replace` on the same `(timing, subject)` so the same-timing re-entry
//! path exercises without needing a real zone-mover fire-site.
//!
//! NOTE: the four scenarios below share process-global atomic counters
//! (`GUARD_SENTINEL_WWLBA` / `GUARD_SENTINEL_WWBD`). Cargo's default
//! `--test-threads=N` would race them if each lived in its own `#[test]`, so
//! they are intentionally collapsed into a single `#[test]` that runs them
//! sequentially in one process. Adding new scenarios? Append another
//! `{ reset_sentinels(); ... }` block below — do NOT split into a second
//! `#[test]` unless you also switch to per-scenario atomics.

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

/// Runs all §7.5 guard scenarios sequentially in one process, since they
/// share global atomic counters. See module docs.
#[test]
fn once_per_event_guard_scenarios() {
    // ─── Scenario 1: super-timing double-fire is prevented. ─────────────
    //
    // The WWLBA process re-invokes `try_replace(WWLBA, same_subject, …)` from
    // inside its own run. With the §7.5 guard, the re-entry returns `None`
    // without running the process a second time — the sentinel counter ends
    // at exactly 1 (not 2).
    {
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

    // ─── Scenario 2: distinct call chains each get a fresh guard set. ───
    //
    // Two back-to-back `try_replace` calls on the same `(timing, subject)`
    // both fire because the guard only suppresses re-entry WITHIN a single
    // call chain. The fired-set is cleared at the outermost entry of the
    // second call.
    {
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

    // ─── Scenario 3: different subjects under the same timing are NOT ───
    // coalesced — both permanents' WWBD replacements fire independently in
    // separate call chains.
    {
        reset_sentinels();
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("TEST-P7-GUARD-SENTINEL", "Guard Sentinel"))
            .start();
        let h1 = r.place_on_field(0, "TEST-P7-GUARD-SENTINEL", Some(0));
        let h2 = r.place_on_field(0, "TEST-P7-GUARD-SENTINEL", Some(0));
        assert_ne!(h1, h2);

        // Fire WWBD for h1 then h2 in sequence. Neither subject was fired in
        // the other's call chain, so both process closures should run. Each
        // call is its own chain (clears the set on entry), but the counter is
        // global, so the total is 2.
        //
        // h1's permanent carries a WWBD replacement; so does h2's. Per the
        // dispatcher's collect phase, the subject's own effects come first;
        // in this test each subject's card has exactly one WWBD effect that
        // matches its own handle, and the cross-permanent one runs too (Phase
        // 7 v1 has no `source_permanent` filter on non-keyword effects). So a
        // single try_replace call on h1 fires WWBD TWICE — once from h1's own
        // installed effect and once from h2's (they're distinct effect
        // instances pointing at different source_permanents, but the
        // dispatcher does not filter by source_permanent).
        //
        // The assertion that matters for this guard test: within a single
        // `try_replace` call, different subjects in the fired-set do not
        // collide. We verify that by making TWO calls with different subjects
        // and checking the counter doubled (not singled).
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

        // Each call ran at least once; the second call produced additional
        // fires because the fired-set was cleared on its outermost entry.
        assert!(count_after_first >= 1);
        assert!(
            count_after_second > count_after_first,
            "second call's chain started fresh and re-fired for the different subject; \
             got {count_after_first} before, {count_after_second} after"
        );
    }

    // ─── Scenario 4: depth unwinds cleanly even when guard blocks. ──────
    //
    // After a call chain completes, the `replacement_fired` set is preserved
    // until the next outermost entry (which clears it). Outside the window
    // this is fine because no call is active — but we also verify
    // `replacement_depth` returns to 0 cleanly.
    {
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
        // visible effect: the next unrelated call's sentinel fires (guard
        // cleared).
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
}

// ─── v1 known-limitation pin ───────────────────────────────────────────
//
// See `RUST_ENGINE_API.md` §Phase 7 v1 constraints #6. During a commit-
// continuation, the once-per-event guard blocks ANY replacement targeting
// the same subject regardless of timing or source card. That's broader
// than spec §7.5 strictly requires: a `CannotBeReturnedToDeck` passive on
// a Digimon whose deletion was redirected to the deck should still cancel
// the subsequent `WhenWouldBeReturnedToDeck`, but v1 suppresses it.
//
// This test PINS the current (broken-per-spec) behavior so it's observable
// and regression-tracked. A future Phase 8 pass that narrows the guard to
// key on `(timing, subject, source_card)` (Option A in the Task 7 review)
// should flip this test's assertion to the correct spec-§7.5 behavior.
// See also `docs/RUST_ENGINE_API.md` §Phase 7 v1 constraints.

/// Optional WhenWouldBeDeleted replacement that redirects to Deck on accept.
/// Inline test card — registered via `DebugRunner::register_effect`.
struct OptionalRedirectToDeck;
impl digimon_engine::effect::CardEffect for OptionalRedirectToDeck {
    fn effects(
        &self,
        card: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        use digimon_engine::effect::Effect;
        use digimon_engine::enums::Zone;
        vec![Effect::when_would_be_deleted(card)
            .name("optional redirect-to-deck (test)")
            .optional()
            .replacement_process(|rctx| {
                if let digimon_engine::replacement::ReplacementSubject::Permanent(subject) =
                    rctx.subject
                {
                    let me = rctx.effect.source_permanent;
                    if Some(subject) != me {
                        return;
                    }
                    rctx.redirect_to(Zone::Deck);
                }
            })
            .build()]
    }
}

#[test]
fn commit_continuation_broadening_blocks_different_timing_v1_known_limitation() {
    use std::sync::Arc;

    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    use digimon_engine::replacement::ReplacementCause;

    // Build a DebugRunner with a single card that installs an optional
    // WhenWouldBeDeleted → redirect-to-Deck replacement. Keep a small deck
    // so return_to_deck has a valid destination (the bug is that return_to_deck
    // succeeds when it shouldn't).
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_test_card("TEST-P7-GUARD-PIN", "Guard Pin Victim"))
        .deck(0, &["FILLER"; 3])
        .start();
    r.register_effect("TEST-P7-GUARD-PIN", Arc::new(OptionalRedirectToDeck));

    let victim = r.place_on_field(0, "TEST-P7-GUARD-PIN", Some(0));

    // Install a cause-agnostic `CannotBeReturnedToDeck` passive on the
    // victim. (Default cause_filter for this modifier is OpponentEffect;
    // we set None explicitly to keep the test cause-independent.)
    let mut passive = ModifierEntry::passive_replacement(
        ModifierType::CannotBeReturnedToDeck,
        Expiry::Permanent,
        /* source_player = */ 1,
    );
    passive.cause_filter = None;
    r.game.modifiers.add(victim, passive);

    let deck_before = r.game.player(0).deck.len();

    // Trigger deletion with OpponentEffect cause so both the WWBD redirect
    // and the passive's cause_filter (if any) would accept.
    r.game
        .delete_permanent_with_cause(victim, ReplacementCause::OpponentEffect);

    // The optional redirect should have installed a PendingSelection.
    assert!(
        r.game.pending_selection.is_some(),
        "optional redirect-to-deck must install a PendingSelection::Replacement"
    );

    // Accept the redirect. This sets `in_replacement_commit = true` and
    // calls `return_to_deck` → dispatches WhenWouldBeReturnedToDeck →
    // broadening guard blocks the passive's cancel → v1 bug: victim lands
    // in the deck instead of being protected.
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the optional redirect");

    // ── v1 PINNED ASSERTION ──────────────────────────────────────────
    // Victim is in the deck (broadening suppressed the passive cancel).
    // Spec-correct behavior would be: victim stays on field (passive
    // cancels the return-to-deck). Phase 8 narrowing should flip this.
    let deck_after = r.game.player(0).deck.len();
    let on_field_after = r.battle_area_size(0);
    assert_eq!(
        deck_after,
        deck_before + 1,
        "v1 limitation: victim lands in the deck despite CannotBeReturnedToDeck — \
         the commit-continuation broadening blocks the passive cancel. \
         If this assertion starts failing, Phase 8 narrowed the guard — \
         flip this test to assert `deck_after == deck_before && on_field_after == 1`."
    );
    assert_eq!(
        on_field_after, 0,
        "v1 limitation: victim left the field (see above)"
    );
    assert_eq!(r.game.replacement_depth, 0);
}
