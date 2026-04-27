use digimon_dsl::compiled::{CompiledStep, CompiledZone};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::effect_context::EffectContext;
use crate::enums::Zone;
use crate::replacement::ReplacementSubject;

fn map_zone(zone: CompiledZone) -> Zone {
    match zone {
        CompiledZone::Hand => Zone::Hand,
        CompiledZone::Deck => Zone::Deck,
        CompiledZone::Trash => Zone::Trash,
        CompiledZone::BattleArea => Zone::BattleArea,
        CompiledZone::Security => Zone::Security,
        CompiledZone::Breeding => Zone::BreedingArea,
        CompiledZone::Reveal => Zone::Reveal,
        CompiledZone::DigiEggDeck => Zone::DigitamaDeck,
        CompiledZone::Material => Zone::BattleArea,
    }
}

pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &Bindings) -> bool {
    match step {
        CompiledStep::CancelLeave => {
            ctx.cancel_leave();
            true
        }
        CompiledStep::HandleReplacement => {
            ctx.handle_replacement();
            true
        }
        CompiledStep::RedirectReplacement { destination } => {
            ctx.redirect_replacement(map_zone(*destination));
            true
        }
        CompiledStep::SubstitutePermanent { target } => {
            if let Some(ResolvedBinding::Permanent(handle)) =
                resolve_binding_ref(target, ctx, bindings)
            {
                ctx.substitute_replacement(ReplacementSubject::Permanent(handle));
            }
            true
        }
        _ => false,
    }
}
