//! Expand security trash into owned moves and actual-result finalizers.
use super::resolve_player;
use crate::dsl_cards::bindings::Bindings;
use crate::{effect_context::EffectContext, replacement::ReplacementOutcome};
use digimon_dsl::compiled::CompiledStep;

pub(super) fn expand(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> Option<Vec<CompiledStep>> {
    match step {
        CompiledStep::TrashTopSecurity {
            of,
            count,
            leave,
            bind_count_as,
        } => {
            let player = resolve_player(ctx, *of);
            let target = ctx
                .source_permanent
                .unwrap_or(crate::permanent::PermanentHandle {
                    player: ctx.player,
                    index: 0,
                });
            let number = if let Some(leave) = leave {
                ctx.game.player(player).security.len().saturating_sub(
                    super::super::formula_eval::evaluate(leave, ctx, target).max(0) as usize,
                )
            } else {
                count.as_ref().map_or(1, |n| {
                    super::super::formula_eval::evaluate(n, ctx, target).max(0) as usize
                })
            };
            if let Some(name) = bind_count_as {
                bindings.insert_literal(name, 0);
            }
            // The simultaneous operation cannot trash more physical cards
            // than this stack contains. Observers wait until the body ends.
            let number = number.min(ctx.game.player(player).security.len());
            Some(
                (0..number)
                    .map(|_| CompiledStep::SecurityTrashOne {
                        player,
                        bind_as: bind_count_as.clone(),
                        cancel: false,
                    })
                    .collect(),
            )
        }
        CompiledStep::TrashTopSecurityAndCancelReplacement { of, .. } => {
            Some(vec![CompiledStep::SecurityTrashOne {
                player: resolve_player(ctx, *of),
                bind_as: None,
                cancel: true,
            }])
        }
        CompiledStep::SecurityTrashOne {
            player,
            bind_as,
            cancel,
        } => {
            let Some(index) = ctx.game.player(*player).security.len().checked_sub(1) else {
                return Some(vec![]);
            };
            ctx.game.ensure_security_materialized(*player, index);
            let card = ctx.game.player(*player).security[index].handle();
            ctx.game.security_trash_receipts.push((card.0, false));
            ctx.game
                .start_security_trash(*player, card, ctx.player, true);
            Some(vec![CompiledStep::SecurityTrashFinalize {
                card: card.0,
                bind_as: bind_as.clone(),
                cancel: *cancel,
            }])
        }
        _ => None,
    }
}

pub(super) fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> bool {
    let CompiledStep::SecurityTrashFinalize {
        card,
        bind_as,
        cancel,
    } = step
    else {
        return false;
    };
    let paid = ctx
        .game
        .security_trash_receipts
        .iter()
        .rposition(|(c, _)| c == card)
        .map(|i| ctx.game.security_trash_receipts.remove(i).1)
        .unwrap_or(false);
    if let Some(name) = bind_as {
        let count = bindings.get_literal(name).unwrap_or(0) + i64::from(paid);
        bindings.insert_literal(name, count);
    }
    if paid && *cancel {
        super::replacement_outcome::set_outcome(ctx, ReplacementOutcome::Cancelled);
    }
    true
}
