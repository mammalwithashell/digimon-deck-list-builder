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
        CompiledStep::TrashFromTop { of, count } => {
            let p = resolve_player(ctx, *of);
            ctx.trash_from_top(p, *count);
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
        CompiledStep::TrashTopSecurity { of } => {
            let p = resolve_player(ctx, *of);
            ctx.trash_top_security(p);
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
