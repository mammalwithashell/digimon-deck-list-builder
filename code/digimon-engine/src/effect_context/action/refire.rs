//! Effect re-fire machinery on `EffectContext` — extracted by mechanic.

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

impl Game {
    /// Whether a once-per-turn effect still has a use left for a refire of
    /// `effect` (unlimited effects always do).
    pub(crate) fn refire_effect_slot_available(&self, effect: &ReFireableEffect) -> bool {
        let Some(effects) = self.effects_for_card(&effect.card_id, effect.source_card) else {
            return false;
        };
        let Some(effect_body) = effects.get(effect.effect_id as usize) else {
            return false;
        };
        if effect_body.max_per_turn == 0 {
            return true;
        }
        let Some(perm) = self
            .players
            .get(effect.source.player as usize)
            .and_then(|p| p.battle_area.get(effect.source.index as usize))
        else {
            return false;
        };
        let opt_key = self.opt_key_for_source(
            effect.source,
            effect.source_card,
            effect_body.inherited,
            effect_body.linked,
            effect_body.shared_opt_group.unwrap_or(effect.effect_id),
        );
        perm.activation_count(effect.source_card, opt_key) < effect_body.max_per_turn
    }

    /// Whether `target` has at least one effect a refire with
    /// `filter` could activate right now: a refireable effect of one of the
    /// filter's timings whose once-per-turn slot is still open. This is the
    /// engine's reading of DCGO's `EffectList(OnEnterFieldAnyone).Any(
    /// CanBeEffectCandidate)` (BT24_102.cs:93-98) — the refire's own candidate
    /// set, before the effect's activation condition runs.
    pub fn has_refireable_effect(&self, target: PermanentHandle, filter: TimingFilter) -> bool {
        filter.timing_keys().iter().any(|key| {
            enumerate_refireable_effects(self, target, key)
                .iter()
                .any(|effect| self.refire_effect_slot_available(effect))
        })
    }
}

impl<'a> EffectContext<'a> {
    pub fn refire_effect_from_permanent(
        &mut self,
        source: PermanentHandle,
        timing_key: &str,
        optional: bool,
    ) -> Result<(), EffectRefireError> {
        let Some(timing_filter) = TimingFilter::from_timing_key(timing_key) else {
            return Err(EffectRefireError::InvalidTiming(timing_key.to_string()));
        };
        let _ = self.refire_target_effect_inner(
            source,
            timing_filter,
            self.player,
            false,
            optional,
            false,
        );
        Ok(())
    }

    /// Refire one of `target`'s registered `[On Play]` or `[When Digivolving]`
    /// effects without treating the target as newly played or digivolved.
    ///
    /// Carrier semantics: the refired body sees `source_permanent` as
    /// `target`, so "this Digimon" reads the target permanent. Source
    /// attribution remains this context's `source_card`, so Homeros-style
    /// "this card's effect" predicates read the refire grantor.
    ///
    /// Once-per-turn accounting uses the target effect's normal slot unless
    /// `bypass_once_per_turn` is true.
    pub fn refire_target_effect(
        &mut self,
        target: PermanentHandle,
        timing_filter: TimingFilter,
        selecting_player: PlayerId,
        bypass_once_per_turn: bool,
    ) -> bool {
        self.refire_target_effect_inner(
            target,
            timing_filter,
            selecting_player,
            bypass_once_per_turn,
            false,
            false,
        )
    }

    pub(crate) fn refire_target_effect_inner(
        &mut self,
        target: PermanentHandle,
        timing_filter: TimingFilter,
        selecting_player: PlayerId,
        bypass_once_per_turn: bool,
        optional: bool,
        target_attribution: bool,
    ) -> bool {
        let effects: Vec<ReFireableEffect> = timing_filter
            .timing_keys()
            .iter()
            .flat_map(|timing_key| enumerate_refireable_effects(self.game, target, timing_key))
            .filter(|effect| {
                bypass_once_per_turn || self.game.refire_effect_slot_available(effect)
            })
            .collect();
        let enumerated = effects.len();
        let effects = self.collapse_combined_timing_clauses(effects);
        // DCGO lists one effect object per timing tag of a printed combined
        // effect (excluding already OPT-spent effects via `CanUse`), so a
        // collapsed clause stands for `enumerated` available DCGO entries.
        let dcgo_entries = (effects.len() == 1 && enumerated > 1).then_some(enumerated);
        self.dispatch_refireable_effects(
            effects,
            selecting_player,
            bypass_once_per_turn,
            optional,
            target_attribution,
            dcgo_entries,
        )
    }

    /// A printed `[On Play] [When Digivolving]` effect is ONE effect even
    /// though it lowers to one engine `Effect` per timing. Under an `Either`
    /// filter both timings enumerate it, so keep only the first entry of each
    /// `Effect::clause_group` — the rules grant no choice between two copies
    /// of the same effect. `G-ENGINE-REFIRE-SPLITS-COMBINED-TIMING`.
    fn collapse_combined_timing_clauses(
        &self,
        effects: Vec<ReFireableEffect>,
    ) -> Vec<ReFireableEffect> {
        let mut seen: Vec<(String, CardHandle, u32)> = Vec::new();
        effects
            .into_iter()
            .filter(|e| {
                let group = self
                    .game
                    .effects_for_card(&e.card_id, e.source_card)
                    .and_then(|list| list.get(e.effect_id as usize).and_then(|x| x.clause_group));
                let Some(group) = group else {
                    return true;
                };
                let key = (e.card_id.clone(), e.source_card, group);
                if seen.contains(&key) {
                    false
                } else {
                    seen.push(key);
                    true
                }
            })
            .collect()
    }

    /// Activate one timing-filtered effect printed on a FOREIGN CARD OBJECT
    /// — the source-card refire variant (BT15-102 Apocalymon: "activate 1
    /// [On Play] effect on that card as an effect of this Digimon").
    ///
    /// `card_id` names the foreign card (typically a card just placed as a
    /// digivolution source of `carrier`); the chosen effect body runs with
    /// `carrier` as "this Digimon" (DCGO `EffectList_ForCard(timing, card)`
    /// — see [`enumerate_refireable_effects_for_card`]), attribution stays on
    /// this context's `source_card`, and controller is this context's
    /// player. When the card carries more than one eligible effect an
    /// `EffectChoice` selection surfaces for `selecting_player` (mandatory —
    /// DCGO `canNoSelect: () => false`); exactly one eligible effect runs
    /// directly. Returns `false` when the card has no eligible effect.
    pub fn activate_foreign_card_effect(
        &mut self,
        card_id: &str,
        carrier: PermanentHandle,
        timing_filter: TimingFilter,
        selecting_player: PlayerId,
    ) -> bool {
        let effects: Vec<ReFireableEffect> = timing_filter
            .timing_keys()
            .iter()
            .flat_map(|timing_key| {
                crate::effect::enumerate_refireable_effects_for_card(
                    self.game, card_id, carrier, timing_key,
                )
            })
            .collect();
        let effects = self.collapse_combined_timing_clauses(effects);
        // Once-per-turn accounting is BYPASSED for the foreign card: OPT
        // counts key `(source_card, slot)` on the carrier permanent, and the
        // foreign entries reuse the CARRIER's card handle — consulting or
        // bumping them would alias the carrier's own effect slots. That is
        // also the faithful reading: the placed card sat in the trash all
        // turn, so its effects have no prior activations to gate on (DCGO
        // enumerates a fresh effect instance for the placed card and only
        // checks `CanUse`).
        self.dispatch_refireable_effects(effects, selecting_player, true, false, false, None)
    }

    /// Shared dispatch tail for both refire variants: OPT-slot filtering,
    /// attribution/controller stamping, then run-directly (single, mandatory)
    /// or install the EffectChoice selection.
    fn dispatch_refireable_effects(
        &mut self,
        effects: Vec<ReFireableEffect>,
        selecting_player: PlayerId,
        bypass_once_per_turn: bool,
        optional: bool,
        target_attribution: bool,
        dcgo_entries: Option<usize>,
    ) -> bool {
        let mut effects: Vec<ReFireableEffect> = effects
            .into_iter()
            .filter(|effect| bypass_once_per_turn || self.refire_effect_slot_available(effect))
            .collect();
        for effect in &mut effects {
            if !target_attribution {
                effect.attribution_source_card = Some(self.source_card);
                effect.attribution_source_kind = Some(self.source_kind);
            }
            effect.bypass_once_per_turn = bypass_once_per_turn;
            effect.controller = self.player;
        }
        match effects.as_slice() {
            [] => false,
            [effect] if !optional => {
                if let Some(dcgo_entries) = dcgo_entries {
                    // For a `<Succession>`-adopted effect, DCGO's
                    // `CopyDigivolutionCardEffects` stamps `EffectSourceCard`
                    // as the permanent's top card (not the below-top source),
                    // so `last_collapsed_refire` records the carrier's top card.
                    let carrier_card = self
                        .game
                        .top_card_handle(effect.source)
                        .unwrap_or(effect.source_card);
                    let carrier_card_id = self
                        .game
                        .card_data_for_handle(carrier_card)
                        .map(|d| d.card_id.clone())
                        .unwrap_or_else(|| effect.card_id.clone());
                    self.game.last_collapsed_refire = Some(crate::game::CollapsedRefire {
                        grantor: self.source_card,
                        carrier: effect.source,
                        carrier_card,
                        carrier_card_id,
                        controller: self.player,
                        dcgo_entries,
                        events_len: self.game.events.len(),
                        first_seen_row: None,
                    });
                }
                self.game.run_refired_effect(effect.clone());
                true
            }
            _ => {
                self.game
                    .install_refire_effect_selection(selecting_player, effects, optional);
                true
            }
        }
    }
}
