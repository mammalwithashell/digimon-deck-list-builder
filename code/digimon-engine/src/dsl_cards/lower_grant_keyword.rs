//! Lower whole-card `grant_keyword` declarative clauses (e.g. AD1-025
//! Omnimon grants itself Raid) into a declarative `Effect` that installs
//! a permanent keyword modifier on the source permanent.

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope};

use crate::card_source::CardHandle;
use crate::dsl_cards::modifier_map::lookup_keyword;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::effect::{Effect, EffectBuilder};
use crate::enums::{Expiry, Keyword};

pub fn lower(
    card: CardHandle,
    keyword_name: &str,
    value: Option<i32>,
    scope: CompiledScope,
    overclock_cost_filter: Option<CompiledPredicate>,
) -> Option<Effect> {
    let kw = lookup_keyword(keyword_name, value)?;
    let label = format!("Grant {keyword_name}");

    let mut builder: EffectBuilder = Effect::declarative(card)
        .name(&label)
        .granted_keyword(kw)
        .materializes_declarative_state()
        .process(move |ctx| {
            let Some(handle) = ctx.source_permanent else {
                return;
            };
            ctx.grant_declarative_keyword(handle, kw, Expiry::Permanent);
        });

    if kw == Keyword::Overclock {
        if let Some(filter) = overclock_cost_filter {
            builder = builder.overclock_with_cost_filter(move |rctx, handle| {
                eval_predicate(&filter, rctx, PredicateSubject::Permanent(handle))
            });
        }
    }

    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }

    Some(builder.build())
}
