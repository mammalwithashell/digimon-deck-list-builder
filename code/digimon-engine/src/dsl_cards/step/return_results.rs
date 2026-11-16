//! Explicit simultaneous returns, with actual committed-result accounting.
use crate::dsl_cards::bindings::Bindings;
use crate::effect_context::EffectContext;
use digimon_dsl::compiled::CompiledStep;

pub(super) fn expand(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &Bindings,
) -> Option<Vec<CompiledStep>> {
    let (mut targets, position, include_sources) = match step {
        CompiledStep::ReturnToDeck { target, position, include_sources } => {
            let targets = match crate::dsl_cards::binding_ref::resolve_binding_ref(target, ctx, bindings) {
                Some(crate::dsl_cards::binding_ref::ResolvedBinding::Permanent(h)) => vec![h],
                _ => Vec::new(),
            };
            (targets, Some(*position), *include_sources)
        }
        CompiledStep::ReturnAllToHand { over, selector } => {
            let mut targets = super::permanent_scan::scan(ctx, over, Some(bindings));
            if let Some(selector) = selector {
                if let Some(extreme) = super::selections::selected_field_extreme(ctx.game, &targets, *selector) {
                    targets.retain(|h| super::selections::field_value(ctx.game, *h, *selector) == Some(extreme));
                }
            }
            (targets, None, false)
        }
        CompiledStep::ReturnAllToDeck {
            over,
            selector,
            position,
            include_sources,
        } => {
            let mut targets = super::permanent_scan::scan(ctx, over, Some(bindings));
            if let Some(selector) = selector {
                if let Some(extreme) = super::selections::selected_field_extreme(ctx.game, &targets, *selector) {
                    targets.retain(|h| super::selections::field_value(ctx.game, *h, *selector) == Some(extreme));
                }
            }
            (targets, Some(*position), *include_sources)
        }
        _ => return None,
    };
    targets.sort_by_key(|h| (h.player, h.index));
    targets.dedup();
    targets.retain(|h| ctx.can_affect_permanent(*h));
    if targets.is_empty() {
        return Some(Vec::new());
    }
    let targets = targets
        .into_iter()
        .map(|h| (h.player, ctx.game.capture_digivolve_host_identity(h)))
        .collect();
    let receipt = ctx.game.next_return_receipt;
    ctx.game.next_return_receipt = receipt.checked_add(1).expect("return receipt exhausted");
    Some(vec![
        CompiledStep::ReturnCapturedBatch {
            receipt,
            targets,
            position,
            include_sources,
        },
        CompiledStep::ReturnReceiptFinalize { receipt },
    ])
}
pub(super) fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> bool {
    match step {
        CompiledStep::ReturnCapturedBatch {
            receipt,
            targets,
            position,
            include_sources,
        } => {
            ctx.game.return_receipts.push((*receipt, Vec::new()));
            let targets = targets
                .iter()
                .filter_map(|(p, id)| ctx.game.resolve_digivolve_host_identity(*p, *id))
                .collect();
            // A resumed DSL tail can run after the queue has restored its
            // outer attribution. Preserve the effect controller just as the
            // direct EffectContext return path does, including for the deck
            // insertion observer and opponent-scoped replacement windows.
            let previous = ctx.game.effect_source_player.replace(ctx.player);
            crate::replacement::start_departure_batch(
                ctx.game,
                targets,
                *receipt,
                position.map(super::map_stack_position),
                *include_sources,
            );
            ctx.game.effect_source_player = previous;
            true
        }
        CompiledStep::ReturnReceiptFinalize { receipt } => {
            if let Some(i) = ctx
                .game
                .return_receipts
                .iter()
                .position(|(id, _)| id == receipt)
            {
                for result in ctx.game.return_receipts.remove(i).1 {
                    if result.to_hand {
                        bindings.record_returned_card(result.card);
                        if result.added_to_hand {
                            bindings.record_added_to_hand(result.card);
                        }
                    } else {
                        bindings.record_returned_to_deck(result.card);
                    }
                }
            }
            true
        }
        _ => false,
    }
}
