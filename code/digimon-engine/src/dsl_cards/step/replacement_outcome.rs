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
