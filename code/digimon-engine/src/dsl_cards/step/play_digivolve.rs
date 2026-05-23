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
use crate::selection::UnionZoneOrigin;

/// Translate the IR's `CompiledCostDelta` to the engine's `CostDelta`.
///
///   `None`              → `CostDelta::Reduce(0)` (default to printed cost)
///   `Some(Printed)`     → `CostDelta::Reduce(0)`
///   `Some(Free)`        → `CostDelta::Free`
///   `Some(Literal(n))`  → `CostDelta::Fixed(n as i16)`
///   `Some(Reduce(n))`   → `CostDelta::Reduce(n as i16)`
///   `Some(ReduceFn(f))` → `CostDelta::Reduce(N)` where `N` is the formula
///                         result computed against `ctx`/`bindings` at
///                         resolution time (G-FORMULA-COST-DELTA). A negative
///                         result is clamped to 0 — a "reduction" can only
///                         lower, never raise, the printed cost.
fn lower_cost_delta(
    d: Option<&digimon_dsl::compiled::CompiledCostDelta>,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
) -> CostDelta {
    use digimon_dsl::compiled::CompiledCostDelta;
    match d {
        None | Some(CompiledCostDelta::Printed) => CostDelta::Reduce(0),
        Some(CompiledCostDelta::Free) => CostDelta::Free,
        Some(CompiledCostDelta::Literal(n)) => CostDelta::Fixed(*n as i16),
        Some(CompiledCostDelta::Reduce(n)) => CostDelta::Reduce(*n as i16),
        Some(CompiledCostDelta::ReduceFn(formula)) => {
            let target = ctx
                .source_permanent
                .unwrap_or(crate::permanent::PermanentHandle {
                    player: ctx.player,
                    index: 0,
                });
            let raw = crate::dsl_cards::formula_eval::evaluate_with_bindings(
                formula,
                ctx,
                target,
                Some(bindings),
            );
            CostDelta::Reduce(raw.max(0).min(i16::MAX as i32) as i16)
        }
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
    // `DeckTop` is a card-source-only binding (no `ResolvedBinding` form):
    // resolve the owning player and address the top of their deck directly.
    if let digimon_dsl::compiled::CompiledBindingRef::DeckTop(of) = source {
        return Some(CardSourceRef::DeckTop(resolve_player(ctx, *of)));
    }
    match resolve_binding_ref(source, ctx, bindings)? {
        ResolvedBinding::HandIndex(owner, i) => Some(CardSourceRef::Hand(owner, i as usize)),
        ResolvedBinding::TrashIndex(owner, i) => Some(CardSourceRef::Trash(owner, i as usize)),
        ResolvedBinding::Card(h) => resolve_card_handle_source_ref(ctx, h),
        // Other kinds (permanent / list): not addressable as a card source.
        _ => None,
    }
}

fn resolve_card_handle_source_ref(ctx: &EffectContext<'_>, h: CardHandle) -> Option<CardSourceRef> {
    for pid in 0..ctx.game.players.len() {
        let player_id = pid as crate::enums::PlayerId;
        let player = ctx.game.player(player_id);
        if let Some(idx) = player.hand.iter().position(|c| c.handle() == h) {
            return Some(CardSourceRef::Hand(player_id, idx));
        }
        if let Some(idx) = player.trash.iter().position(|c| c.handle() == h) {
            return Some(CardSourceRef::Trash(player_id, idx));
        }
        if let Some(idx) = player.security.iter().position(|c| c.handle() == h) {
            return Some(CardSourceRef::Security(player_id, idx));
        }
        for (perm_idx, perm) in player.battle_area.iter().enumerate() {
            if let Some(src_idx) = perm.card_sources.iter().position(|c| c.handle() == h) {
                return Some(CardSourceRef::Material(
                    crate::permanent::PermanentHandle {
                        player: player_id,
                        index: perm_idx as u8,
                    },
                    src_idx,
                ));
            }
        }
    }
    if ctx.game.revealed_cards.iter().any(|c| c.handle() == h) {
        return Some(CardSourceRef::Reveal(h));
    }
    None
}

/// Resolve a `CardHandle` to its current index within `carrier`'s
/// digivolution-source stack, or `None` if the handle is not (or no
/// longer) a source of that permanent. Used by `PlayFromMaterials` to
/// re-resolve each batched pick immediately before its play — the index
/// must be looked up fresh because each play shifts later indices down.
fn source_index_of(
    ctx: &EffectContext<'_>,
    carrier: crate::permanent::PermanentHandle,
    card: CardHandle,
) -> Option<usize> {
    crate::effect_context::material_carrier_permanent(ctx.game, carrier).and_then(|perm| {
        perm.card_sources
            .iter()
            .position(|source| source.handle() == card)
    })
}

/// Try to handle `step` as a play / digivolve / placement variant.
/// Returns `true` if the variant was matched (regardless of whether the
/// underlying engine call succeeded).
pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &mut Bindings) -> bool {
    match step {
        // ── Play primitives (hand) ────────────────────────────────────────
        CompiledStep::PlayFromHand {
            of: _,
            hand_index,
            cost_delta,
        } => {
            if let Some(ResolvedBinding::HandIndex(owner, i)) =
                resolve_binding_ref(hand_index, ctx, bindings)
            {
                let delta = lower_cost_delta(cost_delta.as_ref(), ctx, bindings);
                if let Some(played) = ctx.play_from_hand_with_cost(owner, i as usize, delta) {
                    bindings.record_played(played);
                }
            }
            true
        }
        CompiledStep::PlayFromHandFree {
            of: _,
            hand_index,
            bind_as,
        } => {
            if let Some(ResolvedBinding::HandIndex(owner, i)) =
                resolve_binding_ref(hand_index, ctx, bindings)
            {
                if let Some(played) = ctx.play_from_hand_free(owner, i as usize) {
                    bindings.record_played(played);
                    // G-PLAY-FROM-HAND-FREE-BIND-AS: expose the played
                    // permanent's handle to subsequent steps in the same body.
                    if let Some(name) = bind_as {
                        bindings.insert_permanent(name, played);
                    }
                }
            }
            true
        }

        // ── Play primitives (trash) ───────────────────────────────────────
        CompiledStep::PlayFromTrash {
            of: _,
            trash_index,
            cost_delta,
        } => {
            if let Some(ResolvedBinding::TrashIndex(owner, i)) =
                resolve_binding_ref(trash_index, ctx, bindings)
            {
                let delta = lower_cost_delta(cost_delta.as_ref(), ctx, bindings);
                if let Some(played) = ctx.play_from_trash_with_cost(owner, i as usize, delta) {
                    bindings.record_played(played);
                }
            }
            true
        }
        CompiledStep::PlayFromTrashFree {
            of: _,
            trash_index,
            suppress_on_play,
        } => {
            // `play_from_trash_free_unsuspended` takes a `CardHandle`; the
            // IR addresses by trash index so we must look up the handle.
            if let Some(ResolvedBinding::TrashIndex(owner, i)) =
                resolve_binding_ref(trash_index, ctx, bindings)
            {
                let handle: Option<CardHandle> = ctx
                    .game
                    .player(owner)
                    .trash
                    .get(i as usize)
                    .map(|cs| cs.handle());
                if let Some(h) = handle {
                    // PUPPETS-G030 — when `suppress_on_play` is set, the
                    // played Digimon's own `[On Play]` effects do not fire
                    // for this play event (BT5-106 [Security]).
                    let played = if *suppress_on_play {
                        ctx.play_from_trash_free_unsuspended_suppress_on_play(h)
                    } else {
                        ctx.play_from_trash_free_unsuspended(h)
                    };
                    if let Some(played) = played {
                        bindings.record_played(played);
                    }
                }
            }
            true
        }

        // ── Origin-preserving union-zone play (PUPPETS-G014) ──────────────
        CompiledStep::PlayUnionBoundFree {
            binding,
            bind_as,
            suppress_on_play,
        } => {
            // Resolve the union-zone binding: the picked card, the zone it
            // came from (hand vs trash), and the owner of that zone. The
            // binding is read directly (not via `resolve_binding_ref`) so the
            // origin tag is preserved — `ResolvedBinding` has no zone-tagged
            // card variant. Missing / wrong-kind binding: silent no-op, per
            // the module strictness convention.
            if let Some((card, origin, owner)) = bindings.get_union_card(binding) {
                let played = match origin {
                    UnionZoneOrigin::Hand => {
                        // Locate the card in the owner's hand by handle —
                        // the index may have shifted since selection.
                        ctx.game
                            .player(owner)
                            .hand
                            .iter()
                            .position(|c| c.handle() == card)
                            .and_then(|idx| {
                                ctx.play_from_hand_free_suppress_on_play(
                                    owner,
                                    idx,
                                    *suppress_on_play,
                                )
                            })
                    }
                    UnionZoneOrigin::Trash => {
                        // Locate the card in the owner's trash by handle and
                        // play it for free (CostDelta::Free → no cost paid).
                        ctx.game
                            .player(owner)
                            .trash
                            .iter()
                            .position(|c| c.handle() == card)
                            .and_then(|idx| {
                                ctx.play_from_trash_with_cost_suppress_on_play(
                                    owner,
                                    idx,
                                    CostDelta::Free,
                                    *suppress_on_play,
                                )
                            })
                    }
                    UnionZoneOrigin::Material {
                        carrier,
                        source_index,
                    } => ctx.play_from_materials_suppress_on_play(
                        carrier,
                        source_index as usize,
                        CostDelta::Free,
                        *suppress_on_play,
                    ),
                };
                if let Some(played) = played {
                    bindings.record_played(played);
                    // Expose the played permanent's handle for later steps
                    // (e.g. a Task 11 cleanup), mirroring PlayFromHandFree.
                    if let Some(name) = bind_as {
                        bindings.insert_permanent(name, played);
                    }
                }
            }
            true
        }

        // ── Provenance-bound turn-end self-delete (PUPPETS-G003 / G016) ──
        CompiledStep::ScheduleDeletePlayedAtTurnEnd {
            binding,
            at_opponents_turn,
        } => {
            // Resolve the permanent binding produced by a preceding free-play
            // step (`play_union_bound_free` / `play_from_hand_free` /
            // `play_token` bind_as). Captures the permanent's stable
            // `ProvenanceToken` now, so the turn-end deletion hits the right
            // permanent even after battle-area indices shift. Missing /
            // wrong-kind binding: silent no-op (e.g. the optional play was
            // declined).
            if let Some(ResolvedBinding::Permanent(handle)) = resolve_binding_ref(
                &digimon_dsl::compiled::CompiledBindingRef::Named(binding.clone()),
                ctx,
                bindings,
            ) {
                if *at_opponents_turn {
                    ctx.schedule_delete_at_end_of_opponents_turn(handle);
                } else {
                    ctx.schedule_delete_at_end_of_turn(handle);
                }
            }
            true
        }

        // ── Play primitives (security / materials) ────────────────────────
        CompiledStep::PlayFromSecurity => {
            // Dispatch depends on context:
            //   - Security-skill timing: the card was already popped from the
            //     security zone by the combat loop and parked in
            //     `Game::pending_security`. Use `play_pending_security()` to
            //     mark the `played` bit so it doesn't get trashed after the
            //     check (BT21-015, BT5-093, BT9-092, BT22-084, etc.).
            //   - All other timings (e.g. "[When Digivolving] play 1 card from
            //     security"): use `play_from_security(player)` which pops
            //     from the live security zone.
            if ctx.game.pending_security.is_some() {
                ctx.play_pending_security();
            } else {
                if let Some(played) = ctx.play_from_security(ctx.player) {
                    bindings.record_played(played);
                }
            }
            true
        }
        // ── Play a specific bound card from the security stack ────────────
        CompiledStep::PlaySecurityCard { of, card } => {
            // G-PLAY-SELECTED-SECURITY-CARD: `card` is a CardHandle binding
            // (typically from a prior `select_security`). Resolve the owner
            // and play exactly that security card free.
            let owner = resolve_player(ctx, *of);
            if let Some(ResolvedBinding::Card(handle)) = resolve_binding_ref(card, ctx, bindings) {
                if let Some(played) = ctx.play_from_security_card(owner, handle) {
                    bindings.record_played(played);
                }
            }
            true
        }
        // ── Trash a specific bound card from the security stack ───────────
        CompiledStep::TrashSelectedSecurity { of, card } => {
            // G-TRASH-SELECTED-SECURITY: `card` is a CardHandle binding
            // (typically from a prior `select_security`). Trash exactly that
            // security card. No-op when the binding is unset — i.e. the
            // player declined an optional `select_security`.
            let owner = resolve_player(ctx, *of);
            if let Some(ResolvedBinding::Card(handle)) = resolve_binding_ref(card, ctx, bindings) {
                let _ = ctx.trash_security_card(owner, handle);
            }
            true
        }
        CompiledStep::PlayFromMaterials {
            target,
            source_index,
            cost_delta,
            suppress_on_play,
            bind_as,
        } => {
            let target_handle = match resolve_binding_ref(target, ctx, bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                _ => return true,
            };
            let delta = lower_cost_delta(cost_delta.as_ref(), ctx, bindings);

            // `source_index` addresses the carrier's digivolution source(s)
            // to play. Three binding shapes are accepted:
            //   - `Literal(n)`  — a single fixed `card_sources` index.
            //   - `Card(h)`     — a single source identified by its handle.
            //   - `CardList(v)` — a BATCH of sources (e.g. the picks from a
            //     `select_materials` multi-pick); every one is played.
            // For the batch path each handle is re-resolved to its CURRENT
            // index immediately before its play, because `play_from_materials`
            // removes the consumed source and shifts later indices down.
            match resolve_binding_ref(source_index, ctx, bindings) {
                Some(ResolvedBinding::Literal(v)) if v >= 0 => {
                    if let Some(played) = ctx.play_from_materials_suppress_on_play(
                        target_handle,
                        v as usize,
                        delta,
                        *suppress_on_play,
                    ) {
                        bindings.record_played(played);
                        if let Some(name) = bind_as {
                            bindings.insert_permanent(name, played);
                        }
                    }
                }
                Some(ResolvedBinding::Card(card)) => {
                    if let Some(idx) = source_index_of(ctx, target_handle, card) {
                        if let Some(played) = ctx.play_from_materials_suppress_on_play(
                            target_handle,
                            idx,
                            delta,
                            *suppress_on_play,
                        ) {
                            bindings.record_played(played);
                            if let Some(name) = bind_as {
                                bindings.insert_permanent(name, played);
                            }
                        }
                    }
                }
                Some(ResolvedBinding::CardList(cards)) => {
                    // Batch source play: each picked source becomes a fresh
                    // battle-area permanent. The last successful play is
                    // recorded under `bind_as` (matches the single-card path's
                    // single-handle binding contract).
                    let mut last_played = None;
                    for card in cards {
                        let Some(idx) = source_index_of(ctx, target_handle, card) else {
                            // Handle no longer in the carrier's stack (already
                            // consumed earlier in this batch, or removed) —
                            // skip it; the remaining picks still play.
                            continue;
                        };
                        if let Some(played) = ctx.play_from_materials_suppress_on_play(
                            target_handle,
                            idx,
                            delta,
                            *suppress_on_play,
                        ) {
                            bindings.record_played(played);
                            last_played = Some(played);
                        }
                    }
                    if let (Some(name), Some(played)) = (bind_as, last_played) {
                        bindings.insert_permanent(name, played);
                    }
                }
                _ => {}
            }
            true
        }

        // ── Digivolve primitives ──────────────────────────────────────────
        CompiledStep::EffectInitiatedDigivolve {
            target,
            from_hand,
            cost,
            ignore_requirements,
        } => {
            let target_handle = match resolve_binding_ref(target, ctx, bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                _ => return true,
            };
            let Some(source_ref) = resolve_card_source_ref(from_hand, ctx, bindings) else {
                return true;
            };
            let delta = lower_cost_delta(Some(cost), ctx, bindings);
            // The effect runs on the target's controller (the digivolve is
            // applied to `target`; the result card can now come from any
            // supported source zone.
            let player = target_handle.player;
            let success = if *ignore_requirements {
                ctx.effect_initiated_digivolve_from_source_ignore_requirements(
                    player,
                    source_ref,
                    target_handle,
                    delta,
                )
            } else {
                ctx.effect_initiated_digivolve_from_source(
                    player,
                    source_ref,
                    target_handle,
                    delta,
                    false,
                )
            };
            if success {
                bindings.record_digivolved(target_handle);
            }
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
                Some(ResolvedBinding::HandIndex(owner, i)) => {
                    match ctx.game.player(owner).hand.get(i as usize) {
                        Some(cs) => cs.handle(),
                        None => return true,
                    }
                }
                _ => return true,
            };
            let cost = match lower_cost_delta(Some(cost), ctx, bindings) {
                CostDelta::Free => 0,
                CostDelta::Fixed(n) => n,
                CostDelta::Reduce(n) => {
                    // DNA effect path still takes a final i32 cost. Until the
                    // engine primitive is widened, `Reduce(n)` is interpreted
                    // as "reduce printed DNA cost"; DNA printed-cost lookup is
                    // not available here, so clamp to zero and keep the shape
                    // ready for the primitive follow-up.
                    debug_assert!(n >= 0, "negative DNA cost reduction is not meaningful");
                    0
                }
            };
            let success = ctx.effect_initiated_dna_digivolve(
                a,
                b,
                from_card,
                cost as i32,
                *ignore_requirements,
            );
            if let Some(played) = success {
                bindings.record_digivolved(played);
            }
            true
        }
        CompiledStep::EffectInitiatedDnaDigivolveHandPartner {
            target,
            hand_partner,
            from_hand,
            cost,
            ignore_requirements,
        } => {
            let target_handle = match resolve_binding_ref(target, ctx, bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                _ => return true,
            };
            let partner_card = match resolve_binding_ref(hand_partner, ctx, bindings) {
                Some(ResolvedBinding::Card(h)) => h,
                Some(ResolvedBinding::HandIndex(owner, i)) => {
                    match ctx.game.player(owner).hand.get(i as usize) {
                        Some(cs) => cs.handle(),
                        None => return true,
                    }
                }
                _ => return true,
            };
            let result_card = match resolve_binding_ref(from_hand, ctx, bindings) {
                Some(ResolvedBinding::Card(h)) => h,
                Some(ResolvedBinding::HandIndex(owner, i)) => {
                    match ctx.game.player(owner).hand.get(i as usize) {
                        Some(cs) => cs.handle(),
                        None => return true,
                    }
                }
                _ => return true,
            };
            let cost = match lower_cost_delta(Some(cost), ctx, bindings) {
                CostDelta::Free => 0,
                CostDelta::Fixed(n) => n,
                CostDelta::Reduce(n) => {
                    debug_assert!(n >= 0, "negative DNA cost reduction is not meaningful");
                    0
                }
            };
            let success = ctx.effect_initiated_dna_digivolve_with_hand_partner(
                target_handle,
                partner_card,
                result_card,
                cost as i32,
                *ignore_requirements,
            );
            if let Some(played) = success {
                bindings.record_digivolved(played);
            }
            true
        }

        // ── Token / placement ─────────────────────────────────────────────
        CompiledStep::PlayToken {
            controller,
            token_name,
            bind_as,
        } => {
            let p = resolve_player(ctx, *controller);
            if let Some(played) = ctx.play_token(p, token_name) {
                bindings.record_played(played);
                // G016 binding half: expose the created token's handle to
                // subsequent steps in the same body (mirrors PlayFromHandFree).
                if let Some(name) = bind_as {
                    bindings.insert_permanent(name, played);
                }
            }
            true
        }
        CompiledStep::PlaceOnSecurity {
            of,
            source,
            position,
            face_up,
        } => {
            let p = resolve_player(ctx, *of);
            let Some(source_ref) = resolve_card_source_ref(source, ctx, bindings) else {
                return true;
            };
            let _ = ctx.place_on_security(
                p,
                source_ref,
                super::map_stack_position(*position),
                *face_up,
            );
            true
        }
        CompiledStep::PlaceAsBottomSource {
            source,
            target,
            face_down,
        } => {
            let target_handle = match resolve_binding_ref(target, ctx, bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                _ => return true,
            };
            if let Some(ResolvedBinding::Permanent(source_handle)) =
                resolve_binding_ref(source, ctx, bindings)
            {
                let _ = ctx.place_permanent_as_bottom_sources(source_handle, target_handle);
                return true;
            }
            let Some(source_ref) = resolve_card_source_ref(source, ctx, bindings) else {
                return true;
            };
            let _ = ctx.place_as_bottom_source(source_ref, target_handle, *face_down);
            true
        }
        CompiledStep::PlaceTopSourceAsBottom { target } => {
            let Some(ResolvedBinding::Permanent(target_handle)) =
                resolve_binding_ref(target, ctx, bindings)
            else {
                return true;
            };
            let _ = ctx.place_top_source_as_bottom(target_handle);
            true
        }
        CompiledStep::TrashTopSource { target } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                let _ = ctx.trash_top_source(h);
            }
            true
        }

        _ => false,
    }
}
