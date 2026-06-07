//! Replacement-process outcome steps.
//!
//! These are thin DSL verbs over the engine's existing replacement outcome
//! APIs. During a synchronous replacement body the dispatcher has not created
//! `Game::parked_replacement` yet, so we write a temporary outcome bridge.
//! During a parked selection callback the normal `EffectContext::*_replacement`
//! methods write directly to `Game::parked_replacement`.

use digimon_dsl::compiled::{CompiledStep, CompiledZone};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::effect_context::EffectContext;
use crate::enums::Zone;
use crate::replacement::{ReplacementOutcome, ReplacementSubject};

pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &mut Bindings) -> bool {
    match step {
        CompiledStep::CancelReplacement => {
            set_outcome(ctx, ReplacementOutcome::Cancelled);
            true
        }
        CompiledStep::TrashOwnLinkCardAndCancelLeave => {
            // Gap 3a — the leaving permanent is the replacement subject, bound
            // as `replacement_subject` by lower_replacement. Install the
            // link-card-trash selection; its callback trashes the chosen card
            // and cancels the leave. With no subject (shouldn't happen for a
            // leave replacement) this no-ops and the leave proceeds.
            if let Some(host) = bindings.get_permanent("replacement_subject") {
                ctx.trash_own_link_card_and_cancel_leave(host);
            }
            true
        }
        CompiledStep::TrashTopSecurityAndCancelReplacement { of } => {
            let player = crate::dsl_cards::step::resolve_player(ctx, *of);
            if ctx.trash_top_security_and_cancel_current_replacement(player) {
                set_outcome(ctx, ReplacementOutcome::Cancelled);
            }
            true
        }
        CompiledStep::PlacePermanentBottomSecurityAndCancelReplacement { of, target } => {
            let player = crate::dsl_cards::step::resolve_player(ctx, *of);
            if let Some(ResolvedBinding::Permanent(handle)) =
                resolve_binding_ref(target, ctx, bindings)
            {
                if ctx.place_sourceless_permanent_bottom_security_and_cancel_current_replacement(
                    player, handle,
                ) {
                    set_outcome(ctx, ReplacementOutcome::Cancelled);
                }
            }
            true
        }
        CompiledStep::PlacePermanentOnSecurityAndHandleReplacement {
            of,
            target,
            position,
            face_up,
        } => {
            let player = crate::dsl_cards::step::resolve_player(ctx, *of);
            if let Some(ResolvedBinding::Permanent(handle)) =
                resolve_binding_ref(target, ctx, bindings)
            {
                let position = crate::dsl_cards::step::map_stack_position(*position);
                if ctx.place_permanent_on_security_and_handle_current_replacement(
                    player, handle, position, *face_up,
                ) {
                    set_outcome(ctx, ReplacementOutcome::CustomHandled);
                }
            }
            true
        }
        CompiledStep::HandleReplacement => {
            set_outcome(ctx, ReplacementOutcome::CustomHandled);
            true
        }
        CompiledStep::RedirectReplacement { zone } => {
            if let Some(zone) = lower_zone(*zone) {
                set_outcome(ctx, ReplacementOutcome::Redirected(zone));
            }
            true
        }
        CompiledStep::SubstituteReplacement { subject } => {
            if let Some(subject) = resolve_replacement_subject(subject, ctx, bindings) {
                set_outcome(ctx, ReplacementOutcome::Substituted(subject));
            }
            true
        }
        CompiledStep::ReduceLinkCost { amount } => {
            // Gap 5 — reduce the in-flight `WhenWouldLink` link cost. This does
            // NOT set a replacement outcome: the link still resolves (the
            // replacement stays `None`), but `commit_digimon_link` then pays the
            // reduced cost. One-shot mutation of `pending_digimon_link.cost`.
            ctx.reduce_pending_link_cost(*amount);
            true
        }
        _ => false,
    }
}

fn set_outcome(ctx: &mut EffectContext<'_>, outcome: ReplacementOutcome) {
    if ctx.game.parked_replacement.is_some() {
        match outcome {
            ReplacementOutcome::None => {}
            ReplacementOutcome::Cancelled => ctx.cancel_leave(),
            ReplacementOutcome::Redirected(zone) => ctx.redirect_replacement(zone),
            ReplacementOutcome::Substituted(subject) => ctx.substitute_replacement(subject),
            ReplacementOutcome::CustomHandled => ctx.handle_replacement(),
        }
    } else {
        ctx.game.dsl_replacement_outcome = Some(outcome);
    }
}

fn resolve_replacement_subject(
    subject: &digimon_dsl::compiled::CompiledBindingRef,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
) -> Option<ReplacementSubject> {
    match resolve_binding_ref(subject, ctx, bindings)? {
        ResolvedBinding::Permanent(h) => Some(ReplacementSubject::Permanent(h)),
        _ => None,
    }
}

fn lower_zone(zone: CompiledZone) -> Option<Zone> {
    match zone {
        CompiledZone::Hand => Some(Zone::Hand),
        CompiledZone::Deck => Some(Zone::Deck),
        CompiledZone::Trash => Some(Zone::Trash),
        CompiledZone::BattleArea => Some(Zone::BattleArea),
        CompiledZone::Security => Some(Zone::Security),
        CompiledZone::Breeding => Some(Zone::BreedingArea),
        CompiledZone::Reveal => Some(Zone::Reveal),
        CompiledZone::DigiEggDeck => Some(Zone::DigitamaDeck),
        CompiledZone::Material => None,
    }
}
