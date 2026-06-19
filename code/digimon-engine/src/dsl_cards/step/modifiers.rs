//! Synchronous modifier-step lowering (Phase 2c).
//!
//! Verbs: AddDpModifier, AddModifier, GrantKeyword. All are binding-target
//! (filter-target form for AddModifier is Phase 2d). Unknown expiry strings,
//! unknown modifier names, or unknown keyword names cause the step to no-op —
//! same strictness convention as 2b (invalid references don't panic).
//!
//! Phase 2c handlers: AddDpModifier, AddModifier, GrantKeyword.
//! Phase 2f2 Task 3: `CompiledModifierValue::Formula(_)` is now lowered
//! through `formula_eval::evaluate`. The evaluator returns `i32` and both
//! `EffectContext::add_dp_modifier` / `add_modifier` take `i32`, so no
//! narrowing or saturation happens at this boundary today. If the engine
//! ever narrows the modifier value type (e.g. to `i16`), this dispatch
//! site is where saturation logic would need to land — see
//! `phase2f2_modifier_formula::add_dp_modifier_formula_large_value_passes_through`
//! which guards against an unannounced narrowing change.

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledCardKind, CompiledColor, CompiledEffectController,
    CompiledEffectSourceKind, CompiledModifierTarget, CompiledModifierValue, CompiledStep,
    CompiledSynthIdentity,
};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::expiry_map::lookup_expiry;
use crate::dsl_cards::formula_eval;
use crate::dsl_cards::step::{resolve_player, StepRuntime};
use crate::effect_context::EffectContext;
use crate::enums::EffectSourceKind;
use crate::modifiers::{EffectControllerFilter, PlayerModifierEntry};
use crate::permanent::PermanentHandle;

/// Resolve a `CompiledModifierValue` to the `i32` the engine modifier APIs
/// take. Literal values are returned verbatim; Formula values are evaluated
/// with `target` as the resolution point for per-selectors (stack_size,
/// material_count, digivolution_color_count).
///
/// `target` MUST be the permanent the modifier is being applied to — for the
/// filter-target form this means the per-iteration handle, NOT the source
/// permanent. Hoisting the call outside a `for h in matches` loop would tie
/// the formula to the source's stack instead of the matched target's
/// (Susanoomon-style "+2000 DP per material on this Digimon" must give
/// different DP per match if matches have different material counts).
fn resolve_modifier_value(
    value: &CompiledModifierValue,
    ctx: &EffectContext<'_>,
    target: PermanentHandle,
    bindings: &Bindings,
    runtime: &StepRuntime,
) -> i32 {
    match value {
        CompiledModifierValue::Literal(n) => *n,
        CompiledModifierValue::Formula(f) => formula_eval::evaluate_with_raw_and_bindings(
            f,
            ctx,
            target,
            runtime.raw(),
            Some(bindings),
        ),
    }
}

fn resolve_player_modifier_value(
    value: &CompiledModifierValue,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
    runtime: &StepRuntime,
) -> i32 {
    match value {
        CompiledModifierValue::Literal(n) => *n,
        CompiledModifierValue::Formula(f) => ctx
            .source_permanent
            .map(|h| {
                formula_eval::evaluate_with_raw_and_bindings(
                    f,
                    ctx,
                    h,
                    runtime.raw(),
                    Some(bindings),
                )
            })
            .unwrap_or(0),
    }
}

/// Resolve an authored expiry string. Unknown strings still no-op (same
/// strictness convention as the rest of Phase 2c), but in debug builds we
/// emit a warning so the silent no-op stops being invisible — this is the
/// failure mode that motivated Phase 2f3's expiry parity sweep.
fn resolve_expiry(verb: &str, raw: &str) -> Option<crate::enums::Expiry> {
    match lookup_expiry(raw) {
        Some(e) => Some(e),
        None => {
            #[cfg(debug_assertions)]
            eprintln!(
                "[debug] dsl::{verb}: unknown expiry {:?} — modifier will not apply. \
                 The DSL validator should have rejected this at compile time; \
                 this warning indicates a validator/engine drift.",
                raw
            );
            None
        }
    }
}

fn resolve_permanent_target(
    target: &CompiledBindingRef,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
) -> Option<PermanentHandle> {
    if matches!(target, CompiledBindingRef::SelfRef) {
        return ctx.source_permanent;
    }
    match resolve_binding_ref(target, ctx, bindings) {
        Some(ResolvedBinding::Permanent(h)) => Some(h),
        _ => None,
    }
}

pub(crate) fn lower_effect_source_kind(kind: CompiledEffectSourceKind) -> EffectSourceKind {
    match kind {
        CompiledEffectSourceKind::Digimon => EffectSourceKind::Digimon,
        CompiledEffectSourceKind::Tamer => EffectSourceKind::Tamer,
        CompiledEffectSourceKind::Option => EffectSourceKind::Option,
        CompiledEffectSourceKind::Rule => EffectSourceKind::Rule,
    }
}

pub(crate) fn lower_effect_controller(controller: CompiledEffectController) -> EffectControllerFilter {
    match controller {
        CompiledEffectController::Any => EffectControllerFilter::Any,
        CompiledEffectController::Opponent => EffectControllerFilter::OpponentOnly,
        CompiledEffectController::Own => EffectControllerFilter::OwnOnly,
    }
}

fn compiled_kind_to_card_kind(k: CompiledCardKind) -> crate::enums::CardKind {
    use crate::enums::CardKind as K;
    match k {
        CompiledCardKind::Digimon => K::Digimon,
        CompiledCardKind::Tamer => K::Tamer,
        CompiledCardKind::Option => K::Option,
        CompiledCardKind::DigiEgg => K::DigiEgg,
        CompiledCardKind::Token => K::Token,
        CompiledCardKind::Dual => K::Dual,
    }
}

fn compiled_color_to_card_color(c: CompiledColor) -> crate::enums::CardColor {
    use crate::enums::CardColor as C;
    match c {
        CompiledColor::Red => C::Red,
        CompiledColor::Blue => C::Blue,
        CompiledColor::Yellow => C::Yellow,
        CompiledColor::Green => C::Green,
        CompiledColor::Black => C::Black,
        CompiledColor::Purple => C::Purple,
        CompiledColor::White => C::White,
    }
}

/// Lower a compiled `synth_identity:` block to the engine's
/// `ModifierPayload::SynthIdentity` (the data a `TreatAsDigimon` target is
/// treated as while the modifier is live).
pub(crate) fn build_synth_payload(s: &CompiledSynthIdentity) -> crate::modifiers::ModifierPayload {
    crate::modifiers::ModifierPayload::SynthIdentity {
        kind: compiled_kind_to_card_kind(s.kind),
        level: s.level,
        colors: s.colors.iter().copied().map(compiled_color_to_card_color).collect(),
        traits: s.traits.clone(),
        dp: s.dp,
    }
}

/// Returns `true` if `step` is a modifier family handled here.
/// Unknown steps fall through (the caller may try other families).
pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    runtime: &StepRuntime,
) -> bool {
    match step {
        CompiledStep::AddDpModifier {
            target,
            value,
            expiry,
        } => {
            let Some(expiry) = resolve_expiry("add_dp_modifier", expiry) else {
                return true;
            };
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                let n = resolve_modifier_value(value, ctx, h, bindings, runtime);
                ctx.add_dp_modifier(h, n, expiry);
            }
            true
        }
        CompiledStep::AddModifier {
            target,
            modifier,
            value,
            expiry,
            synth_identity,
            continuous,
        } => {
            let Some(expiry) = resolve_expiry("add_modifier", expiry) else {
                return true;
            };
            let Some(modifier_ty) = crate::dsl_cards::modifier_map::lookup_modifier_type(modifier)
            else {
                return true;
            };
            // `synth_identity:` (TreatAsDigimon) installs the structured
            // `ModifierPayload::SynthIdentity` instead of a bare scalar.
            let payload = synth_identity.as_ref().map(build_synth_payload);
            // Closure installs with-or-without payload depending on `payload`.
            let install = |ctx: &mut crate::effect_context::EffectContext, h, n| {
                match &payload {
                    Some(p) => ctx.add_modifier_with_payload(h, modifier_ty, n, expiry, p.clone()),
                    None => ctx.add_modifier(h, modifier_ty, n, expiry),
                }
            };
            match target {
                CompiledModifierTarget::Binding(b) => {
                    if let Some(ResolvedBinding::Permanent(h)) =
                        resolve_binding_ref(b, ctx, bindings)
                    {
                        let n = resolve_modifier_value(value, ctx, h, bindings, runtime);
                        install(ctx, h, n);
                    }
                }
                CompiledModifierTarget::Filter(pred) => {
                    if *continuous {
                        // CONTINUOUS mass modifier (G-CONTINUOUS-MASS-DP-DEBUFF):
                        // register a source-independent floating descriptor that
                        // re-applies to the live candidate set every tick, instead
                        // of a one-time scan. The magnitude is resolved ONCE at
                        // install (these "all X get ±Y until Z" cards use a literal
                        // ±Y, not a per-permanent formula); `dummy` only seeds the
                        // value-formula context and is irrelevant for literals.
                        let dummy = ctx.source_permanent.unwrap_or(PermanentHandle {
                            player: ctx.player,
                            index: 0,
                        });
                        let n = resolve_modifier_value(value, ctx, dummy, bindings, runtime);
                        ctx.game.add_floating_mass_modifier(
                            pred.clone(),
                            modifier_ty,
                            n,
                            ctx.source_card,
                            ctx.player,
                            expiry,
                            None,
                            // Carry the TreatAsDigimon SynthIdentity into the
                            // continuous mass path (was dropped) — closes
                            // G-DSL-AURA-TREAT-AS-DIGIMON-SYNTH (BT25-104).
                            payload.clone(),
                        );
                    } else {
                        // Phase 2d Task 8: scan battle-area, apply modifier to every match.
                        // Phase 2f2 Task 3: formula evaluation is INSIDE the loop with
                        // `h` as the target, so per-selectors (StackSize etc.) resolve
                        // against each matched permanent — NOT hoisted outside.
                        let matches =
                            crate::dsl_cards::step::permanent_scan::scan(ctx, pred, Some(bindings));
                        for h in matches {
                            let n = resolve_modifier_value(value, ctx, h, bindings, runtime);
                            install(ctx, h, n);
                        }
                    }
                }
            }
            true
        }
        CompiledStep::AddPlayerModifier {
            target_player,
            modifier,
            value,
            expiry,
        } => {
            let Some(expiry) = resolve_expiry("add_player_modifier", expiry) else {
                return true;
            };
            let Some(modifier_ty) = crate::dsl_cards::modifier_map::lookup_modifier_type(modifier)
            else {
                return true;
            };
            let player = resolve_player(ctx, *target_player);
            let n = resolve_player_modifier_value(value, ctx, bindings, runtime);
            ctx.game.modifiers.add_player_modifier(
                player,
                PlayerModifierEntry::simple(
                    modifier_ty,
                    n,
                    expiry,
                    ctx.source_permanent,
                    ctx.player,
                ),
            );
            true
        }
        CompiledStep::GrantKeyword {
            target,
            keyword,
            expiry,
            value,
        } => {
            let Some(expiry) = resolve_expiry("grant_keyword", expiry) else {
                return true;
            };
            let Some(kw) = crate::dsl_cards::modifier_map::lookup_keyword(keyword, *value) else {
                return true;
            };
            if let Some(h) = resolve_permanent_target(target, ctx, bindings) {
                ctx.grant_keyword(h, kw, expiry);
            }
            true
        }
        CompiledStep::GrantEffectImmunity {
            target,
            source_kind,
            source_controller,
            expiry,
            continuous,
            targets,
        } => {
            let Some(expiry) = resolve_expiry("grant_effect_immunity", expiry) else {
                return true;
            };
            if *continuous {
                // G-DSL-CONTINUOUS-CONTROLLED-IMMUNITY-AURA (Q28 /
                // BT20-059): a source-independent floating immunity over
                // the live `targets` set — permanents played later in the
                // window are covered by the per-tick re-scan.
                let Some(filter) = targets else {
                    return true;
                };
                let immunity = crate::modifiers::EffectImmunityFilter {
                    source_kind: Some(lower_effect_source_kind(*source_kind)),
                    controller: lower_effect_controller(*source_controller),
                };
                ctx.game.add_floating_mass_modifier(
                    filter.clone(),
                    crate::enums::ModifierType::CannotBeAffected,
                    0,
                    ctx.source_card,
                    ctx.player,
                    expiry,
                    Some(immunity),
                    None,
                );
                return true;
            }
            let Some(target) = target.as_ref() else {
                return true;
            };
            let Some(h) = resolve_permanent_target(target, ctx, bindings) else {
                return true;
            };
            ctx.add_effect_immunity_modifier(
                h,
                lower_effect_source_kind(*source_kind),
                lower_effect_controller(*source_controller),
                expiry,
            );
            true
        }
        CompiledStep::GrantNarrowOpponentEffectProtection { target, expiry } => {
            let Some(expiry) = resolve_expiry("grant_narrow_opponent_effect_protection", expiry)
            else {
                return true;
            };
            if let Some(h) = resolve_binding_ref(target, ctx, bindings).and_then(|b| match b {
                ResolvedBinding::Permanent(h) => Some(h),
                _ => None,
            }) {
                ctx.grant_narrow_opponent_effect_protection(h, expiry);
            }
            true
        }
        _ => false,
    }
}
