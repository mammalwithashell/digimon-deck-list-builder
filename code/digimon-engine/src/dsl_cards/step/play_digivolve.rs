//! Play / digivolve / placement step lowering (Phase 2f1 Task 4).
//!
//! Wires 12 `CompiledStep` variants to engine primitives:
//!   - `PlayFromHand`, `PlayFromHandFree`
//!   - `PlayFromTrash`, `PlayFromTrashFree`
//!   - `PlayFromSecurity`
//!   - `PlayFromMaterials`
//!   - `EffectInitiatedDigivolve`, `EffectInitiatedDnaDigivolve`
//!   - `PlayToken`
//!   - `PlaceOnSecurity`, `PlaceAsBottomSource`, `TrashTopSource`
//!
//! All variants are synchronous (no selection prompts). Engine primitives
//! return `Option<PermanentHandle>` / `bool` — Phase 2f1 v1 discards the
//! return because the IR does not yet bind the resulting handle. Future
//! phases adding `bind_handle_as: Option<String>` to the variant shapes
//! will wire that here.
//!
//! Strictness convention (matches `modifiers::try_run`): if a binding
//! cannot be resolved or has the wrong kind (e.g. `Permanent` where a
//! `HandIndex` was expected), the step silently no-ops — the dispatcher
//! returns `true` because the variant was *matched*, not because it
//! produced an effect.

use digimon_dsl::compiled::CompiledStep;

use crate::card_source::CardHandle;
use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::resolve_player;
use crate::effect_context::EffectContext;
use crate::enums::CardSourceRef;
use crate::enums::CostDelta;

/// Translate the IR's `CompiledCostDelta` to the engine's `CostDelta`.
///
///   `None`              → `CostDelta::Reduce(0)` (default to printed cost)
///   `Some(Printed)`     → `CostDelta::Reduce(0)`
///   `Some(Free)`        → `CostDelta::Free`
///   `Some(Literal(n))`  → `CostDelta::Fixed(n as i16)`
fn lower_cost_delta(d: Option<&digimon_dsl::compiled::CompiledCostDelta>) -> CostDelta {
    use digimon_dsl::compiled::CompiledCostDelta;
    match d {
        None | Some(CompiledCostDelta::Printed) => CostDelta::Reduce(0),
        Some(CompiledCostDelta::Free) => CostDelta::Free,
        Some(CompiledCostDelta::Literal(n)) => CostDelta::Fixed(*n as i16),
    }
}

/// Resolve a `source: CompiledBindingRef` to a `CardSourceRef`, defaulting the
/// source-zone owner to `ctx.player` (the effect controller) for hand/trash
/// binding kinds. Returns `None` if the binding can't be resolved or has an
/// unsupported kind. See I2 above for the owner-heuristic limitation.
fn resolve_card_source_ref(
    source: &digimon_dsl::compiled::CompiledBindingRef,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
) -> Option<CardSourceRef> {
    let owner = ctx.player;
    match resolve_binding_ref(source, ctx, bindings)? {
        ResolvedBinding::HandIndex(i) => Some(CardSourceRef::Hand(owner, i as usize)),
        ResolvedBinding::TrashIndex(i) => Some(CardSourceRef::Trash(owner, i as usize)),
        ResolvedBinding::Card(h) => Some(CardSourceRef::Reveal(h)),
        // DeckTop and other kinds: no IR binding produces them today.
        // Future: widen as IR evolves.
        _ => None,
    }
}

/// Try to handle `step` as a play / digivolve / placement variant.
/// Returns `true` if the variant was matched (regardless of whether the
/// underlying engine call succeeded).
pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> bool {
    match step {
        // ── Play primitives (hand) ────────────────────────────────────────
        CompiledStep::PlayFromHand { of, hand_index, cost_delta } => {
            let p = resolve_player(ctx, *of);
            if let Some(ResolvedBinding::HandIndex(i)) =
                resolve_binding_ref(hand_index, ctx, bindings)
            {
                let delta = lower_cost_delta(cost_delta.as_ref());
                let _ = ctx.play_from_hand_with_cost(p, i as usize, delta);
            }
            true
        }
        CompiledStep::PlayFromHandFree { of, hand_index } => {
            let p = resolve_player(ctx, *of);
            if let Some(ResolvedBinding::HandIndex(i)) =
                resolve_binding_ref(hand_index, ctx, bindings)
            {
                let _ = ctx.play_from_hand_free(p, i as usize);
            }
            true
        }

        // ── Play primitives (trash) ───────────────────────────────────────
        CompiledStep::PlayFromTrash { of, trash_index, cost_delta } => {
            let p = resolve_player(ctx, *of);
            if let Some(ResolvedBinding::TrashIndex(i)) =
                resolve_binding_ref(trash_index, ctx, bindings)
            {
                let delta = lower_cost_delta(cost_delta.as_ref());
                let _ = ctx.play_from_trash_with_cost(p, i as usize, delta);
            }
            true
        }
        CompiledStep::PlayFromTrashFree { of, trash_index } => {
            let p = resolve_player(ctx, *of);
            // `play_from_trash_free_unsuspended` takes a `CardHandle`; the
            // IR addresses by trash index so we must look up the handle.
            if let Some(ResolvedBinding::TrashIndex(i)) =
                resolve_binding_ref(trash_index, ctx, bindings)
            {
                let handle: Option<CardHandle> = ctx
                    .game
                    .player(p)
                    .trash
                    .get(i as usize)
                    .map(|cs| cs.handle());
                if let Some(h) = handle {
                    let _ = ctx.play_from_trash_free_unsuspended(h);
                }
            }
            true
        }

        // ── Play primitives (security / materials) ────────────────────────
        CompiledStep::PlayFromSecurity => {
            // Operates on the controller (effect owner), per the IR's
            // zero-field shape. Distinct from `play_pending_security`,
            // which handles the security-skill replay path.
            let _ = ctx.play_from_security(ctx.player);
            true
        }
        CompiledStep::PlayFromMaterials { target, source_index, cost_delta } => {
            let target_handle = match resolve_binding_ref(target, ctx, bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                _ => return true,
            };
            let src_idx = match resolve_binding_ref(source_index, ctx, bindings) {
                Some(ResolvedBinding::Literal(v)) if v >= 0 => v as usize,
                _ => return true,
            };
            let delta = lower_cost_delta(cost_delta.as_ref());
            let _ = ctx.play_from_materials(target_handle, src_idx, delta);
            true
        }

        // ── Digivolve primitives ──────────────────────────────────────────
        CompiledStep::EffectInitiatedDigivolve { target, from_hand, cost, ignore_requirements } => {
            let target_handle = match resolve_binding_ref(target, ctx, bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                _ => return true,
            };
            let hand_index = match resolve_binding_ref(from_hand, ctx, bindings) {
                Some(ResolvedBinding::HandIndex(i)) => i as usize,
                _ => return true,
            };
            // The engine signature takes a `CostDelta`; the IR carries
            // `cost: i32`. Map a positive cost to `CostDelta::Fixed(cost)`
            // and `cost == 0` to `CostDelta::Free` so memory is genuinely
            // unaffected.
            //
            // IR carries cost as i32, which can express Free (0) and Fixed(n) but not
            // CostDelta::Reduce(n) (printed cost minus N). Phase 3 may widen the IR
            // field if a card needs that semantics. cost == 0 collapses to Free
            // (semantically identical for memory accounting).
            debug_assert!(*cost >= 0, "EffectInitiatedDigivolve cost: i32 went negative");
            let delta = if *cost == 0 {
                CostDelta::Free
            } else {
                CostDelta::Fixed(*cost as i16)
            };
            // The effect runs on the target's controller (the digivolve is
            // applied to `target` and the source is from that player's hand).
            let player = target_handle.player;
            let _ = ctx.effect_initiated_digivolve(
                player,
                hand_index,
                target_handle,
                delta,
                *ignore_requirements,
            );
            true
        }
        CompiledStep::EffectInitiatedDnaDigivolve {
            target_a,
            target_b,
            from_hand,
            cost,
            ignore_requirements,
        } => {
            let a = match resolve_binding_ref(target_a, ctx, bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                _ => return true,
            };
            let b = match resolve_binding_ref(target_b, ctx, bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                _ => return true,
            };
            let from_card = match resolve_binding_ref(from_hand, ctx, bindings) {
                Some(ResolvedBinding::Card(h)) => h,
                _ => return true,
            };
            let _ = ctx.effect_initiated_dna_digivolve(a, b, from_card, *cost, *ignore_requirements);
            true
        }

        // ── Token / placement ─────────────────────────────────────────────
        CompiledStep::PlayToken { controller, token_name } => {
            let p = resolve_player(ctx, *controller);
            let _ = ctx.play_token(p, token_name);
            true
        }
        CompiledStep::PlaceOnSecurity { of, source, position, face_up } => {
            let p = resolve_player(ctx, *of);
            // TODO(2f-followup): BindingValue::HandIndex / TrashIndex carry no PlayerId,
            // so we default the source-zone owner to ctx.player (effect controller).
            // If a future card needs to place an opponent's hand card, the IR's
            // BindingValue::HandIndex will need to widen to include a PlayerId, OR a
            // new HandIndexFor variant must be added.
            let Some(source_ref) = resolve_card_source_ref(source, ctx, bindings) else { return true; };
            let _ = ctx.place_on_security(p, source_ref, super::map_stack_position(*position), *face_up);
            true
        }
        CompiledStep::PlaceAsBottomSource { source, target } => {
            let target_handle = match resolve_binding_ref(target, ctx, bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                _ => return true,
            };
            // TODO(2f-followup): BindingValue::HandIndex / TrashIndex carry no PlayerId,
            // so we default the source-zone owner to ctx.player (effect controller).
            // If a future card needs to place an opponent's hand card, the IR's
            // BindingValue::HandIndex will need to widen to include a PlayerId, OR a
            // new HandIndexFor variant must be added.
            let Some(source_ref) = resolve_card_source_ref(source, ctx, bindings) else { return true; };
            let _ = ctx.place_as_bottom_source(source_ref, target_handle);
            true
        }
        CompiledStep::TrashTopSource { target } => {
            if let Some(ResolvedBinding::Permanent(h)) =
                resolve_binding_ref(target, ctx, bindings)
            {
                let _ = ctx.trash_top_source(h);
            }
            true
        }

        _ => false,
    }
}
