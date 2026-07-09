//! Draw/trash/shuffle/hatch/trash_top_security step lowering.

use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::step::resolve_player;
use crate::effect_context::EffectContext;

pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>) -> bool {
    match step {
        CompiledStep::Draw { of, count } => {
            let p = resolve_player(ctx, *of);
            ctx.draw(p, *count);
            true
        }
        CompiledStep::TrashFromTop {
            of,
            count,
            count_fn,
        } => {
            let p = resolve_player(ctx, *of);
            let n = match count_fn {
                None => *count as usize,
                Some(formula) => {
                    // Formula counts (BT15-102: "trash the top 2 cards …
                    // for each of this Digimon's level 6 digivolution
                    // cards") evaluate against the effect carrier so
                    // `source_stack_*` leaves read "this Digimon".
                    let target =
                        ctx.source_permanent
                            .unwrap_or(crate::permanent::PermanentHandle {
                                player: ctx.player,
                                index: 0,
                            });
                    crate::dsl_cards::formula_eval::evaluate(formula, ctx, target).max(0) as usize
                }
            };
            ctx.trash_from_top(p, n.min(u8::MAX as usize) as u8);
            true
        }
        CompiledStep::ShuffleDeck { of } => {
            let p = resolve_player(ctx, *of);
            ctx.shuffle_deck(p);
            true
        }
        CompiledStep::ShuffleSecurity { of } => {
            let p = resolve_player(ctx, *of);
            ctx.shuffle_security(p);
            true
        }
        CompiledStep::Hatch { of } => {
            let p = resolve_player(ctx, *of);
            ctx.hatch(p);
            true
        }
        CompiledStep::MoveFromBreeding { of } => {
            let p = resolve_player(ctx, *of);
            ctx.move_from_breeding_by_effect(p);
            true
        }
        CompiledStep::TrashTopSecurity { of, count, leave } => {
            let p = resolve_player(ctx, *of);
            let target = ctx
                .source_permanent
                .unwrap_or(crate::permanent::PermanentHandle {
                    player: ctx.player,
                    index: 0,
                });
            let n = if let Some(formula) = leave {
                // "so that it has N cards left" — trash from the top until
                // `leave` cards remain: `max(0, security_count - leave)`. Clamps
                // so an already-short stack trashes nothing (never underflows /
                // over-trashes). DCGO `Enemy.SecurityCards.Count - 1` with
                // `fromTop: true` (BT21-098). G-DSL-TRASH-TOP-SECURITY-LEAVE.
                let leave_n =
                    crate::dsl_cards::formula_eval::evaluate(formula, ctx, target).max(0) as usize;
                ctx.game.player(p).security.len().saturating_sub(leave_n)
            } else {
                match count {
                    None => 1,
                    Some(formula) => crate::dsl_cards::formula_eval::evaluate(formula, ctx, target)
                        .max(0) as usize,
                }
            };
            for _ in 0..n {
                if ctx.game.player(p).security.is_empty() {
                    break;
                }
                ctx.trash_top_security(p);
            }
            true
        }
        CompiledStep::TrashBottomSecurity { of } => {
            let p = resolve_player(ctx, *of);
            ctx.trash_bottom_security(p);
            true
        }
        _ => false,
    }
}
