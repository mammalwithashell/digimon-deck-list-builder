//! Binding-consuming zone-move step lowering. Phase 2b covers
//! `AddToHandFromDeck` and `AddToHandFromTrash`; Task 5 extends with
//! reveal-pool + security-mark variants.

use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::resolve_player;
use crate::effect_context::EffectContext;

/// Returns `true` if `step` is a zone-move family handled here. Unknown
/// steps fall through (the caller may try other families).
pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &Bindings,
) -> bool {
    match step {
        CompiledStep::AddToHandFromTrash { of, card } => {
            let Some(resolved) = resolve_binding_ref(card, ctx, bindings) else {
                return true;
            };
            let p = resolve_player(ctx, *of);
            // Resolve the trash slot → CardHandle → engine API. If the
            // binding is a CardHandle directly, pass it through.
            let handle = match resolved {
                ResolvedBinding::TrashIndex(i) => {
                    let player = ctx.game.player(p);
                    player.trash.get(i as usize).map(|cs| cs.handle())
                }
                ResolvedBinding::Card(h) => Some(h),
                _ => None,
            };
            if let Some(h) = handle {
                ctx.add_to_hand_from_trash(p, h);
            }
            true
        }
        CompiledStep::AddToHandFromDeck { of, card } => {
            // Phase 2b has no way to bind a deck card (no SelectDeck variant
            // and RevealTopDeck binds into the reveal pool, not deck). The
            // only reachable case is a direct Card(handle) binding, which a
            // future step could set up. Keep behaviour strict: no-op unless
            // we have a concrete CardHandle.
            let Some(resolved) = resolve_binding_ref(card, ctx, bindings) else {
                return true;
            };
            let p = resolve_player(ctx, *of);
            if let ResolvedBinding::Card(h) = resolved {
                let _ = ctx.add_to_hand_from_deck(p, h);
            }
            true
        }
        _ => false,
    }
}
