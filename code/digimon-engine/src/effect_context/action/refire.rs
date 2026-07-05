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
        let _ =
            self.refire_target_effect_inner(source, timing_filter, self.player, false, optional);
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
        )
    }

    pub(crate) fn refire_target_effect_inner(
        &mut self,
        target: PermanentHandle,
        timing_filter: TimingFilter,
        selecting_player: PlayerId,
        bypass_once_per_turn: bool,
        optional: bool,
    ) -> bool {
        let effects: Vec<ReFireableEffect> = timing_filter
            .timing_keys()
            .iter()
            .flat_map(|timing_key| enumerate_refireable_effects(self.game, target, timing_key))
            .collect();
        self.dispatch_refireable_effects(effects, selecting_player, bypass_once_per_turn, optional)
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
        // Once-per-turn accounting is BYPASSED for the foreign card: OPT
        // counts key `(source_card, slot)` on the carrier permanent, and the
        // foreign entries reuse the CARRIER's card handle — consulting or
        // bumping them would alias the carrier's own effect slots. That is
        // also the faithful reading: the placed card sat in the trash all
        // turn, so its effects have no prior activations to gate on (DCGO
        // enumerates a fresh effect instance for the placed card and only
        // checks `CanUse`).
        self.dispatch_refireable_effects(effects, selecting_player, true, false)
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
    ) -> bool {
        let mut effects: Vec<ReFireableEffect> = effects
            .into_iter()
            .filter(|effect| bypass_once_per_turn || self.refire_effect_slot_available(effect))
            .collect();
        for effect in &mut effects {
            effect.attribution_source_card = Some(self.source_card);
            effect.attribution_source_kind = Some(self.source_kind);
            effect.bypass_once_per_turn = bypass_once_per_turn;
            effect.controller = self.player;
        }
        match effects.as_slice() {
            [] => false,
            [effect] if !optional => {
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
