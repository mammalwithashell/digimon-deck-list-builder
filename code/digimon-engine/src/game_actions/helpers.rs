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
    /// One source-payment event: remove the selected physical cards first,
    /// apply owner-relative Overflow, and enqueue observers without interleaving.
    pub(crate) fn trash_source_prefix_payment_batch(
        &mut self,
        host: PermanentHandle,
        cards: &[CardHandle],
    ) -> usize {
        let cause = crate::trigger_context::EventCause::from(self.infer_effect_cause(host.player));
        let Some(perm) = self
            .player_mut(host.player)
            .battle_area
            .get_mut(host.index as usize)
        else {
            return 0;
        };
        if cards.iter().any(|c| {
            !perm
                .card_sources
                .iter()
                .take(perm.card_sources.len().saturating_sub(1))
                .any(|s| s.handle() == *c)
        }) {
            return 0;
        }
        let host_card = perm.top_card().handle();
        let mut removed = Vec::new();
        for card in cards {
            let i = perm
                .card_sources
                .iter()
                .position(|s| s.handle() == *card)
                .expect("validated physical source");
            removed.push(perm.card_sources.remove(i));
        }
        let count = removed.len();
        for source in &removed {
            self.player_mut(source.owner).trash.push(source.clone());
        }
        self.tick_declarative_effects();
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
        self.apply_ace_overflow_for_sources(&removed);
        // One payment trashes every selected card at the same time: one
        // event (§15-5-2).
        self.enqueue_digivolution_cards_trashed(
            removed
                .iter()
                .map(|source| super::TrashedSource {
                    player: host.player,
                    host,
                    host_card,
                    card: source.handle(),
                })
                .collect(),
            cause,
        );
        count
    }

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
    /// Tier-2 `<De-Digivolve>` rules machinery (§16-12). Fires
    /// `WhenWouldBeDeDigivolved` once, then:
    ///
    /// - §16-12-1 / §16-12-3: the effect's controller DECLARES a number from 1
    ///   up to the number specified, and 0 can't be chosen. When `declaration`
    ///   carries the resolving effect's provenance, `amount` is a printed
    ///   number, and at least 2 cards are trashable, a non-optional count
    ///   choice (1..=min(N, trashable)) parks through the resumable count VM
    ///   (`ResumeSelectKind::DeDigivolveCount`) and this returns 0; the
    ///   declared number is trashed when the choice resolves
    ///   (`de_digivolve_commit_declared`). With 1 trashable card the number
    ///   is forced and no prompt is installed (DCGO `IDegeneration` opens a
    ///   `SelectCountEffect` with `CanNoSelect: false`). `amount = None` is
    ///   the unnumbered pop-everything helper, not a printed <De-Digivolve N>,
    ///   and never declares.
    /// - §16-12-4: the TOP card is never trashed while its level is at most
    ///   `stop_at_level` (the keyword's floor is 3); a top card with no level
    ///   (a Tamer in the stack, §16-12-6's category change) does not stop it.
    ///   The trashing also stops when the Digimon has no digivolution cards.
    /// - §4-8-1 / §16-12-5: every trashed card is the top card when trashed,
    ///   so no digivolution card is trashed; the instance enqueues
    ///   `OnTopCardTrashed` ONCE with every card it trashed (DCGO stacks
    ///   `WhenTopCardTrashed` once per `IDegeneration`), never
    ///   `OnDigivolutionCardTrashed`.
    ///
    /// Returns the number of cards trashed now (0 when a declaration parked).
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
        declaration: Option<crate::resume::ResumeProvenance>,
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
        let outcome = self.try_replace(EffectTiming::WhenWouldBeDeDigivolved, subject, cause, None);
        if self.pending_selection.is_some() {
            return 0;
        }
        let effective_target = match outcome {
            ReplacementOutcome::None => target,
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                return 0;
            }
            ReplacementOutcome::Redirected(_) => {
                debug_assert!(
                    false,
                    "Redirected not meaningful for WhenWouldBeDeDigivolved"
                );
                target
            }
            ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other)) => other,
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(
                    false,
                    "non-Permanent substitute for WhenWouldBeDeDigivolved"
                );
                target
            }
        };
        let target = effective_target;
        let cause =
            crate::trigger_context::EventCause::from(self.infer_effect_cause(target.player));

        let max = amount.unwrap_or(u8::MAX);
        if let (Some(prov), Some(_)) = (declaration, amount) {
            let trashable = self.de_digivolve_trashable_count(target, stop_at_level, max);
            if trashable >= 2 {
                let state = DeDigivolveCountState {
                    host: crate::count_choice::CountChoiceHost {
                        player: target.player,
                        field_index: target.index,
                        battle_identity: self.capture_digivolve_host_identity(target),
                    },
                    stop_at_level,
                    immunity_recheck,
                    cause,
                };
                self.install_de_digivolve_declaration(prov, trashable, state);
                return 0;
            }
        }
        self.de_digivolve_trash_from_top(target, stop_at_level, max, immunity_recheck, cause)
    }

    /// How many cards one `<De-Digivolve>` instance of up to `max` can trash
    /// from `target` (§16-12-1 / §16-12-4): walk down from the top while the
    /// stack keeps a card under the top and the top's level is above the
    /// floor (the live top is read through `level_for_rules`, lower cards by
    /// their printed level); an exposed Digi-Egg ends the walk.
    pub(crate) fn de_digivolve_trashable_count(
        &self,
        target: PermanentHandle,
        stop_at_level: Option<u8>,
        max: u8,
    ) -> u8 {
        let Some(perm) = self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
        else {
            return 0;
        };
        let stack = &perm.card_sources;
        let mut len = stack.len();
        let mut n: u8 = 0;
        while n < max && len > 1 {
            let level = if n == 0 {
                perm.level_for_rules(&self.card_data, &self.modifiers, target)
            } else {
                stack[len - 1].level(&self.card_data)
            };
            if let (Some(floor), Some(level)) = (stop_at_level, level) {
                if level <= floor {
                    break;
                }
            }
            n += 1;
            len -= 1;
            if stack[len - 1].card_kind(&self.card_data) == crate::enums::CardKind::DigiEgg {
                break;
            }
        }
        n
    }

    /// Park the §16-12-1 count declaration: a non-optional count choice with
    /// the values `1..=trashable` for the resolving effect's controller.
    fn install_de_digivolve_declaration(
        &mut self,
        prov: crate::resume::ResumeProvenance,
        trashable: u8,
        state: DeDigivolveCountState,
    ) {
        use crate::count_choice::{CountChoicePage, CountChoiceValue};
        let values = (1..=u64::from(trashable))
            .map(|n| CountChoiceValue {
                declared: n,
                payable: n,
            })
            .collect();
        let Some(page) = CountChoicePage::new(values, Some(state.host)) else {
            // Unreachable (1..=trashable, trashable >= 2, is strictly
            // ascending); never skip the mandatory trash (§16-12-3): fall back
            // to trashing the maximum.
            debug_assert!(false, "<De-Digivolve> declaration page rejected 1..={trashable}");
            if let Some(target) =
                self.resolve_digivolve_host_identity(state.host.player, state.host.battle_identity)
            {
                self.de_digivolve_trash_from_top(
                    target,
                    state.stop_at_level,
                    trashable,
                    state.immunity_recheck,
                    state.cause,
                );
            }
            return;
        };
        {
            let mut ctx = crate::effect_context::EffectContext::new_with_source_kind_and_override(
                self,
                prov.source_card,
                prov.source_permanent,
                prov.source_kind,
                prov.controller,
                prov.override_pin,
            );
            crate::count_choice::install_count_choice(
                &mut ctx,
                page.clone(),
                false,
                "<De-Digivolve>: declare how many cards to trash".into(),
                None,
                Vec::new(),
                crate::dsl_cards::bindings::Bindings::default(),
                crate::dsl_cards::step::StepRuntime::default(),
            );
        }
        if let Some(crate::resume::ResumeFrame::RunTail { select_kind, .. }) = self
            .pending_selection_resume
            .as_mut()
            .and_then(|s| s.frames.last_mut())
        {
            *select_kind = crate::resume::ResumeSelectKind::DeDigivolveCount { page, state };
        }
    }

    /// Resolve a parked §16-12-1 declaration: trash the declared number of
    /// cards from the top of the (still present) declared Digimon.
    pub(crate) fn de_digivolve_commit_declared(
        &mut self,
        state: &DeDigivolveCountState,
        declared: u64,
    ) -> u8 {
        let Some(target) =
            self.resolve_digivolve_host_identity(state.host.player, state.host.battle_identity)
        else {
            return 0;
        };
        let max = u8::try_from(declared).unwrap_or(u8::MAX);
        self.de_digivolve_trash_from_top(
            target,
            state.stop_at_level,
            max,
            state.immunity_recheck,
            state.cause,
        )
    }

    /// Trash up to `max` cards from the top of `target`, one at a time
    /// (§16-12-5), stopping at a top card of level `<= stop_at_level`
    /// (§16-12-4) or when no digivolution card remains, then enqueue
    /// `OnTopCardTrashed` once with every trashed card (§4-8-1: these are top
    /// cards, not digivolution cards).
    fn de_digivolve_trash_from_top(
        &mut self,
        target: PermanentHandle,
        stop_at_level: Option<u8>,
        max: u8,
        immunity_recheck: Option<(crate::enums::PlayerId, crate::enums::EffectSourceKind)>,
        cause: crate::trigger_context::EventCause,
    ) -> u8 {
        let owner = target.player;
        let mut trashed: Vec<CardHandle> = Vec::new();
        let mut exposed_egg = false;

        while (trashed.len() as u8) < max {
            let perm = match self.player(owner).battle_area.get(target.index as usize) {
                Some(p) => p,
                None => break,
            };

            if perm.stack_size() <= 1 {
                break;
            }

            // §16-12-4: the card being trashed is the TOP card; a level 3 (or
            // lower) top card is never trashed. DCGO `IDegeneration` stops on
            // the permanent's own level (`_permanent.Level == 3 &&
            // TopCard.HasLevel`); a level-less top card does not stop it.
            if let (Some(floor), Some(level)) = (
                stop_at_level,
                perm.level_for_rules(&self.card_data, &self.modifiers, target),
            ) {
                if level <= floor {
                    break;
                }
            }

            let popped_card = {
                let stack =
                    &mut self.player_mut(owner).battle_area[target.index as usize].card_sources;
                debug_assert!(stack.len() >= 2, "stack_size-guard failed");
                stack.pop().expect("stack_size-guarded pop")
            };
            trashed.push(popped_card.handle());
            self.player_mut(owner).trash.push(popped_card);
            // The departed top card's continuous text stops applying and the
            // new top card's applies before the next trash.
            self.tick_declarative_effects();

            // Inlined cleanup_exposed_battle_area_digi_egg: if popping exposed
            // a DigiEgg on top, the permanent is deleted and trashing stops.
            if self
                .player(owner)
                .battle_area
                .get(target.index as usize)
                .is_some_and(|perm| {
                    perm.top_card().card_kind(&self.card_data) == crate::enums::CardKind::DigiEgg
                })
            {
                exposed_egg = true;
                break;
            }

            // De-Digivolve trashes ONE card at a time: if the exposed card is
            // now unaffected by this effect (e.g. EX8-073's [All Turns]
            // opponent-Digimon-effect immunity, judge-quiz Q15), the remaining
            // pops are halted. The re-tick above also keeps the registry fresh
            // for any FOLLOW-UP `de_digivolve` call in the same resolving
            // process (repeated `<De-Digivolve 1>` instances, BT19-073).
            if let Some((effect_controller, source_kind)) = immunity_recheck {
                if (trashed.len() as u8) < max
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

        if !trashed.is_empty() {
            let host_card = self
                .player(owner)
                .battle_area
                .get(target.index as usize)
                .map(|perm| perm.top_card().handle())
                .unwrap_or(trashed[trashed.len() - 1]);
            self.enqueue_top_cards_trashed(owner, target, host_card, trashed.clone(), cause);
            self.mark_until_condition_dirty();
            self.reevaluate_until_condition_modifiers_if_dirty();
        }
        if exposed_egg {
            self.delete_permanent_with_effects(target);
        }
        trashed.len() as u8
    }
}

/// A parked §16-12-1 `<De-Digivolve>` count declaration (plain data, clone
/// safe): the declared Digimon by battle identity plus everything the trash
/// needs once the number is chosen.
#[derive(Debug, Clone)]
pub struct DeDigivolveCountState {
    pub host: crate::count_choice::CountChoiceHost,
    pub stop_at_level: Option<u8>,
    pub immunity_recheck: Option<(crate::enums::PlayerId, crate::enums::EffectSourceKind)>,
    pub cause: crate::trigger_context::EventCause,
}
