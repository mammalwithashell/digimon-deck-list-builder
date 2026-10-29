//! Conservative proof that a security-removal observer never reads event-card
//! identity. Unrecognized predicates, steps and binding kinds remain strict.
use digimon_dsl::compiled::{
    CompiledActivationCostKind, CompiledBindingRef, CompiledPlayerRef, CompiledPredicate,
    CompiledStep, CompiledTiming, CompiledTriggeredClause,
};

fn predicate(p: &CompiledPredicate) -> bool {
    let mut rest = p.clone();
    rest.kind = None;
    rest.name_contains = None;
    rest.zone.clear();
    rest.source_is_unsuspended = None;
    rest.all_turns = None;
    rest.your_turn = None;
    rest.opponents_turn = None;
    let children = std::mem::take(&mut rest.all_of)
        .into_iter()
        .chain(std::mem::take(&mut rest.any_of))
        .chain(rest.not.take().map(|p| *p));
    children.into_iter().all(|p| predicate(&p)) && rest == CompiledPredicate::default()
}

fn named(r: &CompiledBindingRef) -> bool {
    matches!(
        r,
        CompiledBindingRef::Named(_) | CompiledBindingRef::Binding(_)
    )
}

fn steps(body: &[CompiledStep]) -> bool {
    body.iter().all(|s| match s {
        CompiledStep::GainMemory(_) => true,
        CompiledStep::ActivationCost {
            kind: CompiledActivationCostKind::SuspendSelf,
        } => true,
        CompiledStep::SelectOwnPermanent {
            filter,
            selector: None,
            then,
            ..
        } => predicate(filter) && steps(then),
        CompiledStep::SelectHand {
            of: CompiledPlayerRef::You,
            filter,
            then,
            ..
        } => predicate(filter) && steps(then),
        CompiledStep::EffectInitiatedDigivolve {
            target,
            from_hand,
            cost,
            ..
        } => {
            named(target)
                && named(from_hand)
                && !matches!(cost, digimon_dsl::compiled::CompiledCostDelta::ReduceFn(_))
        }
        _ => false,
    })
}

/// A deliberately small, extensible proof surface, independent of card IDs.
/// Timing/owner/cause information stays intact; only unused card references
/// can be erased from a hypothetical queued context. Raw Rust is never certified.
pub fn ignores_event_card_identity(clause: &CompiledTriggeredClause) -> bool {
    !clause.when.is_empty()
        && clause
            .when
            .iter()
            .all(|t| matches!(t, CompiledTiming::OnOwnSecurityRemoved))
        && clause
            .active_when
            .iter()
            .chain(&clause.condition)
            .chain(&clause.trigger_if)
            .all(predicate)
        && clause
            .timing_conditions
            .iter()
            .all(|c| predicate(&c.condition))
        && steps(&clause.process)
}
