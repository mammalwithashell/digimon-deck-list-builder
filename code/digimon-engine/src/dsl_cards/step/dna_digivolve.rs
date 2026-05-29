//! G-DSL-EOT-DNA-INLINE — step lowering for `CompiledStep::MayDnaDigivolveNow`.
//!
//! Bridges the DSL `CompiledPredicate` filters into runtime closures that
//! capture `Bindings`, `source_card / source_permanent / source_kind`,
//! and the controller's player id — then calls the
//! `EffectContext::may_dna_digivolve_now` orchestrator which owns the
//! three-stage selection chain (partner pick → target pick → DNA digivolve).
//!
//! Used by the 6 cards that print `[End of Your Turn] This Digimon and any
//! of your other Digimon may DNA digivolve into a Digimon card in the hand`
//! (BT12-021, BT12-047, BT17-007, BT17-019, BT22-008, BT22-017). See the
//! YAML migration for the authoring shape.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledStep};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use crate::effect_context::{EffectContext, EffectReadContext};

pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &Bindings) -> bool {
    match step {
        CompiledStep::MayDnaDigivolveNow {
            anchor,
            partner_filter,
            target_filter,
            cost,
            ignore_requirements,
            optional,
            prompt,
        } => {
            // Resolve anchor binding to a permanent handle. Defaults to the
            // trigger's source permanent (`anchor: source` → `CompiledBindingRef::Source`).
            let anchor_handle = match resolve_binding_ref(anchor, ctx, bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                _ => {
                    // Anchor binding unresolved → silent no-op.
                    return true;
                }
            };

            // Snapshot context the predicate-eval closures need to rebuild
            // `EffectReadContext` at install/callback time.
            let source_card = ctx.source_card;
            let source_permanent = ctx.source_permanent;
            let source_kind = ctx.source_kind;
            let player = ctx.player;
            let bindings_snapshot = bindings.clone();

            let partner_pred: CompiledPredicate = partner_filter.clone();
            let target_pred: CompiledPredicate = target_filter.clone();
            let bindings_for_partner = bindings_snapshot.clone();
            let bindings_for_target = bindings_snapshot;

            let partner_closure: Arc<
                dyn Fn(&crate::game::Game, crate::permanent::PermanentHandle) -> bool + Send + Sync,
            > = Arc::new(move |game, h| {
                let read_ctx = EffectReadContext::new_with_source_kind(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    player,
                );
                eval_predicate_with_bindings(
                    &partner_pred,
                    &read_ctx,
                    PredicateSubject::Permanent(h),
                    Some(&bindings_for_partner),
                )
            });

            let target_closure: Arc<dyn Fn(&crate::game::Game, usize) -> bool + Send + Sync> =
                Arc::new(move |game, hand_idx| {
                    let card = match game.player(player).hand.get(hand_idx).map(|c| c.handle()) {
                        Some(c) => c,
                        None => return false,
                    };
                    let read_ctx = EffectReadContext::new_with_source_kind(
                        game,
                        source_card,
                        source_permanent,
                        source_kind,
                        player,
                    );
                    eval_predicate_with_bindings(
                        &target_pred,
                        &read_ctx,
                        PredicateSubject::Card(card),
                        Some(&bindings_for_target),
                    )
                });

            ctx.may_dna_digivolve_now(
                anchor_handle,
                partner_closure,
                target_closure,
                *cost,
                *ignore_requirements,
                *optional,
                prompt.as_deref(),
                None,
            );
            true
        }
        _ => false,
    }
}
