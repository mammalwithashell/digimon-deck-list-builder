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

fn reference(r: &CompiledBindingRef) -> bool {
    named(r)
        || matches!(
            r,
            CompiledBindingRef::Source | CompiledBindingRef::Carrier | CompiledBindingRef::SelfRef
        )
}

fn steps(body: &[CompiledStep]) -> bool {
    body.iter().all(|s| match s {
        CompiledStep::GainMemory(_) => true,
        CompiledStep::Unsuspend { target } => reference(target),
        CompiledStep::AddDpModifier {
            target,
            value: digimon_dsl::compiled::CompiledModifierValue::Literal(_),
            ..
        } => reference(target),
        CompiledStep::DeDigivolve {
            target,
            amount_fn: None,
            ..
        } => reference(target),
        CompiledStep::PerSelected { body, .. } => steps(body),
        CompiledStep::SelectCountCappedMulti {
            of: CompiledPlayerRef::You | CompiledPlayerRef::Opponent,
            zone: digimon_dsl::compiled::CompiledZone::BattleArea,
            max: digimon_dsl::compiled::CompiledCountBound::Literal(_),
            filter,
            ..
        } => predicate(filter),
        CompiledStep::ActivationCost {
            kind: CompiledActivationCostKind::SuspendSelf,
        } => true,
        CompiledStep::SelectOwnPermanent {
            filter,
            selector: None,
            then,
            ..
        } => predicate(filter) && steps(then),
        CompiledStep::SelectOpponentPermanent {
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

/// Conservative direct-reference facts, independent of card IDs.
/// This is NOT permission to erase a captured context: even literal memory or
/// named-target evolution can invoke readers outside this compiled clause.
/// Raw Rust and unrecognized syntax remain unsupported.
pub fn direct_references_unused(clause: &CompiledTriggeredClause) -> bool {
    !clause.when.is_empty()
        && clause.when.iter().all(|t| {
            matches!(
                t,
                CompiledTiming::OnOwnSecurityRemoved | CompiledTiming::OnOpponentSecurityRemoved
            )
        })
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

/// Unconditional projection certificate. Nonempty bodies require transitive
/// reader proof spanning execution and parked work, which is not yet admitted.
/// A direct-reference whitelist cannot grant that proof. Keep the public
/// admission predicate strict while invocation guards remain an inactive
/// prototype; only a context-independent empty body is certified here.
pub fn ignores_event_card_identity(clause: &CompiledTriggeredClause) -> bool {
    clause.process.is_empty() && direct_references_unused(clause)
}
