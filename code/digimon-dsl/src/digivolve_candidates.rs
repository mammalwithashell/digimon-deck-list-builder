//! Compile-time "a digivolve candidate must be able to digivolve" marking.
//!
//! An effect that lets a Digimon "digivolve into a [X] card in the hand / trash" is authored as a card selection
//! (`select_hand` / `select_trash` / `select_zone_cards` / `select_reveal` / `select_union_zone`, `bind_as: card`)
//! followed by `effect_initiated_digivolve { target, from_hand: card }`. Unless the effect ignores digivolution
//! requirements, only a card that can actually digivolve onto `target` is a candidate: DCGO filters the selection
//! with `CanPlayCardTargetFrame(target.PermanentFrame, ...)` (e.g. BT24-003 Tsunomon `SelectSourceCard`), so a card
//! that fails the requirements is never offered and an effect with no such card asks nothing. Without this filter
//! the engine offered such a card (a Lv.6 [Shaman] onto a Lv.4) and the digivolve then did nothing: a choice DCGO
//! does not have, found by the DCGO playback of a sim game (2026-11-11).
//!
//! This pass adds `can_digivolve_onto: <target>` to the filter of the selection that binds the digivolve's card,
//! for every card that has not authored it (or `can_digivolve_onto_with_cost`) itself. It is added only when the
//! target is known when the selection runs (the effect's own `source` / `event_target`, or a binding made by an
//! earlier step), so a selection whose target is chosen afterwards is left alone.

use std::collections::{BTreeMap, BTreeSet};

use crate::compiled::{
    CompiledBindingRef, CompiledCard, CompiledClause, CompiledDeclarativeClause, CompiledPredicate, CompiledStep,
};
use crate::deletion_cap::{child_bodies, child_bodies_mut};

/// Run the pass over every step body of `card`.
pub fn mark_card(card: &mut CompiledCard) {
    for clause in &mut card.effects {
        match clause {
            CompiledClause::Triggered(t) => mark_body(&mut t.process),
            CompiledClause::Declarative(d) => match d {
                CompiledDeclarativeClause::CostReduction { pay_cost, .. } => mark_body(pay_cost),
                CompiledDeclarativeClause::Replacement { process, .. }
                | CompiledDeclarativeClause::Partition { process, .. }
                | CompiledDeclarativeClause::Delay { process, .. } => mark_body(process),
                _ => {}
            },
        }
    }
}

/// The target a digivolve names, as the string `can_digivolve_onto` evaluates.
fn target_name(b: &CompiledBindingRef) -> Option<String> {
    match b {
        CompiledBindingRef::Named(n) | CompiledBindingRef::Permanent(n) | CompiledBindingRef::Binding(n) => {
            Some(n.clone())
        }
        CompiledBindingRef::Source | CompiledBindingRef::SelfRef | CompiledBindingRef::Carrier => {
            Some("source".into())
        }
        CompiledBindingRef::EventTarget => Some("event_target".into()),
        _ => None,
    }
}

fn card_binding(b: &CompiledBindingRef) -> Option<&str> {
    match b {
        CompiledBindingRef::Named(n) | CompiledBindingRef::Binding(n) => Some(n.as_str()),
        _ => None,
    }
}

/// card binding -> digivolve target, for every requirement-honouring effect digivolve under `step`.
fn collect(step: &CompiledStep, out: &mut BTreeMap<String, String>) {
    if let CompiledStep::EffectInitiatedDigivolve { target, from_hand, ignore_requirements, ignore_level, .. } = step
    {
        if !*ignore_requirements && !*ignore_level {
            if let (Some(card), Some(t)) = (card_binding(from_hand), target_name(target)) {
                out.insert(card.to_string(), t);
            }
        }
    }
    for body in child_bodies(step) {
        for s in body {
            collect(s, out);
        }
    }
}

fn mark_body(steps: &mut [CompiledStep]) {
    let mut wanted = BTreeMap::new();
    for s in steps.iter() {
        collect(s, &mut wanted);
    }
    if wanted.is_empty() {
        return;
    }
    let mut bound = BTreeSet::new();
    mark_seq(steps, &wanted, &mut bound);
}

fn bind_of(step: &CompiledStep) -> Option<&String> {
    use CompiledStep as S;
    match step {
        S::SelectHand { bind_as, .. }
        | S::SelectTrash { bind_as, .. }
        | S::SelectZoneCards { bind_as, .. }
        | S::SelectReveal { bind_as, .. }
        | S::SelectUnionZone { bind_as, .. }
        | S::SelectOwnPermanent { bind_as, .. }
        | S::SelectOpponentPermanent { bind_as, .. }
        | S::SelectAnyPermanent { bind_as, .. } => bind_as.as_ref(),
        _ => None,
    }
}

fn mark_seq(steps: &mut [CompiledStep], wanted: &BTreeMap<String, String>, bound: &mut BTreeSet<String>) {
    for step in steps.iter_mut() {
        if let Some(b) = bind_of(step).cloned() {
            if let Some(target) = wanted.get(&b) {
                let known = target == "source" || target == "event_target" || bound.contains(target);
                if known {
                    if let Some(filter) = card_filter(step) {
                        add_onto(filter, target);
                    }
                }
            }
        }
        // a nested body sees what is bound so far, and the selection's own binding inside its `then`
        let mut inner = bound.clone();
        if let Some(b) = bind_of(step) {
            inner.insert(b.clone());
        }
        for body in child_bodies_mut(step) {
            let mut scoped = inner.clone();
            mark_seq(body, wanted, &mut scoped);
        }
        if let Some(b) = bind_of(step) {
            bound.insert(b.clone());
        }
    }
}

fn card_filter(step: &mut CompiledStep) -> Option<&mut CompiledPredicate> {
    use CompiledStep as S;
    match step {
        S::SelectHand { filter, .. }
        | S::SelectTrash { filter, .. }
        | S::SelectZoneCards { filter, .. }
        | S::SelectReveal { filter, .. }
        | S::SelectUnionZone { filter, .. } => Some(filter),
        _ => None,
    }
}

fn add_onto(filter: &mut CompiledPredicate, target: &str) {
    if filter.can_digivolve_onto.is_none() && filter.can_digivolve_onto_with_cost.is_none() {
        filter.can_digivolve_onto = Some(target.to_string());
    }
}
