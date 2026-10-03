//! Compile-time "DP deletion effect" marking (G-ENGINE-DP-DELETION-MAX-MODIFIER).
//!
//! Printed text like "[All Turns] Add 2000 to this Digimon's DP deletion
//! effects' maximums" (EX13-007 Guilmon, EX13-010 Growlmon; BT17-008 et al.
//! memory-gated) raises the DP cap of every *deletion* effect whose source is
//! the carrier permanent — "delete 1 Digimon with 4000 DP or less" becomes
//! 6000 — but NOT the DP cap of a play/suspend/return/bounce effect. DCGO
//! models this by having each card script wrap the cap of its deletion effects
//! in `Player.MaxDP_DeleteEffect(maxDP, activateClass)`, which folds in every
//! active `IChangeDPDeleteEffectMaxDPEffect` (e.g. `ChangeDPDeleteEffectMaxDPClass`).
//!
//! The DSL authors deletions as a target selection followed by a delete step
//! (`select_opponent_permanent { bind_as: victim, filter: { dp_lte: 4000 } }`
//! then `delete_permanent: { target: victim }`), or as a filtered batch
//! (`delete_all_permanents { over: { dp_lte: .. } }`). This pass recovers the
//! "this cap belongs to a deletion effect" fact structurally, so every authored
//! card — not just the ones written after the primitive landed — gets the
//! DCGO-equivalent of `MaxDP_DeleteEffect`:
//!
//! 1. Collect every binding name consumed by a deletion step anywhere in the
//!    clause body (`delete_permanent`, `delete_bound_permanents`,
//!    `delete_permanents`).
//! 2. Every permanent selection whose `bind_as` is in that set has the `dp_lte`
//!    nodes of its filter flagged `dp_lte_deletion_cap` (recursing through
//!    `all_of` / `any_of`, never through `not` — a negated cap is a floor).
//!    `delete_all_permanents.over` and `delete_one_per_opponent_color.filter`
//!    are flagged directly; a deletion-consumed `select_opponent_dp_budget`
//!    gets `deletion_cap` on its total-DP budget (DCGO
//!    `sumDP > MaxDP_DeleteEffect(..)`, e.g. ST7-12 / EX2-011 / BT12-014).
//!
//! The engine adds the effect source permanent's summed
//! `ModifierType::ChangeDPDeleteEffectMaxDP` delta to a flagged cap. With no
//! such modifier on the board the flag is inert, so marking is behaviour-
//! preserving for every card that never meets a deletion-max modifier.

use std::collections::BTreeSet;

use crate::compiled::{
    CompiledAltPath, CompiledBindingRef, CompiledCard, CompiledClause, CompiledDeclarativeClause,
    CompiledPredicate, CompiledStep,
};

/// Run the marking pass over every step body of `card`.
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
    for alt in &mut card.alt_paths {
        mark_alt_path(alt);
    }
}

fn mark_alt_path(alt: &mut CompiledAltPath) {
    mark_body(&mut alt.extra_cost);
    mark_body(&mut alt.on_burst_turn_end);
}

/// Mark one independent step body (a clause's `process`, a cost body, ...).
pub fn mark_body(steps: &mut [CompiledStep]) {
    let mut deleted = BTreeSet::new();
    for step in steps.iter() {
        collect_deleted_bindings(step, &mut deleted);
    }
    for step in steps.iter_mut() {
        mark_step(step, &deleted);
    }
}

fn binding_name(b: &CompiledBindingRef) -> Option<&str> {
    match b {
        CompiledBindingRef::Named(n)
        | CompiledBindingRef::Permanent(n)
        | CompiledBindingRef::Binding(n) => Some(n.as_str()),
        _ => None,
    }
}

/// Visit every directly nested step list of `step`.
fn for_each_child(step: &CompiledStep, f: &mut dyn FnMut(&CompiledStep)) {
    for body in child_bodies(step) {
        for s in body {
            f(s);
        }
    }
}

fn child_bodies(step: &CompiledStep) -> Vec<&Vec<CompiledStep>> {
    use CompiledStep as S;
    match step {
        S::SearchOwnSecurityStack {
            on_select,
            on_no_match,
            ..
        } => {
            let mut v = vec![on_select];
            if let Some(n) = on_no_match {
                v.push(n);
            }
            v
        }
        S::GrantTriggeredEffect { body, .. }
        | S::RepeatEffectChoice { body, .. }
        | S::RepeatEffectChoiceRemaining { body, .. }
        | S::AsSelectingPlayer { body, .. }
        | S::ForEach { body, .. }
        | S::PerSelected { body, .. }
        | S::ScheduleDelayed { body, .. } => vec![body],
        S::SelectOwnPermanent { then, .. }
        | S::SelectOpponentPermanent { then, .. }
        | S::SelectAnyPermanent { then, .. }
        | S::SelectHand { then, .. }
        | S::SelectTrash { then, .. }
        | S::SelectOwnSources { then, .. }
        | S::SelectOpponentSources { then, .. }
        | S::SelectOpponentDpBudget { then, .. }
        | S::SelectOpponentPlayCostBudget { then, .. }
        | S::SelectOwnBreedingPermanent { then, .. }
        | S::SelectReveal { then, .. }
        | S::SelectSecurity { then, .. }
        | S::SelectUnionZone { then, .. } => vec![then],
        S::If {
            then, else_branch, ..
        } => vec![then, else_branch],
        S::Optional(body) => vec![body],
        _ => Vec::new(),
    }
}

fn child_bodies_mut(step: &mut CompiledStep) -> Vec<&mut Vec<CompiledStep>> {
    use CompiledStep as S;
    match step {
        S::SearchOwnSecurityStack {
            on_select,
            on_no_match,
            ..
        } => {
            let mut v = vec![on_select];
            if let Some(n) = on_no_match {
                v.push(n);
            }
            v
        }
        S::GrantTriggeredEffect { body, .. }
        | S::RepeatEffectChoice { body, .. }
        | S::RepeatEffectChoiceRemaining { body, .. }
        | S::AsSelectingPlayer { body, .. }
        | S::ForEach { body, .. }
        | S::PerSelected { body, .. }
        | S::ScheduleDelayed { body, .. } => vec![body],
        S::SelectOwnPermanent { then, .. }
        | S::SelectOpponentPermanent { then, .. }
        | S::SelectAnyPermanent { then, .. }
        | S::SelectHand { then, .. }
        | S::SelectTrash { then, .. }
        | S::SelectOwnSources { then, .. }
        | S::SelectOpponentSources { then, .. }
        | S::SelectOpponentDpBudget { then, .. }
        | S::SelectOpponentPlayCostBudget { then, .. }
        | S::SelectOwnBreedingPermanent { then, .. }
        | S::SelectReveal { then, .. }
        | S::SelectSecurity { then, .. }
        | S::SelectUnionZone { then, .. } => vec![then],
        S::If {
            then, else_branch, ..
        } => vec![then, else_branch],
        S::Optional(body) => vec![body],
        _ => Vec::new(),
    }
}

fn collect_deleted_bindings(step: &CompiledStep, out: &mut BTreeSet<String>) {
    match step {
        CompiledStep::DeletePermanent { target } => {
            if let Some(n) = binding_name(target) {
                out.insert(n.to_string());
            }
        }
        CompiledStep::DeleteBoundPermanents { binding } => {
            out.insert(binding.clone());
        }
        CompiledStep::DeletePermanents { targets } => {
            for t in targets {
                if let Some(n) = binding_name(t) {
                    out.insert(n.to_string());
                }
            }
        }
        _ => {}
    }
    for_each_child(step, &mut |s| collect_deleted_bindings(s, out));
}

fn is_deleted(bind_as: &Option<String>, deleted: &BTreeSet<String>) -> bool {
    bind_as.as_ref().is_some_and(|b| deleted.contains(b))
}

fn mark_step(step: &mut CompiledStep, deleted: &BTreeSet<String>) {
    match step {
        CompiledStep::SelectOwnPermanent {
            filter, bind_as, ..
        }
        | CompiledStep::SelectOpponentPermanent {
            filter, bind_as, ..
        }
        | CompiledStep::SelectAnyPermanent {
            filter, bind_as, ..
        }
        | CompiledStep::SelectCountCappedMulti {
            filter, bind_as, ..
        } => {
            if is_deleted(bind_as, deleted) {
                mark_predicate(filter);
            }
        }
        CompiledStep::SelectOpponentDpBudget {
            filter,
            bind_as,
            deletion_cap,
            ..
        } => {
            if is_deleted(bind_as, deleted) {
                *deletion_cap = true;
                mark_predicate(filter);
            }
        }
        CompiledStep::DeleteAllPermanents { over, .. } => mark_predicate(over),
        CompiledStep::DeleteOnePerOpponentColor {
            filter: Some(filter),
            ..
        } => mark_predicate(filter),
        _ => {}
    }
    for body in child_bodies_mut(step) {
        for s in body.iter_mut() {
            mark_step(s, deleted);
        }
    }
}

/// Flag every positive `dp_lte` in `pred` as a deletion cap. Recurses through
/// `all_of` / `any_of`; a `not` subtree is left alone (`not { dp_lte }` is a
/// DP floor, not a maximum).
pub fn mark_predicate(pred: &mut CompiledPredicate) {
    if pred.dp_lte.is_some() {
        pred.dp_lte_deletion_cap = true;
    }
    for p in pred.all_of.iter_mut().chain(pred.any_of.iter_mut()) {
        mark_predicate(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled::CompiledDpConstraint;

    fn capped(n: i32) -> CompiledPredicate {
        CompiledPredicate {
            dp_lte: Some(CompiledDpConstraint::Literal(n)),
            ..CompiledPredicate::default()
        }
    }

    #[test]
    fn mark_predicate_skips_not_subtree() {
        let mut p = CompiledPredicate {
            all_of: vec![capped(4000)],
            not: Some(Box::new(capped(2000))),
            ..CompiledPredicate::default()
        };
        mark_predicate(&mut p);
        assert!(p.all_of[0].dp_lte_deletion_cap);
        assert!(!p.not.as_ref().unwrap().dp_lte_deletion_cap);
    }
}
