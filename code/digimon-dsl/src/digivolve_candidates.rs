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
//! for every card that has not authored a candidate leaf itself. It is added only when the target is known when the
//! selection runs (the effect's own `source` / `event_target`, or a binding made by an earlier step), so a selection
//! whose target is chosen afterwards is left alone.
//!
//! An `ignore_level` digivolve ("digivolve into [X] ... ignoring level") waives ONLY the level: DCGO filters with
//! `CanPlayCardTargetFrame(target, true, ..., IgnoreRequirement.Level)` (EX13_071.cs:357-361; ruling Q7449). The
//! route-only `can_digivolve_onto` honours the level, so such a pick gets
//! `can_digivolve_onto_with_cost { target, cost: <the step's cost>, ignore_level: true }` instead: the level-waived
//! routes, paid at the cost the digivolve actually charges (CR 4.3 1-3-11-1). Inside a `cost_reduction` clause's
//! `pay_cost` body the payment probe would evaluate the reducer that is being asked about (the validator forbids
//! the leaf there for the same reason), so there the pick gets the route-only `can_digivolve_onto_ignoring_level`.
//! G-DSL-CAN-DIGIVOLVE-ONTO-IGNORE-LEVEL, G-DSL-CAN-DIGIVOLVE-ONTO-IGNORE-LEVEL-ROUTE-ONLY.
//!
//! A candidate leaf the card authored is kept, so it must agree with the digivolve's waiver: a level-honouring leaf
//! on an `ignore_level` pick re-imposes the level (it excludes cards DCGO offers), and a level-waiving leaf on a
//! normal pick offers cards the digivolve refuses. Each is a compile error, as is the same disagreement in a
//! `has_digivolve_candidate` leaf on the pick that binds the digivolve's target
//! (G-DSL-HAS-DIGIVOLVE-CANDIDATE-IGNORE-LEVEL). A FREE `ignore_requirements` step is not checked: the compiled flag
//! also comes from sugar (a replacement's `digivolve_without_cost`, always `cost: free`, BT17-097) whose authored
//! route leaf is the printed requirement check. A PAID one (EX13-074, P-092: "for a digivolution cost of 3, ignoring
//! digivolution requirements") is always authored as such, so any leaf on its pick that keeps a requirement
//! (`can_digivolve_onto`, `can_digivolve_from_source`, `has_digivolve_candidate`, a cost leaf without
//! `ignore_requirements`) is an error. A requirement-waiving cost leaf (`ignore_requirements: true`) on a pick whose
//! digivolve keeps any requirement offers cards the digivolve refuses, and is an error.
//!
//! Needs are keyed by binding name, so a binding that feeds two digivolves needing different checks (target, cost
//! or waiver) is a compile error: one pick would otherwise check the other digivolve's payment.
//!
//! A PAID digivolve (any cost but free / a fixed 0) also needs the payment checked: a card whose cost can't be paid
//! can't be digivolved (CR 4.3 1-3-11-1; 8-1-2-6: it goes back and memory doesn't move; 4-2-2 / 1-4-2-2: the gauge
//! stops at 10), so offering it is a dead choice, reachable whenever memory is low (an [Opponent's Turn] trigger).
//! DCGO passes `payCost: true` for these (`DigivolveIntoHandOrTrashCard`, CardEffectCommons.cs:820-878, e.g.
//! AD1_011.cs:141-147, EX13_074.cs:234-240), but `CardSource.CanPlayCardTargetFrame` honours `PayCost` only for an
//! empty frame (CardSource.cs:1150-1161); the rules manual governs. So where the payment probe is allowed, a paid
//! digivolve's pick gets `can_digivolve_onto_with_cost { target, cost: <the step's cost> }` with the step's waiver
//! (`ignore_level`, or `ignore_requirements` for a fixed-cost "ignoring digivolution requirements" digivolve,
//! EX13-074 / P-092), unless it authored a cost leaf itself. A route-only leaf the card authored stays, in
//! conjunction. G-DSL-PAID-DIGIVOLVE-PICK-PAYMENT.
//!
//! The HOST pick of a paid digivolve ("1 of your Digimon may digivolve into a card in the hand ...") filters its
//! hosts with an authored `has_digivolve_candidate { zone, filter }`. That leaf is route-only unless it carries a
//! `cost`, so a host whose only candidate can't be paid was offered and the card pick that followed was empty. The
//! pass gives the authored leaf on the pick that binds a paid digivolve's target the step's `cost` (unless it
//! authored one), exactly as the card pick gets its cost leaf. An activation gate (a clause's `condition` /
//! `active_when`) that counts route-only candidates onto the effect's own `source` (`can_digivolve_onto: source` /
//! `can_digivolve_from_source: true` in a `count_gte` filter, or a `has_digivolve_candidate` on the `is_source`
//! permanent) gets the same check, so the effect is not offered with nothing payable. Free digivolves keep route-only
//! gates. DCGO's gates check the route alone (`CanPlayCardTargetFrame` honours `PayCost` only for an empty frame,
//! CardSource.cs:1150-1161); CR 4.3 1-3-11-1 governs. G-DSL-PAID-DIGIVOLVE-HOST-PICK-ROUTE-ONLY.
//!
//! `has_digivolve_candidate { ignore_requirements: true }` is the host-pick leaf of an `ignore_requirements`
//! digivolve ("1 of your [Gabumon] may digivolve into [MetalGarurumon] in the hand, ignoring digivolution
//! requirements", BT22-013; DCGO `CanPlayCardTargetFrame(.., IgnoreRequirement.All)`, BT22_013.cs:145-150): it
//! waives the route and keeps the prohibitions. It must agree with the digivolve's waiver like every other leaf.
//! G-DSL-HAS-DIGIVOLVE-CANDIDATE-IGNORE-REQUIREMENTS.

use std::collections::{BTreeMap, BTreeSet};

use crate::compiled::{
    CompiledBindingRef, CompiledCard, CompiledClause, CompiledCostDelta, CompiledDeclarativeClause,
    CompiledDigivolveCostPredicate, CompiledPredicate, CompiledStep,
};
use crate::deletion_cap::{child_bodies, child_bodies_mut};
use crate::errors::ValidationError;

/// Run the pass over every step body of `card`. Returns the authored candidate leaves that disagree with their
/// digivolve's waiver.
pub fn mark_card(card: &mut CompiledCard) -> Vec<ValidationError> {
    let mut ctx = Ctx { card_id: card.card.clone(), cost_probe_allowed: true, errors: Vec::new() };
    for (i, clause) in card.effects.iter_mut().enumerate() {
        let path = format!("effects[{i}]");
        match clause {
            CompiledClause::Triggered(t) => {
                let mut gates: Vec<&mut CompiledPredicate> =
                    t.condition.iter_mut().chain(t.active_when.iter_mut()).collect();
                ctx.mark_body(&mut t.process, true, &mut gates, &format!("{path}.process"))
            }
            CompiledClause::Declarative(d) => match d {
                CompiledDeclarativeClause::CostReduction { pay_cost, .. } => {
                    ctx.mark_body(pay_cost, false, &mut [], &format!("{path}.pay_cost"))
                }
                CompiledDeclarativeClause::Replacement { process, active_when, .. }
                | CompiledDeclarativeClause::Partition { process, active_when, .. }
                | CompiledDeclarativeClause::Delay { process, active_when, .. } => {
                    let mut gates: Vec<&mut CompiledPredicate> = active_when.iter_mut().collect();
                    ctx.mark_body(process, true, &mut gates, &format!("{path}.process"))
                }
                _ => {}
            },
        }
    }
    ctx.errors
}

/// Which digivolution requirements an `effect_initiated_digivolve` waives.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Waiver {
    None,
    Level,
    All,
}

impl Waiver {
    fn of(ignore_requirements: bool, ignore_level: bool) -> Self {
        if ignore_requirements {
            Self::All
        } else if ignore_level {
            Self::Level
        } else {
            Self::None
        }
    }

    /// Why a candidate leaf waiving `leaf` disagrees with this waiver on a digivolve that is `paid` or free, if it
    /// does. A FREE requirement-ignoring step is not checked: the compiled `ignore_requirements` also comes from
    /// sugar such as a replacement's `digivolve_without_cost` (always `cost: free`), whose printed text keeps the
    /// requirements and whose authored route leaf IS the requirement check (BT17-097's [Imperialdramon] <Delay>). A
    /// PAID one is always authored as "for a digivolution cost of N, ignoring digivolution requirements" (EX13-074,
    /// P-092), so a leaf that keeps any requirement excludes cards it accepts.
    fn disagreement(self, leaf: Waiver, paid: bool) -> Option<&'static str> {
        if self == Self::All {
            return (paid && leaf != Self::All).then_some(
                "the paid digivolve ignores every digivolution requirement (`ignore_requirements: true`), so the \
                 candidate leaf must waive them too (`can_digivolve_onto_with_cost { ..., ignore_requirements: true \
                 }`): a leaf that keeps any requirement (`can_digivolve_onto`, `can_digivolve_from_source`, \
                 `has_digivolve_candidate`, a cost leaf without `ignore_requirements`) excludes cards the digivolve \
                 accepts",
            );
        }
        let leaf_waives_level = leaf != Self::None;
        if leaf == Self::All {
            return Some(
                "the candidate leaf waives every digivolution requirement (`ignore_requirements: true`) but the \
                 digivolve keeps them, so it offers cards the digivolve refuses",
            );
        }
        match (self, leaf_waives_level) {
            (Self::Level, false) => Some(
                "the digivolve has `ignore_level: true`, so the candidate leaf must waive only the level too \
                 (`can_digivolve_onto_with_cost { ..., ignore_level: true }`, `can_digivolve_onto_ignoring_level` \
                 inside a cost_reduction, or `has_digivolve_candidate { ..., ignore_level: true }`): a \
                 level-honouring leaf excludes cards the digivolve accepts (DCGO IgnoreRequirement.Level, ruling \
                 Q7449)",
            ),
            (Self::None, true) => Some(
                "the digivolve honours the level (no `ignore_level`), so the candidate leaf must too: a \
                 level-waiving leaf (`ignore_level: true` / `can_digivolve_onto_ignoring_level`) offers cards the \
                 digivolve refuses",
            ),
            _ => None,
        }
    }
}

/// What the selection that binds a digivolve's card must be able to do.
#[derive(Clone, PartialEq)]
struct Wanted {
    /// The digivolve's target, as the string the leaf evaluates.
    target: String,
    waiver: Waiver,
    /// The digivolve step's own cost delta, for the payment-checking cost leaf.
    cost: CompiledCostDelta,
}

impl Wanted {
    /// The digivolve charges memory, so its payment can fail. Free ("without paying the cost", DCGO `payCost:
    /// false`) and a fixed 0 are always payable.
    fn paid(&self) -> bool {
        is_paid(&self.cost)
    }
}

fn is_paid(cost: &CompiledCostDelta) -> bool {
    !matches!(cost, CompiledCostDelta::Free | CompiledCostDelta::Literal(0))
}

/// What the pick that binds a digivolve's HOST must check.
#[derive(Clone, PartialEq)]
struct HostNeed {
    waiver: Waiver,
    /// The digivolve step's own cost delta, for the host leaf's payment check.
    cost: CompiledCostDelta,
}

impl HostNeed {
    fn paid(&self) -> bool {
        is_paid(&self.cost)
    }
}

#[derive(Default)]
struct Collected {
    /// card binding -> what its pick must check.
    cards: BTreeMap<String, Wanted>,
    /// target (host) binding -> what the pick that binds it must check.
    hosts: BTreeMap<String, HostNeed>,
    /// target name (`source`, `event_target` or a binding) -> the distinct PAID digivolves onto it, for the activation
    /// gates that probe the same target before the body runs.
    paid_onto: BTreeMap<String, Vec<(Waiver, CompiledCostDelta)>>,
    /// Bindings that feed digivolves needing different checks. The pass keys by binding name, so each pick would
    /// get one digivolve's target / cost / waiver; that is a compile error instead.
    conflicts: BTreeSet<String>,
}

impl Collected {
    fn want_card(&mut self, binding: &str, w: Wanted) {
        match self.cards.get(binding) {
            Some(prev) if *prev != w => {
                self.conflicts.insert(binding.to_string());
            }
            Some(_) => {}
            None => {
                self.cards.insert(binding.to_string(), w);
            }
        }
    }

    fn want_host(&mut self, binding: &str, need: HostNeed) {
        match self.hosts.get(binding) {
            Some(prev) if *prev != need => {
                self.conflicts.insert(binding.to_string());
            }
            Some(_) => {}
            None => {
                self.hosts.insert(binding.to_string(), need);
            }
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

/// The binding a digivolve's target names, when it is a permanent picked by the effect.
fn host_binding(b: &CompiledBindingRef) -> Option<&str> {
    match b {
        CompiledBindingRef::Named(n) | CompiledBindingRef::Permanent(n) | CompiledBindingRef::Binding(n) => {
            Some(n.as_str())
        }
        _ => None,
    }
}

fn card_binding(b: &CompiledBindingRef) -> Option<&str> {
    match b {
        CompiledBindingRef::Named(n) | CompiledBindingRef::Binding(n) => Some(n.as_str()),
        _ => None,
    }
}

/// Every effect digivolve under `step`: its card binding and its host binding.
fn collect(step: &CompiledStep, out: &mut Collected) {
    if let CompiledStep::EffectInitiatedDigivolve { target, from_hand, cost, ignore_requirements, ignore_level } = step
    {
        let waiver = Waiver::of(*ignore_requirements, *ignore_level);
        if let (Some(card), Some(t)) = (card_binding(from_hand), target_name(target)) {
            out.want_card(card, Wanted { target: t, waiver, cost: cost.clone() });
        }
        if let Some(host) = host_binding(target) {
            out.want_host(host, HostNeed { waiver, cost: cost.clone() });
        }
        if is_paid(cost) {
            if let Some(t) = target_name(target) {
                let needs = out.paid_onto.entry(t).or_default();
                let need = (waiver, cost.clone());
                if !needs.contains(&need) {
                    needs.push(need);
                }
            }
        }
    }
    for body in child_bodies(step) {
        for s in body {
            collect(s, out);
        }
    }
}

struct Ctx {
    card_id: String,
    /// False inside a `cost_reduction` `pay_cost` body, where the payment probe is forbidden.
    cost_probe_allowed: bool,
    errors: Vec<ValidationError>,
}

impl Ctx {
    fn mark_body(
        &mut self,
        steps: &mut [CompiledStep],
        cost_probe_allowed: bool,
        gates: &mut [&mut CompiledPredicate],
        path: &str,
    ) {
        let mut wanted = Collected::default();
        for s in steps.iter() {
            collect(s, &mut wanted);
        }
        if wanted.cards.is_empty() && wanted.hosts.is_empty() && wanted.paid_onto.is_empty() {
            return;
        }
        for b in &wanted.conflicts {
            self.errors.push(ValidationError {
                card_id: self.card_id.clone(),
                path: path.to_string(),
                message: format!(
                    "the binding `{b}` feeds effect_initiated_digivolve steps with different targets, costs or \
                     waivers; the candidate pass keys a pick by its binding, so one pick would check another \
                     digivolve's payment / route. Give each pick its own `bind_as`"
                ),
            });
        }
        self.cost_probe_allowed = cost_probe_allowed;
        if cost_probe_allowed {
            for gate in gates.iter_mut() {
                self.mark_gate(gate, &wanted, false, path);
            }
        }
        let mut bound = BTreeSet::new();
        self.mark_seq(steps, &wanted, &mut bound, path);
    }

    /// The activation gate (`condition` / `active_when`) of a clause that digivolves for a cost: a route-only
    /// candidate leaf onto the effect's own `source` / `event_target` (known before the body runs) gets the payment
    /// check of the paid digivolve onto it. Only `all_of` conjuncts and a `count_gte` / `any_permanent` filter are
    /// walked: a negated or disjunctive gate is not a "there is a payable candidate" test and is left alone.
    fn mark_gate(&mut self, p: &mut CompiledPredicate, wanted: &Collected, on_source_host: bool, path: &str) {
        let node_host =
            on_source_host || p.is_source == Some(true) || p.all_of.iter().any(|c| c.is_source == Some(true));
        // Card-subject route-only leaves: the card probed can digivolve onto `target`.
        let route_target = match (&p.can_digivolve_onto, p.can_digivolve_from_source) {
            (Some(t), _) => Some(t.clone()),
            (None, Some(true)) => Some("source".to_string()),
            _ => None,
        };
        if let Some(target) = route_target {
            if p.can_digivolve_onto_with_cost.is_none() {
                if let Some((waiver, cost)) = self.gate_need(&target, wanted, path) {
                    self.check(waiver, Waiver::None, true, &target, &format!("{path}.gate"));
                    p.can_digivolve_onto_with_cost = Some(CompiledDigivolveCostPredicate {
                        target,
                        cost,
                        ignore_level: waiver == Waiver::Level,
                        ignore_requirements: waiver == Waiver::All,
                    });
                }
            }
        }
        // Permanent-subject leaf on the effect's own source: some card can digivolve onto this Digimon.
        if node_host {
            if let Some(leaf) = &mut p.has_digivolve_candidate {
                if leaf.cost.is_none() {
                    if let Some((waiver, cost)) = self.gate_need("source", wanted, path) {
                        let leaf_waiver = Waiver::of(leaf.ignore_requirements, leaf.ignore_level);
                        self.check(waiver, leaf_waiver, true, "source", &format!("{path}.gate.has_digivolve_candidate"));
                        leaf.cost = Some(cost);
                    }
                }
            }
        }
        for c in &mut p.all_of {
            self.mark_gate(c, wanted, node_host, path);
        }
        if let Some(c) = &mut p.count_gte {
            self.mark_gate(&mut c.filter, wanted, false, path);
        }
        if let Some(e) = &mut p.any_permanent {
            self.mark_gate(&mut e.predicate, wanted, false, path);
        }
    }

    /// The one paid digivolve a gate probing `target` stands for: its waiver and cost. None when no paid digivolve
    /// goes onto `target` (the probe stays route-only). A gate can't tell which of two digivolves with different
    /// costs or waivers it mirrors, so that is a compile error: author the payment leaf on the gate.
    fn gate_need(&mut self, target: &str, wanted: &Collected, path: &str) -> Option<(Waiver, CompiledCostDelta)> {
        // Only a target that exists before the body runs can be probed by a gate.
        if target != "source" && target != "event_target" {
            return None;
        }
        let needs = wanted.paid_onto.get(target)?;
        match needs.as_slice() {
            [] => None,
            [one] => Some(one.clone()),
            _ => {
                self.errors.push(ValidationError {
                    card_id: self.card_id.clone(),
                    path: format!("{path}.gate"),
                    message: format!(
                        "the clause gate counts route-only candidates onto `{target}`, but the body digivolves it \
                         with different costs or waivers; the pass can't tell which one the gate mirrors. Author \
                         `can_digivolve_onto_with_cost` on the gate"
                    ),
                });
                None
            }
        }
    }

    fn mark_seq(&mut self, steps: &mut [CompiledStep], wanted: &Collected, bound: &mut BTreeSet<String>, path: &str) {
        for (i, step) in steps.iter_mut().enumerate() {
            let step_path = format!("{path}[{i}]");
            if let Some(b) = bind_of(step).cloned() {
                if let Some(w) = wanted.cards.get(&b) {
                    let known = w.target == "source" || w.target == "event_target" || bound.contains(&w.target);
                    if let Some(filter) = card_filter(step) {
                        self.mark_card_pick(filter, w, known, &b, &step_path);
                    }
                }
                if let Some(need) = wanted.hosts.get(&b) {
                    if let Some(filter) = permanent_filter(step) {
                        let mut leaves = Vec::new();
                        host_leaves(filter, &mut leaves);
                        for leaf in leaves {
                            let at = format!("{step_path}.filter.has_digivolve_candidate");
                            self.check(need.waiver, leaf, need.paid(), &b, &at);
                        }
                        // A paid digivolve's host leaf checks the payment too (G-DSL-PAID-DIGIVOLVE-HOST-PICK-
                        // ROUTE-ONLY); the probe is forbidden inside a cost_reduction `pay_cost` body.
                        if self.cost_probe_allowed && need.paid() {
                            add_host_cost(filter, &need.cost);
                        }
                    }
                }
            }
            // a nested body sees what is bound so far, and the selection's own binding inside its `then`
            let mut inner = bound.clone();
            if let Some(b) = bind_of(step) {
                inner.insert(b.clone());
            }
            for (k, body) in child_bodies_mut(step).into_iter().enumerate() {
                let mut scoped = inner.clone();
                self.mark_seq(body, wanted, &mut scoped, &format!("{step_path}.body{k}"));
            }
            if let Some(b) = bind_of(step) {
                bound.insert(b.clone());
            }
        }
    }

    /// Check the candidate leaves the pick authored against the waiver, and add the ones the digivolve calls for: the
    /// payment check of a paid digivolve (G-DSL-PAID-DIGIVOLVE-PICK-PAYMENT) and, with no authored leaf, the route.
    fn mark_card_pick(&mut self, filter: &mut CompiledPredicate, w: &Wanted, known: bool, binding: &str, path: &str) {
        let mut leaves = Vec::new();
        card_leaves(filter, &mut leaves);
        for leaf in &leaves {
            self.check(w.waiver, *leaf, w.paid(), binding, &format!("{path}.filter"));
        }
        // `can_digivolve_from_source` is a requirement-keeping route leaf too (onto the effect's source). It is only
        // checked here, not counted as the pick's route leaf: its target is the source, not necessarily `w.target`.
        if has_from_source_leaf(filter) {
            self.check(w.waiver, Waiver::None, w.paid(), binding, &format!("{path}.filter.can_digivolve_from_source"));
        }
        if !known {
            return;
        }
        // The cost leaf checks the route (with the step's waiver) AND the payment at the step's own cost. An
        // `ignore_level` pick always gets it when it authored no leaf (the level-waived route set needs it).
        let wants_cost_leaf = w.paid() || (w.waiver == Waiver::Level && leaves.is_empty());
        if self.cost_probe_allowed && wants_cost_leaf && !has_cost_leaf(filter) {
            filter.can_digivolve_onto_with_cost = Some(CompiledDigivolveCostPredicate {
                target: w.target.clone(),
                cost: w.cost.clone(),
                ignore_level: w.waiver == Waiver::Level,
                ignore_requirements: w.waiver == Waiver::All,
            });
            return;
        }
        if !leaves.is_empty() {
            return;
        }
        match w.waiver {
            Waiver::All => {}
            Waiver::None => filter.can_digivolve_onto = Some(w.target.clone()),
            // Only reached inside a cost_reduction `pay_cost` body, where the payment probe is forbidden.
            Waiver::Level => filter.can_digivolve_onto_ignoring_level = Some(w.target.clone()),
        }
    }

    fn check(&mut self, waiver: Waiver, leaf: Waiver, paid: bool, binding: &str, path: &str) {
        if let Some(why) = waiver.disagreement(leaf, paid) {
            self.errors.push(ValidationError {
                card_id: self.card_id.clone(),
                path: path.to_string(),
                message: format!("the pick binding `{binding}` feeds an effect_initiated_digivolve: {why}"),
            });
        }
    }
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
        | S::SelectOwnBreedingPermanent { bind_as, .. }
        | S::SelectAnyPermanent { bind_as, .. } => bind_as.as_ref(),
        _ => None,
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

fn permanent_filter(step: &mut CompiledStep) -> Option<&mut CompiledPredicate> {
    use CompiledStep as S;
    match step {
        S::SelectOwnPermanent { filter, .. }
        | S::SelectOpponentPermanent { filter, .. }
        | S::SelectAnyPermanent { filter, .. }
        | S::SelectOwnBreedingPermanent { filter, .. } => Some(filter),
        _ => None,
    }
}

/// Give every authored `has_digivolve_candidate` leaf on `filter` (itself or a conjunct) that has no `cost` the
/// digivolve's cost.
fn add_host_cost(filter: &mut CompiledPredicate, cost: &CompiledCostDelta) {
    if let Some(leaf) = &mut filter.has_digivolve_candidate {
        if leaf.cost.is_none() {
            leaf.cost = Some(cost.clone());
        }
    }
    for conjunct in &mut filter.all_of {
        add_host_cost(conjunct, cost);
    }
}

/// The card-subject candidate leaves `filter` authors (itself or a conjunct), each as the requirements it waives.
fn card_leaves(filter: &CompiledPredicate, out: &mut Vec<Waiver>) {
    if filter.can_digivolve_onto.is_some() {
        out.push(Waiver::None);
    }
    if let Some(leaf) = &filter.can_digivolve_onto_with_cost {
        out.push(Waiver::of(leaf.ignore_requirements, leaf.ignore_level));
    }
    if filter.can_digivolve_onto_ignoring_level.is_some() {
        out.push(Waiver::Level);
    }
    for conjunct in &filter.all_of {
        card_leaves(conjunct, out);
    }
}

/// Whether `filter` (itself or a conjunct) authors `can_digivolve_from_source: true`.
fn has_from_source_leaf(filter: &CompiledPredicate) -> bool {
    filter.can_digivolve_from_source == Some(true) || filter.all_of.iter().any(has_from_source_leaf)
}

/// Whether `filter` (itself or a conjunct) already checks the payment.
fn has_cost_leaf(filter: &CompiledPredicate) -> bool {
    filter.can_digivolve_onto_with_cost.is_some() || filter.all_of.iter().any(has_cost_leaf)
}

/// The `has_digivolve_candidate` leaves `filter` authors (itself or a conjunct), each as the requirements it waives.
fn host_leaves(filter: &CompiledPredicate, out: &mut Vec<Waiver>) {
    if let Some(leaf) = &filter.has_digivolve_candidate {
        out.push(Waiver::of(leaf.ignore_requirements, leaf.ignore_level));
    }
    for conjunct in &filter.all_of {
        host_leaves(conjunct, out);
    }
}

#[cfg(test)]
#[path = "digivolve_candidates_payment_tests.rs"]
mod payment_tests;
