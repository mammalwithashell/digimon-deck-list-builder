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

/// `has_digivolve_candidate` objects under `v`, and how many of them carry a `cost`.
fn host_leaf_counts(v: &serde_json::Value) -> (usize, usize) {
    let mut total = 0;
    let mut with_cost = 0;
    let mut stack = vec![v];
    while let Some(v) = stack.pop() {
        match v {
            serde_json::Value::Object(m) => {
                if let Some(leaf) = m.get("has_digivolve_candidate").and_then(|l| l.as_object()) {
                    total += 1;
                    if leaf.get("cost").is_some_and(|c| !c.is_null()) {
                        with_cost += 1;
                    }
                }
                stack.extend(m.values());
            }
            serde_json::Value::Array(a) => stack.extend(a.iter()),
            _ => {}
        }
    }
    (total, with_cost)
}

/// The HOST pick and the activation gate of a paid digivolve stay route-only unless the card authors a `cost`: DCGO's
/// host / gate probes are `CanPlayCardTargetFrame(.., false | true, ..)` on an occupied frame, which never checks the
/// payment (CardSource.cs:1150-1161), and CR 4.3 1-3-2 lets a player choose a host whose candidate can't be paid
/// (1-3-11-1 only forbids declaring the CARD, which the card pick checks). So the pass must not add a cost to any
/// authored `has_digivolve_candidate` leaf: the compiled card has exactly as many costed leaves as its YAML.
/// G-DSL-PAID-DIGIVOLVE-HOST-PICK-ROUTE-ONLY.
#[test]
fn the_pass_never_adds_a_cost_to_a_host_or_gate_leaf() {
    let mut checked = 0;
    for path in yaml_files() {
        let Ok(spec) = crate::loader::load_file(&path) else { continue };
        let Ok(card) = crate::compile::compile(&spec) else { continue };
        let authored = host_leaf_counts(&serde_json::to_value(&spec).unwrap());
        let compiled = host_leaf_counts(&serde_json::to_value(&card).unwrap());
        if authored.0 == 0 {
            continue;
        }
        checked += 1;
        assert_eq!(
            authored.1,
            compiled.1,
            "{}: the pass changed the number of costed has_digivolve_candidate leaves (authored {authored:?}, compiled {compiled:?})",
            path.display()
        );
    }
    assert!(checked > 20, "the sweep reached too few cards ({checked})");
}
