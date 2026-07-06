use digimon_dsl::compiled::{CompiledBindingRef, CompiledColor, CompiledStep};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::effect::TimingFilter;
use crate::effect_context::EffectContext;
use crate::enums::CardColor;
use crate::permanent::PermanentHandle;

/// Map a DSL `CompiledColor` to the engine's `CardColor`.
fn compiled_color_to_card_color(c: CompiledColor) -> CardColor {
    match c {
        CompiledColor::Red => CardColor::Red,
        CompiledColor::Blue => CardColor::Blue,
        CompiledColor::Yellow => CardColor::Yellow,
        CompiledColor::Green => CardColor::Green,
        CompiledColor::Black => CardColor::Black,
        CompiledColor::Purple => CardColor::Purple,
        CompiledColor::White => CardColor::White,
    }
}

fn resolve_permanent_ref(
    target: &CompiledBindingRef,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
) -> Option<PermanentHandle> {
    if matches!(
        target,
        CompiledBindingRef::SelfRef | CompiledBindingRef::Source
    ) {
        return ctx.source_permanent;
    }

    match resolve_binding_ref(target, ctx, bindings) {
        Some(ResolvedBinding::Permanent(handle)) => Some(handle),
        _ => None,
    }
}

pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &Bindings) -> bool {
    match step {
        CompiledStep::RefireEffect {
            source,
            timing,
            optional,
        } => {
            if let Some(source) = resolve_permanent_ref(source, ctx, bindings) {
                if timing == "on_play_or_when_digivolving" {
                    let _ =
                        ctx.refire_target_effect(source, TimingFilter::Either, ctx.player, false);
                } else {
                    let _ = ctx.refire_effect_from_permanent(source, timing, *optional);
                }
            }
            true
        }
        // Foreign-card refire (BT15-102 Apocalymon): activate a timing-
        // filtered effect printed on a bound CARD OBJECT that is currently a
        // digivolution source of this effect's carrier. The bound handle is
        // typically produced by `place_as_bottom_source.bind_placed_as`; an
        // absent binding (nothing was placed — the optional pick was
        // declined) is a silent no-op so the rest of the clause (the "Then,"
        // tail) still runs.
        CompiledStep::RefireCardEffect { card, timing } => {
            let Some(ResolvedBinding::Card(card_handle)) =
                resolve_binding_ref(card, ctx, bindings)
            else {
                return true;
            };
            let Some(carrier) = ctx.source_permanent else {
                return true;
            };
            // The card must (still) be a digivolution source of the carrier
            // — "activate 1 [On Play] effect ON THAT CARD as an effect of
            // this Digimon" is scoped to the just-placed source.
            let is_carrier_source = ctx
                .game
                .players
                .get(carrier.player as usize)
                .and_then(|p| p.battle_area.get(carrier.index as usize))
                .is_some_and(|perm| {
                    perm.card_sources
                        .iter()
                        .rev()
                        .skip(1)
                        .any(|c| c.handle() == card_handle)
                });
            if !is_carrier_source {
                return true;
            }
            let Some(card_id) = ctx
                .game
                .card_data_for_handle(card_handle)
                .map(|cd| cd.card_id.clone())
            else {
                return true;
            };
            let timing_filter = match timing.as_str() {
                "on_play" => TimingFilter::OnPlay,
                "when_digivolving" => TimingFilter::WhenDigivolving,
                _ => TimingFilter::Either,
            };
            let _ = ctx.activate_foreign_card_effect(&card_id, carrier, timing_filter, ctx.player);
            true
        }
        // G-COST-REDUCE-ALLY-DIGIVOLVE — install a player-scoped one-shot
        // future-digivolve cost reducer (BT3-103 Hidden Potential Discovered!).
        CompiledStep::ArmDigivolveCostReducer {
            amount,
            single_fire,
            target_color,
            suspend_cost,
        } => {
            ctx.arm_player_digivolve_cost_reducer(
                *amount,
                *single_fire,
                target_color.map(compiled_color_to_card_color),
                *suspend_cost,
            );
            true
        }
        _ => false,
    }
}
