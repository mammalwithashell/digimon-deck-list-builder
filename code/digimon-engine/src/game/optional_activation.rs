//! The optional-ACTIVATION mark: which parked prompt is the folded "you
//! may" decision of an optional effect.
//!
//! DCGO asks every `IsOptional` effect "use this optional effect?" before its
//! body runs: `ICardEffect.Activate_Optional_Effect_Execute`
//! (`DCGO/Assets/Scripts/Script/ICardEffect.cs:1241-1252`) starts
//! `Activate_Optional`, which opens `OptionalSkill`
//! (`Script/OptionalSkill.cs:97-100`, answered by `SetUseOptional`). The
//! engine asks that "may" in one of three shapes: an outer accept/decline
//! prompt, the pre-cost order prompt of an effect with an activation cost, or
//! -- when neither exists -- folded into the effect's FIRST decision, whose
//! PASS declines the whole effect. This module marks the third shape, so a
//! reader (the DCGO replay) can tell that gate from a choice inside an
//! effect body.
//!
//! Contract:
//! - **First decision only.** `run_queued_effect` opens an
//!   [`OptionalActivationWindow`] for the effect's body. The first leaf step
//!   the DSL step runner reaches claims it ([`Game::claim_first_decision`]);
//!   when that leaf parks its own prompt the window records it, otherwise
//!   (no candidates, auto-resolved, or its tail ran another leaf) the window
//!   is spent and no later decision of the effect is marked. A body that
//!   runs no DSL leaf (a keyword's native body) marks the prompt it parks
//!   directly. The window lives only while the body runs synchronously, so a
//!   pick reached later through a resumed tail never sees it.
//! - **One prompt.** The mark records the prompt it was set on (player,
//!   kind, prompt text, source card, source permanent) and the queued effect
//!   it belongs to ([`OptionalActivationEffect`]). It matches only that
//!   prompt, and it is dropped when that prompt resolves
//!   (`resolve_selection`), when another effect body starts while it is not
//!   the pending prompt, and at every player decision boundary where it is
//!   not the pending prompt ([`Game::sync_optional_activation_mark`]).
//! - **Which effects.** An optional effect (`Effect::optional`) with no outer
//!   prompt and no activation cost, or an effect flagged
//!   `Effect::optional_activation_folded` (keyword bodies that DCGO
//!   activates as an `IsOptional` `ActivateClass` while the engine's effect
//!   itself is mandatory and its body offers the decline).

use crate::card_source::CardHandle;
use crate::enums::{EffectTiming, Keyword, PlayerId};
use crate::permanent::PermanentHandle;
use crate::selection::{PendingSelection, SelectionKind};

use super::Game;

/// The queued effect whose first decision a mark (or window) belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionalActivationEffect {
    pub controller: PlayerId,
    pub source_card: CardHandle,
    pub effect_slot: u8,
    pub timing: EffectTiming,
    /// `Some` for a keyword auto-effect the queue re-synthesizes (an
    /// aura/registry-granted keyword), else `None`.
    pub keyword: Option<Keyword>,
    pub trigger_batch: u32,
}

impl OptionalActivationEffect {
    pub(crate) fn of(qe: &crate::selection::QueuedEffect) -> Self {
        Self {
            controller: qe.controller,
            source_card: qe.source_card,
            effect_slot: qe.effect_slot,
            timing: qe.timing,
            keyword: qe.keyword_effect,
            trigger_batch: qe.trigger_batch,
        }
    }
}

/// The visible identity of one parked prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PromptIdentity {
    player: PlayerId,
    kind: SelectionKind,
    prompt: String,
    source_card: CardHandle,
    source_permanent: Option<PermanentHandle>,
}

impl PromptIdentity {
    fn of(sel: &PendingSelection) -> Self {
        Self {
            player: sel.selecting_player,
            kind: sel.kind,
            prompt: sel.prompt.clone(),
            source_card: sel.source_card,
            source_permanent: sel.source_permanent,
        }
    }
}

/// See the module docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OptionalActivationMark {
    prompt: PromptIdentity,
    effect: OptionalActivationEffect,
}

/// Where an effect body is in reaching its first decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FirstDecision {
    /// No DSL leaf step has run yet.
    Open,
    /// The first leaf step is running.
    Claimed,
    /// The first leaf step parked its own prompt.
    Parked,
    /// The first decision is behind us without a prompt to mark (skipped,
    /// auto-resolved, or a later leaf ran), or was handed off elsewhere.
    Spent,
}

/// The first-decision tracker of the effect body running now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OptionalActivationWindow {
    pub(crate) effect: OptionalActivationEffect,
    /// Cards a prompt of this effect may name as its source: the queued
    /// source card, its attribution card, and the carrier's top card when
    /// the body ran (an inherited keyword's prompt names the carrier).
    pub(crate) cards: Vec<CardHandle>,
    pub(crate) state: FirstDecision,
}

impl Game {
    /// Is the parked prompt the folded "you may" ACTIVATION decision of an
    /// optional effect (see the module docs)? Its PASS declines the whole
    /// effect, as an outer accept/decline would. Read by the DCGO replay.
    pub fn pending_is_optional_activation(&self) -> bool {
        self.pending_optional_activation_effect().is_some()
    }

    /// The queued effect whose folded "you may" decision is the parked
    /// prompt, if it is one (see [`Game::pending_is_optional_activation`]).
    pub fn pending_optional_activation_effect(&self) -> Option<&OptionalActivationEffect> {
        let sel = self.pending_selection.as_ref()?;
        let mark = self.optional_activation_mark.as_ref()?;
        (sel.is_optional && mark.prompt == PromptIdentity::of(sel)).then_some(&mark.effect)
    }

    /// Drop the mark unless it names the prompt pending right now. Called
    /// where a prompt may have left `pending_selection` without resolving:
    /// before an effect body starts and at player decision boundaries.
    pub(crate) fn sync_optional_activation_mark(&mut self) {
        let keep = match (self.pending_selection.as_ref(), self.optional_activation_mark.as_ref()) {
            (Some(sel), Some(mark)) => mark.prompt == PromptIdentity::of(sel),
            _ => false,
        };
        if !keep {
            self.optional_activation_mark = None;
        }
    }

    /// Mark `sel` as the first decision of `window`'s effect when it is that
    /// effect's declinable prompt: optional (PASS legal), asked of the
    /// effect's controller, from one of its cards, and neither an
    /// immediate-replacement accept nor an order prompt (those are asked by
    /// the framework, not by the effect body).
    fn mark_if_first_decision(&mut self, window: &OptionalActivationWindow) {
        let Some(sel) = self.pending_selection.as_ref() else {
            return;
        };
        if sel.is_optional
            && sel.selecting_player == window.effect.controller
            && !matches!(sel.kind, SelectionKind::Replacement | SelectionKind::TriggerOrder)
            && window.cards.contains(&sel.source_card)
        {
            self.optional_activation_mark = Some(OptionalActivationMark {
                prompt: PromptIdentity::of(sel),
                effect: window.effect,
            });
        }
    }

    /// Open a first-decision window for the body about to run, returning the
    /// enclosing one (a nested effect body hides it until it is restored by
    /// [`Game::close_optional_activation_window`]).
    pub(crate) fn open_optional_activation_window(
        &mut self,
        window: Option<OptionalActivationWindow>,
    ) -> Option<OptionalActivationWindow> {
        std::mem::replace(&mut self.optional_activation_window, window)
    }

    /// The body has returned: mark its first decision if it parked one,
    /// restore the enclosing window, and spend that window's claimed leaf if
    /// this nested body left a prompt parked (it is not the leaf's own).
    pub(crate) fn close_optional_activation_window(
        &mut self,
        enclosing: Option<OptionalActivationWindow>,
    ) {
        let window = std::mem::replace(&mut self.optional_activation_window, enclosing);
        if let Some(window) = window {
            if matches!(window.state, FirstDecision::Open | FirstDecision::Parked) {
                self.mark_if_first_decision(&window);
            }
        }
        if self.pending_selection.is_some() {
            if let Some(outer) = self.optional_activation_window.as_mut() {
                if outer.state == FirstDecision::Claimed {
                    outer.state = FirstDecision::Spent;
                }
            }
        }
    }

    /// The DSL step runner reached a leaf step (a selection or synchronous
    /// step, not a control-flow container). Returns `true` when this leaf is
    /// the body's first: the caller then reports its outcome with
    /// [`Game::settle_first_decision`]. A leaf reached while the first one is
    /// still running (its tail ran synchronously) spends the window.
    pub(crate) fn claim_first_decision(&mut self) -> bool {
        let Some(window) = self.optional_activation_window.as_mut() else {
            return false;
        };
        match window.state {
            FirstDecision::Open => {
                window.state = FirstDecision::Claimed;
                true
            }
            FirstDecision::Claimed => {
                window.state = FirstDecision::Spent;
                false
            }
            FirstDecision::Parked | FirstDecision::Spent => false,
        }
    }

    /// The first leaf step returned: it parked its own prompt when nothing
    /// else ran inside it and a prompt is pending; otherwise the first
    /// decision passed without one.
    pub(crate) fn settle_first_decision(&mut self) {
        let parked = self.pending_selection.is_some();
        if let Some(window) = self.optional_activation_window.as_mut() {
            if window.state == FirstDecision::Claimed {
                window.state = if parked { FirstDecision::Parked } else { FirstDecision::Spent };
            }
        }
    }

    /// A decision was skipped without reaching a leaf step (an `optional`
    /// substep whose leading pick has no candidate): when the body's first
    /// decision was still open, it is now behind us.
    pub(crate) fn spend_first_decision(&mut self) {
        if let Some(window) = self.optional_activation_window.as_mut() {
            if window.state == FirstDecision::Open {
                window.state = FirstDecision::Spent;
            }
        }
    }

    /// A native body hands its first decision to a later installation (an
    /// end-of-turn keyword offer waiting for the attack in progress): spend
    /// the window here and return the effect the later prompt belongs to.
    pub(crate) fn defer_first_decision(&mut self) -> Option<OptionalActivationEffect> {
        let window = self.optional_activation_window.as_mut()?;
        if window.state != FirstDecision::Open {
            return None;
        }
        window.state = FirstDecision::Spent;
        Some(window.effect)
    }

    /// Mark the prompt just installed for a deferred first decision (see
    /// [`Game::defer_first_decision`]).
    pub(crate) fn mark_deferred_first_decision(
        &mut self,
        effect: OptionalActivationEffect,
        cards: Vec<CardHandle>,
    ) {
        let window = OptionalActivationWindow { effect, cards, state: FirstDecision::Parked };
        self.mark_if_first_decision(&window);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_runner::DebugRunner;

    fn effect(card: CardHandle) -> OptionalActivationEffect {
        OptionalActivationEffect {
            controller: 0,
            source_card: card,
            effect_slot: 0,
            timing: EffectTiming::OnPlay,
            keyword: None,
            trigger_batch: 0,
        }
    }

    fn window(card: CardHandle) -> OptionalActivationWindow {
        OptionalActivationWindow { effect: effect(card), cards: vec![card], state: FirstDecision::Open }
    }

    #[test]
    fn first_leaf_claims_and_a_nested_leaf_spends() {
        let mut r = DebugRunner::builder().start();
        let g = &mut r.game;
        assert!(!g.claim_first_decision(), "no window: nothing to claim");
        g.optional_activation_window = Some(window(CardHandle(1)));
        assert!(g.claim_first_decision(), "the first leaf claims");
        assert!(!g.claim_first_decision(), "a leaf inside it (its tail) does not");
        g.settle_first_decision();
        assert_eq!(g.optional_activation_window.as_ref().unwrap().state, FirstDecision::Spent);
        assert!(!g.claim_first_decision(), "a later leaf never claims a spent window");
    }

    #[test]
    fn a_first_leaf_without_a_prompt_spends_the_window() {
        let mut r = DebugRunner::builder().start();
        let g = &mut r.game;
        g.optional_activation_window = Some(window(CardHandle(1)));
        assert!(g.claim_first_decision());
        g.settle_first_decision();
        assert_eq!(g.optional_activation_window.as_ref().unwrap().state, FirstDecision::Spent);
    }

    #[test]
    fn a_deferred_first_decision_spends_the_window_once() {
        let mut r = DebugRunner::builder().start();
        let g = &mut r.game;
        g.optional_activation_window = Some(window(CardHandle(1)));
        assert_eq!(g.defer_first_decision(), Some(effect(CardHandle(1))));
        assert_eq!(g.defer_first_decision(), None, "only the first decision defers");
        assert!(!g.claim_first_decision());
    }

    #[test]
    fn sync_without_a_pending_prompt_drops_the_mark() {
        let mut r = DebugRunner::builder().start();
        let g = &mut r.game;
        g.optional_activation_mark = Some(OptionalActivationMark {
            prompt: PromptIdentity {
                player: 0,
                kind: SelectionKind::OwnField,
                prompt: "p".into(),
                source_card: CardHandle(1),
                source_permanent: None,
            },
            effect: effect(CardHandle(1)),
        });
        assert!(g.pending_selection.is_none());
        g.sync_optional_activation_mark();
        assert!(g.optional_activation_mark.is_none());
    }
}
