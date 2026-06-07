//! Read-only query + aura-bonus helpers (Tier 1) — impl Game.

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
    /// Returns `true` when `card` may digivolve onto `perm` per standard
    /// evo-cost rules: `card` has an `EvoCost` entry whose `level` matches
    /// `perm.top_card()`'s level and whose color is present on
    /// `perm.top_card()`'s color list.
    ///
    /// Memory cost is **not** checked — blast digivolve bypasses memory,
    /// and regular digivolve pays memory at the call site. Mirrors
    /// Python's `can_digivolve(card, base_perm)` validator. Used by
    /// `combat::try_enter_counter` for §2.3 parity.
    pub fn can_digivolve(&self, card: &CardSource, perm: &crate::permanent::Permanent) -> bool {
        let base_top = perm.top_card();
        let Some(base_level) = base_top.digimon_level(&self.card_data) else {
            return false;
        };
        let base_colors = base_top.digimon_colors(&self.card_data);
        card.digivolution_costs(&self.card_data).iter().any(|ec| {
            ec.level == base_level
                && crate::action::mask::evo_color(ec.card_color)
                    .map(|c| base_colors.contains(&c))
                    .unwrap_or(false)
        })
    }

    /// Unified keyword query — returns `true` if the permanent's top card
    /// has `keyword` either printed natively on its face (from
    /// `CardData.keywords`) OR granted by an active modifier.
    ///
    /// This is the canonical engine-wide keyword lookup. Engine code MUST
    /// NOT call `self.modifiers.has_keyword(...)` directly — that only
    /// sees granted keywords and would miss native printed keywords.
    ///
    /// Returns `false` for out-of-range handles (e.g. player index or
    /// battle-area index doesn't exist) so callers don't need a guard.
    pub fn has_keyword(&self, handle: PermanentHandle, keyword: crate::enums::Keyword) -> bool {
        // Modifier-granted (end-of-turn grants, Ally buffs, etc.)
        if self.modifiers.has_keyword(handle, keyword) {
            return true;
        }
        // Native printed on the top card's face.
        let Some(player) = self.players.get(handle.player as usize) else {
            return false;
        };
        let Some(perm) = player.battle_area.get(handle.index as usize) else {
            return false;
        };
        let top = perm.top_card();
        // `data_index` is a direct Vec index — O(1), no iteration needed.
        let card_data = &self.card_data[top.data_index];
        if face_keywords(card_data).contains(&keyword) {
            return true;
        }
        // Inherited keyword grants from digivolution sources. Only cards
        // under the top card contribute inherited text, and any active_when
        // condition must pass before the keyword is considered live.
        let stack_size = perm.card_sources.len();
        let source_ids: Vec<(usize, usize, String, crate::card_source::CardHandle)> = perm
            .card_sources
            .iter()
            .enumerate()
            .map(|(i, s)| {
                (
                    i,
                    s.data_index,
                    s.card_id(&self.card_data).to_string(),
                    s.handle(),
                )
            })
            .collect();
        for (source_index, data_index, src_id, src_handle) in source_ids {
            let is_under = source_index + 1 < stack_size;
            if !is_under {
                continue;
            }
            if inherited_keywords(&self.card_data[data_index]).contains(&keyword) {
                return true;
            }
            let Some(effects) = self.effects_for_card(&src_id, src_handle) else {
                continue;
            };
            for effect in &effects {
                if !effect.declarative || !effect.inherited {
                    continue;
                }
                if effect.granted_keyword != Some(keyword) {
                    continue;
                }
                if let Some(cond) = &effect.condition {
                    let ctx = crate::effect_context::EffectReadContext::new(
                        self,
                        src_handle,
                        Some(handle),
                        handle.player,
                    );
                    if !cond(&ctx) {
                        continue;
                    }
                }
                return true;
            }
        }
        false
    }

    /// Gate predicate for the `<Progress>` keyword **and** the
    /// `ImmunityToOpponentEffects` modifier (both surface the same
    /// "opponent cannot target this with effects while it is the current
    /// attacker" rule; bundling them keeps every opponent-effect call-site
    /// to one branch).
    ///
    /// Returns `true` when:
    ///   - `target` is the current attacker (`current_attacker() == Some(target)`), AND
    ///   - `source` is `Some(pid)` where `pid != target.player`, AND
    ///   - `target` has either `Keyword::Progress` (printed or granted) **or**
    ///     `ModifierType::ImmunityToOpponentEffects`.
    ///
    /// Returns `false` if `source` is `None` (rule-driven mutations: battle,
    /// cost, rule checks). Opponent *effects* are gated; battle damage and
    /// cost-triggered cleanup are not.
    ///
    /// `ImmunityToOpponentEffects` is currently only applied with
    /// attack-scoped expiry (`EndOfAttack` / `EndOfBattle`), so the
    /// `current_attacker` gate is always satisfied when the modifier is
    /// live. If a future card grants the modifier with broader expiry,
    /// split this into `progress_excludes` (Progress only) +
    /// `effect_immunity_excludes` (modifier only) and update both
    /// call-sites; the helpers' shape is identical so the split is
    /// mechanical.
    ///
    /// Callers: `select_opponent_permanent` (selection-time gate, Phase A)
    /// and the script-API mutation entry points on `EffectContext` (Phase B,
    /// broadened in Phase E prep): `delete_permanent`, `return_to_hand`,
    /// `return_to_deck`, `de_digivolve`, `suspend`, and `add_modifier` /
    /// `add_dp_modifier`. The `add_modifier` site is unconditional — every
    /// `ModifierType` and every value (positive, negative, or zero) is gated,
    /// matching DCGO's `CanNotAffected` semantics literally.
    pub fn progress_excludes(
        &self,
        target: PermanentHandle,
        source: Option<crate::enums::PlayerId>,
    ) -> bool {
        let Some(src) = source else { return false };
        if src == target.player {
            return false;
        }
        if self.current_attacker() != Some(target) {
            return false;
        }
        self.has_keyword(target, crate::enums::Keyword::Progress)
            || self.modifiers.has(
                target,
                crate::enums::ModifierType::ImmunityToOpponentEffects,
            )
    }

    pub fn permanent_is_unaffected_by_effect(
        &self,
        target: PermanentHandle,
        effect_controller: crate::enums::PlayerId,
        source_kind: crate::enums::EffectSourceKind,
    ) -> bool {
        use crate::modifiers::EffectControllerFilter;

        self.modifiers
            .get(target, crate::enums::ModifierType::CannotBeAffected)
            .into_iter()
            .any(|entry| {
                let Some(filter) = entry.effect_immunity_filter else {
                    return true;
                };
                let source_kind_matches = filter
                    .source_kind
                    .map(|expected| expected == source_kind)
                    .unwrap_or(true);
                if !source_kind_matches {
                    return false;
                }
                match filter.controller {
                    EffectControllerFilter::Any => true,
                    EffectControllerFilter::OpponentOnly => effect_controller != target.player,
                    EffectControllerFilter::OwnOnly => effect_controller == target.player,
                }
            })
    }

    /// Returns `true` when an effect is currently resolving AND its
    /// controller is not `target`'s controller. The "opponent effect is
    /// targeting me" predicate that drives Mephistomon-style OnDeletion
    /// riders, Scapegoat eligibility (cause ≠ OwnEffect), and the
    /// `was_deleted_by_opponent` accessor.
    ///
    /// Returns `false` when:
    ///   - no effect is currently resolving (`effect_source_player == None`),
    ///   - the resolving effect's controller equals `target.player`.
    ///
    /// Phase B §B5.
    pub fn opponent_sourced_mutation(&self, target: crate::permanent::PermanentHandle) -> bool {
        match self.effect_source_player {
            Some(src) => src != target.player,
            None => false,
        }
    }

    /// Sum the net security-attack modifier contributed by native printed
    /// `<Security A. +N>` and `<Security A. -N>` keywords on `target`.
    /// Called by `resolve_player_security_loop` alongside the existing
    /// `ModifierType::SecurityAttackChange` sum so cards with only the
    /// printed keyword behave correctly without a hand-rolled script.
    pub fn security_attack_keyword_bonus(&self, target: crate::permanent::PermanentHandle) -> i32 {
        use crate::enums::Keyword;
        let Some(player) = self.players.get(target.player as usize) else {
            return 0;
        };
        let Some(perm) = player.battle_area.get(target.index as usize) else {
            return 0;
        };
        // Top-card face keywords count; buried sources only contribute
        // inherited text keywords.
        let mut total = 0i32;
        let stack_size = perm.card_sources.len();
        for (source_index, src) in perm.card_sources.iter().enumerate() {
            let card_data = &self.card_data[src.data_index];
            let keywords = if source_index + 1 == stack_size {
                face_keywords(card_data)
            } else {
                inherited_keywords(card_data)
            };
            for kw in &keywords {
                match kw {
                    Keyword::SecurityAttackPlus(n) => total += *n as i32,
                    Keyword::SecurityAttackMinus(n) => total -= *n as i32,
                    _ => {}
                }
            }
        }
        // Fold in registry-side granted keywords (e.g. an aura's
        // `grant_keyword: SecurityAttackPlus`). Printed keywords above come
        // from `card_sources`; aura grants live in `Modifiers::permanent_keywords`.
        total += self.modifiers.granted_security_attack_keyword_bonus(target);
        total
    }

    pub fn dynamic_dp_aura_bonus(&self, target: crate::permanent::PermanentHandle) -> i32 {
        self.live_declarative_formula_sum(target, false).0
    }

    pub fn static_dp_aura_bonus(&self, target: crate::permanent::PermanentHandle) -> i32 {
        use crate::effect_context::EffectReadContext;

        let Some(permanent) = self
            .players
            .get(target.player as usize)
            .and_then(|player| player.battle_area.get(target.index as usize))
        else {
            return 0;
        };

        let stack_size = permanent.card_sources.len();
        let mut total = 0;
        for (source_index, source) in permanent.card_sources.iter().enumerate() {
            let inherited_source = source_index + 1 < stack_size;
            let card_id = source.card_id(&self.card_data).to_string();
            let Some(effects) = self.effects_for_card(&card_id, source.handle()) else {
                continue;
            };
            for effect in effects {
                if !effect.declarative || effect.inherited != inherited_source {
                    continue;
                }
                if effect.materializes_declarative_state
                    || effect.dp_modifier == 0
                    || effect.dp_modifier_fn.is_some()
                    || effect.applies_to_opponent_security_dp
                {
                    continue;
                }
                let rctx =
                    EffectReadContext::new(self, source.handle(), Some(target), target.player);
                if let Some(condition) = &effect.condition {
                    if !condition(&rctx) {
                        continue;
                    }
                }
                total += effect.dp_modifier;
            }
        }
        total
    }

    pub fn dynamic_security_attack_aura_bonus(
        &self,
        target: crate::permanent::PermanentHandle,
    ) -> Option<i32> {
        let (value, found) = self.live_declarative_formula_sum(target, true);
        found.then_some(value)
    }

    /// True when `target` currently has any Security Attack delta. Printed
    /// and modifier-granted `<Security A. +/-N>` keywords, temporary
    /// `SecurityAttackChange` modifiers, and formula-driven declarative
    /// security-attack auras all count.
    pub fn has_security_attack_change(&self, target: crate::permanent::PermanentHandle) -> bool {
        self.security_attack_keyword_bonus(target) != 0
            || self
                .modifiers
                .sum(target, ModifierType::SecurityAttackChange)
                != 0
            || self
                .dynamic_security_attack_aura_bonus(target)
                .is_some_and(|bonus| bonus != 0)
    }

    /// Shared Digimon-target attack gate for target-scoped combat
    /// restrictions. `CanAttackTargetDefendingPermanent` is the established
    /// affirmative override for target-carried attack bans.
    pub fn attack_target_blocked_by_modifier(
        &self,
        attacker: crate::permanent::PermanentHandle,
        target: crate::permanent::PermanentHandle,
    ) -> bool {
        if self
            .modifiers
            .has(target, ModifierType::CanAttackTargetDefendingPermanent)
        {
            return false;
        }
        if self.modifiers.has(target, ModifierType::CannotAttackTarget) {
            return true;
        }
        self.modifiers.has(
            target,
            ModifierType::CannotBeAttackedBySecurityAttackChanged,
        ) && self.has_security_attack_change(attacker)
    }

    /// Sum of static `dp_modifier` values from a single source's effects
    /// that pass the inherited/top filter and their condition (if any).
    /// Returns a signed raw DP delta. Tensor writes this divided by DP_NORM.
    pub fn source_dp_contribution(
        &self,
        perm: crate::permanent::PermanentHandle,
        source_index: usize,
    ) -> i32 {
        use crate::effect_context::EffectReadContext;
        let Some(permanent) = self
            .players
            .get(perm.player as usize)
            .and_then(|p| p.battle_area.get(perm.index as usize))
        else {
            return 0;
        };
        let stack_size = permanent.card_sources.len();
        let Some(source) = permanent.card_sources.get(source_index) else {
            return 0;
        };
        let is_under = source_index + 1 < stack_size;
        let card_id = source.card_id(&self.card_data).to_string();
        let Some(impl_) = self.effect_registry.get(&card_id) else {
            return 0;
        };
        let effects = impl_.effects(source.handle());

        let mut total = 0i32;
        for effect in &effects {
            if effect.dp_modifier == 0 && effect.dp_modifier_fn.is_none() {
                continue;
            }
            if is_under != effect.inherited {
                continue;
            }
            let ctx = EffectReadContext::new(self, source.handle(), Some(perm), perm.player);
            if let Some(cond) = &effect.condition {
                if !cond(&ctx) {
                    continue;
                }
            }
            total += effect.dp_modifier;
            if let Some(formula_fn) = effect.dp_modifier_fn.as_ref() {
                if let Some(value) = formula_fn(&ctx, perm) {
                    total += value;
                }
            }
        }
        total
    }
}
