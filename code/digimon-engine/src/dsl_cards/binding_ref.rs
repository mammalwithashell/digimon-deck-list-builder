//! Resolve `CompiledBindingRef` variants against the current effect
//! context + named bindings.

use digimon_dsl::compiled::CompiledBindingRef;

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::{BindingValue, Bindings};
use crate::effect_context::EffectContext;
use crate::enums::PlayerId;
use crate::permanent::PermanentHandle;
use crate::selection::SourceSelectionRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedBinding {
    Permanent(PermanentHandle),
    Card(CardHandle),
    HandIndex(PlayerId, u16),
    TrashIndex(PlayerId, u16),
    Literal(i64),
    PermanentList(Vec<PermanentHandle>),
    CardList(Vec<CardHandle>),
    SourceRefs(Vec<SourceSelectionRef>),
}

pub fn resolve_binding_ref(
    r: &CompiledBindingRef,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
) -> Option<ResolvedBinding> {
    match r {
        CompiledBindingRef::SelfRef => Some(ResolvedBinding::Card(ctx.source_card)),
        CompiledBindingRef::Source | CompiledBindingRef::Carrier => {
            resolve_source_permanent(ctx).map(ResolvedBinding::Permanent)
        }
        CompiledBindingRef::Named(name)
        | CompiledBindingRef::Binding(name)
        | CompiledBindingRef::Permanent(name)
        | CompiledBindingRef::OfPermanent(name) => resolve_named(name, bindings),
        CompiledBindingRef::EventTarget => {
            ctx.game.current_trigger_context.as_ref().and_then(|t| {
                if let Some(snapshot) = t.deleted_object.as_ref() {
                    return Some(ResolvedBinding::Card(snapshot.top_card));
                }
                if let Some(handle) = t.event_permanent {
                    if live_event_permanent(ctx, handle, t.event_card).is_some() {
                        return Some(ResolvedBinding::Permanent(handle));
                    }
                    return t.event_card.map(ResolvedBinding::Card);
                }

                t.target_permanent
                    .map(ResolvedBinding::Permanent)
                    .or_else(|| t.target_card.or(t.event_card).map(ResolvedBinding::Card))
            })
        }
        CompiledBindingRef::EventCard => ctx
            .game
            .current_trigger_context
            .as_ref()
            .and_then(|t| {
                t.deleted_object
                    .as_ref()
                    .map(|snapshot| snapshot.top_card)
                    .or(t.event_card)
                    .or(t.target_card)
            })
            .map(ResolvedBinding::Card),
        // `DeckTop` is a card-source-only binding: it has no `ResolvedBinding`
        // (permanent / card-handle) form. Card-source steps resolve it
        // directly via `resolve_card_source_ref`; here it resolves to nothing.
        CompiledBindingRef::DeckTop(_) => None,
    }
}

/// Resolve the live `PermanentHandle` carrying the effect's source card.
///
/// PUPPETS-G018 — `ctx.source_permanent` caches a `PermanentHandle { player,
/// index }`, but the index is a battle-area position that shifts whenever a
/// lower-indexed permanent leaves play (`Player::delete_permanent` does
/// `battle_area.remove(index)`). When an effect deletes one of its own
/// permanents mid-body as a cost (EX9-032 Karakurumon), the cached index can
/// point at the wrong slot — or out of range — by the time a later step
/// (`effect_initiated_digivolve { target: source }`) resolves the `Source`
/// ref.
///
/// `CardHandle` (the source card's `card_index`) is a stable identity: it is
/// allocated once at registration and never recycled. We use the cached
/// handle as a fast path when it still carries the source card, and otherwise
/// re-locate the live permanent by `card_index` via the existing
/// `ProvenanceToken` machinery (`Game::resolve_provenance_token`), which
/// scans every battle area's `card_sources` for the matching card.
fn resolve_source_permanent(ctx: &EffectContext<'_>) -> Option<PermanentHandle> {
    let cached = ctx.source_permanent?;
    if cached_handle_carries_source(ctx, cached) {
        return Some(cached);
    }
    // The cached index is stale (a mid-body delete shifted the battle area).
    // Re-locate the carrier by the source card's stable `card_index`.
    let token = crate::trigger_context::ProvenanceToken::from(ctx.source_card);
    match ctx.game.resolve_provenance_token(token) {
        Some(crate::trigger_context::EventSubject::Permanent(handle)) => Some(handle),
        // The source card is no longer in a battle-area permanent (it left
        // play entirely). Nothing live to bind — fall back to the cached
        // handle so downstream out-of-range checks reject it explicitly
        // rather than silently re-targeting some unrelated slot.
        _ => Some(cached),
    }
}

/// True when the permanent at `handle` currently contains the effect's source
/// card anywhere in its stack. Checks the whole stack (not just the top card)
/// so the identity holds even after the source carrier digivolves.
fn cached_handle_carries_source(ctx: &EffectContext<'_>, handle: PermanentHandle) -> bool {
    let player = ctx.game.player(handle.player);
    let perm = if handle.index == crate::action::space::BREEDING_TARGET as u8 {
        player.breeding_area.as_ref()
    } else {
        player.battle_area.get(handle.index as usize)
    };
    perm.is_some_and(|perm| {
        perm.card_sources
            .iter()
            .any(|source| source.handle() == ctx.source_card)
    })
}

fn live_event_permanent(
    ctx: &EffectContext<'_>,
    handle: PermanentHandle,
    expected: Option<CardHandle>,
) -> Option<PermanentHandle> {
    let card = ctx
        .game
        .player(handle.player)
        .battle_area
        .get(handle.index as usize)
        .map(|perm| perm.top_card().handle())?;
    match expected {
        Some(expected) if card != expected => None,
        _ => Some(handle),
    }
}

pub(crate) fn resolve_named(name: &str, bindings: &Bindings) -> Option<ResolvedBinding> {
    match bindings.get(name)? {
        BindingValue::Permanent(h) => Some(ResolvedBinding::Permanent(h)),
        BindingValue::Card(h) => Some(ResolvedBinding::Card(h)),
        BindingValue::HandIndex(p, i) => Some(ResolvedBinding::HandIndex(p, i)),
        BindingValue::TrashIndex(p, i) => Some(ResolvedBinding::TrashIndex(p, i)),
        BindingValue::Literal(v) => Some(ResolvedBinding::Literal(v)),
        BindingValue::PermanentList(v) => Some(ResolvedBinding::PermanentList(v)),
        BindingValue::CardList(v) => Some(ResolvedBinding::CardList(v)),
        BindingValue::SourceRefs(v) => Some(ResolvedBinding::SourceRefs(v)),
        // Surface a breeding permanent binding as a sentinel `PermanentHandle`
        // (`index = BREEDING_TARGET`). Engine APIs that accept
        // `PermanentHandle` and operate on breeding permanents already
        // recognize this sentinel (e.g. `place_as_bottom_source_observed`,
        // `effective_dp`). Consumers that mean only "battle-area permanent"
        // already filter by `h.index != BREEDING_TARGET as u8`. This unblocks
        // `place_as_bottom_source: { target: <breeding-binding> }`.
        BindingValue::BreedingPermanentRef(r) => {
            Some(ResolvedBinding::Permanent(PermanentHandle {
                player: r.player,
                index: crate::action::space::BREEDING_TARGET as u8,
            }))
        }
        // A union-zone pick surfaces to generic handle-consuming steps as a
        // plain `Card` (just the handle). `play_union_bound_free` reads the
        // binding directly via `Bindings::get_union_card` when it needs the
        // origin zone — the origin is intentionally not modeled in
        // `ResolvedBinding`, which has no zone-tagged card variant.
        BindingValue::UnionCard { card, .. } => Some(ResolvedBinding::Card(card)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl_cards::bindings::Bindings;
    use crate::permanent::PermanentHandle;

    #[test]
    fn resolves_permanent_list() {
        let mut b = Bindings::new();
        let h = PermanentHandle {
            player: 0,
            index: 0,
        };
        b.insert_permanent_list("xs", vec![h]);

        let r = resolve_named("xs", &b).expect("named binding");
        match r {
            ResolvedBinding::PermanentList(v) => assert_eq!(v, vec![h]),
            other => panic!("expected PermanentList, got {other:?}"),
        }
    }

    #[test]
    fn resolves_card_list() {
        let mut b = Bindings::new();
        let c = CardHandle(7);
        b.insert_card_list("picks", vec![c]);

        let r = resolve_named("picks", &b).expect("named binding");
        match r {
            ResolvedBinding::CardList(v) => assert_eq!(v, vec![c]),
            other => panic!("expected CardList, got {other:?}"),
        }
    }
}
