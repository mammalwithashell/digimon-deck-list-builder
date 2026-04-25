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
