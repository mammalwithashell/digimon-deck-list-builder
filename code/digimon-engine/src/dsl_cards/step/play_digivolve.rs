//! Play / digivolve / placement step lowering (Phase 2f1 Task 4).
//!
//! Wires 12 `CompiledStep` variants to engine primitives:
//!   - `PlayFromHand`, `PlayFromHandFree`
//!   - `PlayFromTrash`, `PlayFromTrashFree`
//!   - `PlayFromSecurity`
//!   - `PlayFromMaterials`
//!   - `EffectInitiatedDigivolve`, `EffectInitiatedDnaDigivolve`
//!   - `PlayToken`
//!   - `PlaceOnSecurity`, `PlaceAsBottomSource`, `PlaceAsTopSource`, `TrashTopSource`
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
use crate::dsl_cards::binding_ref::{
    resolve_binding_ref, resolve_played_permanent_permissive, ResolvedBinding,
};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::resolve_player;
use crate::effect_context::EffectContext;
use crate::enums::CardSourceRef;
use crate::enums::CostDelta;
use crate::permanent::PermanentHandle;
use crate::selection::UnionZoneOrigin;
use crate::trigger_context::ProvenanceToken;

// ─── Cards currently affected by play-verb `bind_as` provenance ───────────
//
// As of change `fix-played-binding-uses-provenance`, the following cards
// combine `bind_as` on a play verb with a downstream consumer that needs
// identity resolution after a possible stack-changing event:
//
//   - BT16-085 Davis Motomiya & Ken Ichijoji — `play_from_hand_free` +
//     `schedule_delayed` with `return_to_hand: { target: { permanent: played
//     } }`. STRICT semantics: the bounce fizzles once the played
//     Veemon/Wormmon is no longer a battle-area top (DNA-consumed, regular
//     digivolve, deleted). Routed through `resolve_binding_ref`'s strict
//     path → `Game::resolve_token_as_battle_area_top`.
//
//   - EX11-022 Karakurumon, EX11-061 Mirai Kinosaki, P-165 ShoeShoemon —
//     `play_from_hand_free` / `play_token` + `schedule_delete_played_at_turn_end`.
//     PERMISSIVE semantics: the carrier is deleted even if the played card
//     became a digivolution card. Routed through
//     `resolve_played_permanent_permissive` → `Game::resolve_provenance_token`.
//
// New cards combining a play-verb `bind_as` with a delayed consumer should
// pick the matching resolver based on the printed-text reading.

/// Bind a play-verb result by stable identity (`ProvenanceToken`) instead
/// of a positional `PermanentHandle`. Reads the played permanent's top card
/// and derives the token deterministically (`From<CardHandle>`).
///
/// The fallback handle is stored for diagnostics; production consumers MUST
/// route through `resolve_binding_ref` (strict) or
/// `resolve_played_permanent_permissive` (carrier-aware) and treat token
/// resolution as the source of truth.
fn bind_played_with_provenance(
    bindings: &mut Bindings,
    ctx: &EffectContext<'_>,
    name: &str,
    played: PermanentHandle,
) {
    let top_handle = ctx
        .game
        .player(played.player)
        .battle_area
        .get(played.index as usize)
        .map(|perm| perm.top_card().handle());
    if let Some(handle) = top_handle {
        let token = ProvenanceToken::from(handle);
        bindings.insert_played_permanent(name, token, played);
    }
    // If the slot is empty (shouldn't happen for a just-played permanent),
    // skip the bind — same silent no-op convention as the rest of the
    // module uses for unresolvable / unexpected state.
}

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
pub(crate) fn lower_cost_delta(
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

/// Lower a DNA-digivolve step's `cost` (a `CompiledCostDelta`) to the final
/// `i32` memory charge passed to the effect-initiated DNA primitives.
///
/// The DNA verbs take a fixed final cost (not a `CostDelta`), so the cost
/// keywords are interpreted for the DNA path specifically:
///
///   `Printed`   → the RESULT card's printed DNA cost, resolved via
///                 `printed_cost` against the chosen materials (DCGO
///                 `payCost: true` → `condition.cost`). If the pair does not
///                 satisfy any printed recipe (so no printed cost resolves),
///                 fall back to 0 — the merge is then recipe-rejected anyway.
///   `Free`      → 0 (free DNA digivolve).
///   `Literal(n)`→ exactly `n` (an authored fixed cost, e.g. the shipped
///                 `cost: 0` Omnimon users).
///   `Reduce(n)` → 0 for `n == 0` (legacy `cost: printed` alias that older
///                 authoring flattened to `Reduce(0)`); for `n > 0` a
///                 printed-cost reduction: printed cost minus `n`, floored at 0.
///   `ReduceFn`  → same as `Reduce(N)` with `N` the evaluated formula.
///
/// `printed_cost` is only invoked for the `Printed` / `Reduce` shapes that need
/// the result card's printed DNA cost, so callers can pass a closure that
/// borrows `ctx` for the lookup.
fn lower_dna_cost(
    cost: &digimon_dsl::compiled::CompiledCostDelta,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
    printed_cost: impl Fn() -> Option<i32>,
) -> i32 {
    use digimon_dsl::compiled::CompiledCostDelta;
    match cost {
        CompiledCostDelta::Printed => printed_cost().unwrap_or(0).max(0),
        CompiledCostDelta::Free => 0,
        CompiledCostDelta::Literal(n) => (*n).max(0),
        CompiledCostDelta::Reduce(n) => {
            let reduce = (*n).max(0);
            if reduce == 0 {
                // Legacy `cost: printed` alias — pay the full printed DNA cost.
                printed_cost().unwrap_or(0).max(0)
            } else {
                (printed_cost().unwrap_or(0) - reduce).max(0)
            }
        }
        CompiledCostDelta::ReduceFn(formula) => {
            let target = ctx
                .source_permanent
                .unwrap_or(crate::permanent::PermanentHandle {
                    player: ctx.player,
                    index: 0,
                });
            let reduce = crate::dsl_cards::formula_eval::evaluate_with_bindings(
                formula,
                ctx,
                target,
                Some(bindings),
            )
            .max(0);
            (printed_cost().unwrap_or(0) - reduce).max(0)
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
        ResolvedBinding::SourceRefs(refs) => refs
            .first()
            .map(|source| CardSourceRef::Material(source.permanent, source.source_index as usize)),
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

/// G-DSL-OUTER-TAIL-NESTED-PARK (effect-driven Option-USE facet).
///
/// When `use_option_from` parks (the used Option's `[Main]` body installed a
/// selection), a body that parked MID-BODY — e.g. inside an `if` with steps
/// still to run after it (EX7-070 Der Blitz: delete pick, THEN the
/// `place_self_under_permanent` leg) — left those remaining steps in the
/// GLOBAL `Game::dsl_outer_tail` slot. If we return to the outer `run_steps`
/// loop with that slot still occupied, the outer scope's
/// `run_tail_preserving_trigger_context` drains it as if it were the OUTER
/// clause's own parked tail: the Option's remaining body steps get re-wrapped
/// under the OUTER card's provenance and sequenced AFTER the outer clause's
/// tail. Two observable corruptions (EX7-048 + EX7-070 end-to-end):
///   1. ordering — the outer tail (remainder-to-deck placement) resolves in
///      the MIDDLE of the Option's `[Main]`, and
///   2. provenance — `place_self_under_permanent` runs with `source_card` =
///      the outer card, so its `pending_option` claim fails and the used
///      Option is mis-disposed to trash instead of seating itself.
///
/// Fix: immediately after the use, while the park is still fresh (before the
/// outer loop's `park_pending_selection_tail` composes the outer tail),
/// re-compose the slot onto the parked selection under the OPTION's own
/// provenance. Frame `outer_conts` then hold [option-body-tail, outer-tail]
/// in the correct order, and the body tail runs with the Option as
/// `source_card` (its `pending_option` claim succeeds).
///
/// NOTE: the sibling verbs `use_option_from_sources` / `use_option_bound`
/// and the from-hand bucket terminal (`run_reveal_bucket_use_option_
/// terminal`) share this latent exposure but have no driver card exercising
/// a mid-body park yet — see qa/archetype-qa/engine-gaps.md.
fn compose_parked_used_option_body_tail(
    ctx: &mut EffectContext<'_>,
    option_card: CardHandle,
    owner: crate::enums::PlayerId,
) {
    if ctx.game.pending_selection.is_some() && ctx.game.dsl_outer_tail.is_some() {
        let mut option_ctx = EffectContext::new(ctx.game, option_card, None, owner);
        crate::dsl_cards::step::drain_dsl_outer_tail(&mut option_ctx);
    }
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
            bind_as,
        } => {
            if let Some(ResolvedBinding::HandIndex(owner, i)) =
                resolve_binding_ref(hand_index, ctx, bindings)
            {
                let delta = lower_cost_delta(cost_delta.as_ref(), ctx, bindings);
                if let Some(played) = ctx.play_from_hand_with_cost(owner, i as usize, delta) {
                    bindings.record_played(played);
                    // G-DSL-SECURITY-EOT-PLAY-AND-PLACE-SELF-UNDER: expose the
                    // played permanent's handle to subsequent steps in the same
                    // body (mirrors PlayFromHandFree's bind_as threading).
                    if let Some(name) = bind_as {
                        bind_played_with_provenance(bindings, ctx, name, played);
                    }
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
                        bind_played_with_provenance(bindings, ctx, name, played);
                    }
                }
            }
            true
        }
        // ── Unified play-OR-use (G-PLAY-OR-USE-FROM-HAND) ─────────────────
        CompiledStep::PlayOrUseFromHand {
            of: _,
            hand_index,
            cost_delta,
        } => {
            if let Some(ResolvedBinding::HandIndex(owner, i)) =
                resolve_binding_ref(hand_index, ctx, bindings)
            {
                let delta = lower_cost_delta(cost_delta.as_ref(), ctx, bindings);
                // Routes by CardKind: Digimon/Tamer → play, Option → use, Dual
                // → "Play as Digimon / Use as Option" face choice. A played
                // Digimon's handle is recorded for any later step; the Option /
                // Dual-Option path manages its own lifecycle internally.
                ctx.play_or_use_from_hand_with_cost(owner, i as usize, delta);
            }
            true
        }
        CompiledStep::PlayFromRevealedFree {
            of,
            card,
            bind_as,
            cost_delta,
        } => {
            let owner = resolve_player(ctx, *of);
            // None => FREE (prior behavior — unlike play_from_trash, whose None
            // means "pay full"); Some => pay the reduced cost.
            let delta = match cost_delta {
                None => CostDelta::Free,
                Some(_) => lower_cost_delta(cost_delta.as_ref(), ctx, bindings),
            };
            match resolve_binding_ref(card, ctx, bindings) {
                Some(ResolvedBinding::Card(handle)) => {
                    if let Some(played) = ctx.play_from_revealed_with_cost(owner, handle, delta) {
                        bindings.record_played(played);
                        if let Some(name) = bind_as {
                            bind_played_with_provenance(bindings, ctx, name, played);
                        }
                    }
                }
                Some(ResolvedBinding::CardList(handles)) => {
                    let mut last_played = None;
                    for handle in handles {
                        if let Some(played) =
                            ctx.play_from_revealed_with_cost(owner, handle, delta.clone())
                        {
                            bindings.record_played(played);
                            last_played = Some(played);
                        }
                    }
                    if let (Some(name), Some(played)) = (bind_as, last_played) {
                        bind_played_with_provenance(bindings, ctx, name, played);
                    }
                }
                _ => {}
            }
            true
        }

        // ── Option-source USE (binding-consuming, non-parking) ────────────
        // These consume a binding produced by an upstream select step
        // (`select_reveal` / `select_own_sources` / `select_union_zone`) and
        // route the picked Option through the SHARED round-1
        // `Game::use_option_from` pipeline (`play_option_core` [Main]
        // resolution + `dispose_option`) — only the `OptionSource` origin
        // differs. The upstream select already parked; this step is a
        // synchronous-family verb. The Option's own body may install a
        // data-driven resume frame if it parks (composed by the outer
        // `run_steps` loop); no bespoke closure is installed here, so the step
        // is clone-safe. `cost_delta: None` = FREE (mirrors PlayFromRevealedFree).
        // `G-DSL-USE-OPTION-FROM-SOURCES`.
        CompiledStep::UseOptionFromRevealed {
            of,
            card,
            cost_delta,
        } => {
            let owner = resolve_player(ctx, *of);
            let delta = match cost_delta {
                None => CostDelta::Free,
                Some(_) => lower_cost_delta(cost_delta.as_ref(), ctx, bindings),
            };
            // The `select_reveal` binding resolves to the reveal-pool card
            // handle; route it through the `OptionSource::Revealed` fork.
            // A `select_reveal_buckets` binding instead resolves to a
            // `CardList` (0..N handles — 0 or 1 for a min:0/max:1 bucket);
            // accept both shapes, mirroring `PlayFromRevealedFree` above
            // (EX7-048 relies on the bucket shape so the mandatory
            // remainder-placement tail still runs when the pick is declined).
            match resolve_binding_ref(card, ctx, bindings) {
                Some(ResolvedBinding::Card(handle)) => {
                    ctx.game.use_option_from(
                        owner,
                        crate::game_actions::OptionSource::Revealed(handle),
                        delta,
                    );
                    compose_parked_used_option_body_tail(ctx, handle, owner);
                }
                Some(ResolvedBinding::CardList(handles)) => {
                    for handle in handles {
                        ctx.game.use_option_from(
                            owner,
                            crate::game_actions::OptionSource::Revealed(handle),
                            delta.clone(),
                        );
                        compose_parked_used_option_body_tail(ctx, handle, owner);
                    }
                }
                _ => {}
            }
            true
        }
        CompiledStep::UseOptionFromSources {
            of,
            card,
            cost_delta,
        } => {
            let owner = resolve_player(ctx, *of);
            let delta = match cost_delta {
                None => CostDelta::Free,
                Some(_) => lower_cost_delta(cost_delta.as_ref(), ctx, bindings),
            };
            // A `select_own_sources` binding yields one or more
            // `SourceSelectionRef`s (carrier permanent + card handle). Use the
            // first — the driver clauses pick exactly one. The engine
            // `OptionSource::Source` fork resolves both `card_sources` and
            // `linked_cards`.
            if let Some(ResolvedBinding::SourceRefs(refs)) = resolve_binding_ref(card, ctx, bindings)
            {
                if let Some(sref) = refs.first() {
                    ctx.game.use_option_from(
                        owner,
                        crate::game_actions::OptionSource::Source {
                            host: sref.permanent,
                            card: sref.card,
                        },
                        delta,
                    );
                }
            }
            true
        }
        CompiledStep::UseOptionBound { binding, cost_delta } => {
            let delta = match cost_delta {
                None => CostDelta::Free,
                Some(_) => lower_cost_delta(cost_delta.as_ref(), ctx, bindings),
            };
            // Resolve the `select_union_zone` binding: the picked Option, its
            // origin zone (hand or trash), and that zone's owner. Route to the
            // matching origin through `Game::use_option_from`. A `Material`
            // origin is not a legal Option-use zone (no driver uses it) — silent
            // no-op. The binding is read directly (not via `resolve_binding_ref`)
            // so the origin tag is preserved.
            if let Some((card, origin, owner)) = bindings.get_union_card(binding) {
                match origin {
                    UnionZoneOrigin::Hand => {
                        if let Some(idx) = ctx
                            .game
                            .player(owner)
                            .hand
                            .iter()
                            .position(|c| c.handle() == card)
                        {
                            ctx.game.use_option_from(
                                owner,
                                crate::game_actions::OptionSource::Hand(idx),
                                delta,
                            );
                        }
                    }
                    UnionZoneOrigin::Trash => {
                        if let Some(idx) = ctx
                            .game
                            .player(owner)
                            .trash
                            .iter()
                            .position(|c| c.handle() == card)
                        {
                            ctx.game.use_option_from(
                                owner,
                                crate::game_actions::OptionSource::Trash(idx),
                                delta,
                            );
                        }
                    }
                    UnionZoneOrigin::Material { .. } => {}
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
            suspended,
        } => {
            // `play_from_trash_free_unsuspended` takes a `CardHandle`. The
            // binding may address the card either by trash index
            // (`ResolvedBinding::TrashIndex`) or directly by card handle
            // (`ResolvedBinding::Card`) — the latter lets a card-identity
            // binding such as `event_card` / `event_target` (which, inside an
            // [On Deletion] body, resolves to the just-deleted carrier's
            // top card now in trash) feed a free replay. This is what BT3-109
            // "Back for Revenge!" needs to "Play this card" from trash. The
            // engine call self-guards: it returns `None` if the handle is not
            // actually in the controller's trash.
            // The playing CONTROLLER follows the card's trash owner — for
            // `of: opponent` ("YOUR OPPONENT plays 1 ... from THEIR trash",
            // EX5-060 / Q28) the binding's owner IS the opponent; the card
            // enters THEIR battle area. A card-handle binding locates its
            // owner by trash scan; fallback: the effect controller.
            let resolved: Option<(crate::enums::PlayerId, CardHandle)> =
                match resolve_binding_ref(trash_index, ctx, bindings) {
                    Some(ResolvedBinding::TrashIndex(owner, i)) => ctx
                        .game
                        .player(owner)
                        .trash
                        .get(i as usize)
                        .map(|cs| (owner, cs.handle())),
                    Some(ResolvedBinding::Card(h)) => {
                        let owner = (0..ctx.game.players.len() as crate::enums::PlayerId)
                            .find(|&p| ctx.game.player(p).trash.iter().any(|c| c.handle() == h))
                            .unwrap_or(ctx.player);
                        Some((owner, h))
                    }
                    _ => None,
                };
            if let Some((controller, h)) = resolved {
                // G-PLAY-ENTERS-SUSPENDED (Q28 / EX5-060): a flagged play
                // enters suspended — the commit site consumes the slot.
                if *suspended {
                    ctx.game.play_enters_suspended = true;
                }
                // PUPPETS-G030 — when `suppress_on_play` is set, the
                // played Digimon's own `[On Play]` effects do not fire
                // for this play event (BT5-106 [Security]). The skip is an
                // EFFECT of this card on the played Digimon, so record the
                // suppressor identity — `fire_play_event_triggers` consults
                // `permanent_is_unaffected_by_effect` against it before
                // honoring the skip (judge-quiz Q28).
                let played = if *suppress_on_play {
                    ctx.game.on_play_suppressor = Some((ctx.player, ctx.source_kind));
                    ctx.play_from_trash_free_unsuspended_for(controller, h, true)
                } else {
                    ctx.play_from_trash_free_unsuspended_for(controller, h, false)
                };
                // The play may have been rejected (full field, lock) — clear
                // any unconsumed slots so they cannot leak to a later play.
                ctx.game.play_enters_suspended = false;
                ctx.game.on_play_suppressor = None;
                if let Some(played) = played {
                    bindings.record_played(played);
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
            // came from (hand, trash, or material), and the owner of that zone. The
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
                        bind_played_with_provenance(bindings, ctx, name, played);
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
            // `play_token` bind_as). PERMISSIVE resolver: if the played card
            // is now a digivolution card under another top, the carrier is
            // still the rules-correct deletion target (EX11-022 Karakurumon,
            // EX11-061 Mirai Kinosaki, P-165 ShoeShoemon semantics — the
            // `schedule_provenance_deletion` machinery exists exactly for
            // this). Missing / wrong-kind binding: silent no-op (e.g. the
            // optional play was declined).
            if let Some(handle) = resolve_played_permanent_permissive(binding, ctx, bindings) {
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
        CompiledStep::ReturnSelectedSecurityToDeck { of, card, position } => {
            // G-DSL-RETURN-SELECTED-SECURITY-TO-DECK: `card` is a CardHandle
            // binding (typically from a prior `select_security`). Move exactly
            // that security card to its owner's deck (top or bottom). No-op
            // when the binding is unset (declined optional select).
            let owner = resolve_player(ctx, *of);
            let to_bottom = matches!(
                super::map_stack_position(*position),
                crate::enums::StackPosition::Bottom
            );
            if let Some(ResolvedBinding::Card(handle)) = resolve_binding_ref(card, ctx, bindings) {
                let _ = ctx.return_security_card_to_deck(owner, handle, to_bottom);
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
                            bind_played_with_provenance(bindings, ctx, name, played);
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
                                bind_played_with_provenance(bindings, ctx, name, played);
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
                        bind_played_with_provenance(bindings, ctx, name, played);
                    }
                }
                _ => {}
            }
            true
        }
        CompiledStep::PlayUnderTamerSource {
            source_refs,
            cost_delta,
            bind_as,
        } => {
            let delta = lower_cost_delta(cost_delta.as_ref(), ctx, bindings);
            let mut last_played = None;
            if let Some(source_refs) = bindings.get_source_refs(source_refs) {
                for source_ref in source_refs {
                    if let Some(played) = ctx.play_under_tamer_source_with_cost(source_ref, delta) {
                        bindings.record_played(played);
                        last_played = Some(played);
                    }
                }
            }
            if let (Some(name), Some(played)) = (bind_as, last_played) {
                bindings.insert_permanent(name, played);
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
        CompiledStep::AppFuse {
            from_zone,
            result_filter,
            optional,
        } => {
            ctx.initiate_effect_app_fuse(*from_zone, result_filter.as_ref(), *optional);
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
            // G-ENGINE-DNA-PRINTED-COST (gap 1): `cost: printed` pays the
            // RESULT card's printed DNA cost against the chosen {a, b} pair
            // (DCGO `payCost: true` → `condition.cost`). Look it up from the
            // materials; if the pair doesn't satisfy the recipe there is no
            // printed cost — the merge itself will then be recipe-rejected.
            let dna_cost =
                lower_dna_cost(cost, ctx, bindings, || ctx.printed_dna_cost_for_pair(a, b, from_card));
            let success = ctx.effect_initiated_dna_digivolve(
                a,
                b,
                from_card,
                dna_cost,
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
            // G-ENGINE-DNA-PRINTED-COST (gap 1): `cost: printed` pays the
            // result card's printed DNA cost against the {field target, hand
            // partner} pair (DCGO `payCost: true`).
            let dna_cost = lower_dna_cost(cost, ctx, bindings, || {
                ctx.printed_dna_cost_for_hand_partner(target_handle, partner_card, result_card)
            });
            let success = ctx.effect_initiated_dna_digivolve_with_hand_partner(
                target_handle,
                partner_card,
                result_card,
                dna_cost,
                *ignore_requirements,
            );
            if let Some(played) = success {
                bindings.record_digivolved(played);
            }
            true
        }
        CompiledStep::EffectInitiatedDnaDigivolveTrashPartner {
            target,
            trash_partner,
            from_hand,
            cost,
            ignore_requirements,
        } => {
            // G-DSL-DNA-TRASH-PARTNER: DNA digivolve where one material is a
            // battle-area permanent and the other is a card in the
            // controller's TRASH (BT18-015 / BT18-073 [On Deletion]). Lowers
            // to `effect_initiated_dna_digivolve_trash_partner`
            // (G-ENGINE-DNA-TRASH-MATERIAL): the trash material moves
            // STRAIGHT into the merged stack — never an independently played
            // permanent, no [On Play] fires (DCGO CreateNewPermanent).
            let target_handle = match resolve_binding_ref(target, ctx, bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                _ => return true,
            };
            let trash_card = match resolve_binding_ref(trash_partner, ctx, bindings) {
                Some(ResolvedBinding::Card(h)) => h,
                Some(ResolvedBinding::TrashIndex(owner, i)) => {
                    match ctx.game.player(owner).trash.get(i as usize) {
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
            // G-ENGINE-DNA-PRINTED-COST: `cost: printed` pays the result
            // card's printed DNA cost against the {field target, trash
            // partner} pair (DCGO `payCost: true`).
            let dna_cost = lower_dna_cost(cost, ctx, bindings, || {
                ctx.printed_dna_cost_for_field_trash_pair(target_handle, trash_card, result_card)
            });
            let success = ctx.effect_initiated_dna_digivolve_trash_partner(
                target_handle,
                trash_card,
                result_card,
                dna_cost,
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
                    bind_played_with_provenance(bindings, ctx, name, played);
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
        // Top-position sibling of PlaceAsBottomSource — the card lands as
        // `target`'s TOP digivolution source (directly beneath the active
        // top card). DCGO `AddDigivolutionCardsTop` explicitly cannot place
        // a whole field permanent under a stack (its doc points at
        // IPlacePermanentToDigivolutionCards for that), so unlike the bottom
        // arm there is no permanent-source branch here — a binding that
        // resolves to a Permanent is a no-op. G-DSL-PLACE-AS-TOP-SOURCE.
        CompiledStep::PlaceAsTopSource {
            source,
            target,
            face_down,
        } => {
            let target_handle = match resolve_binding_ref(target, ctx, bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                _ => return true,
            };
            let Some(source_ref) = resolve_card_source_ref(source, ctx, bindings) else {
                return true;
            };
            let _ = ctx.place_as_top_source(source_ref, target_handle, *face_down);
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
        CompiledStep::TrashBottomSources { target, count } => {
            if let Some(ResolvedBinding::Permanent(h)) = resolve_binding_ref(target, ctx, bindings)
            {
                let _ = ctx.trash_bottom_sources(h, *count);
            }
            true
        }

        _ => false,
    }
}
