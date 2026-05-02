//! Lower `CompiledDeclarativeClause::Partition` into engine effects.
//!
//! Phase 1 scope: grant the `Keyword::Partition` keyword and install a
//! declarative marker. Full source-list enforcement is engine-side
//! (replacement dispatch) and orthogonal to this lowering.
//! `active_when` and `sources` are accepted but ignored for now. A configured
//! process body is emitted as an `OnDeletion` body at the partition event.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope, CompiledStep};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::dsl_cards::step::{run_steps_with_runtime, StepRuntime};
use crate::effect::Effect;
use crate::enums::{Expiry, Keyword};

/// Lower a `Partition` declarative clause.
///
/// Returns effects that grant `Keyword::Partition` to the source permanent and,
/// when `process` is present, run that body when the carrier is deleted.
///
/// `_active_when` and `_sources` are deferred to a future phase when
/// engine-side dispatch enforces the source list.
pub fn lower(
    card: CardHandle,
    scope: CompiledScope,
    _active_when: Option<&CompiledPredicate>,
    _sources: &[CompiledPredicate],
    exclude_cause: &[String],
    process: &[CompiledStep],
) -> Vec<Effect> {
    lower_with_raw(
        card,
        scope,
        _active_when,
        _sources,
        exclude_cause,
        process,
        Arc::new(EngineRawRustRegistry::new()),
    )
}

pub fn lower_with_raw(
    card: CardHandle,
    scope: CompiledScope,
    _active_when: Option<&CompiledPredicate>,
    _sources: &[CompiledPredicate],
    exclude_cause: &[String],
    process: &[CompiledStep],
    raw: Arc<EngineRawRustRegistry>,
) -> Vec<Effect> {
    let mut builder = Effect::declarative(card)
        .name("Partition")
        .materializes_declarative_state()
        .process(move |ctx| {
            let Some(handle) = ctx.source_permanent else {
                return;
            };
            ctx.grant_declarative_keyword(handle, Keyword::Partition, Expiry::Permanent);
        });
    if matches!(scope, CompiledScope::Inherited) {
        builder = builder.inherited();
    }
    let mut out = vec![builder.build()];

    if !process.is_empty() {
        let process_arc: Arc<[CompiledStep]> = Arc::from(process);
        let exclude_cause: Arc<[String]> = Arc::from(exclude_cause);
        let runtime = StepRuntime::new(raw);
        let mut process_builder =
            Effect::on_deletion(card)
                .name("Partition process")
                .process(move |ctx| {
                    let cause = ctx.deletion_cause();
                    if cause_is_excluded(cause, &exclude_cause) {
                        return;
                    }
                    let mut bindings = Bindings::new();
                    let _ = run_steps_with_runtime(&process_arc, ctx, &mut bindings, &runtime);
                });
        if matches!(scope, CompiledScope::Inherited) {
            process_builder = process_builder.inherited();
        }
        out.push(process_builder.build());
    }

    out
}

fn cause_is_excluded(
    cause: Option<crate::replacement::ReplacementCause>,
    exclude: &[String],
) -> bool {
    let Some(cause) = cause else {
        return false;
    };
    let normalized = match cause {
        crate::replacement::ReplacementCause::Battle => "battle",
        crate::replacement::ReplacementCause::OwnEffect => "own_effect",
        crate::replacement::ReplacementCause::OpponentEffect => "opponent_effect",
        crate::replacement::ReplacementCause::SecurityCheck => "security_check",
        crate::replacement::ReplacementCause::Cost => "cost",
    };
    exclude.iter().any(|s| s == normalized)
}
