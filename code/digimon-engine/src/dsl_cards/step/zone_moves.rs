//! Binding-consuming zone-move step lowering. Phase 2b covers
//! `AddToHandFromDeck`, `AddToHandFromTrash`, and the reveal-pool / security-mark
//! family added in Task 5.

use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::resolve_player;
use crate::effect_context::EffectContext;

fn singleton_card(resolved: ResolvedBinding) -> Option<crate::card_source::CardHandle> {
    match resolved {
        ResolvedBinding::Card(h) => Some(h),
        ResolvedBinding::CardList(cards) if cards.len() == 1 => cards.first().copied(),
        _ => None,
    }
}

/// Returns `true` if `step` is a zone-move family handled here. Unknown
/// steps fall through (the caller may try other families).
///
/// Changed in Task 5: third parameter is now `&mut Bindings` so that
/// `RevealTopDeck` can produce a named binding for the revealed card.
pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &mut Bindings) -> bool {
    match step {
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
                ctx.add_to_hand_from_trash(owner, h);
            }
            true
        }
        CompiledStep::AddToHandFromSecurity { of, card } => {
            let Some(resolved) = resolve_binding_ref(card, ctx, bindings) else {
                return true;
            };
            let p = resolve_player(ctx, *of);
            if let ResolvedBinding::Card(h) = resolved {
                let _ = ctx.add_to_hand_from_security(p, h);
            }
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
                let _ = ctx.add_to_hand_from_deck(p, h);
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
                ctx.add_to_hand_from_reveal(p, h);
            }
            true
        }

        CompiledStep::AddThisOptionToHand => {
            ctx.add_pending_security_to_hand();
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
                ctx.return_to_deck_from_reveal(p, h, super::map_stack_position(*position));
            }
            true
        }

        CompiledStep::RevealTopDeck {
            of,
            count,
            zone: _,
            bind_as,
        } => {
            let p = resolve_player(ctx, *of);
            let handles = ctx.reveal_top_deck(p, *count);
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
            // The `zone` field selects which zone is revealed; the engine's
            // `reveal_top_deck` always reveals from the deck — full zone routing
            // is Phase 2c scope.
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

        _ => false,
    }
}
