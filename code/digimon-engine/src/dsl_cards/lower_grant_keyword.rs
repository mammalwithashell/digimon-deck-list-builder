//! Lower whole-card `grant_keyword` declarative clauses (e.g. AD1-025
//! Omnimon grants itself Raid) into a declarative `Effect` that installs
//! a permanent keyword modifier on the source permanent.

use digimon_dsl::compiled::CompiledScope;

use crate::card_source::CardHandle;
use crate::dsl_cards::modifier_map::lookup_keyword;
use crate::effect::{Effect, EffectBuilder};
use crate::enums::Expiry;

pub fn lower(
    card: CardHandle,
    keyword_name: &str,
    value: Option<i32>,
    scope: CompiledScope,
) -> Option<Effect> {
    let kw = lookup_keyword(keyword_name, value)?;
    let label = format!("Grant {keyword_name}");

    let mut builder: EffectBuilder = Effect::declarative(card)
        .name(&label)
        .granted_keyword(kw)
        .process(move |ctx| {
            let Some(handle) = ctx.source_permanent else {
                return;
            };
            ctx.grant_keyword(handle, kw, Expiry::Permanent);
        });

    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }

    Some(builder.build())
}
