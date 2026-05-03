use std::sync::Arc;

use digimon_dsl::compiled::{CompiledAltPath, CompiledPredicate, CompiledScope, CompiledTiming};

use crate::card_source::CardHandle;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::timing_map::compiled_timing_to_engine;
use crate::effect::{Effect, EffectBuilder};

pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    active_when: Option<CompiledPredicate>,
    trigger: CompiledTiming,
    applies_to: Option<CompiledPredicate>,
    registers: CompiledAltPath,
) -> Option<Effect> {
    let timing = compiled_timing_to_engine(trigger)?;
    let applies_to = applies_to.map(Arc::new);
    let registers = Arc::new(registers);
    let mut builder = EffectBuilder::new(card, timing).alt_path_registration(applies_to, registers);
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    if let Some(predicate) = active_when {
        builder = builder.condition(move |ctx| {
            let subject = ctx
                .source_permanent
                .map(PredicateSubject::Permanent)
                .unwrap_or(PredicateSubject::None);
            eval_predicate(&predicate, ctx, subject)
        });
    }
    Some(builder.build())
}
