//! Selected "by deleting ... it doesn't leave" conditions. Ordinary deletion
//! effects keep their usual target set, including unaffected permanents.
use digimon_dsl::compiled::{CompiledBindingRef, CompiledStep};

use crate::dsl_cards::{
    binding_ref::{resolve_binding_ref, ResolvedBinding},
    bindings::Bindings,
};
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::permanent::PermanentHandle;
use crate::replacement::ReplacementCause;

/// A selection's nested body runs before its lexical tail. Recognize the
/// payment only when the deletion consumes this selection's own binding.
pub(crate) fn selection_pays_deletion(
    bind_as: Option<&str>,
    then: &[CompiledStep],
    tail: &[CompiledStep],
) -> bool {
    let Some(name) = bind_as else { return false };
    let mut following = then.iter().chain(tail);
    matches!((following.next(), following.next()),
        (Some(CompiledStep::DeleteForCostReduction { target, fixed_amount: Some(_) }), _)
        | (Some(CompiledStep::DeletePermanent { target }), Some(CompiledStep::CancelReplacement))
        if matches!(target,
            CompiledBindingRef::Named(n) | CompiledBindingRef::Binding(n)
            | CompiledBindingRef::Permanent(n) | CompiledBindingRef::OfPermanent(n) if n == name))
}

/// Whether a pick hides payers whose deletion is impossible (`can_pay`): a
/// paid-deletion pick (`selection_pays_deletion`) that did not opt into
/// `offer_unpayable_deletion_payers`. With the opt-in (owner ruling
/// 2026-11-24, DCGO parity on four clauses) every printed match is offered;
/// an unpayable one, once chosen, reaches `expand` below, which skips the
/// deletion and the cancel, so nothing is deleted and the carrier leaves.
pub(crate) fn filters_unpayable_payers(
    bind_as: Option<&str>,
    then: &[CompiledStep],
    tail: &[CompiledStep],
    offer_unpayable_payers: bool,
) -> bool {
    !offer_unpayable_payers && selection_pays_deletion(bind_as, then, tail)
}

/// CR 15-7-4 excludes intrinsically impossible payments. Effect replacements
/// (including optional prevention) remain legal attempts; the receipt decides
/// whether they actually paid. Call after computing printed extrema, so an
/// immune lowest-DP Digimon never promotes a higher-DP candidate to lowest.
pub(crate) fn can_pay(ctx: &EffectReadContext<'_>, payer: PermanentHandle) -> bool {
    ctx.game
        .player(payer.player)
        .battle_area
        .get(payer.index as usize)
        .is_some()
        && ctx
            .game
            .effect_can_affect_from(payer, ctx.player, ctx.source_kind, ctx.source_identity)
        && !crate::replacement::deletion_passively_prevented_by(
            ctx.game,
            payer,
            if payer.player == ctx.player {
                ReplacementCause::OwnEffect
            } else {
                ReplacementCause::OpponentEffect
            },
            Some(ctx.player),
        )
}

pub(super) fn expand(
    steps: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: &Bindings,
) -> Option<Vec<CompiledStep>> {
    let [CompiledStep::DeletePermanent { target }, CompiledStep::CancelReplacement, tail @ ..] =
        steps
    else {
        return None;
    };
    let mut expanded = Vec::new();
    if let Some(ResolvedBinding::Permanent(payer_handle)) =
        resolve_binding_ref(target, ctx, bindings)
    {
        if can_pay(&ctx.as_read(), payer_handle) {
            if let Some((receipt, payer)) =
                super::replacement_outcome::start_deletion_payment(ctx, payer_handle)
            {
                // The existing continuation is cloneable and matches both the
                // physical top card and host identity. Cancelling, redirecting,
                // or deleting a substitute does not pay for this selected card.
                expanded.push(CompiledStep::PaidDeletionPreventionFinalize { receipt, payer });
            }
        }
    }
    expanded.extend_from_slice(tail);
    Some(expanded)
}
