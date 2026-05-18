//! Resolve `CompiledBindingRef` variants against the current effect
//! context + named bindings.

use digimon_dsl::compiled::CompiledBindingRef;

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::{BindingValue, Bindings};
use crate::effect_context::EffectContext;
use crate::enums::PlayerId;
use crate::permanent::PermanentHandle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedBinding {
    Permanent(PermanentHandle),
    Card(CardHandle),
    HandIndex(PlayerId, u16),
    TrashIndex(PlayerId, u16),
    Literal(i64),
    PermanentList(Vec<PermanentHandle>),
    CardList(Vec<CardHandle>),
}

pub fn resolve_binding_ref(
    r: &CompiledBindingRef,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
) -> Option<ResolvedBinding> {
    match r {
        CompiledBindingRef::SelfRef => Some(ResolvedBinding::Card(ctx.source_card)),
        CompiledBindingRef::Source | CompiledBindingRef::Carrier => {
            ctx.source_permanent.map(ResolvedBinding::Permanent)
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
    }
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
        BindingValue::SourceRefs(_) => None,
        // Surface a breeding permanent binding as a sentinel `PermanentHandle`
        // (`index = BREEDING_TARGET`). Engine APIs that accept
        // `PermanentHandle` and operate on breeding permanents already
        // recognize this sentinel (e.g. `place_as_bottom_source_observed`,
        // `effective_dp`). Consumers that mean only "battle-area permanent"
        // already filter by `h.index != BREEDING_TARGET as u8`. This unblocks
        // `place_as_bottom_source: { target: <breeding-binding> }`.
        BindingValue::BreedingPermanentRef(r) => Some(ResolvedBinding::Permanent(PermanentHandle {
            player: r.player,
            index: crate::action::space::BREEDING_TARGET as u8,
        })),
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
