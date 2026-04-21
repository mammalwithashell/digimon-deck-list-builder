//! Replacement-effect framework — "Would*" timings + dispatcher.
//!
//! See docs/superpowers/specs/2026-04-21-would-replacement-timings-design.md.
//!
//! This module lands the pure data types in Task 1. The `try_replace`
//! dispatcher, layering, and PendingSelection::Replacement emission land
//! in Task 2.

use crate::card_source::CardHandle;
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::{PlayerId, Zone};
use crate::permanent::PermanentHandle;

/// Why a state change is happening — consumed by replacement effects that
/// filter on cause (e.g. "cannot be trashed by your opponent's effects").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReplacementCause {
    Battle,
    OwnEffect,
    OpponentEffect,
    SecurityCheck,
    Cost,
}

/// What's about to happen — a permanent leaving the field, a card being
/// trashed from hand, a player about to draw, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplacementSubject {
    Permanent(PermanentHandle),
    Card(CardHandle, Zone),
    Player(PlayerId),
}

/// The outcome a replacement effect sets. Mutually exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplacementOutcome {
    None,
    Cancelled,
    Redirected(Zone),
    Substituted(ReplacementSubject),
    CustomHandled,
}

/// Closure type for passive modifier replacement conditions. Evaluated at
/// try_replace time to decide whether the modifier applies.
pub type ReplacementConditionFn =
    Box<dyn Fn(&EffectReadContext, &ReplacementSubject) -> bool + Send + Sync + 'static>;

/// Closure type for replacement effect processes. Receives a
/// ReplacementContext so the process can mutate state AND set the outcome.
pub type ReplacementProcessFn =
    Box<dyn Fn(&mut ReplacementContext<'_>) + Send + Sync + 'static>;

/// Passed to Would* effect processes. `effect` is the underlying effect ctx;
/// `cause`, `subject`, `original_destination` are snapshot event data; the
/// process sets `outcome` via helpers to tell the dispatcher what to do.
pub struct ReplacementContext<'g> {
    pub effect: &'g mut EffectContext<'g>,
    pub cause: ReplacementCause,
    pub subject: ReplacementSubject,
    pub original_destination: Option<Zone>,
    pub(crate) outcome: ReplacementOutcome,
}

impl<'g> ReplacementContext<'g> {
    pub fn cancel(&mut self) {
        self.outcome = ReplacementOutcome::Cancelled;
    }
    pub fn redirect_to(&mut self, dest: Zone) {
        self.outcome = ReplacementOutcome::Redirected(dest);
    }
    pub fn substitute(&mut self, subject: ReplacementSubject) {
        self.outcome = ReplacementOutcome::Substituted(subject);
    }
    pub fn handled(&mut self) {
        self.outcome = ReplacementOutcome::CustomHandled;
    }

    /// Read-only access to the current outcome.
    pub fn outcome(&self) -> ReplacementOutcome {
        self.outcome
    }
}
