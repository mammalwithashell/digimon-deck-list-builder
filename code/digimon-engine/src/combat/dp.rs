//! DP computation + security DP adjustments (Tier 1).

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
    pub fn permanent_is_digimon_for_rules(&self, handle: PermanentHandle) -> bool {
        self.player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|perm| perm.is_digimon_for_rules(&self.card_data, &self.modifiers, handle))
            .unwrap_or(false)
    }

    /// Compute a permanent's effective DP (base + modifier sum).
    ///
    /// Track C / D consult site (2026-05-08): if the target carries any
    /// `ModifierType::ImmuneFromDPMinus` entry, negative `ChangeDp`
    /// modifiers are filtered out before summing — the protection
    /// narrows to DP-minus only, leaving positive `ChangeDp` and the
    /// dynamic aura bonus untouched.
    ///
    /// PUPPETS-G024 (2026-05-20): each `ImmuneFromDPMinus` entry's
    /// `effect_immunity_filter.controller` now scopes which negative
    /// `ChangeDp` deltas it suppresses:
    ///   - `OpponentOnly` — suppress only deltas whose `source_player`
    ///     is the protected permanent's opponent (printed text "can't
    ///     have its DP reduced **by your opponent's effects**"). The
    ///     controller's own DP-reduction still applies.
    ///   - `Any` / no filter — suppress every negative delta (the broad
    ///     variant; back-compat for entries installed without a filter).
    /// A negative `ChangeDp` delta is suppressed if ANY `ImmuneFromDPMinus`
    /// entry's scope covers it.
    pub fn effective_dp(&self, handle: PermanentHandle) -> Option<i32> {
        use crate::modifiers::EffectControllerFilter;
        let perm = self
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)?;
        let base = perm.base_dp_for_rules(&self.card_data, &self.modifiers, handle)?;
        let immunity_scopes: Vec<EffectControllerFilter> = self
            .modifiers
            .get(handle, ModifierType::ImmuneFromDPMinus)
            .iter()
            .map(|entry| {
                entry
                    .effect_immunity_filter
                    .map(|f| f.controller)
                    .unwrap_or(EffectControllerFilter::Any)
            })
            .collect();
        let change_dp_sum: i32 = self
            .modifiers
            .get(handle, ModifierType::ChangeDp)
            .iter()
            .filter(|entry| {
                if entry.value >= 0 {
                    return true;
                }
                // Negative delta — suppress if any ImmuneFromDPMinus
                // entry's scope covers this delta's source.
                let from_opponent = entry.source_player != handle.player;
                let suppressed = immunity_scopes.iter().any(|scope| match scope {
                    EffectControllerFilter::Any => true,
                    EffectControllerFilter::OpponentOnly => from_opponent,
                    EffectControllerFilter::OwnOnly => !from_opponent,
                });
                !suppressed
            })
            .map(|entry| entry.value)
            .sum();
        let bonus =
            change_dp_sum + self.static_dp_aura_bonus(handle) + self.dynamic_dp_aura_bonus(handle);
        // DP floors at 0 (rules manual 17-1-3-1 speaks of DP "becoming 0",
        // never negative; DCGO `Permanent.DP` ends `if (DP < 0) DP = 0`).
        // Judge-quiz Q24: an <Alliance> ally debuffed below 0 contributes
        // +0 DP, not a negative delta. The ≤0 rules check still fires —
        // a floored 0 is exactly the deletable state.
        Some((base + bonus).max(0))
    }

    /// Sum the attacker's digivolution-stack DP adjustments that apply to
    /// the opposing security Digimon during a security DP battle (§2.5e).
    /// Walks every `CardSource` in the stack — any effect flagged with
    /// `applies_to_opponent_security_dp` contributes its `dp_modifier` to
    /// the security Digimon's effective DP, including effects on the top
    /// card. Returns 0 when the attacker has no such effects.
    pub fn attacker_security_dp_adjustment(&self, attacker: PermanentHandle) -> i32 {
        let Some(perm) = self
            .player(attacker.player)
            .battle_area
            .get(attacker.index as usize)
        else {
            return 0;
        };
        let mut total: i32 = 0;
        for source in &perm.card_sources {
            let card_id = source.card_id(&self.card_data);
            let Some(effects) = self.effects_for_card(card_id, source.handle()) else {
                continue;
            };
            for effect in effects.iter() {
                if !effect.applies_to_opponent_security_dp {
                    continue;
                }
                // Honor the aura's `active_when`/condition gate (mirrors
                // `static_dp_aura_bonus`). Controller is the attacker.
                let rctx = crate::effect_context::EffectReadContext::new(
                    self,
                    source.handle(),
                    None,
                    attacker.player,
                );
                if let Some(condition) = &effect.condition {
                    if !condition(&rctx) {
                        continue;
                    }
                }
                total = total.saturating_add(effect.dp_modifier);
            }
        }
        total
    }

    /// Sum defender-side security Digimon DP modifiers for the player whose
    /// security stack is being checked.
    pub fn defender_security_dp_adjustment(&self, defender: PlayerId) -> i32 {
        let mut total: i32 = self
            .modifiers
            .player_modifiers_iter(defender)
            .filter(|entry| {
                matches!(
                    entry.modifier,
                    ModifierType::SecurityDpChange | ModifierType::ChangeOwnSecurityDigimonDp
                )
            })
            .map(|entry| entry.value)
            .sum();
        for perm in &self.player(defender).battle_area {
            for source in &perm.card_sources {
                let card_id = source.card_id(&self.card_data);
                let Some(effects) = self.effects_for_card(card_id, source.handle()) else {
                    continue;
                };
                for effect in effects.iter() {
                    if !effect.applies_to_own_security_dp {
                        continue;
                    }
                    // Honor the aura's `active_when`/condition gate — e.g. T.K.
                    // Takaishi's "[Opponent's Turn] ... +2000" must not apply on
                    // the controller's own turn (mirrors `static_dp_aura_bonus`).
                    // The controller is `defender`.
                    let rctx = crate::effect_context::EffectReadContext::new(
                        self,
                        source.handle(),
                        None,
                        defender,
                    );
                    if let Some(condition) = &effect.condition {
                        if !condition(&rctx) {
                            continue;
                        }
                    }
                    total = total.saturating_add(effect.dp_modifier);
                }
            }
        }
        total
    }
}
