//! DigiXros transaction mutations on `EffectContext` — extracted by mechanic.

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
    pub fn allow_digixros_material_zone(
        &mut self,
        zone: DigiXrosMaterialZone,
        max_count: Option<u8>,
    ) -> bool {
        let Some(transaction) = self.game.pending_digixros_transaction_mut() else {
            return false;
        };
        transaction.allow_zone(DigiXrosZoneAllowance { zone, max_count });
        true
    }

    pub fn allow_digixros_under_tamer_materials(&mut self, max_count: Option<u8>) -> bool {
        self.allow_digixros_material_zone(DigiXrosMaterialZone::UnderTamer, max_count)
    }

    pub fn allow_digixros_trash_materials(&mut self, max_count: Option<u8>) -> bool {
        self.allow_digixros_material_zone(DigiXrosMaterialZone::Trash, max_count)
    }

    pub fn add_digixros_one_shot_cost_delta(&mut self, delta: i16) -> bool {
        let Some(transaction) = self.game.pending_digixros_transaction_mut() else {
            return false;
        };
        transaction.add_one_shot_cost_delta(delta);
        true
    }

    pub fn register_digixros_wildcard_for_current_turn(
        &mut self,
        material_card: CardHandle,
        required_zone: Option<DigiXrosMaterialZone>,
    ) -> bool {
        self.game.register_digixros_wildcard_for_current_turn(
            self.player,
            material_card,
            required_zone,
        );
        true
    }

    pub fn add_digixros_wildcard_to_pending_transaction(
        &mut self,
        material_card: CardHandle,
        required_zone: Option<DigiXrosMaterialZone>,
    ) -> bool {
        let Some(transaction) = self.game.pending_digixros_transaction_mut() else {
            return false;
        };
        let substitution = match required_zone {
            Some(zone) => {
                crate::digixros::DigiXrosWildcardSubstitution::once_from_zone(material_card, zone)
            }
            None => crate::digixros::DigiXrosWildcardSubstitution::once(material_card),
        };
        transaction.add_wildcard_substitution(substitution);
        true
    }

    pub fn apply_optional_digixros_transaction_modifier<F>(
        &mut self,
        accepted: bool,
        apply: F,
    ) -> bool
    where
        F: FnOnce(&mut DigiXrosTransaction) -> bool,
    {
        let Some(transaction) = self.game.pending_digixros_transaction_mut() else {
            return false;
        };
        transaction.apply_optional_modifier(accepted, apply)
    }

    pub fn preattach_digixros_material(
        &mut self,
        origin: DigiXrosMaterialOrigin,
        card: &CardData,
        cost_delta: i16,
    ) -> Result<usize, DigiXrosMaterialValidationError> {
        let Some(transaction) = self.game.pending_digixros_transaction_mut() else {
            return Err(DigiXrosMaterialValidationError::NoMatchingRecipeSlot);
        };
        transaction.try_pre_attach_material(origin, card, cost_delta)
    }

    pub fn preattach_digixros_material_by_handle(
        &mut self,
        card: CardHandle,
        cost_delta: i16,
    ) -> Result<usize, DigiXrosMaterialValidationError> {
        let Some(card_data) = self.game.card_data_for_handle(card).cloned() else {
            return Err(DigiXrosMaterialValidationError::NoMatchingRecipeSlot);
        };
        let Some(origin) = find_digixros_material_origin(self.game, card) else {
            return Err(DigiXrosMaterialValidationError::NoMatchingRecipeSlot);
        };
        self.preattach_digixros_material(origin, &card_data, cost_delta)
    }
}
