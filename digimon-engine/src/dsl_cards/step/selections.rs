//! Selection-step lowering: install a `PendingSelection` with the
//! remainder of the process-step slice as its callback.
//!
//! Phase 2b: `SelectHand`, `SelectTrash`, `SelectOwnPermanent`,
//! `SelectOpponentPermanent`.
//!
//! **Known limitation (Phase 2b):** the `EffectContext::select_*` filter
//! closure is `Fn(&Game, ...) -> bool`, not `Fn(&EffectReadContext, ...)`.
//! Evaluating a `CompiledPredicate` needs the full read-context tuple
//! (`source_card`, `source_permanent`, `player`), so Phase 2b accepts
//! all candidates at install time. Phase 2c widens the filter signature.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPlayerRef, CompiledStep, CompiledZone};

use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::{drain_dsl_outer_tail, resolve_player, run_steps};
use crate::effect_context::{CountCappedZone, EffectContext};
use crate::permanent::PermanentHandle;

/// Returns `true` if `step` was a selection step and the remainder was
/// installed as its callback. Returns `false` for any non-selection
/// step, letting `run_steps` fall through to the synchronous path.
pub fn try_install(
    step: &CompiledStep,
    tail: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: Bindings,
) -> bool {
    match step {
        CompiledStep::SelectHand { of, bind_as, prompt, optional, .. } => {
            install_select_hand(
                ctx,
                *of,
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
            );
            true
        }
        CompiledStep::SelectTrash { of, bind_as, prompt, optional, .. } => {
            install_select_trash(
                ctx,
                *of,
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
            );
            true
        }
        CompiledStep::SelectOwnPermanent { bind_as, prompt, optional, .. } => {
            install_select_own_permanent(
                ctx,
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
            );
            true
        }
        CompiledStep::SelectOpponentPermanent { bind_as, prompt, optional, .. } => {
            install_select_opponent_permanent(
                ctx,
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
            );
            true
        }
        CompiledStep::SelectCountCappedMulti {
            of, zone, max, bind_as, prompt, optional_zero, ..
        } => {
            install_select_count_capped_multi(
                ctx,
                *of,
                *zone,
                *max,
                bind_as.clone(),
                prompt.clone(),
                *optional_zero,
                tail.to_vec(),
                bindings,
            );
            true
        }
        CompiledStep::SelectEffectChoice { labels, bind_as, prompt, .. } => {
            install_select_effect_choice(
                ctx,
                labels.clone(),
                bind_as.clone(),
                prompt.clone(),
                tail.to_vec(),
                bindings,
            );
            true
        }
        CompiledStep::SelectReveal { of: _, bind_as, prompt, optional, .. } => {
            install_select_reveal(
                ctx,
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
            );
            true
        }
        CompiledStep::SelectSecurity { of, bind_as, prompt, optional, .. } => {
            install_select_security(
                ctx,
                *of,
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
            );
            true
        }
        CompiledStep::SelectMaterial { of_permanent, bind_as, prompt, optional, .. } => {
            use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
            let perm = match resolve_binding_ref(of_permanent, ctx, &bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                // Missing binding or wrong type: silent no-op (2b/2c convention).
                // Return false so run_steps falls through and the tail runs synchronously.
                _ => return false,
            };
            install_select_material(
                ctx,
                perm,
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
            );
            true
        }
        CompiledStep::SelectUnionZone { of, zones, bind_as, prompt, optional, .. } => {
            use crate::selection::UnionZoneSet;
            let mut zoneset = UnionZoneSet(0);
            for z in zones {
                match z {
                    CompiledZone::Hand => zoneset |= UnionZoneSet::HAND,
                    CompiledZone::Trash => zoneset |= UnionZoneSet::TRASH,
                    // Other zones not yet exposed by UnionZoneSet bitfield.
                    // Silently skip — Phase 2f+ widens engine API as needed.
                    _ => {}
                }
            }
            if zoneset.0 == 0 {
                // No supported zones: silent no-op; tail runs synchronously.
                return false;
            }
            install_select_union_zone(
                ctx,
                *of,
                zoneset,
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
            );
            true
        }
        CompiledStep::SelectOrderedPermutation { items, bind_as, prompt, .. } => {
            use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
            let item_list = match resolve_binding_ref(items, ctx, &bindings) {
                Some(ResolvedBinding::CardList(v)) => v,
                // Missing binding or wrong type: silent no-op.
                _ => return false,
            };
            install_select_ordered_permutation(
                ctx,
                item_list,
                bind_as.clone(),
                prompt.clone(),
                tail.to_vec(),
                bindings,
            );
            true
        }
        _ => false,
    }
}

fn install_select_hand(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    let target_player = resolve_player(ctx, of);
    let tail = Arc::new(tail);
    ctx.select_hand(
        target_player,
        &prompt,
        optional,
        |_game, _idx| true, // Phase 2b: accept-all filter (see module header).
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_hand_index(name, idx as u16);
            }
            run_steps(&tail, cb_ctx, &mut b);
            // Phase 2d Task 7: drain outer tail captured by run_steps when
            // this selection was installed inside a control-flow body.
            drain_dsl_outer_tail(cb_ctx);
        },
    );
}

fn install_select_trash(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    let target_player = resolve_player(ctx, of);
    let tail = Arc::new(tail);
    ctx.select_trash(
        target_player,
        &prompt,
        optional,
        |_game, _idx| true,
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_trash_index(name, idx as u16);
            }
            run_steps(&tail, cb_ctx, &mut b);
            // Phase 2d Task 7: drain outer tail.
            drain_dsl_outer_tail(cb_ctx);
        },
    );
}

fn install_select_own_permanent(
    ctx: &mut EffectContext<'_>,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    let tail = Arc::new(tail);
    ctx.select_own_permanent(
        &prompt,
        optional,
        |_game, _handle| true,
        move |cb_ctx, handle: PermanentHandle| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_permanent(name, handle);
            }
            run_steps(&tail, cb_ctx, &mut b);
            // Phase 2d Task 7: drain outer tail.
            drain_dsl_outer_tail(cb_ctx);
        },
    );
}

fn install_select_opponent_permanent(
    ctx: &mut EffectContext<'_>,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    let tail = Arc::new(tail);
    ctx.select_opponent_permanent(
        &prompt,
        optional,
        |_game, _handle| true,
        move |cb_ctx, handle: PermanentHandle| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_permanent(name, handle);
            }
            run_steps(&tail, cb_ctx, &mut b);
            // Phase 2d Task 7: drain outer tail.
            drain_dsl_outer_tail(cb_ctx);
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn install_select_count_capped_multi(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    zone: CompiledZone,
    max: u8,
    bind_as: Option<String>,
    prompt: String,
    optional_zero: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    let target_player = resolve_player(ctx, of);
    let engine_zone = match zone {
        CompiledZone::Hand => CountCappedZone::Hand,
        CompiledZone::Trash => CountCappedZone::Trash,
        // Phase 2d scope: only Hand/Trash supported. Other zones (Materials,
        // Security, Reveal, Source, Field, Deck, Breeding) silently no-op
        // here; Phase 2e+ adds the missing engine API hooks.
        _ => return,
    };
    let tail = Arc::new(tail);
    ctx.select_count_capped_multi(
        target_player,
        engine_zone,
        max,
        &prompt,
        optional_zero,
        |_game, _card| true, // Phase 2b/2c precedent: accept-all filter.
        move |cb_ctx, picks| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_card_list(name, picks);
            }
            run_steps(&tail, cb_ctx, &mut b);
            // Phase 2d Task 7: drain outer tail captured by run_steps when
            // this selection was installed inside a control-flow body.
            drain_dsl_outer_tail(cb_ctx);
        },
    );
}

fn install_select_effect_choice(
    ctx: &mut EffectContext<'_>,
    labels: Vec<String>,
    bind_as: Option<String>,
    prompt: String,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    let tail = Arc::new(tail);
    ctx.select_effect_choice(
        &prompt,
        labels,
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_literal(name, idx as i64);
            }
            run_steps(&tail, cb_ctx, &mut b);
            // Phase 2d Task 7: drain outer tail captured by run_steps when
            // this selection was installed inside a control-flow body.
            drain_dsl_outer_tail(cb_ctx);
        },
    );
}

fn install_select_reveal(
    ctx: &mut EffectContext<'_>,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    let tail = Arc::new(tail);
    ctx.select_reveal(
        &prompt,
        optional,
        |_game, _idx| true, // Phase 2b precedent: accept-all filter.
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                // Resolve the picked reveal index into a stable CardHandle.
                if let Some(card) = cb_ctx.game.revealed_cards.get(idx) {
                    b.insert_card(name, card.handle());
                }
                // If the index has gone stale (the reveal pile mutated mid-
                // resolution — currently impossible but defensive), silently
                // skip the binding; downstream verbs that consume it no-op
                // per the 2b/2c missing-binding convention.
            }
            run_steps(&tail, cb_ctx, &mut b);
            drain_dsl_outer_tail(cb_ctx);
        },
    );
}

fn install_select_security(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    let target_player = resolve_player(ctx, of);
    let tail = Arc::new(tail);
    ctx.select_security(
        target_player,
        &prompt,
        optional,
        |_game, _idx| true,
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                if let Some(card) = cb_ctx.game.player(target_player).security.get(idx) {
                    b.insert_card(name, card.handle());
                }
            }
            run_steps(&tail, cb_ctx, &mut b);
            drain_dsl_outer_tail(cb_ctx);
        },
    );
}

fn install_select_material(
    ctx: &mut EffectContext<'_>,
    perm: PermanentHandle,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    let tail = Arc::new(tail);
    // Exclude the top card (last index): only offer non-top sources as candidates.
    // Mirrors select_count_capped_multi(Material) which does stack_len - 1.
    ctx.select_material(
        perm,
        &prompt,
        optional,
        move |game, src_idx| {
            let total = game
                .player(perm.player)
                .battle_area
                .get(perm.index as usize)
                .map(|p| p.card_sources.len())
                .unwrap_or(0);
            // Exclude top card: top is at index total-1.
            src_idx + 1 < total
        },
        move |cb_ctx, src_idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                let perm_owner = perm.player;
                let perm_index = perm.index as usize;
                if let Some(card) = cb_ctx
                    .game
                    .player(perm_owner)
                    .battle_area
                    .get(perm_index)
                    .and_then(|p| p.card_sources.get(src_idx))
                {
                    b.insert_card(name, card.handle());
                }
            }
            run_steps(&tail, cb_ctx, &mut b);
            drain_dsl_outer_tail(cb_ctx);
        },
    );
}

fn install_select_ordered_permutation(
    ctx: &mut EffectContext<'_>,
    items: Vec<crate::card_source::CardHandle>,
    bind_as: Option<String>,
    prompt: String,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    let tail = Arc::new(tail);
    ctx.select_ordered_permutation(
        items,
        &prompt,
        move |cb_ctx, ordered| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_card_list(name, ordered);
            }
            run_steps(&tail, cb_ctx, &mut b);
            drain_dsl_outer_tail(cb_ctx);
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn install_select_union_zone(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    zoneset: crate::selection::UnionZoneSet,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
) {
    let target_player = resolve_player(ctx, of);
    let tail = Arc::new(tail);
    ctx.select_union_zone(
        target_player,
        zoneset,
        &prompt,
        optional,
        |_game, _card| true, // Phase 2e: accept-all filter.
        move |cb_ctx, handle| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_card(name, handle);
            }
            run_steps(&tail, cb_ctx, &mut b);
            drain_dsl_outer_tail(cb_ctx);
        },
    );
}
