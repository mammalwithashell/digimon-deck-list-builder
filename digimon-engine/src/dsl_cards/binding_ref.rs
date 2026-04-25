//! Resolve `CompiledBindingRef` variants against the current effect
//! context + named bindings.

use digimon_dsl::compiled::CompiledBindingRef;

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::{BindingValue, Bindings};
use crate::effect_context::EffectContext;
use crate::permanent::PermanentHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedBinding {
    Permanent(PermanentHandle),
    Card(CardHandle),
    HandIndex(u16),
    TrashIndex(u16),
    Literal(i64),
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
        CompiledBindingRef::EventTarget | CompiledBindingRef::EventCard => {
            // Phase 2b: engine event context not yet wired to the DSL layer.
            // Returns None so steps relying on these silently no-op.
            None
        }
    }
}

fn resolve_named(name: &str, bindings: &Bindings) -> Option<ResolvedBinding> {
    match bindings.get(name)? {
        BindingValue::Permanent(h) => Some(ResolvedBinding::Permanent(h)),
        BindingValue::Card(h) => Some(ResolvedBinding::Card(h)),
        BindingValue::HandIndex(i) => Some(ResolvedBinding::HandIndex(i)),
        BindingValue::TrashIndex(i) => Some(ResolvedBinding::TrashIndex(i)),
        BindingValue::Literal(v) => Some(ResolvedBinding::Literal(v)),
    }
}
