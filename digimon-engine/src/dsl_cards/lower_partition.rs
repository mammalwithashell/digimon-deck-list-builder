//! Lower `CompiledDeclarativeClause::Partition` into a declarative engine
//! `Effect` that grants `Keyword::Partition` to the source permanent.
//!
//! Phase 1 scope: grant the `Keyword::Partition` keyword and install a
//! declarative marker. Full source-list enforcement is engine-side
//! (replacement dispatch) and orthogonal to this lowering.
//! `active_when`, `sources`, and `exclude_cause` are accepted but ignored
//! for now.

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope};

use crate::card_source::CardHandle;
use crate::effect::Effect;
use crate::enums::{Expiry, Keyword};

/// Lower a `Partition` declarative clause.
///
/// Returns an `Effect` that grants `Keyword::Partition` to the source
/// permanent when its declarative process fires.
///
/// `_active_when`, `_sources`, and `_exclude_cause` are deferred to a future
/// phase when engine-side replacement dispatch enforces the source list.
pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    _active_when: Option<CompiledPredicate>,
    _sources: Vec<CompiledPredicate>,
    _exclude_cause: Vec<String>,
) -> Effect {
    let mut builder = Effect::declarative(card)
        .name("Partition")
        .process(move |ctx| {
            let Some(handle) = ctx.source_permanent else { return; };
            ctx.grant_keyword(handle, Keyword::Partition, Expiry::Permanent);
        });
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    builder.build()
}
