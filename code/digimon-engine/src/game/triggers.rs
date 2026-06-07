//! Event drain + declarative/granted trigger machinery (Tier 1) — impl Game.

#![allow(unused_imports)]
use super::*;
use crate::aura::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::effect::*;
use crate::enums::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::selection::*;
use crate::trigger_context::*;

impl Game {
    /// Allocate the next monotonic event sequence number.
    pub fn next_event_seq(&mut self) -> u64 {
        let s = self.event_seq;
        self.event_seq += 1;
        s
    }

    /// Drain accumulated events, returning them in emission order. The
    /// `HeadlessRunner::step` wrapper calls this after each action so the
    /// PyO3 layer can expose a per-step event list.
    pub fn drain_events(&mut self) -> Vec<crate::events::GameEvent> {
        std::mem::take(&mut self.events)
    }

    /// Borrow accumulated events without draining them. Debug tooling uses
    /// this to checkpoint and assert on incremental event emission.
    pub fn events(&self) -> &[crate::events::GameEvent] {
        &self.events
    }

    /// Re-install declarative process-backed effects from permanents currently
    /// on the field. Static effect builders still expose pure fields directly;
    /// this dispatcher is for declarative clauses lowered to process closures,
    /// such as filtered auras and player-scoped flood gates.
    pub fn tick_declarative_effects(&mut self) {
        self.modifiers.clear_materialized_declaratives();

        let mut sources = Vec::new();
        for (pid, player) in self.players.iter().enumerate() {
            let player_id = pid as PlayerId;
            for (index, perm) in player.battle_area.iter().enumerate() {
                let handle = PermanentHandle {
                    player: player_id,
                    index: index as u8,
                };
                let top = perm.top_card();
                sources.push((
                    top.card_id(&self.card_data).to_string(),
                    top.handle(),
                    Some(handle),
                    player_id,
                    false,
                ));

                let stack_size = perm.card_sources.len();
                for (source_index, source) in perm.card_sources.iter().enumerate() {
                    if source_index + 1 >= stack_size {
                        continue;
                    }
                    sources.push((
                        source.card_id(&self.card_data).to_string(),
                        source.handle(),
                        Some(handle),
                        player_id,
                        true,
                    ));
                }
            }

            if let Some(perm) = player.breeding_area.as_ref() {
                let handle = PermanentHandle {
                    player: player_id,
                    index: crate::action::space::BREEDING_TARGET as u8,
                };
                let top = perm.top_card();
                sources.push((
                    top.card_id(&self.card_data).to_string(),
                    top.handle(),
                    Some(handle),
                    player_id,
                    false,
                ));
            }

            // Track H §5 — security-zone-sourced auras. Face-up security
            // cards can carry `kind: aura, scope: security` declarative
            // clauses that grant DP/keyword/modifier to filter-matched
            // battle-area permanents while the source remains face-up in
            // the security stack. Source-permanent is `None` because
            // security entries have no battle-area handle; the install
            // closures still target battle-area handles for the matches.
            // Cleanup is automatic — each tick clears materialized
            // declaratives, then re-installs from active sources, so a
            // card leaving security simply stops re-installing on the
            // next tick. Mirrors DCGO `BT21_095.cs:CanUseCondition` →
            // `IsExistInSecurity(card, false)`.
            for card in &player.security {
                if !player.face_up_security.contains(&card.card_index) {
                    continue;
                }
                sources.push((
                    card.card_id(&self.card_data).to_string(),
                    card.handle(),
                    None,
                    player_id,
                    false,
                ));
            }
        }

        for (card_id, source_card, source_permanent, controller, inherited_source) in sources {
            let Some(effects) = self.effects_for_card(&card_id, source_card) else {
                continue;
            };
            for effect in effects {
                if !effect.declarative || effect.inherited != inherited_source {
                    continue;
                }
                if !effect.materializes_declarative_state || effect.process.is_none() {
                    continue;
                }
                if let Some(condition) = &effect.condition {
                    let rctx = crate::effect_context::EffectReadContext::new(
                        self,
                        source_card,
                        source_permanent,
                        controller,
                    );
                    if !condition(&rctx) {
                        continue;
                    }
                }
                if let Some(process) = effect.process.as_ref() {
                    let mut ctx = crate::effect_context::EffectContext::new(
                        self,
                        source_card,
                        source_permanent,
                        controller,
                    );
                    process(&mut ctx);
                }
            }
        }
    }

    /// Track H Phase 4k — clear all per-permanent state when a
    /// permanent leaves the field, AND prune the corresponding
    /// granted-triggered-effect body registry entries. Wraps the
    /// narrower `ModifierRegistry::clear_permanent` so call sites
    /// don't have to remember the body-registry cleanup separately.
    /// Returns the count of body-registry entries removed (mostly for
    /// tests / instrumentation).
    pub fn clear_permanent_full(&mut self, handle: crate::permanent::PermanentHandle) -> usize {
        let body_ids = self
            .modifiers
            .drain_granted_triggered_ids_on_carrier(handle);
        self.modifiers.clear_permanent(handle);
        let mut removed = 0usize;
        for id in body_ids {
            if self
                .effect_queue
                .iter()
                .any(|queued| queued.granted_effect_id == Some(id))
            {
                continue;
            }
            if self.granted_effect_bodies.remove(id).is_some() {
                removed += 1;
            }
        }
        removed
    }

    /// Track H §3 — fire all granted triggered effects on `carrier`
    /// whose registered timing matches `timing`. Each body runs
    /// inline with `EffectContext::source_card` set to the grantor
    /// (mirroring DCGO `EffectSourceCard`) and `source_permanent` set
    /// to the carrier (mirroring DCGO `EffectSourcePermanent`).
    ///
    /// Inline-fire model (v1): the body runs synchronously, before
    /// `drain_effect_queue` resolves the rest of the trigger fan-out.
    /// Suitable for grants that mutate state without prompting (memory
    /// gain, modifier installs, etc.). Selection-driving granted
    /// bodies are not yet supported — they belong on the standard
    /// queue/drain path, which is a follow-up.
    pub fn fire_granted_triggered_effects(
        &mut self,
        carrier: crate::permanent::PermanentHandle,
        timing: crate::enums::EffectTiming,
    ) {
        let entries = self.modifiers.granted_triggered_for_timing(carrier, timing);
        for (source_card, source_player, body) in entries {
            let mut ctx = crate::effect_context::EffectContext::new(
                self,
                source_card,
                Some(carrier),
                source_player,
            );
            body(&mut ctx);
        }
    }
}
