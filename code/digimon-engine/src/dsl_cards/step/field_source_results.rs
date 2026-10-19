//! Publish actual field-top movement after the complete immediate window.
use crate::dsl_cards::{
    binding_ref::{resolve_binding_ref, ResolvedBinding},
    bindings::Bindings,
};
use crate::effect_context::EffectContext;
use crate::enums::CardSourceRef;
use digimon_dsl::compiled::CompiledStep;
pub(super) fn expand(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> Option<Vec<CompiledStep>> {
    let CompiledStep::PlaceAsBottomSource {
        source,
        target,
        face_down,
        bind_placed_as: Some(name),
    } = step
    else {
        return None;
    };
    let Some(CardSourceRef::Material(donor, index)) =
        super::play_digivolve::resolve_card_source_ref(source, ctx, bindings)
    else {
        return None;
    };
    let p = ctx
        .game
        .player(donor.player)
        .battle_area
        .get(donor.index as usize)?;
    if index + 1 != p.card_sources.len() {
        return None;
    }
    let Some(ResolvedBinding::Permanent(target)) = resolve_binding_ref(target, ctx, bindings)
    else {
        return None;
    };
    bindings.remove(name);
    let Some(donor) = ctx.game.source_move_anchor(donor) else {
        return Some(Vec::new());
    };
    let target_breeding = target.index == crate::action::space::BREEDING_TARGET as u8;
    let Some(target) = ctx.game.source_move_anchor(target) else {
        return Some(Vec::new());
    };
    let source_breeding = ctx
        .source_permanent
        .is_some_and(|h| h.index == crate::action::space::BREEDING_TARGET as u8);
    let source_anchor = ctx
        .source_permanent
        .and_then(|h| ctx.game.source_move_anchor(h));
    // Receipt names depend only on outstanding work, never completed history.
    let mut receipt = 0;
    while ctx
        .game
        .field_source_receipts
        .iter()
        .any(|(id, _)| *id == receipt)
    {
        receipt += 1;
    }
    ctx.game.field_source_receipts.push((receipt, None));
    Some(vec![
        CompiledStep::FieldSourceCaptured {
            receipt,
            donor,
            target,
            target_breeding,
            face_down: *face_down,
        },
        CompiledStep::FieldSourceFinalize {
            receipt,
            bind_as: name.clone(),
            source_anchor,
            source_breeding,
        },
    ])
}
pub(super) fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> bool {
    match step {
        CompiledStep::FieldSourceCaptured {
            receipt,
            donor,
            target,
            target_breeding,
            face_down,
        } => {
            if let (Some(donor), Some(target)) = (
                ctx.game.resolve_source_move_anchor(*donor, false),
                ctx.game
                    .resolve_source_move_anchor(*target, *target_breeding),
            ) {
                ctx.game.begin_field_top_source_move_with_receipt(
                    donor,
                    target,
                    ctx.player,
                    *face_down,
                    false,
                    Some(ctx.source_card),
                    ctx.source_kind,
                    Some(*receipt),
                );
            }
            true
        }
        CompiledStep::FieldSourceFinalize {
            receipt,
            bind_as,
            source_anchor,
            source_breeding,
        } => {
            if let Some(index) = ctx
                .game
                .field_source_receipts
                .iter()
                .position(|(id, _)| id == receipt)
            {
                let (_, result) = ctx.game.field_source_receipts.remove(index);
                if let Some(card) = result {
                    bindings.insert_card(bind_as, card);
                }
            }
            if let Some(anchor) = source_anchor {
                ctx.source_permanent = ctx
                    .game
                    .resolve_source_move_anchor(*anchor, *source_breeding);
            }
            true
        }
        _ => false,
    }
}
