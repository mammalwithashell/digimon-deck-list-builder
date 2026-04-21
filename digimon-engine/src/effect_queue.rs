//! Triggered-effect queue + drainer.
//!
//! When one or more effects fire at the same timing window (OnPlay, OnAttack,
//! OnDeletion, EndOfYourTurn, ...), `enqueue_triggered` collects them into
//! `Game.effect_queue`, then `drain_effect_queue` resolves them one at a
//! time.
//!
//! Ordering rules (per Digimon TCG, confirmed against DCGO):
//! - The **turn player** resolves all of their queued triggers before the
//!   non-turn-player resolves any of theirs.
//! - Within a single controller's bundle, **the controller picks the
//!   order**. If only one effect is queued for them, it auto-fires; if
//!   multiple are queued, a `TriggerOrder` selection prompts for order.
//! - Optional triggers may be declined individually. When the remaining
//!   triggers for the current chooser are all optional, the prompt carries
//!   a PASS bit that declines **all of them at once**.
//!
//! The drainer has a hard cap of `MAX_CHAIN_DEPTH` iterations as a safety
//! rail against self-triggering loops. Matches Python's
//! `_resolve_effect_stack` max=50 bound.

use crate::action::space::{HAND_EFFECT_END, HAND_EFFECT_START, HAND_MAIN_LIMIT, PASS};
use crate::card_source::CardHandle;
use crate::effect_context::EffectContext;
use crate::enums::{EffectTiming, GamePhase, PlayerId};
use crate::game::Game;
use crate::permanent::PermanentHandle;
use crate::selection::{
    EffectChoiceEntry, PendingSelection, QueuedEffect, SelectionKind, TriggerSource,
};

/// Max iterations the drainer will take before aborting a suspected infinite
/// chain. Matches Python's `_resolve_effect_stack` limit.
pub const MAX_CHAIN_DEPTH: u16 = 50;

impl Game {
    // ─── Public API ─────────────────────────────────────────────────

    /// Collect every effect on `source` whose timing matches `timing` and
    /// whose `is_*` flag matches the timing, append them to `effect_queue`.
    ///
    /// **Does not drive execution.** Call `drain_effect_queue()` afterward
    /// to resolve the collected effects, or call `enqueue_triggered` for
    /// multiple sources first and drain once at the end.
    pub fn enqueue_triggered(&mut self, timing: EffectTiming, source: TriggerSource) {
        match source {
            TriggerSource::Permanent(handle) => {
                self.enqueue_from_permanent(timing, handle);
            }
            TriggerSource::PlayerBattleArea(player) => {
                // Snapshot indices up-front. Firing an effect via the drainer
                // can mutate the battle_area, but enqueueing itself is pure.
                let count = self.player(player).battle_area.len();
                for i in 0..count {
                    let handle = PermanentHandle {
                        player,
                        index: i as u8,
                    };
                    self.enqueue_from_permanent(timing, handle);
                }
            }
            TriggerSource::SecurityRevealed { defender, card } => {
                self.enqueue_from_security_card(timing, defender, card);
            }
            TriggerSource::OnSecurityCheck { defender, .. } => {
                // Observer timing: scan every permanent in the defender's
                // battle area for `OnSecurityCheck`-timed effects. Attacker
                // + revealed card metadata are carried through
                // `game.security_resolution` for the drained effects to
                // read via `EffectContext::attacker` / `security_digimon`
                // / the defender's `last_security_reveal` snapshot.
                let count = self.player(defender).battle_area.len();
                for i in 0..count {
                    let handle = PermanentHandle {
                        player: defender,
                        index: i as u8,
                    };
                    self.enqueue_from_permanent(timing, handle);
                }
            }
        }
    }

    /// Drain the effect queue. Fires each queued effect in order, pausing
    /// when an effect installs a `pending_selection` or when the queue
    /// contains multiple triggers for a single chooser (installs a
    /// `TriggerOrder` selection and returns).
    ///
    /// Idempotent — safe to call when the queue is already empty. Callers
    /// should invoke this after every `enqueue_triggered` call, and again
    /// after `resolve_selection` unless that call installed a new selection.
    pub fn drain_effect_queue(&mut self) {
        loop {
            if self.pending_selection.is_some() {
                return;
            }
            if self.effect_queue.is_empty() {
                self.effect_chain_depth = 0;
                return;
            }

            self.effect_chain_depth = self.effect_chain_depth.saturating_add(1);
            if self.effect_chain_depth > MAX_CHAIN_DEPTH {
                // Suspected self-triggering loop — drop the remaining queue.
                // Matches Python's defensive behavior.
                self.effect_queue.clear();
                self.effect_chain_depth = 0;
                return;
            }

            let Some(chooser) = self.next_chooser() else {
                self.effect_chain_depth = 0;
                return;
            };

            let bundle: Vec<usize> = self
                .effect_queue
                .iter()
                .enumerate()
                .filter_map(|(i, qe)| (qe.controller == chooser).then_some(i))
                .collect();

            debug_assert!(!bundle.is_empty(), "next_chooser returned a player with no queued effects");

            if bundle.len() == 1 {
                // Single trigger — auto-fire, no prompt.
                let idx = bundle[0];
                let qe = self
                    .effect_queue
                    .remove(idx)
                    .expect("bundle index in-bounds by construction");
                self.run_queued_effect(qe);
                continue;
            }

            // Multi-trigger bundle — install a TriggerOrder selection.
            // Cap at HAND_MAIN_LIMIT (30) to fit the reused 30-59 action
            // range. Overflow auto-fires in collection order after the prompt
            // completes (rare; see the cap handling inside install_*).
            let any_mandatory = bundle
                .iter()
                .any(|&i| !self.effect_queue[i].is_optional);
            self.install_trigger_order_selection(chooser, &bundle, !any_mandatory);
            return;
        }
    }

    // ─── Internal helpers ───────────────────────────────────────────

    /// Collect `SecuritySkill` effects off a revealed security card. The
    /// card is expected to be parked in `Game.pending_security` (popped off
    /// the defender's security stack but not yet disposed). Only effects
    /// whose `security` flag is set are enqueued — matches Python's
    /// `is_security_effect` filter.
    fn enqueue_from_security_card(
        &mut self,
        timing: EffectTiming,
        defender: PlayerId,
        card: CardHandle,
    ) {
        let Some(pending) = self.pending_security.as_ref() else {
            return;
        };
        if pending.card.handle() != card {
            return;
        }
        let card_id = pending.card.card_id(&self.card_data).to_string();
        let source_card = card;

        let Some(effects) = self.effects_for_card(&card_id, source_card) else {
            return;
        };

        let tp = self.turn_player();
        let is_turn_player = defender == tp;

        for (slot, effect) in effects.iter().enumerate() {
            if !timing_flag_matches(effect, timing) {
                continue;
            }
            // Security trigger specifically: ignore effects that don't carry
            // the security flag. Matches Python's
            // `if getattr(effect, 'is_security_effect', False)` filter.
            if timing == EffectTiming::SecuritySkill && !effect.security {
                continue;
            }
            self.effect_queue.push_back(QueuedEffect {
                source_card,
                source_permanent: None,
                controller: defender,
                timing,
                effect_slot: slot as u8,
                is_optional: effect.optional,
                is_turn_player,
                card_id: card_id.clone(),
            });
        }
    }

    /// Collect effects for a single permanent. Applies the same timing +
    /// flag filter as the legacy `fire_*` loops so enqueue is a drop-in
    /// replacement.
    fn enqueue_from_permanent(&mut self, timing: EffectTiming, handle: PermanentHandle) {
        let Some(perm) = self
            .players
            .get(handle.player as usize)
            .and_then(|p| p.battle_area.get(handle.index as usize))
        else {
            return;
        };
        let top = perm.top_card();
        let card_id = top.card_id(&self.card_data).to_string();
        let source_card = top.handle();

        let Some(effects) = self.effects_for_card(&card_id, source_card) else {
            return;
        };

        let tp = self.turn_player();
        let is_turn_player = handle.player == tp;

        for (slot, effect) in effects.iter().enumerate() {
            if !timing_flag_matches(effect, timing) {
                continue;
            }
            self.effect_queue.push_back(QueuedEffect {
                source_card,
                source_permanent: Some(handle),
                controller: handle.player,
                timing,
                effect_slot: slot as u8,
                is_optional: effect.optional,
                is_turn_player,
                card_id: card_id.clone(),
            });
        }
    }

    /// Who gets to choose the next effect to resolve. Turn player first,
    /// then clockwise through the remaining players.
    fn next_chooser(&self) -> Option<PlayerId> {
        if self.effect_queue.is_empty() {
            return None;
        }
        let n = self.turn_order.len();
        for offset in 0..n {
            let idx = (self.turn_player_idx + offset) % n;
            let pid = self.turn_order[idx];
            if self.effect_queue.iter().any(|qe| qe.controller == pid) {
                return Some(pid);
            }
        }
        // Defensive fallback — if somehow no turn-order player owns any
        // queued effect (e.g. eliminated controller), use the front entry.
        self.effect_queue.front().map(|qe| qe.controller)
    }

    /// Execute a single queued effect: re-look-up, condition check,
    /// process. Exits silently on any validity gap (permanent deleted,
    /// effect removed from the registry, etc.) — same tolerance the legacy
    /// `fire_*` loops had.
    fn run_queued_effect(&mut self, qe: QueuedEffect) {
        // Set the effect-source attribution for replacement-cause inference.
        // Saved on entry, restored on exit — supports nested drains (an
        // effect queues another effect that recursively drains before this
        // one returns).
        let prev_effect_source = self.effect_source_player;
        self.effect_source_player = Some(qe.controller);
        let out = self.run_queued_effect_inner(qe);
        self.effect_source_player = prev_effect_source;
        out
    }

    fn run_queued_effect_inner(&mut self, qe: QueuedEffect) {
        // Source permanent may have been deleted by a prior effect in this
        // batch. Skip silently — matches Python behavior.
        if let Some(perm_handle) = qe.source_permanent {
            let Some(perm) = self
                .players
                .get(perm_handle.player as usize)
                .and_then(|p| p.battle_area.get(perm_handle.index as usize))
            else {
                return;
            };
            // Also skip if the specific source card has been shuffled out
            // of the top-card slot (e.g. permanent digivolved mid-batch).
            // The card_index on the top card must match what we queued.
            if perm.top_card().card_index != qe.source_card.0 {
                return;
            }
        }

        let Some(effects) = self.effects_for_card(&qe.card_id, qe.source_card) else {
            return;
        };
        let Some(effect) = effects.get(qe.effect_slot as usize) else {
            return;
        };

        // Python parity (§2.5h): `_fire_security_skill` iterates
        // `effect_list(SecuritySkill)` and invokes the callback directly —
        // it never evaluates `effect.can_use_condition`. Matching that
        // behavior here so a conditional `[Security]` effect
        // (`[Security] If opp has a Digimon, delete it.`) fires with the
        // same semantics on both engines. The script is responsible for
        // any conditionality via an `if` inside its `process` closure.
        let skip_condition = qe.timing == EffectTiming::SecuritySkill;
        if !skip_condition {
            if let Some(cond) = &effect.condition {
                let ctx = EffectContext::new(
                    self,
                    qe.source_card,
                    qe.source_permanent,
                    qe.controller,
                );
                if !cond(&ctx.as_read()) {
                    return;
                }
            }
        }
        // Note: pay_cost_fn is NOT gated by skip_condition. For SecuritySkill
        // timing, pay_cost_fn still fires (intentional — pay-costs are
        // orthogonal to the condition-skipping behavior for security effects).
        //
        // Phase 5 Task 3: pay-cost hook — fires after condition passes, before
        // process. Mirrors the condition-check pattern above: borrow
        // `&effect.pay_cost_fn` read-only, construct a fresh `EffectContext`
        // for the mutable call, then drop both before the process block.
        //
        // v1 constraint: pay_cost_fn must be synchronous. Installing a
        // PendingSelection inside the closure is undefined behavior for v1;
        // cards needing selection-gated pay-costs should fold the selection
        // into `process` instead. See Phase 5 non-goals.
        if let Some(pay_cost) = &effect.pay_cost_fn {
            let mut ctx = EffectContext::new(
                self,
                qe.source_card,
                qe.source_permanent,
                qe.controller,
            );
            if !pay_cost(&mut ctx) {
                return; // cost not paid; skip process (silent abort, mirrors failed condition)
            }
        }

        if let Some(process) = &effect.process {
            let mut ctx = EffectContext::new(
                self,
                qe.source_card,
                qe.source_permanent,
                qe.controller,
            );
            process(&mut ctx);
        }
    }

    /// Install a `TriggerOrder` selection offering `bundle` indices as
    /// resolution picks. `allow_decline_all` enables PASS = decline every
    /// remaining optional trigger controlled by `chooser`.
    ///
    /// Bundle size is capped at `HAND_MAIN_LIMIT` (30) to fit the reused
    /// 30-59 action ID range. If the caller passes more, the overflow
    /// entries are auto-fired in collection order after the prompt
    /// resolves — documented-worst-case behavior, not expected in practice.
    fn install_trigger_order_selection(
        &mut self,
        chooser: PlayerId,
        bundle: &[usize],
        allow_decline_all: bool,
    ) {
        debug_assert!(
            bundle.len() >= 2,
            "install_trigger_order_selection requires a multi-trigger bundle"
        );

        // Map each bundle position to an action ID in the 30-59 range.
        // action_id = HAND_EFFECT_START + position.
        let capped = bundle.len().min(HAND_MAIN_LIMIT);
        let mut valid_action_ids: Vec<u16> = Vec::with_capacity(capped);
        let mut choices: Vec<EffectChoiceEntry> = Vec::with_capacity(capped);
        for pos in 0..capped {
            let qe_idx = bundle[pos];
            let qe = &self.effect_queue[qe_idx];
            let action_id = HAND_EFFECT_START + pos as u16;
            debug_assert!(action_id < HAND_EFFECT_END);
            valid_action_ids.push(action_id);
            choices.push(EffectChoiceEntry {
                label: format!(
                    "{} slot {} ({})",
                    qe.card_id,
                    qe.effect_slot,
                    if qe.is_optional { "optional" } else { "mandatory" },
                ),
                action_id,
            });
        }

        // Provenance: point at the first queued effect's source. This is a
        // debug aid — the selection itself doesn't need a true source.
        let head_qe = &self.effect_queue[bundle[0]];
        let source_card = head_qe.source_card;
        let source_permanent = head_qe.source_permanent;

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::EffectChoice;

        self.pending_selection = Some(PendingSelection {
            kind: SelectionKind::TriggerOrder,
            selecting_player: chooser,
            previous_phase,
            valid_action_ids,
            is_optional: allow_decline_all,
            prompt: format!(
                "Choose which triggered effect to resolve next ({} pending)",
                capped,
            ),
            effect_choices: Some(choices),
            source_card,
            source_permanent,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let pos = action_id.saturating_sub(HAND_EFFECT_START) as usize;
                // Find the i-th entry in `game.effect_queue` controlled by
                // `chooser` — this is the same bundle position the prompt
                // offered. Recompute defensively; single-threaded + paused
                // selection guarantees the queue hasn't shifted.
                let target_idx = game
                    .effect_queue
                    .iter()
                    .enumerate()
                    .filter(|(_, qe)| qe.controller == chooser)
                    .nth(pos)
                    .map(|(i, _)| i);
                if let Some(idx) = target_idx {
                    if let Some(qe) = game.effect_queue.remove(idx) {
                        game.run_queued_effect(qe);
                    }
                }
                // Generic resolver will call `drain_effect_queue` after we
                // return — no need to drain from inside the callback.
            }),
            on_decline: if allow_decline_all {
                Some(Box::new(move |game: &mut Game| {
                    game.effect_queue
                        .retain(|qe| !(qe.controller == chooser && qe.is_optional));
                    // Drain is the generic resolver's responsibility.
                }))
            } else {
                None
            },
        });
    }

    /// Resolve any pending selection — `TriggerOrder`, `Target`, `Hand`,
    /// `OppField`, etc. Factored here so the effect-queue module owns the
    /// validate → take → invoke → drain sequence that every selection kind
    /// needs.
    ///
    /// Specifically:
    /// 1. Validate `player` matches `selecting_player`.
    /// 2. Validate `action_id` is either in `valid_action_ids` or is PASS
    ///    with `is_optional` set.
    /// 3. Take the selection out of `Game` and restore the previous phase
    ///    *before* invoking the callback, so the callback can inspect
    ///    `current_phase` or install a follow-up selection cleanly.
    /// 4. Fire the appropriate callback (main / `on_decline`).
    /// 5. Resume draining the effect queue — unless the callback installed
    ///    a new `pending_selection`, in which case draining is deferred
    ///    until that one resolves.
    pub(crate) fn resolve_generic_selection(
        &mut self,
        player: PlayerId,
        action_id: u16,
    ) -> Result<(), crate::selection::SelectionError> {
        use crate::selection::SelectionError;

        let sel = self
            .pending_selection
            .as_ref()
            .ok_or(SelectionError::NoPendingSelection)?;
        if sel.selecting_player != player {
            return Err(SelectionError::WrongPlayer);
        }
        let is_pass = action_id == PASS;
        if is_pass && !sel.is_optional {
            return Err(SelectionError::InvalidAction);
        }
        if !is_pass && !sel.valid_action_ids.contains(&action_id) {
            return Err(SelectionError::InvalidAction);
        }

        // Take the selection, restore phase, invoke the appropriate callback.
        let sel = self.pending_selection.take().expect("checked Some above");
        self.current_phase = sel.previous_phase;
        if is_pass {
            if let Some(on_decline) = sel.on_decline {
                on_decline(self);
            }
        } else {
            (sel.callback)(self, action_id);
        }

        // If the callback parked a fresh selection, leave the drainer alone.
        // Otherwise resume — this covers both the normal post-callback case
        // and the `TriggerOrder` "continue picking the next bundle entry"
        // flow, so callers don't have to remember to drain.
        if self.pending_selection.is_none() {
            self.drain_effect_queue();
        }
        // After any post-callback draining, re-enter the security state
        // machine if a check is mid-resolve (RUST_PYTHON_PARITY §2.5j).
        // Idempotent when `security_resolution.is_none()`; safe to call
        // unconditionally. Nested selections (the callback installed a
        // further select) leave `pending_selection = Some(...)` so the
        // advance guards re-pause cleanly.
        if self.pending_selection.is_none() {
            self.advance_security_resolution();
        }
        Ok(())
    }
}

/// Match an effect's timing + legacy bool flags against the triggering
/// timing. Mirrors the filter the legacy `fire_*` loops applied.
fn timing_flag_matches(effect: &crate::effect::Effect, timing: EffectTiming) -> bool {
    match timing {
        EffectTiming::OnPlay => effect.on_play,
        EffectTiming::OnAttack => effect.on_attack,
        EffectTiming::OnDeletion => effect.on_deletion,
        _ => effect.timing == timing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_runner::{make_test_card, DebugRunner};

    /// Chain cap safety rail: if the chain_depth counter has reached the
    /// cap (simulating a long sequence of recursive triggers), the next
    /// drain iteration trips the guard, clears the remaining queue, and
    /// resets the counter. Prevents a pathological self-triggering chain
    /// from hanging the engine.
    #[test]
    fn chain_cap_terminates_runaway_queue() {
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("TEST-006", "TestSix"))
            .start();
        r.place_on_field(0, "TEST-006", Some(0));

        // Simulate having just finished 50 chained resolutions — the next
        // resolution should be capped.
        r.game.effect_chain_depth = MAX_CHAIN_DEPTH;

        // Enqueue a fresh mandatory trigger; single-bundle, would normally
        // auto-fire. With the cap already reached, drain should abort.
        r.game.enqueue_triggered(
            EffectTiming::EndOfYourTurn,
            TriggerSource::PlayerBattleArea(0),
        );
        let memory_before = r.game.memory;

        r.game.drain_effect_queue();

        assert!(
            r.game.effect_queue.is_empty(),
            "runaway queue must be cleared after hitting the cap"
        );
        assert_eq!(
            r.game.effect_chain_depth, 0,
            "chain depth must reset after the cap clears the queue"
        );
        assert!(r.game.pending_selection.is_none());
        assert_eq!(
            r.game.memory, memory_before,
            "capped effect must not have fired"
        );
    }

    /// Stable intra-bundle ordering: when the turn player has multiple
    /// permanents each with one mandatory trigger, the bundle entries appear
    /// in battle_area order. Verified by inspecting `effect_queue` before
    /// the drainer consumes it.
    #[test]
    fn bundle_preserves_battle_area_order() {
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("TEST-006", "TestSix"))
            .add_card(make_test_card("TEST-008", "TestEight"))
            .start();
        let _h0 = r.place_on_field(0, "TEST-006", Some(0));
        let _h1 = r.place_on_field(0, "TEST-008", Some(0));
        let _h2 = r.place_on_field(0, "TEST-006", Some(0));

        r.game.enqueue_triggered(
            EffectTiming::EndOfYourTurn,
            TriggerSource::PlayerBattleArea(0),
        );

        assert_eq!(r.game.effect_queue.len(), 3);
        assert_eq!(
            r.game.effect_queue[0].card_id, "TEST-006",
            "first slot of battle_area comes first"
        );
        assert_eq!(r.game.effect_queue[1].card_id, "TEST-008");
        assert_eq!(r.game.effect_queue[2].card_id, "TEST-006");
    }
}
