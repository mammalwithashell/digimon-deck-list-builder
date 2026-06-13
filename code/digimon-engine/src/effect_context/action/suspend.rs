//! Suspend / unsuspend mutations on `EffectContext` — extracted by mechanic.

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
    /// Suspend a permanent and fire `OnSuspend` observers.
    /// Delegates to `Game::suspend` — the canonical single-target chokepoint.
    pub fn suspend(&mut self, target: PermanentHandle) {
        if !self.can_affect_permanent(target) {
            return;
        }
        // Effect-context suspends are effect-initiated by definition —
        // tags the OnSuspend event for `event_is_effect_initiated`
        // ("when an EFFECT suspends…", G-SUSPEND-EFFECT-INITIATED).
        self.game.suspend_with_cause(target, true);
    }

    /// Pay the source permanent's suspend-self activation cost.
    ///
    /// Used as the closure body for [`crate::effect::EffectBuilder::activation_cost`]
    /// on Tamer triggered abilities like "by suspending this Tamer, gain 1
    /// memory" (BT4-097 / BT8-090 / BT13-101 family). Returns `false` if
    /// the source permanent is gone (extremely unlikely mid-trigger) or
    /// is already suspended — in which case the body silently aborts and
    /// the OPT slot is consumed by the queue dispatcher. Returns `true`
    /// after delegating to [`Self::suspend`] (which fires `OnSuspend`
    /// observers and the canonical single-target chokepoint).
    ///
    /// No-approximations note: this helper does NOT prompt — the player's
    /// "may you accept" prompt belongs to [`crate::effect::EffectBuilder::optional`]
    /// and runs BEFORE the cost. The cost is intrinsic to the trigger,
    /// not a player decision (Working Rule 17).
    pub fn suspend_self_as_cost(&mut self) -> bool {
        let Some(handle) = self.source_permanent else {
            return false;
        };
        let already_suspended = self
            .source_permanent()
            .map(|perm| perm.is_suspended)
            .unwrap_or(true);
        if already_suspended {
            return false;
        }
        // A `CannotSuspend` carrier cannot pay a suspend-self cost (DCGO
        // `CanActivateSuspendCostEffect` → `CanSuspend`; prohibition
        // precedence 15-1-3). Without this the chokepoint gate inside
        // `Game::suspend_with_cause` would silently no-op the suspension
        // while this helper reported the cost as paid.
        if self
            .game
            .modifiers
            .has(handle, ModifierType::CannotSuspend)
        {
            return false;
        }
        self.suspend(handle);
        true
    }

    /// Unsuspend a permanent and fire `OnUnsuspend` observers.
    /// Delegates to `Game::unsuspend` — the canonical single-target chokepoint.
    pub fn unsuspend(&mut self, target: PermanentHandle) {
        if !self.can_affect_permanent(target) {
            return;
        }
        // See `suspend` — the effect-initiated tag's unsuspend twin.
        self.game.unsuspend_with_cause(target, true);
    }
}
