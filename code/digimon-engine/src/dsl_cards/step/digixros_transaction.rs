use digimon_dsl::compiled::{CompiledBindingRef, CompiledStep, CompiledZone};

use crate::digixros::DigiXrosMaterialZone;
use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::effect_context::EffectContext;

pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &Bindings) -> bool {
    match step {
        CompiledStep::AllowDigixrosMaterialZone { zone, max_count } => {
            if let Some(zone) = map_material_zone(*zone) {
                ctx.allow_digixros_material_zone(zone, *max_count);
            }
            true
        }
        CompiledStep::AddDigixrosCostDelta { delta } => {
            ctx.add_digixros_one_shot_cost_delta(*delta);
            true
        }
        CompiledStep::PreattachDigixrosMaterial { card, cost_delta } => {
            if let Some(handle) = resolve_card_ref(card, ctx, bindings) {
                let _ = ctx.preattach_digixros_material_by_handle(handle, *cost_delta);
            }
            true
        }
        CompiledStep::RegisterDigixrosWildcardForTurn { card, zone } => {
            if let Some(handle) = resolve_card_ref(card, ctx, bindings) {
                let required_zone = zone.and_then(map_material_zone);
                ctx.register_digixros_wildcard_for_current_turn(handle, required_zone);
            }
            true
        }
        CompiledStep::AddDigixrosWildcardToPendingTransaction { card, zone } => {
            if let Some(handle) = resolve_card_ref(card, ctx, bindings) {
                let required_zone = zone.and_then(map_material_zone);
                ctx.add_digixros_wildcard_to_pending_transaction(handle, required_zone);
            }
            true
        }
        _ => false,
    }
}

fn resolve_card_ref(
    card: &CompiledBindingRef,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
) -> Option<crate::card_source::CardHandle> {
    match resolve_binding_ref(card, ctx, bindings) {
        Some(ResolvedBinding::Card(handle)) => Some(handle),
        Some(ResolvedBinding::Permanent(handle)) => ctx
            .game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|permanent| permanent.top_card().handle()),
        _ => None,
    }
}

fn map_material_zone(zone: CompiledZone) -> Option<DigiXrosMaterialZone> {
    match zone {
        CompiledZone::Hand => Some(DigiXrosMaterialZone::Hand),
        CompiledZone::BattleArea => Some(DigiXrosMaterialZone::BattleArea),
        CompiledZone::Trash => Some(DigiXrosMaterialZone::Trash),
        CompiledZone::Material => Some(DigiXrosMaterialZone::UnderTamer),
        _ => None,
    }
}
