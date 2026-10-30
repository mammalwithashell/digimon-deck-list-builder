//! Synchronous permanent-mutation step lowering (Phase 2c).
//!
//! Verbs: DeletePermanent, ReturnToHand, ReturnToDeck, Suspend, Unsuspend,
//! DeDigivolve. All are binding-consuming: the `target` field resolves to
//! `ResolvedBinding::Permanent`; any other variant is silently skipped (same
//! convention as the 2b zone-move handlers). BindPermanentProperty also
//! accepts a bound physical card and captures its printed metadata.

use digimon_dsl::compiled::{CompiledPermanentProperty, CompiledStep};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::effect_context::EffectContext;

/// A formula-valued `de_digivolve` amount is printed "<De-Digivolve 1> for
/// each …" / "for every 2 …" (BT19-073, BT21-060, BT24-041, BT25-075,
/// EX13-066): that many SEPARATE `<De-Digivolve 1>` instances, each processed
/// separately with the chosen card's state confirmed after each (§16-12-7 /
/// §16-12-8), and each trashing exactly 1 card so no number is declared
/// (§16-12-1 applies per instance). Expand it into that many literal
/// `amount: 1` steps so every instance runs (and may park) through the normal
/// step executor. A literal `amount: N` stays one `<De-Digivolve N>` whose
/// count the controller declares. G-DEDIGIVOLVE-NO-DECLARATION.
pub fn expand_de_digivolve_instances(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &Bindings,
) -> Option<Vec<CompiledStep>> {
    let CompiledStep::DeDigivolve {
        target,
        amount_fn: Some(formula),
        stop_at_level,
        ..
    } = step
    else {
        return None;
    };
    let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings) else {
        return None;
    };
    let instances = crate::dsl_cards::formula_eval::evaluate_with_bindings(
        formula,
        ctx,
        h,
        Some(bindings),
    )
    .clamp(0, i32::from(u8::MAX)) as usize;
    Some(
        (0..instances)
            .map(|_| CompiledStep::DeDigivolve {
                target: target.clone(),
                amount: Some(1),
                amount_fn: None,
                stop_at_level: *stop_at_level,
            })
            .collect(),
    )
}

/// Returns `true` if `step` is a permanent-mutation family handled here.
/// Unknown steps fall through (the caller may try other families).
pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &mut Bindings) -> bool {
    match step {
        CompiledStep::DeletePermanent { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                let existed = ctx
                    .game
                    .player(h.player)
                    .battle_area
                    .get(h.index as usize)
                    .is_some();
                // Capture the pre-removal effective DP while the carrier is
                // still live in battle_area; after deletion it has moved to
                // trash (rule 25) and a live DP read is unavailable. Same
                // modifier-aware read `build_snapshot_for_handle` uses for
                // `dp_just_before`.
                let dp = if existed {
                    ctx.game.effective_dp(h)
                } else {
                    None
                };
                let is_digimon=ctx.game.permanent_is_digimon_for_rules(h);
                ctx.delete_permanent(h);
                if existed {
                    bindings.record_deleted(h, dp, is_digimon);
                }
            }
            true
        }
        // DeleteForCostReduction and DeleteBoundPermanents are expanded by
        // deletion_results into a resumable batch and completed-result finalizer.
        // G-DSL-DELETE-PERMANENTS-BATCH (EX7-071 "Delete 1 of your opponent's
        // level 3, 1 of their level 4, and 1 of their level 5 Digimon"): the
        // picks are separate selections but the deletion is ONE event (DCGO
        // `EX7_071.cs` accumulates `selectedPermanents`, then a single
        // `DestroyPermanentsClass(list).Destroy()`; general_rule 15-4-3-2).
        // Unset bindings (self-skipped arms) are ignored, duplicates collapse,
        // vanished / effect-immune permanents are skipped, then ONE batched
        // deletion runs so every target leaves the field simultaneously.
        CompiledStep::DeletePermanents { targets } => {
            let mut handles: Vec<crate::permanent::PermanentHandle> = Vec::new();
            for target in targets {
                if let Some(ResolvedBinding::Permanent(h)) =
                    resolve_binding_ref(target, ctx, bindings)
                {
                    let exists = ctx
                        .game
                        .player(h.player)
                        .battle_area
                        .get(h.index as usize)
                        .is_some();
                    if exists && !handles.contains(&h) && ctx.can_affect_permanent(h) {
                        handles.push(h);
                    }
                }
            }
            if handles.is_empty() {
                return true;
            }
            let snapshots: Vec<(crate::permanent::PermanentHandle, Option<i32>, bool)> = handles
                .iter()
                .map(|&h| (h, ctx.game.effective_dp(h), ctx.game.permanent_is_digimon_for_rules(h)))
                .collect();
            let cause = ctx.game.infer_deletion_cause(handles[0]);
            ctx.game.delete_permanents_batch(handles, cause);
            for (h, dp, is_digimon) in snapshots {
                bindings.record_deleted(h, dp, is_digimon);
            }
            true
        }
        // G-DSL-DELETE-ALL-PERMANENTS (BT6-105 "Delete all Digimon with play
        // costs of 7 or less"): scan BOTH battle areas once, drop targets this
        // effect cannot affect (immunity — the same gate `ctx.delete_permanent`
        // applies), snapshot each survivor's pre-removal DP for the result
        // log (rule 25), then run ONE batched deletion so the whole set leaves
        // the field simultaneously (DCGO `DestroyPermanentsClass(list)`).
        CompiledStep::DeleteAllPermanents { over, selector } => {
            let mut scanned: Vec<crate::permanent::PermanentHandle> =
                crate::dsl_cards::step::permanent_scan::scan(ctx, over, Some(bindings));
            // G-DSL-DELETE-ALL-EXTREME-AMONG-FILTER: keep only the matches at
            // the selector's extreme value among ALL matches (DCGO `IsMinDP`
            // is computed over the condition-filtered set, before immunity).
            if let Some(sel) = selector {
                if let Some(extreme) = crate::dsl_cards::step::selections::selected_field_extreme(
                    ctx.game, &scanned, *sel,
                ) {
                    scanned.retain(|h| {
                        crate::dsl_cards::step::selections::field_value(ctx.game, *h, *sel)
                            == Some(extreme)
                    });
                }
            }
            let matches: Vec<crate::permanent::PermanentHandle> = scanned
                .into_iter()
                .filter(|h| ctx.can_affect_permanent(*h))
                .collect();
            if matches.is_empty() {
                return true;
            }
            let snapshots: Vec<(crate::permanent::PermanentHandle, Option<i32>, bool)> = matches
                .iter()
                .map(|&h| (h, ctx.game.effective_dp(h), ctx.game.permanent_is_digimon_for_rules(h)))
                .collect();
            let cause = ctx.game.infer_deletion_cause(matches[0]);
            ctx.game.delete_permanents_batch(matches, cause);
            for (h, dp, is_digimon) in snapshots {
                bindings.record_deleted(h, dp, is_digimon);
            }
            true
        }
        CompiledStep::TrashBreedingPermanent { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                ctx.trash_breeding_permanent(h);
            }
            true
        }
        CompiledStep::ReturnToHand { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                if let Some(result) = ctx.return_to_hand_with_receipt(h) {
                    bindings.record_returned_card(result.card);
                    if result.added_to_hand {
                        bindings.record_added_to_hand(result.card);
                    }
                }
            }
            true
        }
        CompiledStep::Suspend { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                let was_unsuspended = ctx
                    .game
                    .player(h.player)
                    .battle_area
                    .get(h.index as usize)
                    .is_some_and(|p| !p.is_suspended);
                ctx.suspend(h);
                if was_unsuspended {
                    bindings.record_suspended(h);
                }
            }
            true
        }
        CompiledStep::Unsuspend { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                ctx.unsuspend(h);
            }
            true
        }
        CompiledStep::ReturnToDeck {
            target,
            position,
            include_sources,
        } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                let position = super::map_stack_position(*position);
                let top_card = ctx
                    .game
                    .player(h.player)
                    .battle_area
                    .get(h.index as usize)
                    .map(|p| p.top_card().handle());
                if *include_sources {
                    if ctx.return_stack_to_deck(h, position) {
                        if let Some(card) = top_card {
                            bindings.record_returned_to_deck(card);
                        }
                    }
                } else if ctx.return_to_deck(h, position) {
                    if let Some(card) = top_card {
                        bindings.record_returned_to_deck(card);
                    }
                }
            }
            true
        }
        CompiledStep::PlacePermanentOnSecurity {
            of,
            target,
            position,
            face_up,
            to_owner,
            bind_placed_as,
            result_receipt,
        } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                let top = ctx
                    .game
                    .player(h.player)
                    .battle_area
                    .get(h.index as usize)
                    .map(|p| (p.top_card().owner, p.top_card().handle()));
                let player = match (*to_owner, top) {
                    // DCGO AddSecurityCard -> `cardSource.Owner.SecurityCards`.
                    (true, Some((owner, _))) => owner,
                    _ => crate::dsl_cards::step::resolve_player(ctx, *of),
                };
                let position = super::map_stack_position(*position);
                // A bound payment is expanded into a correlated receipt and
                // finalizer before reaching this synchronous operation.
                debug_assert!(bind_placed_as.is_none());
                ctx.place_permanent_on_security_with_receipt(player, h, position, *face_up, *result_receipt);
            }
            true
        }
        CompiledStep::DeDigivolve {
            target,
            amount,
            amount_fn,
            stop_at_level,
        } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                let resolved_amount = amount_fn.as_ref().map(|formula| {
                    let raw = crate::dsl_cards::formula_eval::evaluate_with_bindings(
                        formula,
                        ctx,
                        h,
                        Some(bindings),
                    );
                    raw.clamp(0, i32::from(u8::MAX)) as u8
                });
                let amount = resolved_amount.or(*amount);
                // Engine signature is (target, stop_at_level, amount) — note stop_at_level
                // precedes amount here, opposite to CompiledStep field order.
                ctx.de_digivolve(h, *stop_at_level, amount);
            }
            true
        }
        CompiledStep::TrashAllSources { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                ctx.trash_all_sources(h);
            }
            true
        }
        CompiledStep::BindPermanentProperty {
            from,
            property,
            bind_as,
        } => {
            let value = match resolve_binding_ref(from, ctx, bindings) {
                Some(ResolvedBinding::Permanent(h)) => permanent_property(ctx, h, *property),
                Some(ResolvedBinding::Card(card)) => match property {
                    CompiledPermanentProperty::Level => ctx.game.card_data_for_handle(card)
                        .and_then(|data| data.level).map(i64::from),
                },
                _ => None,
            };
            if let Some(value) = value {
                bindings.insert_literal(bind_as, value);
            }
            true
        }
        _ => false,
    }
}

fn permanent_property(
    ctx: &EffectContext<'_>,
    handle: crate::permanent::PermanentHandle,
    property: CompiledPermanentProperty,
) -> Option<i64> {
    let permanent = if handle.index == crate::action::space::BREEDING_TARGET as u8 {
        ctx.game.player(handle.player).breeding_area.as_ref()?
    } else {
        ctx.game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)?
    };
    match property {
        CompiledPermanentProperty::Level => permanent.level(&ctx.game.card_data).map(i64::from),
    }
}
