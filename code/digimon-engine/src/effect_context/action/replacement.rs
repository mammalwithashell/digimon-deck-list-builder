//! Replacement-window handling on `EffectContext` — extracted by mechanic.

#![allow(unused_imports)]
use crate::action::mask::*;
use crate::action::space::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::combat::*;
use crate::digixros::*;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::StepRuntime;
use crate::effect::*;
use crate::effect_context::*;
use crate::enums::*;
use crate::game::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::scheduled_effects::*;
use crate::selection::*;
use crate::token_registry::*;
use crate::trigger_context::*;

impl<'a> EffectContext<'a> {
    /// Cancel the parked leave-the-field event. The carrier stays on the
    /// field; the original deletion / return / etc. is suppressed.
    ///
    /// Writes `ReplacementOutcome::Cancelled` to `Game.parked_replacement.outcome`.
    /// Calling this outside a parked-replacement scope is a `debug_assert!`
    /// panic in dev builds; release builds silently no-op.
    ///
    /// Typical use: inside a `select_*` callback that runs as the body of a
    /// `WhenWouldBeDeleted` replacement-process closure (e.g., Save:
    /// "you may pick a Tamer to slide under instead of being deleted").
    pub fn cancel_leave(&mut self) {
        debug_assert!(
            self.game.parked_replacement.is_some(),
            "cancel_leave called outside a replacement-process callback; \
             the outcome would be silently dropped"
        );
        if let Some(parked) = self.game.parked_replacement.as_mut() {
            parked.outcome = crate::replacement::ReplacementOutcome::Cancelled;
        }
    }

    /// Alias for [`Self::cancel_leave`] for replacement-process callbacks
    /// whose card text names the current replacement rather than "leaving".
    pub fn cancel_current_replacement(&mut self) {
        self.cancel_leave();
    }

    /// Mark the parked replacement as custom-handled — the process body has
    /// already mutated state and the original event should be skipped.
    /// Distinct from `cancel_leave` only at the doc level; both result in
    /// `commit_deferred_outcome` taking the no-op arm.
    ///
    /// Writes `ReplacementOutcome::CustomHandled` to the parked slot.
    /// Calling this outside a parked-replacement scope is a `debug_assert!`
    /// panic in dev builds; release builds silently no-op.
    pub fn handle_replacement(&mut self) {
        debug_assert!(
            self.game.parked_replacement.is_some(),
            "handle_replacement called outside a replacement-process callback"
        );
        if let Some(parked) = self.game.parked_replacement.as_mut() {
            parked.outcome = crate::replacement::ReplacementOutcome::CustomHandled;
        }
    }

    /// Redirect the parked event to a different zone (e.g., Trash → Deck for
    /// Evade, Trash → Hand for return-to-hand replacement).
    ///
    /// Writes `ReplacementOutcome::Redirected(zone)` to the parked slot.
    /// Honored by `commit_deferred_outcome`'s existing redirect arms.
    /// Calling outside a parked-replacement scope is a `debug_assert!` panic
    /// in dev builds; release builds silently no-op.
    pub fn redirect_replacement(&mut self, zone: crate::enums::Zone) {
        debug_assert!(
            self.game.parked_replacement.is_some(),
            "redirect_replacement called outside a replacement-process callback"
        );
        if let Some(parked) = self.game.parked_replacement.as_mut() {
            parked.outcome = crate::replacement::ReplacementOutcome::Redirected(zone);
        }
    }

    /// Substitute a different subject for the parked event. `commit_deferred_outcome`
    /// recursively dispatches the original event against the substituted subject
    /// (e.g., Decoy: replace deletion-target with self).
    ///
    /// Writes `ReplacementOutcome::Substituted(subject)` to the parked slot.
    /// Calling outside a parked-replacement scope is a `debug_assert!` panic
    /// in dev builds; release builds silently no-op.
    pub fn substitute_replacement(&mut self, subject: crate::replacement::ReplacementSubject) {
        debug_assert!(
            self.game.parked_replacement.is_some(),
            "substitute_replacement called outside a replacement-process callback"
        );
        if let Some(parked) = self.game.parked_replacement.as_mut() {
            parked.outcome = crate::replacement::ReplacementOutcome::Substituted(subject);
        }
    }
}
