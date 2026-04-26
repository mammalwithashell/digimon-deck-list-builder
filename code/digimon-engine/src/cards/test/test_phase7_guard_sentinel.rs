//! TEST-P7-GUARD-SENTINEL: mandatory replacement effects at two different
//! timings (`WhenWouldLeaveBattleArea` and `WhenWouldBeDeleted`) that each
//! increment a shared atomic fire-counter. Used by Phase 7 Task 7 §7.5
//! once-per-event-guard tests to verify that the guard suppresses double-fires
//! for the same `(timing, subject)` pair within a single call chain, and that
//! the counter resets cleanly between distinct call chains.
//!
//! The super-timing variant also re-invokes `try_replace` on the same subject
//! with `WhenWouldLeaveBattleArea` so the same-timing re-entry path can be
//! exercised without needing a real fire-site.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};
use crate::enums::EffectTiming;
use crate::replacement::{ReplacementCause, ReplacementSubject};

/// Fire-count sentinel incremented by the replacement process. Tests should
/// `.store(0, …)` before exercising the guard and `.load(…)` after.
pub static GUARD_SENTINEL_WWLBA: AtomicU32 = AtomicU32::new(0);
pub static GUARD_SENTINEL_WWBD: AtomicU32 = AtomicU32::new(0);

pub struct TestP7GuardSentinel;

impl CardEffect for TestP7GuardSentinel {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::when_would_leave_battle_area(card)
                .name("Guard sentinel — count + re-enter WWLBA")
                .replacement_process(|rctx| {
                    GUARD_SENTINEL_WWLBA.fetch_add(1, Ordering::SeqCst);
                    // Re-invoke try_replace on the same timing + subject. With
                    // the §7.5 guard, this re-entry returns None without
                    // incrementing the counter again.
                    let game = &mut *rctx.effect.game;
                    if let ReplacementSubject::Permanent(h) = rctx.subject {
                        let _ = game.try_replace(
                            EffectTiming::WhenWouldLeaveBattleArea,
                            ReplacementSubject::Permanent(h),
                            ReplacementCause::OwnEffect,
                            None,
                        );
                    }
                    rctx.cancel();
                })
                .build(),
            Effect::when_would_be_deleted(card)
                .name("Guard sentinel — count (WWBD)")
                .replacement_process(|_rctx| {
                    GUARD_SENTINEL_WWBD.fetch_add(1, Ordering::SeqCst);
                })
                .build(),
        ]
    }
}
