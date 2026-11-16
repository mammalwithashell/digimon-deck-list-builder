//! A PAID effect digivolve's card pick must check the payment, not only the route.
//!
//! "This Digimon may digivolve into [X] in the hand (with the digivolution cost reduced by N)" charges a cost. A card
//! whose cost can't be paid can't be digivolved (CR 4.3 1-3-11-1: a card can't be declared if its cost can't be paid;
//! 8-1-2-6: a revealed card that can't be digivolved because its cost can't be paid goes back and memory doesn't
//! move; 4-2-2 / 1-4-2-2: the gauge stops at 10, so a cost above memory + 10 can't be paid). Offering such a card is
//! a dead choice the rules do not have. Reachable whenever memory is low, e.g. an [Opponent's Turn] trigger with the
//! gauge on the opponent's side.
//!
//! DCGO passes `payCost: true` for every one of these digivolves (`DigivolveIntoHandOrTrashCard`,
//! CardEffectCommons.cs:820-878, e.g. AD1_011.cs:141-147), but `CardSource.CanPlayCardTargetFrame` only honours
//! `PayCost` for an EMPTY frame (CardSource.cs:1150-1161): for a digivolve onto a permanent it checks the route
//! alone. The rules manual outranks DCGO here; the payment check rests on CR 4.3 1-3-11-1 / 8-1-2-6.
//!
//! The compile pass (`digivolve_candidates`) adds `can_digivolve_onto_with_cost { target, cost: <the step's cost> }`
//! (with the step's `ignore_level` / `ignore_requirements` waiver) to the pick of every paid digivolve, next to any
//! route-only leaf the card authored. G-DSL-PAID-DIGIVOLVE-PICK-PAYMENT.
use std::path::{Path, PathBuf};

use crate::compiled::{
    CompiledBindingRef, CompiledCard, CompiledClause, CompiledCostDelta, CompiledDeclarativeClause,
    CompiledPredicate, CompiledStep,
};
use crate::deletion_cap::child_bodies;

fn cards_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../digimon-engine/cards")
}

fn yaml_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for set in std::fs::read_dir(cards_root()).expect("cards dir") {
        let set = set.unwrap().path();
        if !set.is_dir() {
            continue;
        }
        for f in std::fs::read_dir(&set).unwrap() {
            let f = f.unwrap().path();
            if f.extension().is_some_and(|e| e == "yaml") {
                out.push(f);
            }
        }
    }
    out.sort();
    out
}

fn compile_path(path: &Path) -> Option<CompiledCard> {
    let spec = crate::loader::load_file(path).ok()?;
    crate::compile::compile(&spec).ok()
}

fn compile_id(id: &str) -> CompiledCard {
    let path = yaml_files()
        .into_iter()
        .find(|p| p.file_stem().is_some_and(|s| s.eq_ignore_ascii_case(id)))
        .unwrap_or_else(|| panic!("{id}: no YAML spec"));
    let spec = crate::loader::load_file(&path).unwrap_or_else(|e| panic!("{id}: {e:?}"));
    crate::compile::compile(&spec).unwrap_or_else(|e| panic!("{id}: {e:?}"))
}

/// A paid digivolve the pick must check: its card binding and what the cost leaf must carry.
#[derive(Debug)]
struct PaidDigivolve {
    card_binding: String,
    cost: CompiledCostDelta,
    ignore_level: bool,
    ignore_requirements: bool,
}

/// True when the digivolve charges memory that could exceed what is payable. A free digivolve ("without paying the
/// cost", DCGO `payCost: false`) and a fixed cost of 0 are always payable.
fn is_paid(cost: &CompiledCostDelta) -> bool {
    !matches!(cost, CompiledCostDelta::Free | CompiledCostDelta::Literal(0))
}

fn walk<'a>(steps: &'a [CompiledStep], f: &mut dyn FnMut(&'a CompiledStep)) {
    for s in steps {
        f(s);
        for body in child_bodies(s) {
            walk(body, f);
        }
    }
}

/// Every step body the pass marks with the payment probe allowed (a cost_reduction `pay_cost` body is excluded:
/// the payment probe would evaluate the reducer being asked about).
fn paid_bodies(card: &CompiledCard) -> Vec<&[CompiledStep]> {
    let mut out: Vec<&[CompiledStep]> = Vec::new();
    for clause in &card.effects {
        match clause {
            CompiledClause::Triggered(t) => out.push(&t.process),
            CompiledClause::Declarative(
                CompiledDeclarativeClause::Replacement { process, .. }
                | CompiledDeclarativeClause::Partition { process, .. }
                | CompiledDeclarativeClause::Delay { process, .. },
            ) => out.push(process),
            _ => {}
        }
    }
    out
}

fn paid_digivolves(body: &[CompiledStep]) -> Vec<PaidDigivolve> {
    let mut out = Vec::new();
    walk(body, &mut |s| {
        if let CompiledStep::EffectInitiatedDigivolve { from_hand, cost, ignore_requirements, ignore_level, .. } = s {
            let binding = match from_hand {
                CompiledBindingRef::Named(n) | CompiledBindingRef::Binding(n) => n.clone(),
                _ => return,
            };
            if is_paid(cost) {
                out.push(PaidDigivolve {
                    card_binding: binding,
                    cost: cost.clone(),
                    ignore_level: *ignore_level && !*ignore_requirements,
                    ignore_requirements: *ignore_requirements,
                });
            }
        }
    });
    out
}

fn pick_filters<'a>(body: &'a [CompiledStep], binding: &str) -> Vec<&'a CompiledPredicate> {
    let mut out = Vec::new();
    walk(body, &mut |s| {
        use CompiledStep as S;
        if let S::SelectHand { bind_as: Some(b), filter, .. }
        | S::SelectTrash { bind_as: Some(b), filter, .. }
        | S::SelectZoneCards { bind_as: Some(b), filter, .. }
        | S::SelectReveal { bind_as: Some(b), filter, .. }
        | S::SelectUnionZone { bind_as: Some(b), filter, .. } = s
        {
            if b == binding {
                out.push(filter);
            }
        }
    });
    out
}

/// The cost leaves on `filter` itself or a conjunct.
fn cost_leaves(filter: &CompiledPredicate, out: &mut Vec<(CompiledCostDelta, bool, bool)>) {
    if let Some(leaf) = &filter.can_digivolve_onto_with_cost {
        out.push((leaf.cost.clone(), leaf.ignore_level, leaf.ignore_requirements));
    }
    for c in &filter.all_of {
        cost_leaves(c, out);
    }
}

/// The audit's list: paid effect digivolves whose pick checked only the route.
const AUDITED: &[&str] = &[
    // authored `can_digivolve_onto`
    "AD1-011", "AD1-021", "BT10-085", "BT12-030", "BT18-065", "BT20-012", "BT23-076", "BT25-092", "BT26-035",
    "BT26-038", "BT26-091", "BT9-109", "EX13-004", "EX13-018", "EX13-039", "EX13-055", "EX13-057", "EX13-069",
    "EX13-070", "LM-027", "LM-028", "LM-030", "LM-055", "LM-057", "LM-058", "LM-059", "LM-060", "LM-061", "LM-062",
    "P-166", "P-244",
    // authored `can_digivolve_from_source`
    "BT13-013", "BT20-023", "BT20-040", "BT25-005", "BT26-001", "BT26-044",
    // compile-pass only (<Delay> body)
    "BT22-098",
    // `ignore_requirements` with a fixed cost of 3
    "EX13-074", "P-092",
];

#[test]
fn audited_paid_digivolve_picks_check_payment_at_the_steps_cost() {
    let mut failures = Vec::new();
    for id in AUDITED {
        let card = compile_id(id);
        let mut seen = 0;
        for body in paid_bodies(&card) {
            for d in paid_digivolves(body) {
                seen += 1;
                let filters = pick_filters(body, &d.card_binding);
                if filters.is_empty() {
                    failures.push(format!("{id}: no pick binds `{}`", d.card_binding));
                }
                for filter in filters {
                    let mut leaves = Vec::new();
                    cost_leaves(filter, &mut leaves);
                    let want = (d.cost.clone(), d.ignore_level, d.ignore_requirements);
                    if !leaves.contains(&want) {
                        failures.push(format!(
                            "{id}: pick `{}` lacks can_digivolve_onto_with_cost {{ cost: {:?}, ignore_level: {}, \
                             ignore_requirements: {} }} (has {leaves:?})",
                            d.card_binding, d.cost, d.ignore_level, d.ignore_requirements
                        ));
                    }
                }
            }
        }
        if seen == 0 {
            failures.push(format!("{id}: no paid effect digivolve found"));
        }
    }
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}

/// P-092 Dracomon's inherited effect digivolves "without paying the cost" (`cost: 0`; DCGO P_092.cs:138-144
/// `payCost: false`): no payment check, the authored route leaf stays the only candidate check.
#[test]
fn free_digivolve_pick_gets_no_payment_leaf() {
    let card = compile_id("P-092");
    let CompiledClause::Triggered(inherited) = &card.effects[1] else { panic!("P-092 effects[1] is triggered") };
    let filters = pick_filters(&inherited.process, "wing");
    assert_eq!(filters.len(), 1);
    let mut leaves = Vec::new();
    cost_leaves(filters[0], &mut leaves);
    assert!(leaves.is_empty(), "free digivolve must not get a payment leaf: {leaves:?}");
    assert!(filters[0].can_digivolve_from_source == Some(true) || filters[0].all_of.iter().any(|c| c.can_digivolve_from_source == Some(true)));
}

/// Pool-wide: every paid effect digivolve's pick (outside a cost_reduction) carries a payment leaf with the
/// digivolve's waiver. Picks whose target is chosen after the pick are listed here with their reason.
#[test]
fn every_paid_digivolve_pick_in_the_pool_checks_payment() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for path in yaml_files() {
        // A spec that fails to load or compile would otherwise drop out of the sweep unseen.
        let Some(card) = compile_path(&path) else {
            failures.push(format!("{}: does not load or compile", path.display()));
            continue;
        };
        for body in paid_bodies(&card) {
            for d in paid_digivolves(body) {
                let filters = pick_filters(body, &d.card_binding);
                // A pick kind the pass does not mark (a future selection step) would leave the digivolve unchecked.
                if filters.is_empty() {
                    failures.push(format!(
                        "{} paid digivolve's card `{}` is bound by no pick the pass marks",
                        card.card, d.card_binding
                    ));
                }
                for filter in filters {
                    checked += 1;
                    let mut leaves = Vec::new();
                    cost_leaves(filter, &mut leaves);
                    if !leaves.iter().any(|(_, il, ir)| *il == d.ignore_level && *ir == d.ignore_requirements) {
                        failures.push(format!("{} pick `{}` ({:?})", card.card, d.card_binding, d.cost));
                    } else if !leaves.iter().any(|(c, _, _)| *c == d.cost) {
                        failures.push(format!(
                            "{} pick `{}` checks a different cost than the step charges ({:?} vs {leaves:?})",
                            card.card, d.card_binding, d.cost
                        ));
                    }
                }
            }
        }
    }
    assert!(checked > 40, "the walk found the paid digivolves ({checked})");
    failures.sort();
    failures.dedup();
    assert!(failures.is_empty(), "{} paid picks without a payment check:\n{}", failures.len(), failures.join("\n"));
}

/// A minimal [On Play] host whose process is `process` (indented as a `process:` list).
fn host_yaml(process: &str) -> String {
    format!(
        "card: TEST-PAID-PASS\nname: Paid pass host\nkind: digimon\nlevel: 3\ncolor: [yellow]\ncost: 3\ndp: 3000\n\
         effects:\n  - when: on_play\n    process:\n{process}"
    )
}

/// A hand pick bound to `bind` with `filter`, then a digivolve of it onto the source at `cost` (`extra` = more step
/// lines).
fn pick_and_digivolve(bind: &str, filter: &str, cost: &str, extra: &str) -> String {
    format!(
        "      - select_hand:\n          of: you\n          bind_as: {bind}\n          optional: true\n          \
         filter: {filter}\n          prompt: pick\n      - effect_initiated_digivolve:\n          target: source\n          \
         from_hand: {bind}\n          cost: {cost}\n{extra}"
    )
}

fn compile_yaml(yaml: &str) -> Result<CompiledCard, Vec<String>> {
    let spec: crate::CardSpec = serde_yml::from_str(yaml).unwrap_or_else(|e| panic!("parses: {e}\n{yaml}"));
    crate::compile::compile(&spec).map_err(|errs| errs.into_iter().map(|e| format!("{}: {}", e.path, e.message)).collect())
}

/// A paid "for a digivolution cost of 3, ignoring digivolution requirements" digivolve (EX13-074, P-092) offers any
/// card it can pay for. An authored leaf that keeps the requirements (a route leaf, `can_digivolve_from_source`, or a
/// cost leaf without `ignore_requirements`) would hide cards the digivolve accepts, and an authored cost leaf also
/// stops the pass adding the requirement-waiving one. The free sugar exception (a replacement's
/// `digivolve_without_cost`, BT17-097) is `cost: free`, so a PAID ignore-requirements step is always authored as
/// such and its pick's leaves are checked.
#[test]
fn requirement_keeping_leaf_on_a_paid_ignore_requirements_pick_is_an_error() {
    let ignore = "          ignore_requirements: true\n";
    for leaf in [
        "{all_of: [{kind: digimon}, {can_digivolve_onto: source}]}",
        "{all_of: [{kind: digimon}, {can_digivolve_from_source: true}]}",
        "{can_digivolve_onto_with_cost: {target: source, cost: 3}}",
    ] {
        let errors = compile_yaml(&host_yaml(&pick_and_digivolve("card", leaf, "3", ignore)))
            .err()
            .unwrap_or_else(|| panic!("{leaf}: compiled, but the leaf hides cards the digivolve accepts"));
        assert!(errors.iter().any(|e| e.contains("ignore_requirements")), "{leaf}: {errors:?}");
    }
    // The matching leaf compiles, as does a free requirement-ignoring step with a route leaf (the sugar shape).
    let ok = "{can_digivolve_onto_with_cost: {target: source, cost: 3, ignore_requirements: true}}";
    compile_yaml(&host_yaml(&pick_and_digivolve("card", ok, "3", ignore))).expect("matching leaf compiles");
    let route = "{all_of: [{kind: digimon}, {can_digivolve_onto: source}]}";
    compile_yaml(&host_yaml(&pick_and_digivolve("card", route, "free", ignore))).expect("free sugar shape compiles");
}

/// The pass keys a digivolve's needs by the card binding. Two digivolves in one body that bind the same name with
/// different costs would give both picks one cost (the later one): the earlier pick would check the payment at the
/// wrong cost. That is a compile error (rename one binding); two with the same needs are fine.
#[test]
fn one_card_binding_feeding_digivolves_with_different_costs_is_an_error() {
    let a = pick_and_digivolve("card", "{kind: digimon}", "{reduce: 1}", "");
    let b = pick_and_digivolve("card", "{kind: digimon}", "printed", "");
    let errors = compile_yaml(&host_yaml(&format!("{a}{b}")))
        .err()
        .expect("compiled, but the first pick would check the payment at the second digivolve's cost");
    assert!(errors.iter().any(|e| e.contains("`card`") && e.contains("different")), "{errors:?}");
    // Same binding, same cost and waiver: no ambiguity.
    compile_yaml(&host_yaml(&format!("{a}{a}"))).expect("identical digivolves compile");
    // Distinct bindings: each pick gets its own step's cost.
    let b2 = pick_and_digivolve("card2", "{kind: digimon}", "printed", "");
    let card = compile_yaml(&host_yaml(&format!("{a}{b2}"))).expect("distinct bindings compile");
    let CompiledClause::Triggered(t) = &card.effects[0] else { panic!("triggered") };
    for (i, want) in [(0, CompiledCostDelta::Reduce(1)), (2, CompiledCostDelta::Printed)] {
        let CompiledStep::SelectHand { filter, .. } = &t.process[i] else { panic!("select_hand at {i}") };
        assert_eq!(filter.can_digivolve_onto_with_cost.as_ref().map(|l| l.cost.clone()), Some(want));
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The host pick and the activation gate of a paid digivolve (G-DSL-PAID-DIGIVOLVE-HOST-PICK-ROUTE-ONLY)
// ---------------------------------------------------------------------------------------------------------------

/// A paid digivolve's target as the leaf evaluates it, with the step's needs.
#[derive(Debug, Clone, PartialEq)]
struct PaidOnto {
    target: String,
    cost: CompiledCostDelta,
    ignore_level: bool,
    ignore_requirements: bool,
}

fn paid_ontos(body: &[CompiledStep]) -> Vec<PaidOnto> {
    let mut out = Vec::new();
    walk(body, &mut |s| {
        if let CompiledStep::EffectInitiatedDigivolve { target, cost, ignore_requirements, ignore_level, .. } = s {
            if !is_paid(cost) {
                return;
            }
            let target = match target {
                CompiledBindingRef::Named(n) | CompiledBindingRef::Permanent(n) | CompiledBindingRef::Binding(n) => {
                    n.clone()
                }
                CompiledBindingRef::Source | CompiledBindingRef::SelfRef | CompiledBindingRef::Carrier => "source".into(),
                CompiledBindingRef::EventTarget => "event_target".into(),
                _ => return,
            };
            out.push(PaidOnto {
                target,
                cost: cost.clone(),
                ignore_level: *ignore_level && !*ignore_requirements,
                ignore_requirements: *ignore_requirements,
            });
        }
    });
    out
}

/// The `has_digivolve_candidate` leaves on `filter` itself or a conjunct.
fn host_leaves<'a>(filter: &'a CompiledPredicate, out: &mut Vec<&'a crate::compiled::CompiledDigivolveCandidate>) {
    if let Some(leaf) = &filter.has_digivolve_candidate {
        out.push(leaf);
    }
    for c in &filter.all_of {
        host_leaves(c, out);
    }
}

/// The permanent picks that bind `binding`.
fn host_pick_filters<'a>(body: &'a [CompiledStep], binding: &str) -> Vec<&'a CompiledPredicate> {
    let mut out = Vec::new();
    walk(body, &mut |s| {
        use CompiledStep as S;
        if let S::SelectOwnPermanent { bind_as: Some(b), filter, .. }
        | S::SelectOpponentPermanent { bind_as: Some(b), filter, .. }
        | S::SelectAnyPermanent { bind_as: Some(b), filter, .. }
        | S::SelectOwnBreedingPermanent { bind_as: Some(b), filter, .. } = s
        {
            if b == binding {
                out.push(filter);
            }
        }
    });
    out
}

/// Pool-wide: every `has_digivolve_candidate` leaf on a pick that binds the target of a PAID effect digivolve carries
/// that digivolve's cost (the pass adds it unless the card authored one) and its waiver.
#[test]
fn every_paid_digivolve_host_pick_in_the_pool_checks_payment() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for path in yaml_files() {
        let Some(card) = compile_path(&path) else {
            failures.push(format!("{}: does not load or compile", path.display()));
            continue;
        };
        for body in paid_bodies(&card) {
            for d in paid_ontos(body) {
                if d.target == "source" || d.target == "event_target" {
                    continue;
                }
                for filter in host_pick_filters(body, &d.target) {
                    let mut leaves = Vec::new();
                    host_leaves(filter, &mut leaves);
                    for leaf in leaves {
                        checked += 1;
                        if leaf.cost.as_ref() != Some(&d.cost) {
                            failures.push(format!(
                                "{} host pick `{}`: has_digivolve_candidate cost {:?}, the digivolve charges {:?}",
                                card.card, d.target, leaf.cost, d.cost
                            ));
                        }
                        let leaf_json = serde_json::to_value(leaf).unwrap();
                        let leaf_requirements = leaf_json["ignore_requirements"].as_bool().unwrap_or(false);
                        if leaf.ignore_level != d.ignore_level || leaf_requirements != d.ignore_requirements {
                            failures.push(format!("{} host pick `{}`: waiver differs from the digivolve", card.card, d.target));
                        }
                    }
                }
            }
        }
    }
    assert!(checked > 30, "the walk found the host picks ({checked})");
    failures.sort();
    failures.dedup();
    assert!(failures.is_empty(), "{} host picks that check the route only:\n{}", failures.len(), failures.join("\n"));
}

/// The route-only candidate leaves a gate counts or probes, as `(target, has a payment check)`: `can_digivolve_onto:
/// T`, `can_digivolve_from_source: true` (onto the effect's source), and `has_digivolve_candidate` on an `is_source`
/// permanent. Only `all_of` conjuncts and a `count_gte` / `any_permanent` filter are walked (a negated or
/// disjunctive gate is left alone).
fn gate_route_leaves(p: &CompiledPredicate, on_source_host: bool, out: &mut Vec<(String, bool)>) {
    if let Some(t) = &p.can_digivolve_onto {
        out.push((t.clone(), p.can_digivolve_onto_with_cost.is_some()));
    }
    if p.can_digivolve_from_source == Some(true) {
        out.push(("source".into(), p.can_digivolve_onto_with_cost.is_some()));
    }
    let host = on_source_host || p.is_source == Some(true) || p.all_of.iter().any(|c| c.is_source == Some(true));
    if host {
        if let Some(h) = &p.has_digivolve_candidate {
            out.push(("source".into(), h.cost.is_some()));
        }
    }
    for c in &p.all_of {
        gate_route_leaves(c, host, out);
    }
    if let Some(c) = &p.count_gte {
        gate_route_leaves(&c.filter, false, out);
    }
    if let Some(e) = &p.any_permanent {
        gate_route_leaves(&e.predicate, false, out);
    }
}

/// Pool-wide: a clause gate (`condition` / `active_when`) that counts route-only candidates onto the effect's own
/// source, in a clause whose body digivolves the source for a paid cost, also checks the payment.
#[test]
fn every_paid_digivolve_gate_in_the_pool_checks_payment() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for path in yaml_files() {
        let Some(card) = compile_path(&path) else { continue };
        for clause in &card.effects {
            let (gates, body): (Vec<&CompiledPredicate>, &[CompiledStep]) = match clause {
                CompiledClause::Triggered(t) => (t.condition.iter().chain(t.active_when.iter()).collect(), &t.process),
                CompiledClause::Declarative(
                    CompiledDeclarativeClause::Replacement { process, active_when, .. }
                    | CompiledDeclarativeClause::Delay { process, active_when, .. },
                ) => (active_when.iter().collect(), process),
                _ => continue,
            };
            if !paid_ontos(body).iter().any(|d| d.target == "source") {
                continue;
            }
            for gate in gates {
                let mut leaves = Vec::new();
                gate_route_leaves(gate, false, &mut leaves);
                for (target, has_cost) in leaves {
                    if target == "source" {
                        checked += 1;
                        if !has_cost {
                            failures.push(format!("{}: a gate counts route-only candidates onto the source", card.card));
                        }
                    }
                }
            }
        }
    }
    assert!(checked >= 5, "the walk found the gates ({checked})");
    failures.sort();
    failures.dedup();
    assert!(failures.is_empty(), "{} gates that check the route only:\n{}", failures.len(), failures.join("\n"));
}

/// The audit of the host picks (G-DSL-PAID-DIGIVOLVE-HOST-PICK-ROUTE-ONLY): every card whose host leaf was route-only
/// before the pass gave it the digivolve's cost. Each is checked card by card, so a regression names the card.
const AUDITED_HOSTS: &[&str] = &[
    "AD1-021", "BT10-085", "BT23-076", "BT25-092", "BT26-035", "BT26-038", "BT26-066", "BT26-091", "EX13-069",
    "LM-027", "LM-028", "LM-029", "LM-030", "LM-031", "LM-032", "LM-054", "LM-055", "LM-056", "LM-057", "LM-058",
    "LM-059", "LM-060", "LM-061", "LM-062", "P-244",
];

/// The activation gates that counted route-only candidates onto the effect's own source.
const AUDITED_GATES: &[&str] =
    &["BT20-012", "BT24-003", "BT25-005", "BT26-001", "EX13-055", "EX13-057"];

#[test]
fn audited_host_picks_carry_the_digivolves_cost() {
    let mut failures = Vec::new();
    for id in AUDITED_HOSTS {
        let card = compile_id(id);
        let mut found = 0;
        for body in paid_bodies(&card) {
            for d in paid_ontos(body) {
                for filter in host_pick_filters(body, &d.target) {
                    let mut leaves = Vec::new();
                    host_leaves(filter, &mut leaves);
                    for leaf in leaves {
                        found += 1;
                        if leaf.cost.as_ref() != Some(&d.cost) {
                            failures.push(format!("{id}: host leaf cost {:?}, digivolve {:?}", leaf.cost, d.cost));
                        }
                    }
                }
            }
        }
        if found == 0 {
            failures.push(format!("{id}: no host leaf found on a paid digivolve's host pick"));
        }
    }
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn audited_gates_check_the_payment() {
    let mut failures = Vec::new();
    for id in AUDITED_GATES {
        let card = compile_id(id);
        let mut found = 0;
        for clause in &card.effects {
            let CompiledClause::Triggered(t) = clause else { continue };
            if !paid_ontos(&t.process).iter().any(|d| d.target == "source") {
                continue;
            }
            for gate in t.condition.iter().chain(t.active_when.iter()) {
                let mut leaves = Vec::new();
                gate_route_leaves(gate, false, &mut leaves);
                for (target, has_cost) in leaves {
                    if target == "source" {
                        found += 1;
                        if !has_cost {
                            failures.push(format!("{id}: a route-only gate onto the source"));
                        }
                    }
                }
            }
        }
        if found == 0 {
            failures.push(format!("{id}: no gate leaf found"));
        }
    }
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}

/// Audited and left alone: BT22-098's <Delay> host pick authors no `has_digivolve_candidate` (DCGO offers every
/// [Puppet] Digimon, BT22_098.cs:176-186), so the pass adds none; the card pick already checks the payment.
/// BT22-026's branch-0 host pick is name-only in DCGO (BT22_026.cs:156-160) and stays so. The free digivolves
/// (BT9-104, BT20-018, BT20-056, ST20-06, P-204, BT17-097's replacement) and fixed 0 ones (BT12-092, P-128, EX12-071)
/// keep route-only host leaves: nothing is paid.
#[test]
fn audited_unchanged_cards_keep_their_host_leaves() {
    for id in ["BT22-098", "BT22-026"] {
        let card = compile_id(id);
        let mut picks = 0;
        walk_clauses(&card, &mut |s| {
            if let CompiledStep::SelectOwnPermanent { filter, .. } = s {
                picks += 1;
                let mut leaves = Vec::new();
                host_leaves(filter, &mut leaves);
                assert!(leaves.is_empty(), "{id} authors no host leaf, the pass adds none");
            }
        });
        assert!(picks > 0, "{id}: expected a host pick");
    }
    for id in ["BT9-104", "BT20-018", "BT20-056", "ST20-06", "P-204", "BT12-092", "P-128", "EX12-071"] {
        let card = compile_id(id);
        let mut seen = 0;
        walk_clauses(&card, &mut |s| {
            if let CompiledStep::SelectOwnPermanent { filter, .. } | CompiledStep::SelectOwnBreedingPermanent { filter, .. } = s {
                let mut leaves = Vec::new();
                host_leaves(filter, &mut leaves);
                for leaf in leaves {
                    seen += 1;
                    assert!(leaf.cost.is_none(), "{id}: a free / fixed-0 digivolve keeps a route-only host leaf");
                }
            }
        });
        assert!(seen > 0, "{id}: expected a host leaf");
    }
}

fn walk_clauses<'a>(card: &'a CompiledCard, f: &mut dyn FnMut(&'a CompiledStep)) {
    for clause in &card.effects {
        match clause {
            CompiledClause::Triggered(t) => walk(&t.process, f),
            CompiledClause::Declarative(
                CompiledDeclarativeClause::Replacement { process, .. }
                | CompiledDeclarativeClause::Partition { process, .. }
                | CompiledDeclarativeClause::Delay { process, .. },
            ) => walk(process, f),
            _ => {}
        }
    }
}

/// A host binding feeding digivolves with different costs would give the host pick one of them: compile error.
#[test]
fn one_host_binding_feeding_digivolves_with_different_costs_is_an_error() {
    let host = "      - select_own_permanent:\n          bind_as: host\n          optional: true\n          filter:\n            \
                has_digivolve_candidate: {of: you, zone: [hand]}\n          prompt: pick\n";
    let dig = |bind: &str, cost: &str| {
        format!(
            "      - select_hand:\n          of: you\n          bind_as: {bind}\n          optional: true\n          \
             filter: {{kind: digimon}}\n          prompt: pick\n      - effect_initiated_digivolve:\n          target: host\n          \
             from_hand: {bind}\n          cost: {cost}\n"
        )
    };
    let errors = compile_yaml(&host_yaml(&format!("{host}{}{}", dig("a", "{reduce: 1}"), dig("b", "{reduce: 2}"))))
        .err()
        .expect("compiled, but the host pick would check one digivolve's payment for both");
    assert!(errors.iter().any(|e| e.contains("`host`") && e.contains("different")), "{errors:?}");
}

/// A gate probing the source for a clause that digivolves it with two different costs can't tell which one it
/// mirrors: compile error, author the payment leaf on the gate.
#[test]
fn gate_in_a_clause_with_two_different_source_digivolves_is_an_error() {
    let dig = |bind: &str, cost: &str| pick_and_digivolve(bind, "{kind: digimon}", cost, "");
    let yaml = format!(
        "card: TEST-GATE-AMBIGUOUS\nname: Ambiguous gate\nkind: digimon\nlevel: 3\ncolor: [yellow]\ncost: 3\ndp: 3000\n\
         effects:\n  - when: on_play\n    condition:\n      count_gte:\n        filter:\n          of: you\n          \
         zone: [hand]\n          all_of:\n            - kind: digimon\n            - can_digivolve_onto: source\n        \
         n: 1\n    process:\n{}{}",
        dig("a", "{reduce: 1}"),
        dig("b", "{reduce: 2}")
    );
    let errors = compile_yaml(&yaml).err().expect("compiled, but the gate can't tell which digivolve it mirrors");
    assert!(errors.iter().any(|e| e.contains("gate") && e.contains("different")), "{errors:?}");
}
