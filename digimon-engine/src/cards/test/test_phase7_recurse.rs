//! TEST-P7-RECURSE: A mandatory WhenWouldBeDeleted replacement whose
//! process re-invokes `try_replace` on the same subject/timing. Used to
//! verify the depth-guard safety rail (Task 2 §1). Each nested call
//! increments `replacement_depth`; at `MAX_REPLACEMENT_DEPTH` the guard
//! short-circuits to `None` and the chain unwinds.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};
use crate::enums::EffectTiming;
use crate::permanent::PermanentHandle;
use crate::replacement::{ReplacementCause, ReplacementSubject};

pub struct TestP7Recurse;

impl CardEffect for TestP7Recurse {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("Recursive cancel")
            .replacement_process(|rctx| {
                // Re-enter try_replace on the same subject. The dispatcher's
                // depth guard caps recursion; above the cap this returns None
                // and the outer call proceeds to cancel normally.
                let game = &mut *rctx.effect.game;
                if let Some(h) = rctx.effect.source_permanent {
                    let _ = game.try_replace(
                        EffectTiming::WhenWouldBeDeleted,
                        ReplacementSubject::Permanent(h),
                        ReplacementCause::OwnEffect,
                        None,
                    );
                    // After inner returns, still call cancel() on this level
                    // so the outer dispatcher sees a concrete outcome.
                    let _ = PermanentHandle { player: h.player, index: h.index };
                }
                rctx.cancel();
            })
            .build()]
    }
}
