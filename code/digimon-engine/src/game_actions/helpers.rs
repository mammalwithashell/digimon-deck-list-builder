//! Tier-2 helper primitives added by the facade-decomposition refactor
//! (decompose-engine-facade-by-mechanic). All `impl Game`. See RUST_ENGINE_API.md §3.
#![allow(unused_imports)]
use super::*;
use crate::card_source::*;
use crate::enums::*;
use crate::permanent::*;
use crate::replacement::*;
use crate::trigger_context::*;

impl Game {
    /// Trash one already-removed digivolution source to `trash_owner`'s trash
    /// and fire `OnDigivolutionCardTrashed` for it on `fire_target`.
    ///
    /// The single Tier-2 push+fire primitive for source-trashing where the host
    /// is known BEFORE the push and the cause is the standard effect cause
    /// (`EventCause::from(infer_effect_cause(fire_target.player))`). Callers own
    /// the removal and host-card derivation (those differ per call site). Sites
    /// that derive the host AFTER the push (`trash_top_source`,
    /// `trash_bottom_face_down_source`) or use a non-standard cause
    /// (`EventCause::Return` in the under-tamer drain) intentionally do NOT use
    /// this helper. See the `engine-effect-context-layering` capability spec.
    pub(crate) fn trash_source_and_fire(
        &mut self,
        trash_owner: PlayerId,
        fire_target: PermanentHandle,
        removed: CardSource,
        host_card: crate::card_source::CardHandle,
    ) {
        let cause =
            crate::trigger_context::EventCause::from(self.infer_effect_cause(fire_target.player));
        self.trash_source_and_fire_with_cause(trash_owner, fire_target, removed, host_card, cause);
    }

    /// Like `trash_source_and_fire` but with an explicit `EventCause` for call
    /// sites that attribute the trash to a non-standard cause (`Return` for the
    /// under-tamer source drain, `Cost` for armor-purge). `host_card` must be
    /// resolved by the caller (it differs per site: pre-removal top, post-pop
    /// promoted top, etc.).
    pub(crate) fn trash_source_and_fire_with_cause(
        &mut self,
        trash_owner: PlayerId,
        fire_target: PermanentHandle,
        removed: CardSource,
        host_card: crate::card_source::CardHandle,
        cause: crate::trigger_context::EventCause,
    ) {
        let source_card = removed.handle();
        self.player_mut(trash_owner).trash.push(removed);
        self.fire_digivolution_card_trashed(
            fire_target.player,
            fire_target,
            host_card,
            source_card,
            cause,
        );
    }

    /// Fire the security-removed observer fan-out (Tier 2). `effect_player` is
    /// the controller of the effect that removed the security card (the source
    /// attribution for the observer). Relocated from the facade so the inline
    /// `fire_effect_security_removal` dispatch lives in Tier 2.
    pub(crate) fn fire_security_removed_observers(
        &mut self,
        defender: PlayerId,
        effect_player: PlayerId,
        card: CardSource,
        destination: crate::selection::SecurityRemovalDestination,
    ) {
        let observer_player = self.next_clockwise(defender);
        let cause = crate::trigger_context::EventCause::from(self.infer_effect_cause(defender));
        self.fire_effect_security_removal(
            defender,
            observer_player,
            effect_player,
            cause,
            card,
            destination,
        );
    }

    /// Tier-2 replacement-window dispatch for facade "would-be-X" operations.
    /// Fires `timing` for `subject` (cause inferred from `cause_player`), then
    /// reports whether the operation may PROCEED: `false` if a selection parked
    /// or the window Cancelled/CustomHandled the action; `true` otherwise.
    /// `Redirected`/`Substituted` are not expected here (debug-asserted) — the
    /// substitute-retargeting cases (e.g. de_digivolve) keep their own logic.
    ///
    /// Centralizes the `try_replace` call out of the Tier-3 facade (placement
    /// rule §engine-effect-context-layering). Behavior is identical to the
    /// inlined block each caller previously held.
    pub(crate) fn would_replacement_proceeds(
        &mut self,
        timing: crate::enums::EffectTiming,
        subject: crate::replacement::ReplacementSubject,
        cause_player: PlayerId,
        redirect: Option<crate::enums::Zone>,
    ) -> bool {
        use crate::replacement::ReplacementOutcome;
        let cause = self.infer_effect_cause(cause_player);
        let outcome = self.try_replace(timing, subject, cause, redirect);
        if self.pending_selection.is_some() {
            return false;
        }
        match outcome {
            ReplacementOutcome::None => true,
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => false,
            ReplacementOutcome::Redirected(_) => {
                debug_assert!(false, "Redirected not supported for {:?} v1", timing);
                true
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(false, "Substituted not supported for {:?} v1", timing);
                true
            }
        }
    }

    /// Like `would_replacement_proceeds`, but the caller may proceed ONLY if the
    /// window returned `None` (no selection parked AND no non-None outcome).
    /// Used by `place_self_option_at_security`, which restores its pending state
    /// and bails on ANY interception.
    pub(crate) fn would_replacement_is_clear(
        &mut self,
        timing: crate::enums::EffectTiming,
        subject: crate::replacement::ReplacementSubject,
        cause_player: PlayerId,
        redirect: Option<crate::enums::Zone>,
    ) -> bool {
        let cause = self.infer_effect_cause(cause_player);
        let outcome = self.try_replace(timing, subject, cause, redirect);
        self.pending_selection.is_none()
            && matches!(outcome, crate::replacement::ReplacementOutcome::None)
    }

    /// Tier-2 battle-area source-stack primitives. The facade (effect_context)
    /// must not index `battle_area[..]` directly (placement rule §3); these
    /// encapsulate the single stack mutations it needs.
    pub(crate) fn digivolve_permanent_in_place(
        &mut self,
        target: PermanentHandle,
        card: CardSource,
    ) {
        let turn = self.turn_count;
        self.player_mut(target.player).battle_area[target.index as usize].digivolve(card, turn);
    }

    pub(crate) fn remove_source_from_permanent(
        &mut self,
        target: PermanentHandle,
        source_index: usize,
    ) -> CardSource {
        self.player_mut(target.player).battle_area[target.index as usize]
            .card_sources
            .remove(source_index)
    }

    pub(crate) fn insert_source_into_permanent(
        &mut self,
        target: PermanentHandle,
        source_index: usize,
        card: CardSource,
    ) {
        self.player_mut(target.player).battle_area[target.index as usize]
            .card_sources
            .insert(source_index, card);
    }

    /// Remove the sources at `indices` from `perm` (removing in descending
    /// index order so earlier removals don't shift later indices). Returns the
    /// removed sources in removal order (i.e. highest-index first); callers
    /// that want bottom-to-top order `reverse()` the result.
    pub(crate) fn remove_sources_from_permanent(
        &mut self,
        perm: PermanentHandle,
        indices: &[usize],
    ) -> Vec<CardSource> {
        let p = &mut self.player_mut(perm.player).battle_area[perm.index as usize];
        let mut out = Vec::with_capacity(indices.len());
        for &idx in indices.iter().rev() {
            out.push(p.card_sources.remove(idx));
        }
        out
    }

    /// Low-level source-attribution helper for tests and engine internals.
    ///
    /// Uses the standard De-Digivolve floor (`stop_at_level = Some(3)`) and
    /// returns whether at least one card was popped. Replacement windows are
    /// resolved by `EffectContext::de_digivolve` under the supplied source
    /// attribution. Production card effects should prefer an existing
    /// `EffectContext` so `can_affect_permanent` and source-kind metadata come
    /// from the real resolving card.
    #[doc(hidden)]
    /// Tier-2 de-digivolve rules machinery: fire `WhenWouldBeDeDigivolved`
    /// once, then pop digivolution sources (honoring `stop_at_level` and the
    /// `amount` cap), trash each popped source firing
    /// `OnDigivolutionCardTrashed`, and clean up a newly-exposed DigiEgg.
    /// Returns the number of sources popped.
    ///
    /// Relocated from `EffectContext::de_digivolve` so the rules machinery
    /// lives in Tier 2 (placement rule §engine-effect-context-layering). The
    /// facade `de_digivolve` is now a thin `can_affect_permanent` guard that
    /// delegates here.
    ///
    /// `immunity_recheck` — De-Digivolve trashes one card at a time, and a
    /// newly-exposed top card's continuous text applies IMMEDIATELY between
    /// trashes (judge-quiz Q15: LordKnightmon (X Antibody)'s repeated
    /// `<De-Digivolve 1>` is halted once Gallantmon (X Antibody) EX8-073
    /// becomes the top card and its [All Turns] opponent-Digimon-effect
    /// immunity turns on). When `Some((effect_controller, source_kind))`,
    /// each pop is followed by a declarative re-tick (so the new top card's
    /// continuous effects materialize) and the next pop is gated on
    /// `permanent_is_unaffected_by_effect`. The `WhenWouldBeDeDigivolved`
    /// replacement window still fires once at entry (Phase 7 Task 4).
    pub(crate) fn de_digivolve_core(
        &mut self,
        target: PermanentHandle,
        stop_at_level: Option<u8>,
        amount: Option<u8>,
        immunity_recheck: Option<(crate::enums::PlayerId, crate::enums::EffectSourceKind)>,
    ) -> u8 {
        use crate::enums::EffectTiming;
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // Phase 7 Task 4: fire WhenWouldBeDeDigivolved once at entry (not
        // per iteration of the popping loop). Substitute(Permanent) retargets
        // the loop at another permanent; v1 does not support "reduce N" via
        // mutable ctx — scripts that want to reduce N should cancel and
        // re-call with a lower amount.
        let cause = self.infer_effect_cause(target.player);
        let subject = ReplacementSubject::Permanent(target);
        let outcome =
            self.try_replace(EffectTiming::WhenWouldBeDeDigivolved, subject, cause, None);
        if self.pending_selection.is_some() {
            return 0;
        }
        let effective_target = match outcome {
            ReplacementOutcome::None => target,
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                return 0;
            }
            ReplacementOutcome::Redirected(_) => {
                debug_assert!(false, "Redirected not meaningful for WhenWouldBeDeDigivolved");
                target
            }
            ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other)) => other,
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(false, "non-Permanent substitute for WhenWouldBeDeDigivolved");
                target
            }
        };
        let target = effective_target;

        let max = amount.unwrap_or(u8::MAX);
        let mut popped: u8 = 0;

        while popped < max {
            let perm = match self.player(target.player).battle_area.get(target.index as usize) {
                Some(p) => p,
                None => break,
            };

            if perm.stack_size() <= 1 {
                break;
            }

            let next_top_level = {
                let stack = perm.digivolution_cards();
                let next_top = &stack[stack.len() - 2];
                next_top.level(&self.card_data)
            };

            if let (Some(floor), Some(nt_level)) = (stop_at_level, next_top_level) {
                if nt_level < floor {
                    break;
                }
            }

            let owner = target.player;
            let (popped_card, host_card) = {
                let p = self.player_mut(owner);
                let stack = &mut p.battle_area[target.index as usize].card_sources;
                debug_assert!(stack.len() >= 2, "stack_size-guard failed");
                let popped_card = stack.pop().expect("stack_size-guarded pop");
                let host_card = stack
                    .last()
                    .map(|source| source.handle())
                    .unwrap_or_else(|| popped_card.handle());
                (popped_card, host_card)
            };
            let source_card = popped_card.handle();
            self.player_mut(owner).trash.push(popped_card);
            self.fire_digivolution_card_trashed(
                owner,
                target,
                host_card,
                source_card,
                crate::trigger_context::EventCause::from(self.infer_effect_cause(owner)),
            );
            popped += 1;

            // Inlined cleanup_exposed_battle_area_digi_egg: if popping exposed
            // a DigiEgg on top, delete the permanent and stop.
            let exposed = self
                .player(target.player)
                .battle_area
                .get(target.index as usize)
                .is_some_and(|perm| {
                    perm.top_card().card_kind(&self.card_data) == crate::enums::CardKind::DigiEgg
                });
            if exposed {
                self.delete_permanent_with_effects(target);
                break;
            }

            // De-Digivolve trashes ONE card at a time: the newly-exposed top
            // card's continuous text is live before the next trash.
            // Re-materialize declarative effects and re-check effect
            // immunity — if the exposed card is now unaffected by this
            // effect (e.g. EX8-073's [All Turns] opponent-Digimon-effect
            // immunity, judge-quiz Q15), the remaining pops are halted. The
            // re-tick also keeps the registry fresh for any FOLLOW-UP
            // `de_digivolve` call in the same resolving process (repeated
            // `<De-Digivolve 1>` instances, BT19-073).
            if let Some((effect_controller, source_kind)) = immunity_recheck {
                self.tick_declarative_effects();
                if popped < max
                    && self.permanent_is_unaffected_by_effect(
                        target,
                        effect_controller,
                        source_kind,
                    )
                {
                    break;
                }
            }
        }

        popped
    }
}
