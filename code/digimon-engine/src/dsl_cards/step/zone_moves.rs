//! Binding-consuming zone-move step lowering. Phase 2b covers
//! `AddToHandFromDeck`, `AddToHandFromTrash`, and the reveal-pool / security-mark
//! family added in Task 5.

use digimon_dsl::compiled::{CompiledFormula, CompiledPredicate, CompiledStep, CompiledZone};
use std::sync::Arc;

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::formula_eval;
use crate::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use crate::dsl_cards::step::{resolve_player, run_steps_with_runtime, StepRuntime};
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::{GamePhase, PlayerId};
use crate::permanent::PermanentHandle;
use crate::selection::{PendingSelection, SelectionKind};

fn singleton_card(resolved: ResolvedBinding) -> Option<crate::card_source::CardHandle> {
    match resolved {
        ResolvedBinding::Card(h) => Some(h),
        ResolvedBinding::CardList(cards) if cards.len() == 1 => cards.first().copied(),
        _ => None,
    }
}

fn install_may_add_top_security_to_hand(ctx: &mut EffectContext<'_>, target_player: PlayerId) {
    use crate::action::space::{MAX_SECURITY, SEL_MY_SECURITY_START, SEL_OPP_SECURITY_START};

    let security_len = ctx.game.player(target_player).security.len();
    if security_len == 0 {
        return;
    }

    let base = if target_player == ctx.player {
        SEL_MY_SECURITY_START
    } else {
        SEL_OPP_SECURITY_START
    };
    let top_index = security_len
        .saturating_sub(1)
        .min(MAX_SECURITY.saturating_sub(1));
    let selecting_player = ctx.override_selecting_player().unwrap_or(ctx.player);
    let controller = ctx.player;
    let override_pin = ctx.override_selecting_player();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let previous_phase = ctx.game.current_phase;

    ctx.game.current_phase = GamePhase::SelectSecurity;
    ctx.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::Security,
        selecting_player,
        previous_phase,
        valid_action_ids: vec![base + top_index as u16],
        is_optional: true,
        prompt: "Add top security to hand".to_string(),
        effect_choices: None,
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game, _action_id| {
            let mut cb_ctx = EffectContext::new_with_source_kind_and_override(
                game,
                source_card,
                source_permanent,
                source_kind,
                controller,
                override_pin,
            );
            cb_ctx.add_top_security_to_hand(target_player);
        }),
        on_decline: Some(Box::new(|_game| {})),
    });
}

/// Returns `true` if `step` is a zone-move family handled here. Unknown
/// steps fall through (the caller may try other families).
///
/// Changed in Task 5: third parameter is now `&mut Bindings` so that
/// `RevealTopDeck` can produce a named binding for the revealed card.
pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    runtime: &StepRuntime,
) -> bool {
    match step {
        CompiledStep::BounceSelf => {
            let _ = ctx.bounce_self();
            true
        }
        CompiledStep::PlaceSelfAtSecurity { position, face_up } => {
            let position = super::map_stack_position(*position);
            let _ = ctx.place_self_at_security(position, *face_up);
            true
        }
        CompiledStep::PlaceSelfOptionAtSecurity { position, face_up } => {
            let position = super::map_stack_position(*position);
            let _ = ctx.place_self_option_at_security(position, *face_up);
            true
        }
        CompiledStep::AddToHandFromTrash { of, card } => {
            let Some(resolved) = resolve_binding_ref(card, ctx, bindings) else {
                return true;
            };
            let p = resolve_player(ctx, *of);
            // Resolve the trash slot → CardHandle → engine API. If the
            // binding is a CardHandle directly, pass it through.
            let (owner, handle) = match resolved {
                ResolvedBinding::TrashIndex(owner, i) => {
                    let handle = ctx
                        .game
                        .player(owner)
                        .trash
                        .get(i as usize)
                        .map(|cs| cs.handle());
                    (owner, handle)
                }
                ResolvedBinding::Card(h) => (p, Some(h)),
                _ => (p, None),
            };
            if let Some(h) = handle {
                if ctx.add_to_hand_from_trash(owner, h) {
                    bindings.record_added_to_hand(h);
                }
            }
            true
        }
        CompiledStep::AddToHandFromSecurity { of, card } => {
            let Some(resolved) = resolve_binding_ref(card, ctx, bindings) else {
                return true;
            };
            let p = resolve_player(ctx, *of);
            if let ResolvedBinding::Card(h) = resolved {
                if ctx.add_to_hand_from_security(p, h) {
                    bindings.record_added_to_hand(h);
                }
            }
            true
        }
        CompiledStep::AddTopSecurityToHand { of } => {
            let p = resolve_player(ctx, *of);
            ctx.add_top_security_to_hand(p);
            true
        }
        CompiledStep::MayAddTopSecurityToHand { of } => {
            let p = resolve_player(ctx, *of);
            install_may_add_top_security_to_hand(ctx, p);
            true
        }
        CompiledStep::AddToHandFromDeck { of, card } => {
            // Phase 2b has no way to bind a deck card (no SelectDeck variant
            // and RevealTopDeck binds into the reveal pool, not deck). The
            // only reachable case is a direct Card(handle) binding, which a
            // future step could set up. Keep behaviour strict: no-op unless
            // we have a concrete CardHandle.
            let Some(resolved) = resolve_binding_ref(card, ctx, bindings) else {
                return true;
            };
            let p = resolve_player(ctx, *of);
            if let ResolvedBinding::Card(h) = resolved {
                if ctx.add_to_hand_from_deck(p, h) {
                    bindings.record_added_to_hand(h);
                }
            }
            true
        }

        // ── Task 5: reveal-pool consumers ────────────────────────────────────
        CompiledStep::AddToHandFromReveal { of, card } => {
            let Some(resolved) = resolve_binding_ref(card, ctx, bindings) else {
                return true;
            };
            let p = resolve_player(ctx, *of);
            if let Some(h) = singleton_card(resolved) {
                if ctx.add_to_hand_from_reveal(p, h) {
                    bindings.record_added_to_hand(h);
                }
            }
            true
        }

        CompiledStep::AddThisOptionToHand => {
            ctx.add_pending_security_to_hand();
            true
        }

        CompiledStep::Recover { of, count } => {
            let p = resolve_player(ctx, *of);
            ctx.recover_from_deck(p, *count);
            true
        }

        CompiledStep::TrashFromReveal { of, card } => {
            let Some(resolved) = resolve_binding_ref(card, ctx, bindings) else {
                return true;
            };
            let p = resolve_player(ctx, *of);
            if let Some(h) = singleton_card(resolved) {
                ctx.trash_from_reveal(p, h);
            }
            true
        }

        CompiledStep::TrashSelectedSources { source_refs } => {
            if let Some(source_refs) = bindings.get_source_refs(source_refs) {
                for source_ref in source_refs {
                    ctx.trash_card_source(source_ref.permanent, source_ref.card);
                }
            }
            true
        }

        // G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME — return each selected
        // digivolution source card to its OWNER's hand (a return, not a
        // trash: fires no `OnDigivolutionCardTrashed`).
        CompiledStep::ReturnSelectedSourcesToHand { source_refs } => {
            if let Some(source_refs) = bindings.get_source_refs(source_refs) {
                for source_ref in source_refs {
                    ctx.return_card_source_to_hand(source_ref.permanent, source_ref.card);
                }
            }
            true
        }

        CompiledStep::PlaySelectedSourcesFree { source_refs } => {
            if let Some(source_refs) = bindings.get_source_refs(source_refs) {
                ctx.play_selected_sources_without_cost(source_refs);
            }
            true
        }

        CompiledStep::ReturnToDeckFromReveal { of, card, position } => {
            let Some(resolved) = resolve_binding_ref(card, ctx, bindings) else {
                return true;
            };
            let p = resolve_player(ctx, *of);
            if let Some(h) = singleton_card(resolved) {
                if ctx.return_to_deck_from_reveal(p, h, super::map_stack_position(*position)) {
                    bindings.record_returned_to_deck(h);
                }
            }
            true
        }

        CompiledStep::RevealTopDeck {
            of,
            count,
            zone,
            bind_as,
        } => {
            let p = resolve_player(ctx, *of);
            let handles = match zone {
                Some(CompiledZone::DigiEggDeck) => ctx.reveal_top_digitama(p, *count),
                _ => ctx.reveal_top_deck(p, *count),
            };
            // Single-card reveal with a bind_as name: expose the card to
            // downstream steps via a named binding.
            if *count == 1 {
                if let Some(name) = bind_as {
                    if let Some(&h) = handles.first() {
                        bindings.insert_card(name, h);
                    }
                }
            } else if let Some(name) = bind_as {
                bindings.insert_card_list(name, handles);
            }
            // Other reveal zones still use the normal deck path until they
            // have card-shaped fixtures.
            true
        }

        CompiledStep::PlaceRemainderOnDeck { of, position } => {
            let p = resolve_player(ctx, *of);
            ctx.place_remainder_on_deck(p, super::map_stack_position(*position));
            true
        }

        CompiledStep::TrashFromHandByIndex { of, hand_index } => {
            let Some(resolved) = resolve_binding_ref(hand_index, ctx, bindings) else {
                return true;
            };
            let p = resolve_player(ctx, *of);
            if let ResolvedBinding::HandIndex(owner, i) = resolved {
                debug_assert_eq!(
                    owner, p,
                    "trash_from_hand_by_index used a binding from a different player than `of`"
                );
                ctx.trash_from_hand_by_index(owner, i as usize);
            }
            true
        }

        CompiledStep::MarkSecurityFaceUp { of, card } => {
            let Some(resolved) = resolve_binding_ref(card, ctx, bindings) else {
                return true;
            };
            let p = resolve_player(ctx, *of);
            if let ResolvedBinding::Card(h) = resolved {
                // `mark_security_face_up` needs a &CardSource, but we only hold
                // a CardHandle. Search the target player's security zone for the
                // matching CardSource and clone it (CardSource: Clone) to avoid
                // aliasing `ctx.game` while calling the &mut method.
                //
                // Direct approach: `mark_security_face_up` sets
                // `player.face_up_security.insert(card.card_index)` and
                // `card_index == h.0` (CardHandle wraps the card_index as u16).
                // We replicate that here to avoid the intermediate clone dance.
                let in_security = ctx
                    .game
                    .player(p)
                    .security
                    .iter()
                    .any(|cs| cs.handle() == h);
                if in_security {
                    ctx.game.player_mut(p).face_up_security.insert(h.0);
                }
            }
            true
        }
        CompiledStep::PlacePermanentOnSecurityObserved {
            of,
            target,
            position,
            face_up,
            include_sources,
        } => {
            let player = resolve_player(ctx, *of);
            if let Some(ResolvedBinding::Permanent(handle)) =
                resolve_binding_ref(target, ctx, bindings)
            {
                let position = super::map_stack_position(*position);
                if *include_sources {
                    let _ = ctx.game.place_permanent_on_security_observed(
                        player, handle, position, *face_up, ctx.player,
                    );
                } else {
                    let _ = ctx.place_permanent_on_security(player, handle, position, *face_up);
                }
            }
            true
        }
        CompiledStep::SecurityPlaceStackedCard {
            carrier,
            source,
            source_index_from_top,
            of,
            position,
            face_up,
        } => {
            let target_player = resolve_player(ctx, *of);
            let position = super::map_stack_position(*position);
            let Some(ResolvedBinding::Permanent(carrier)) =
                resolve_binding_ref(carrier, ctx, bindings)
            else {
                return true;
            };
            let source_card = source
                .as_ref()
                .and_then(|source| match resolve_binding_ref(source, ctx, bindings) {
                    Some(ResolvedBinding::Card(card)) => Some(card),
                    _ => None,
                })
                .or_else(|| source_index_from_top.and_then(|i| source_from_top(ctx, carrier, i)));
            if let Some(source_card) = source_card {
                let _ = ctx.security_place_stacked_card(
                    carrier,
                    source_card,
                    target_player,
                    position,
                    *face_up,
                );
            }
            true
        }
        CompiledStep::SecurityPlaceTopStackedCard {
            carrier,
            of,
            position,
            face_up,
        } => {
            let target_player = resolve_player(ctx, *of);
            if let Some(ResolvedBinding::Permanent(carrier)) =
                resolve_binding_ref(carrier, ctx, bindings)
            {
                let position = super::map_stack_position(*position);
                let _ =
                    ctx.security_place_top_stacked_card(carrier, target_player, position, *face_up);
            }
            true
        }
        CompiledStep::ReturnAllTrashToDeckBottom { of } => {
            let player = resolve_player(ctx, *of);
            // The helper returns every moved card handle; record them in the
            // per-effect result log so a later `returned_card_matching` /
            // `effect_returned_any_card` predicate can inspect this return.
            // G-ANY-RETURNED-CARD-PREDICATE.
            let moved = ctx.return_all_trash_to_deck_bottom(player);
            for h in moved {
                bindings.record_returned_to_deck(h);
            }
            true
        }
        CompiledStep::ReturnTrashListToDeckBottom { of, cards } => {
            // G-ZONE-TRASH-TO-DECK — move exactly the bound card list out of
            // trash to the bottom of the deck, leaving the rest of the trash
            // untouched.
            let player = resolve_player(ctx, *of);
            let handles = match resolve_binding_ref(cards, ctx, bindings) {
                Some(ResolvedBinding::CardList(v)) => v,
                Some(ResolvedBinding::Card(h)) => vec![h],
                _ => Vec::new(),
            };
            let moved = ctx.return_trash_cards_to_deck_bottom(player, &handles);
            for h in moved {
                bindings.record_returned_to_deck(h);
            }
            true
        }
        CompiledStep::MoveTrashCardToDeckTop { of, card } => {
            // G-ZONE-SELECTED-TRASH-TO-DECK-TOP — move exactly the one bound
            // trash card to the TOP of its owner's deck. `of` identifies whose
            // trash currently holds the card; the engine routes the card back
            // to its OWNER's deck.
            let Some(resolved) = resolve_binding_ref(card, ctx, bindings) else {
                return true;
            };
            let player = resolve_player(ctx, *of);
            let (owner, handle) = match resolved {
                ResolvedBinding::TrashIndex(owner, i) => {
                    let handle = ctx
                        .game
                        .player(owner)
                        .trash
                        .get(i as usize)
                        .map(|cs| cs.handle());
                    (owner, handle)
                }
                ResolvedBinding::Card(h) => (player, Some(h)),
                _ => (player, None),
            };
            if let Some(h) = handle {
                if ctx.move_trash_card_to_deck_top(owner, h) {
                    bindings.record_returned_to_deck(h);
                }
            }
            true
        }
        CompiledStep::TrashTopNDigivolutionCardsOfEach { of, n } => {
            let player = resolve_player(ctx, *of);
            let n = formula_to_u8(n, ctx, bindings);
            let _ = ctx.trash_top_n_digivolution_cards_of_each(player, n);
            true
        }
        CompiledStep::TrashOpponentHandToCount {
            opponent,
            target_count,
        } => {
            let opponent = resolve_player(ctx, *opponent);
            let target_count = formula_to_u8(target_count, ctx, bindings);
            let _ = ctx.trash_opponent_hand_to_count(opponent, target_count);
            true
        }
        CompiledStep::SearchOwnSecurityStack {
            filter,
            prompt,
            bind_as,
            optional,
            on_select,
            on_no_match,
        } => {
            install_search_own_security_stack(
                ctx,
                bindings,
                runtime,
                filter,
                prompt,
                bind_as.clone(),
                *optional,
                on_select.clone(),
                on_no_match.clone(),
            );
            true
        }

        _ => false,
    }
}

fn source_from_top(
    ctx: &EffectContext<'_>,
    carrier: PermanentHandle,
    index_from_top: u8,
) -> Option<crate::card_source::CardHandle> {
    let perm = ctx
        .game
        .player(carrier.player)
        .battle_area
        .get(carrier.index as usize)?;
    let top_stacked_offset = 2usize + index_from_top as usize;
    let idx = perm.card_sources.len().checked_sub(top_stacked_offset)?;
    Some(perm.card_sources[idx].handle())
}

fn formula_to_u8(formula: &CompiledFormula, ctx: &EffectContext<'_>, bindings: &Bindings) -> u8 {
    let target = ctx.source_permanent.unwrap_or(PermanentHandle {
        player: ctx.player,
        index: 0,
    });
    formula_eval::evaluate_with_bindings(formula, ctx, target, Some(bindings))
        .clamp(0, u8::MAX as i32) as u8
}

fn security_matches(
    filter: &CompiledPredicate,
    game: &crate::game::Game,
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<PermanentHandle>,
    player: PlayerId,
    card: crate::card_source::CardHandle,
    bindings: &Bindings,
) -> bool {
    let read = EffectReadContext::new(game, source_card, source_permanent, player);
    eval_predicate_with_bindings(filter, &read, PredicateSubject::Card(card), Some(bindings))
}

#[allow(clippy::too_many_arguments)]
fn install_search_own_security_stack(
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    runtime: &StepRuntime,
    filter: &CompiledPredicate,
    prompt: &str,
    bind_as: Option<String>,
    optional: bool,
    on_select: Vec<CompiledStep>,
    on_no_match: Option<Vec<CompiledStep>>,
) {
    let player = ctx.player;
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let filter_for_precheck = filter.clone();
    let has_match = ctx.game.player(player).security.iter().any(|card| {
        security_matches(
            &filter_for_precheck,
            ctx.game,
            source_card,
            source_permanent,
            player,
            card.handle(),
            bindings,
        )
    });

    if !has_match {
        if let Some(no_match) = on_no_match {
            run_steps_with_runtime(&no_match, ctx, bindings, runtime);
        }
        return;
    }

    let filter_for_select = filter.clone();
    let filter_bindings = bindings.clone();
    let tail = Arc::new(on_select);
    let runtime = runtime.clone();
    let callback_bindings = bindings.clone();
    ctx.search_own_security_stack(
        prompt,
        optional,
        move |game, card| {
            security_matches(
                &filter_for_select,
                game,
                source_card,
                source_permanent,
                player,
                card.handle(),
                &filter_bindings,
            )
        },
        move |cb_ctx, idx| {
            let mut b = callback_bindings.clone();
            if let Some(name) = &bind_as {
                if let Some(card) = cb_ctx.game.player(player).security.get(idx) {
                    b.insert_card(name, card.handle());
                }
            }
            run_steps_with_runtime(&tail, cb_ctx, &mut b, &runtime);
        },
    );
}
