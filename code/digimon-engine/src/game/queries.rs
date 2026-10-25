//! Read-only query + aura-bonus helpers (Tier 1).

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
    /// Acting-controller eligibility shared by security cost offers and their
    /// actual movement. Card ownership determines destinations separately.
    pub fn can_add_security_by_effect(&self, controller: PlayerId) -> bool {
        let Some(player) = self.players.get(controller as usize) else { return false; };
        !self.modifiers.player_has(controller, ModifierType::CannotAddSecurityByEffect)
            && !player.battle_area.iter().enumerate().any(|(index, _)| {
                self.modifiers.has(PermanentHandle { player: controller, index: index as u8 },
                    ModifierType::CannotAddSecurity)
            })
    }

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
        if keyword == crate::enums::Keyword::Blocker && self.collision_grants_blocker(handle) {
            return true;
        }
        self.has_keyword_from_card_sources(handle, keyword)
    }

    /// §16-30-1 / §16-30-2 / §16-30-4: while a Digimon with `<Collision>` is
    /// attacking, ALL of its controller's opponent's Digimon gain
    /// `<Blocker>` -- a persistent keyword effect, computed live from the
    /// current attack (so it can never outlive the attack), applied with no
    /// activation. "Gain <Blocker>" is processing that affects Digimon
    /// (§16-30-4), so a Digimon the attacker's Digimon effects can't affect
    /// (§15-15-5) does not gain it; the forced block, which affects the
    /// opponent PLAYER, is `try_enter_block`'s mandatory window.
    ///
    /// DCGO `Permanent.HasBlocker`: true for every opponent Digimon during a
    /// `<Collision>` attack unless `TopCard.CanNotBeAffected(collisionEffect)`.
    pub(crate) fn collision_grants_blocker(&self, handle: PermanentHandle) -> bool {
        // `live_attack_attacker` (stack identity), not `current_attacker`
        // (the slot handle): the grant vanishes exactly when the <Collision>
        // Digimon leaves the attack, and field compaction can't aim it at
        // another Digimon that slid into the attacker's old slot.
        let Some(attacker) = self.live_attack_attacker() else {
            return false;
        };
        if attacker.player == handle.player
            || !self.permanent_is_digimon_for_rules(handle)
            || !self.has_keyword(attacker, crate::enums::Keyword::Collision)
        {
            return false;
        }
        self.effect_can_affect(
            handle,
            attacker.player,
            crate::enums::EffectSourceKind::Digimon,
        )
    }

    /// The card-source half of [`Game::has_keyword`]: `keyword` reaches
    /// `handle` from PRINTED text (own face keywords, a below-top source's
    /// inherited keywords, a link card's link keywords) or from a
    /// declarative `grant_keyword` clause that advertises itself with
    /// `Effect::granted_keyword` (a SELF-aura / `scope: inherited` grant).
    /// The modifier registry is deliberately NOT consulted.
    ///
    /// This is exactly the set of origins for which
    /// `Game::build_effects_for_card` already synthesizes the keyword's
    /// auto-effects into some card's effect list — so the trigger-dispatch
    /// scan for AURA-granted keywords
    /// (`G-ENGINE-AURA-GRANT-NO-TRIGGER`) uses it as its
    /// already-covered / no-double-fire guard.
    pub(crate) fn has_keyword_from_card_sources(
        &self,
        handle: PermanentHandle,
        keyword: crate::enums::Keyword,
    ) -> bool {
        self.card_source_keyword_matches(handle, &|k| k == keyword)
    }

    /// The one walk over every card-source origin of a permanent's keywords
    /// (face, inherited, `<Succession>`-adopted, declarative grants, link
    /// cards), with the keyword compare supplied by the caller: exact value
    /// for [`Game::has_keyword_from_card_sources`], keyword type for
    /// [`Game::has_keyword_type`] (Comprehensive Rules 4.3 §16-2).
    fn card_source_keyword_matches(
        &self,
        handle: PermanentHandle,
        matches: &dyn Fn(crate::enums::Keyword) -> bool,
    ) -> bool {
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
        if face_keywords(card_data).iter().any(|k| matches(*k)) {
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
        // `<Succession>` adoption: the adopted card's FACE keywords and its
        // top-scope `grant_keyword` clauses also reach the carrier.
        let adopted = self.succession_source_indices(handle);
        for (source_index, data_index, src_id, src_handle) in source_ids {
            let is_under = source_index + 1 < stack_size;
            let is_adopted = adopted.contains(&source_index);
            if is_under
                && inherited_keywords(&self.card_data[data_index])
                    .iter()
                    .any(|k| matches(*k))
            {
                return true;
            }
            if is_adopted
                && face_keywords(&self.card_data[data_index])
                    .iter()
                    .any(|k| matches(*k))
            {
                return true;
            }
            let Some(effects) = self.effects_for_card(&src_id, src_handle) else {
                continue;
            };
            for effect in effects.iter() {
                if !effect.declarative
                    || !Game::source_effect_is_active(source_index, stack_size, is_adopted, effect)
                {
                    continue;
                }
                if !effect.granted_keyword.is_some_and(matches) {
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
        // DigiLink Shape-B (G-LINK-INHERITED-ESS): a link card's ESS keyword
        // grant reaches its HOST. The `card_sources` walk above never scans
        // `linked_cards`, so a linked Option's `<Piercing>` (BT25-100) or any
        // other link-card keyword grant would not register at query time.
        // Fold in every link card: both its PRINTED link keywords
        // (`link_text`, 4-3-5) and its `.inherited()` / `.linked()`
        // `grant_keyword` clauses count. A single
        // clause is `.inherited()` XOR `.linked()` (scope-driven), so accepting
        // either cannot double-fire. (Materializing `grant_keyword` clauses also
        // reach `modifiers.has_keyword` via `tick_declarative_effects`, checked
        // first at the top of this fn; this pass additionally covers the
        // non-materialized / printed-keyword path for link cards.)
        let linked_ids: Vec<(usize, crate::card_source::CardHandle)> = perm
            .linked_cards
            .iter()
            .map(|c| (c.data_index, c.handle()))
            .collect();
        for (data_index, linked_handle) in linked_ids {
            if link_keywords(&self.card_data[data_index])
                .iter()
                .any(|k| matches(*k))
            {
                return true;
            }
            let linked_id = self.card_data[data_index].card_id.clone();
            let Some(effects) = self.effects_for_card(&linked_id, linked_handle) else {
                continue;
            };
            for effect in effects.iter() {
                if !effect.declarative || !(effect.inherited || effect.linked) {
                    continue;
                }
                if !effect.granted_keyword.is_some_and(matches) {
                    continue;
                }
                if let Some(cond) = &effect.condition {
                    let ctx = crate::effect_context::EffectReadContext::new(
                        self,
                        linked_handle,
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
    ///   - `target` is the attacker (`progress_shield_target`: the attack's
    ///     attacker, resolved by stack identity once declared), AND
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
        self.progress_shield_target() == Some(target)
    }

    /// §16-39-1 / §16-39-3: the Digimon `<Progress>` currently shields -- the
    /// attacker, while it attacks and has `<Progress>` (printed, inherited,
    /// linked or granted) or `ImmunityToOpponentEffects`. The one predicate
    /// behind both [`Game::progress_excludes`] (one-shot processing) and the
    /// read-time shield on the modifier registry
    /// ([`Game::refresh_progress_shield`]).
    ///
    /// The attacker is the `pending_attack` slot until the declaration
    /// captured its stack identity; from then on it is resolved by identity
    /// (`live_attack_attacker`), so a Digimon that slides into the attacker's
    /// slot after a battle-area compaction is never taken for it (DCGO keys
    /// `<Progress>` to `attackProcess.AttackingPermanent`, an object).
    pub(crate) fn progress_shield_target(&self) -> Option<PermanentHandle> {
        let pa = self.pending_attack.as_ref()?;
        let attacker = if pa.declaration_committed && self.attack_attacker_identity.is_some() {
            self.live_attack_attacker()?
        } else {
            pa.attacker
        };
        (self.has_keyword(attacker, crate::enums::Keyword::Progress)
            || self.modifiers.has(
                attacker,
                crate::enums::ModifierType::ImmunityToOpponentEffects,
            ))
        .then_some(attacker)
    }

    /// §16-39-1 / §15-15-5-1 / §15-15-5-2: re-evaluate `<Progress>` and hand
    /// the result to the modifier registry, whose read path hides every
    /// opponent effect's entry on the shielded attacker. Called wherever
    /// [`Game::progress_shield_target`] can change: the attack declaration,
    /// every declarative tick (a `<Progress>` gained or lost mid-attack, by a
    /// grant or a changed stack or condition), a battle-area compaction, each
    /// battle's DP comparison, an effect's modifier install, and the end of
    /// the attack (`cleanup_attack`, where it drops to `None`).
    pub(crate) fn refresh_progress_shield(&mut self) {
        let shield = self.progress_shield_target();
        if self.modifiers.progress_shield() != shield {
            self.modifiers.set_progress_shield(shield);
        }
    }

    /// §15-15-5-1 / §16-39-1: whether an effect of category `source_kind`
    /// controlled by `effect_controller` can affect `target` right now -- not
    /// shielded by `<Progress>` while attacking, and not covered by a
    /// `CannotBeAffected` immunity. The single gate for one-shot processing;
    /// `EffectContext::can_affect_permanent` is this with the context's
    /// controller and kind, and resume terminals that run outside an
    /// `EffectContext` (e.g. the per-color batch delete) call it directly.
    pub fn effect_can_affect(
        &self,
        target: PermanentHandle,
        effect_controller: crate::enums::PlayerId,
        source_kind: crate::enums::EffectSourceKind,
    ) -> bool {
        !self.progress_excludes(target, Some(effect_controller))
            && !self.permanent_is_unaffected_by_effect(target, effect_controller, source_kind)
    }

    /// §15-15-5-1: whether `target` is unaffected by an effect of category
    /// `source_kind` controlled by `effect_controller`. Thin wrapper over
    /// [`crate::modifiers::ModifierRegistry::is_unaffected_by`], whose
    /// predicate also drives the read-time suppression of modifiers that
    /// were applied to `target` (§15-15-5-2).
    pub fn permanent_is_unaffected_by_effect(
        &self,
        target: PermanentHandle,
        effect_controller: crate::enums::PlayerId,
        source_kind: crate::enums::EffectSourceKind,
    ) -> bool {
        self.modifiers
            .is_unaffected_by(target, effect_controller, source_kind)
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
        let adopted = self.succession_source_indices(target);
        for (source_index, src) in perm.card_sources.iter().enumerate() {
            let card_data = &self.card_data[src.data_index];
            let mut keywords = if source_index + 1 == stack_size {
                face_keywords(card_data)
            } else {
                inherited_keywords(card_data)
            };
            if source_index + 1 < stack_size && adopted.contains(&source_index) {
                keywords.extend(face_keywords(card_data));
            }
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
        let adopted = self.succession_source_indices(target);
        let mut total = 0;
        for (source_index, source) in permanent.card_sources.iter().enumerate() {
            let is_adopted = adopted.contains(&source_index);
            let card_id = source.card_id(&self.card_data).to_string();
            let Some(effects) = self.effects_for_card(&card_id, source.handle()) else {
                continue;
            };
            for effect in effects.iter() {
                if !effect.declarative
                    || !Game::source_effect_is_active(source_index, stack_size, is_adopted, effect)
                {
                    continue;
                }
                // `.linked()` effects are Link-ESS auras that boost the HOST when
                // THIS card is a link card attached to a host. They must NOT be
                // summed here (where `target` is the card's own permanent) — the
                // `linked_cards` loop below handles that direction.
                if effect.linked {
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
        // DigiLink Shape-B (G-LINK-INHERITED-ESS): a link card's `.linked()`
        // static-DP Link-ESS applies to its host. The `card_sources` loop above
        // never scans `linked_cards`; fold them in here with the host as target.
        let linked: Vec<(String, crate::card_source::CardHandle)> = permanent
            .linked_cards
            .iter()
            .map(|c| (c.card_id(&self.card_data).to_string(), c.handle()))
            .collect();
        for (card_id, source_card) in linked {
            let Some(effects) = self.effects_for_card(&card_id, source_card) else {
                continue;
            };
            for effect in effects.iter() {
                // DigiLink Shape-B (G-LINK-INHERITED-ESS): a link card's static
                // DP ESS reaches its host under BOTH the `.linked()` (BT25-101)
                // and `.inherited()` conventions. XOR-scoped, so no double-count;
                // the `card_sources` loop above never scans `linked_cards`.
                if !effect.declarative || !(effect.linked || effect.inherited) {
                    continue;
                }
                if effect.materializes_declarative_state
                    || effect.dp_modifier == 0
                    || effect.dp_modifier_fn.is_some()
                    || effect.applies_to_opponent_security_dp
                {
                    continue;
                }
                let rctx = EffectReadContext::new(self, source_card, Some(target), target.player);
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
    /// restrictions. Controller-wide player-only attacks are checked before
    /// `CanAttackTargetDefendingPermanent`, which overrides target-carried bans
    /// but cannot grant permission to attack Digimon in general.
    pub fn attack_target_blocked_by_modifier(
        &self,
        attacker: crate::permanent::PermanentHandle,
        target: crate::permanent::PermanentHandle,
    ) -> bool {
        if self.modifiers.player_has(attacker.player, ModifierType::MayAttackPlayerOnly) {
            return true;
        }
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
        let Some(permanent) = self
            .players
            .get(perm.player as usize)
            .and_then(|p| p.battle_area.get(perm.index as usize))
        else {
            return 0;
        };
        let Some(source) = permanent.card_sources.get(source_index) else {
            return 0;
        };
        let is_adopted = self.succession_source_indices(perm).contains(&source_index);
        let Some(effects) =
            self.registry_effects(source.card_id(&self.card_data), source.handle())
        else {
            return 0;
        };

        self.source_dp_from_effects(perm, source_index, is_adopted, &effects)
    }

    pub(crate) fn source_dp_from_effects(
        &self,
        perm: PermanentHandle,
        source_index: usize,
        is_adopted: bool,
        effects: &[Effect],
    ) -> i32 {
        use crate::effect_context::EffectReadContext;
        let Some(permanent) = self
            .players
            .get(perm.player as usize)
            .and_then(|p| p.battle_area.get(perm.index as usize))
        else {
            return 0;
        };
        let Some(source) = permanent.card_sources.get(source_index) else {
            return 0;
        };
        let stack_size = permanent.card_sources.len();
        let mut total = 0i32;
        for effect in effects.iter() {
            if effect.dp_modifier == 0 && effect.dp_modifier_fn.is_none() {
                continue;
            }
            if !Game::source_effect_is_active(source_index, stack_size, is_adopted, effect) {
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

impl Game {
    /// Keyword query by TYPE: `true` when the permanent has ANY keyword of the
    /// same type as `kind`, whatever its numerical value or specified card
    /// (Comprehensive Rules 4.3 §16-2: "A keyword effect is considered to be
    /// the same type even if it has a different numerical value or specified
    /// card included in the icon enclosed in brackets"). So `<Fragment (1)>`
    /// and `<Fragment (2)>` both answer `has_keyword_type(h, &Fragment(0))`.
    ///
    /// Scans the same origins as [`Game::has_keyword`] (modifier grants, face,
    /// inherited, adopted, declarative grants, link cards). Use this for a
    /// reference that names a keyword without a parameter ("a Digimon with
    /// <Fragment>"); keep [`Game::has_keyword`] where the exact value matters.
    pub fn has_keyword_type(&self, handle: PermanentHandle, kind: &crate::enums::Keyword) -> bool {
        self.modifiers.has_keyword_type(handle, kind)
            || (kind.same_type(&crate::enums::Keyword::Blocker)
                && self.collision_grants_blocker(handle))
            || self.card_source_keyword_matches(handle, &|k| k.same_type(kind))
    }
}
