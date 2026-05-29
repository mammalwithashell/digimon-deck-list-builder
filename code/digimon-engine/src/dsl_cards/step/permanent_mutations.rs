//! Synchronous permanent-mutation step lowering (Phase 2c).
//!
//! Verbs: DeletePermanent, ReturnToHand, ReturnToDeck, Suspend, Unsuspend,
//! DeDigivolve. All are binding-consuming: the `target` field resolves to
//! `ResolvedBinding::Permanent`; any other variant is silently skipped (same
//! convention as the 2b zone-move handlers).

use digimon_dsl::compiled::{CompiledPermanentProperty, CompiledStep};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::effect_context::EffectContext;

/// Returns `true` if `step` is a permanent-mutation family handled here.
/// Unknown steps fall through (the caller may try other families).
pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &mut Bindings) -> bool {
    match step {
        CompiledStep::DeletePermanent { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                if ctx.delete_permanent(h) {
                    bindings.record_deleted(h);
                }
            }
            true
        }
        CompiledStep::DeleteBoundPermanents { binding } => {
            if let Some(mut handles) = bindings.get_permanent_list(binding) {
                handles.sort_by_key(|h| (h.player, h.index));
                handles.reverse();
                for handle in handles {
                    if ctx.delete_permanent(handle) {
                        bindings.record_deleted(handle);
                    }
                }
            }
            true
        }
        CompiledStep::TrashBreedingPermanent { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                ctx.trash_breeding_permanent(h);
            }
            true
        }
        CompiledStep::ReturnToHand { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                if let Some(card) = ctx.return_to_hand(h) {
                    bindings.record_added_to_hand(card);
                }
            }
            true
        }
        CompiledStep::Suspend { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                let was_unsuspended = ctx
                    .game
                    .player(h.player)
                    .battle_area
                    .get(h.index as usize)
                    .is_some_and(|p| !p.is_suspended);
                ctx.suspend(h);
                if was_unsuspended {
                    bindings.record_suspended(h);
                }
            }
            true
        }
        CompiledStep::Unsuspend { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                ctx.unsuspend(h);
            }
            true
        }
        CompiledStep::ReturnToDeck {
            target,
            position,
            include_sources,
        } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                let position = super::map_stack_position(*position);
                let top_card = ctx
                    .game
                    .player(h.player)
                    .battle_area
                    .get(h.index as usize)
                    .map(|p| p.top_card().handle());
                if *include_sources {
                    if ctx.return_stack_to_deck(h, position) {
                        if let Some(card) = top_card {
                            bindings.record_returned_to_deck(card);
                        }
                    }
                } else if ctx.return_to_deck(h, position) {
                    if let Some(card) = top_card {
                        bindings.record_returned_to_deck(card);
                    }
                }
            }
            true
        }
        CompiledStep::PlacePermanentOnSecurity {
            of,
            target,
            position,
            face_up,
        } => {
            let player = crate::dsl_cards::step::resolve_player(ctx, *of);
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                let position = super::map_stack_position(*position);
                let _ = ctx.place_permanent_on_security(player, h, position, *face_up);
            }
            true
        }
        CompiledStep::PlacePermanentOnOwnersSecurity {
            target,
            position,
            face_up,
        } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                let owner = h.player;
                let position = super::map_stack_position(*position);
                let _ = ctx.place_permanent_on_security(owner, h, position, *face_up);
            }
            true
        }
        CompiledStep::DeDigivolve {
            target,
            amount,
            stop_at_level,
        } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                // Engine signature is (target, stop_at_level, amount) — note stop_at_level
                // precedes amount here, opposite to CompiledStep field order.
                ctx.de_digivolve(h, *stop_at_level, *amount);
            }
            true
        }
        CompiledStep::TrashAllSources { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                ctx.trash_all_sources(h);
            }
            true
        }
        CompiledStep::BindPermanentProperty {
            from,
            property,
            bind_as,
        } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(from, ctx, bindings) {
                if let Some(value) = permanent_property(ctx, h, *property) {
                    bindings.insert_literal(bind_as, value);
                }
            }
            true
        }
        _ => false,
    }
}

fn permanent_property(
    ctx: &EffectContext<'_>,
    handle: crate::permanent::PermanentHandle,
    property: CompiledPermanentProperty,
) -> Option<i64> {
    let permanent = if handle.index == crate::action::space::BREEDING_TARGET as u8 {
        ctx.game.player(handle.player).breeding_area.as_ref()?
    } else {
        ctx.game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)?
    };
    match property {
        CompiledPermanentProperty::Level => permanent.level(&ctx.game.card_data).map(i64::from),
    }
}
