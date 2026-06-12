//! Modifier / keyword / immunity grants on `EffectContext` — extracted by mechanic.

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
    pub fn add_dp_modifier(&mut self, target: PermanentHandle, value: i32, expiry: Expiry) {
        // Single source of truth for the gate lives in `add_modifier`.
        self.add_modifier(target, ModifierType::ChangeDp, value, expiry);
    }

    pub fn add_declarative_dp_modifier(
        &mut self,
        target: PermanentHandle,
        value: i32,
        expiry: Expiry,
    ) {
        self.add_declarative_modifier(target, ModifierType::ChangeDp, value, expiry);
    }

    pub fn add_modifier(
        &mut self,
        target: PermanentHandle,
        modifier: ModifierType,
        value: i32,
        expiry: Expiry,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        // `*NextTurn` expiries installed during the about-to-end turn must
        // skip that turn-end (otherwise they expire one turn early). The
        // engine knows the current turn at install time, so compute it
        // here — every `add_modifier` / `add_dp_modifier` caller (DSL and
        // hand-written) gets correct "until end of next turn" semantics.
        let pending_skips = crate::modifiers::pending_skips_for_install(
            expiry,
            self.player,
            self.game.turn_player(),
        );
        self.game.modifiers.add(
            target,
            ModifierEntry::simple(modifier, value, expiry, self.player)
                .with_pending_skips(pending_skips),
        );
        self.game.mark_until_condition_dirty();
        // NOTE: a DP reduction to ≤0 does NOT delete the Digimon here. Deletion
        // is a state-based action (rule 17-1-2-2 / `G-NO-GENERAL-ZERO-DP-RULES-CHECK`)
        // that runs only after the ongoing effect or rule action finishes — see
        // `Game::run_state_based_rules_check`, invoked at the outermost
        // `drain_effect_queue` boundary. Deleting inline here would fire
        // mid-effect and break the judge timing (Q6: Pillomon at 0 DP survives
        // until Flame Hellscythe resolves; Q13/Q14: ShoeShoemon survives until
        // Nyabootmon's [When Digivolving] fully resolves).
    }

    /// Like [`add_modifier`], but carries a structured [`ModifierPayload`]
    /// (e.g. `SynthIdentity` for `TreatAsDigimon`). Honors the same
    /// `can_affect_permanent` guard and `*NextTurn` pending-skip semantics.
    /// Used by the DSL `add_modifier` lowering when a `synth_identity:`
    /// block is present, and available to raw_rust scripts.
    pub fn add_modifier_with_payload(
        &mut self,
        target: PermanentHandle,
        modifier: ModifierType,
        value: i32,
        expiry: Expiry,
        payload: crate::modifiers::ModifierPayload,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        let pending_skips = crate::modifiers::pending_skips_for_install(
            expiry,
            self.player,
            self.game.turn_player(),
        );
        self.game.modifiers.add(
            target,
            ModifierEntry::simple(modifier, value, expiry, self.player)
                .with_payload(payload)
                .with_pending_skips(pending_skips),
        );
        self.game.mark_until_condition_dirty();
    }

    pub fn add_declarative_modifier(
        &mut self,
        target: PermanentHandle,
        modifier: ModifierType,
        value: i32,
        expiry: Expiry,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game.modifiers.add(
            target,
            ModifierEntry::materialized_declarative(
                modifier,
                value,
                expiry,
                self.source_permanent,
                self.player,
            ),
        );
        self.game.mark_until_condition_dirty();
    }

    /// Materialized-declarative install carrying an effect-immunity filter
    /// (continuous controlled immunity - Q28 / BT20-059). Tick-ephemeral
    /// like every materialized declarative; the floating descriptor owns
    /// the lifetime.
    pub fn add_declarative_modifier_with_immunity(
        &mut self,
        target: PermanentHandle,
        modifier: ModifierType,
        value: i32,
        expiry: Expiry,
        immunity: crate::modifiers::EffectImmunityFilter,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game.modifiers.add(
            target,
            ModifierEntry::materialized_declarative(
                modifier,
                value,
                expiry,
                self.source_permanent,
                self.player,
            )
            .with_effect_immunity_filter(immunity),
        );
        self.game.mark_until_condition_dirty();
    }

    pub fn add_declarative_modifier_with_payload(
        &mut self,
        target: PermanentHandle,
        modifier: ModifierType,
        value: i32,
        expiry: Expiry,
        payload: crate::modifiers::ModifierPayload,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game.modifiers.add(
            target,
            ModifierEntry::materialized_declarative(
                modifier,
                value,
                expiry,
                self.source_permanent,
                self.player,
            )
            .with_payload(payload),
        );
        self.game.mark_until_condition_dirty();
    }

    /// Install a `Expiry::UntilCondition`-scoped modifier with a runtime
    /// eviction predicate. Mirrors `add_modifier` (honors the
    /// `can_affect_permanent` guard) but tags the entry with the
    /// supplied `UntilConditionFn`. The UntilCondition controller (PR
    /// #458) evaluates the predicate after every mutation event; once
    /// the predicate returns false, the entry is removed and the
    /// printed-semantics rule applies — `false → true` does NOT
    /// re-install. Used by Track H's `while_condition` aura lowering;
    /// also exposed for raw_rust card scripts that need the same gate.
    pub fn add_modifier_with_until_condition(
        &mut self,
        target: PermanentHandle,
        modifier: ModifierType,
        value: i32,
        predicate: crate::modifiers::UntilConditionFn,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game.modifiers.add(
            target,
            ModifierEntry::simple(modifier, value, Expiry::UntilCondition, self.player)
                .with_until_condition(predicate),
        );
        self.game.mark_until_condition_dirty();
    }

    /// Track H §4 — grant a keyword scoped to `Expiry::UntilCondition`
    /// with a runtime predicate. Mirrors `add_modifier_with_until_condition`
    /// for keyword grants. The UntilCondition controller (PR #458)
    /// evicts on the first false transition; printed-semantics rule
    /// holds (no re-install on false → true). Used by Track H's
    /// `while_condition` aura lowering for keyword grants and by
    /// raw_rust card scripts.
    pub fn grant_keyword_with_until_condition(
        &mut self,
        target: PermanentHandle,
        keyword: Keyword,
        predicate: crate::modifiers::UntilConditionFn,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game.modifiers.grant_keyword_with_until_condition(
            target,
            keyword,
            predicate,
            self.player,
        );
        self.game.mark_until_condition_dirty();
    }

    /// Track H §3 — grant a triggered effect to `carrier` (DCGO
    /// `AddSkillClass.cs` analog). The body fires when the carrier's
    /// matching `timing` event drains, with `EffectContext::source_card`
    /// = grantor (this effect's source) and `EffectContext::source_permanent`
    /// = carrier — mirroring DCGO's
    /// `EffectSourceCard` / `EffectSourcePermanent` distinction.
    ///
    /// v1 dispatch covers `EffectTiming::OnDeletion`; calls with other
    /// timings install the entry on the registry but the dispatcher
    /// hook only consults `OnDeletion` for now (extend
    /// `Game::fire_granted_triggered_effects` and its callers as
    /// further timings come online).
    ///
    /// `expiry` follows the same semantics as `add_modifier`:
    /// `Permanent` for "until carrier leaves the field" (clears via
    /// `clear_permanent`); `EndOfTurn` / `EndOfYourTurn` /
    /// `EndOfOpponentsTurn` for turn-bound grants. The
    /// `can_affect_permanent` guard is honored so opponent-effect
    /// immunities suppress the install.
    pub fn grant_triggered_effect<F>(
        &mut self,
        carrier: PermanentHandle,
        timing: crate::enums::EffectTiming,
        expiry: Expiry,
        body: F,
    ) where
        F: Fn(&mut EffectContext<'_>) + Send + Sync + 'static,
    {
        if !self.can_affect_permanent(carrier) {
            return;
        }
        // Phase 4i — allocate a body id and register the closure in the
        // game-level body registry so the queue-based drainer can fetch
        // it at fire time. The same Arc is also stored on the entry for
        // direct/inline-fire compat (legacy `fire_granted_triggered_effects`
        // path remains, though Phase 4b's drain hook is now the primary
        // dispatcher).
        self.game.next_granted_effect_id = self.game.next_granted_effect_id.saturating_add(1);
        let body_id = self.game.next_granted_effect_id;
        let body_arc: crate::modifiers::GrantedEffectBody = std::sync::Arc::new(body);
        self.game
            .granted_effect_bodies
            .insert(body_id, body_arc.clone());
        self.game.modifiers.add_granted_triggered(
            carrier,
            crate::modifiers::GrantedTriggeredEffect {
                timing,
                source_card: self.source_card,
                source_player: self.player,
                expiry,
                body_id,
                body: body_arc,
            },
        );
    }

    /// Declarative-tick variant of `add_effect_immunity_modifier` — installs
    /// a MATERIALIZED filtered `CannotBeAffected` entry that
    /// `tick_declarative_effects` clears and re-installs each tick. Used by
    /// the aura `effect_immunity` slot (G-DSL-AURA-EFFECT-IMMUNITY) for
    /// continuous printed immunities like EX8-073's "[All Turns] While you
    /// have 0 or less memory, this Digimon isn't affected by the effects of
    /// your opponent's Digimon". `source_kind: None` ⇒ any source kind.
    pub fn add_declarative_effect_immunity_modifier(
        &mut self,
        target: PermanentHandle,
        source_kind: Option<EffectSourceKind>,
        controller: EffectControllerFilter,
    ) {
        self.game.modifiers.add(
            target,
            ModifierEntry::materialized_declarative(
                ModifierType::CannotBeAffected,
                0,
                Expiry::Permanent,
                self.source_permanent,
                self.player,
            )
            .with_effect_immunity_filter(EffectImmunityFilter {
                source_kind,
                controller,
            }),
        );
        self.game.mark_until_condition_dirty();
    }

    pub fn add_effect_immunity_modifier(
        &mut self,
        target: PermanentHandle,
        source_kind: EffectSourceKind,
        controller: EffectControllerFilter,
        expiry: Expiry,
    ) -> bool {
        if !self.can_affect_permanent(target) {
            return false;
        }
        self.game.modifiers.add(
            target,
            ModifierEntry::simple(ModifierType::CannotBeAffected, 0, expiry, self.player)
                .with_effect_immunity_filter(EffectImmunityFilter {
                    source_kind: Some(source_kind),
                    controller,
                }),
        );
        self.game.mark_until_condition_dirty();
        true
    }

    /// Grant the common narrow protection bundle for text like "can't be
    /// returned to hand or deck or de-digivolved by your opponent's effects."
    ///
    /// These are passive replacement modifiers with the default
    /// `OpponentEffect` cause filter, not broad `CannotBeAffected` immunity.
    pub fn grant_zone_return_immunity_to_opponent_effects(
        &mut self,
        target: PermanentHandle,
        expiry: Expiry,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        for modifier in [
            ModifierType::CannotBeReturnedToHand,
            ModifierType::CannotBeReturnedToDeck,
            ModifierType::CannotBeDeDigivolved,
        ] {
            self.game.modifiers.add(
                target,
                ModifierEntry::passive_replacement(modifier, expiry, self.player),
            );
        }
        self.game.mark_until_condition_dirty();
    }

    /// PUPPETS-G024 — grant the narrow "can't have its DP reduced by your
    /// opponent's effects and isn't affected by ＜De-Digivolve＞ effects [by
    /// your opponent's effects]" protection bundle (BT16-055 Namakemon's
    /// high-security clause).
    ///
    /// Both protections are genuinely opponent-effect-scoped:
    ///   - `ImmuneFromDPMinus` is installed with an
    ///     `EffectImmunityFilter { controller: OpponentOnly }` so
    ///     `Game::effective_dp` suppresses only negative `ChangeDp`
    ///     deltas whose source is an opponent effect — the controller's
    ///     own DP-reduction still applies.
    ///   - `CannotBeDeDigivolved` is installed via the
    ///     `passive_replacement()` route so its
    ///     `default_passive_cause_filter` (`ReplacementCause::OpponentEffect`)
    ///     takes effect — own-side De-Digivolve still applies.
    ///
    /// Unrelated opponent effects (non-DP-reduction, non-De-Digivolve) are
    /// not affected — this is a category-scoped protection, not blanket
    /// `CannotBeAffected` immunity.
    pub fn grant_narrow_opponent_effect_protection(
        &mut self,
        target: PermanentHandle,
        expiry: Expiry,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game.modifiers.add(
            target,
            ModifierEntry::simple(ModifierType::ImmuneFromDPMinus, 0, expiry, self.player)
                .with_effect_immunity_filter(EffectImmunityFilter {
                    source_kind: None,
                    controller: EffectControllerFilter::OpponentOnly,
                }),
        );
        self.game.modifiers.add(
            target,
            ModifierEntry::passive_replacement(
                ModifierType::CannotBeDeDigivolved,
                expiry,
                self.player,
            ),
        );
        self.game.mark_until_condition_dirty();
    }

    pub fn ignore_option_color_requirement(&mut self, target_player: PlayerId, expiry: Expiry) {
        self.game.modifiers.add_player_modifier(
            target_player,
            PlayerModifierEntry::simple(
                ModifierType::IgnoreColorRequirement,
                0,
                expiry,
                None,
                self.player,
            ),
        );
        self.game.mark_until_condition_dirty();
    }

    pub fn add_declarative_player_modifier(
        &mut self,
        target_player: PlayerId,
        modifier: ModifierType,
        value: i32,
        expiry: Expiry,
    ) {
        self.game.modifiers.add_player_modifier(
            target_player,
            PlayerModifierEntry::materialized_declarative(
                modifier,
                value,
                expiry,
                self.source_permanent,
                self.player,
            ),
        );
        self.game.mark_until_condition_dirty();
    }

    pub fn grant_keyword(&mut self, target: PermanentHandle, keyword: Keyword, expiry: Expiry) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game
            .modifiers
            .grant_keyword(target, keyword, expiry, self.player);
        self.grant_keyword_triggered_auto_effects(target, keyword, expiry);
        self.game.mark_until_condition_dirty();
    }

    /// Register the granted keyword's TRIGGERED auto-effect(s) so they fire
    /// at runtime — not just register the keyword for `has_keyword`.
    ///
    /// Passive keywords (Blocker, Reboot, …) are surfaced purely through the
    /// modifier registry and yield an empty `keyword_to_auto_effect`, so this
    /// is a no-op for them. Trigger-based keywords (Retaliation, Fortitude,
    /// Partition, …) carry a plain `process` body in `keyword_to_auto_effect`;
    /// because `effects_for_card` only synthesizes keyword auto-effects from
    /// PRINTED + DECLARATIVE-REGISTRY grants (and OnDeletion handlers re-fetch
    /// effects POST-TRASH, CLAUDE.md rule 25, so any board-position synthesis
    /// would have vanished by drain time), a runtime modifier grant would
    /// otherwise never fire. Route each plain-process triggered auto-effect
    /// through the BOARD-INDEPENDENT granted-triggered store, carrying the
    /// grant's `expiry`. Replacement-process keywords (Barrier, Evade, …)
    /// carry no plain `process` and are skipped here.
    pub(crate) fn grant_keyword_triggered_auto_effects(
        &mut self,
        target: PermanentHandle,
        keyword: Keyword,
        expiry: Expiry,
    ) {
        // Stable identity of the carrier's top card (the granted-triggered body
        // re-resolves the carrier from `source_permanent` at fire time, so we
        // only need the keyword's effect template here).
        let card = match self
            .game
            .players
            .get(target.player as usize)
            .and_then(|p| p.battle_area.get(target.index as usize))
        {
            Some(perm) => perm.top_card().handle(),
            None => return,
        };
        let autos = crate::cards::keyword_effects::keyword_to_auto_effect(keyword, card);
        for effect in autos {
            // Only plain-process triggered timings route through the
            // granted-triggered store. Replacement-process keywords (no plain
            // `process`) and declarative-only entries are skipped.
            let Some(process) = effect.process else {
                continue;
            };
            let timing = effect.timing;
            let process = std::sync::Arc::new(process);
            self.grant_triggered_effect(target, timing, expiry, move |inner_ctx| {
                (process)(inner_ctx);
            });
        }
    }

    pub fn grant_declarative_keyword(
        &mut self,
        target: PermanentHandle,
        keyword: Keyword,
        expiry: Expiry,
    ) {
        if !self.can_affect_permanent(target) {
            return;
        }
        self.game.modifiers.grant_declarative_keyword(
            target,
            keyword,
            expiry,
            self.source_permanent,
            self.player,
        );
        self.game.mark_until_condition_dirty();
    }
}
