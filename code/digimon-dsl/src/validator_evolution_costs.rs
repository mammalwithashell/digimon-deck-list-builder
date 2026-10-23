//! Bounded audit of the two affordability predicates. The legacy validator
//! remains responsible for other DSL constructs; this visitor guarantees the
//! new read-only cost contract wherever a typed predicate/formula is embedded.

use super::*;
use crate::predicate::{DigivolveCandidatePredicate, DigivolveCostPredicate, PredicateSpec};
use crate::step::CostDelta;
use serde_json::Value;

pub(super) fn validate_card(
    spec: &CardSpec,
    ctx: &ValidationContext<'_>,
    errors: &mut Vec<ValidationError>,
) {
    let value = serde_json::to_value(spec).expect("typed card serializes");
    if !contains_cost_predicate(&value) {
        return;
    }
    let mut added = Vec::new();
    let mut audit = Audit {
        card_id: &spec.card,
        ctx,
        errors: &mut added,
    };
    audit.walk(&value, "", &BTreeSet::new(), false);
    // A cost formula's normal semantic checks and its complete subtree walk
    // can reach the same leaf. Retain one diagnostic without changing the
    // ordering or multiplicity of unrelated legacy validation errors.
    for error in added {
        if !errors.contains(&error) {
            errors.push(error);
        }
    }
}

fn contains_cost_predicate(value: &Value) -> bool {
    match value {
        Value::Object(fields) => {
            fields.contains_key("can_digivolve_onto_with_cost")
                || fields
                    .get("has_digivolve_candidate")
                    .is_some_and(|v| v.get("cost").is_some())
                || fields.values().any(contains_cost_predicate)
        }
        Value::Array(values) => values.iter().any(contains_cost_predicate),
        _ => false,
    }
}

fn field_path(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.into()
    } else {
        format!("{prefix}.{key}")
    }
}

struct Audit<'a, 'b> {
    card_id: &'a str,
    ctx: &'a ValidationContext<'b>,
    errors: &'a mut Vec<ValidationError>,
}

impl Audit<'_, '_> {
    fn walk(&mut self, value: &Value, path: &str, scope: &BTreeSet<String>, reducer: bool) {
        match value {
            Value::Object(fields) => {
                // This is the actual typed clause discriminator. The
                // cost_assignment deserialize alias serializes canonically.
                let reducer =
                    reducer || fields.get("kind").and_then(Value::as_str) == Some("cost_reduction");
                if let Some(value) = fields.get("can_digivolve_onto_with_cost") {
                    if let Ok(probe) =
                        serde_json::from_value::<DigivolveCostPredicate>(value.clone())
                    {
                        let leaf = field_path(path, "can_digivolve_onto_with_cost");
                        if probe.target != "source" {
                            report_if_undeclared_binding(
                                &probe.target,
                                &format!("{leaf}.target"),
                                self.card_id,
                                scope,
                                self.errors,
                            );
                        }
                        self.cost(&probe.cost, &format!("{leaf}.cost"), scope, reducer);
                    }
                }
                if let Some(value) = fields.get("has_digivolve_candidate") {
                    if let Ok(probe) =
                        serde_json::from_value::<DigivolveCandidatePredicate>(value.clone())
                    {
                        if let Some(cost) = &probe.cost {
                            validate_predicate(
                                &PredicateSpec {
                                    has_digivolve_candidate: Some(probe.clone()),
                                    ..PredicateSpec::default()
                                },
                                path,
                                self.card_id,
                                self.ctx,
                                self.errors,
                            );
                            self.cost(
                                cost,
                                &field_path(path, "has_digivolve_candidate.cost"),
                                scope,
                                reducer,
                            );
                        }
                    }
                }
                for (key, child) in fields {
                    let child_path = field_path(path, key);
                    if let Some(steps) = step_list(child) {
                        self.steps(&steps, &child_path, &mut scope.clone(), reducer);
                    } else {
                        self.walk(child, &child_path, scope, reducer);
                    }
                }
            }
            Value::Array(values) => {
                for (index, child) in values.iter().enumerate() {
                    self.walk(child, &format!("{path}[{index}]"), scope, reducer);
                }
            }
            _ => {}
        }
    }

    fn cost(&mut self, cost: &CostDelta, path: &str, scope: &BTreeSet<String>, reducer: bool) {
        validate_evolution_predicate_cost(cost, path, self.card_id, self.ctx, self.errors);
        if let CostDelta::Reduce { reduce } = cost {
            let value = serde_json::to_value(reduce).expect("typed cost formula serializes");
            self.cost_formula_tree(&value, &format!("{path}.reduce"), scope);
        }
        if reducer {
            self.errors.push(ValidationError {
                card_id: self.card_id.into(),
                path: path.into(),
                message: "cost-aware evolution predicates cannot be used inside cost_reduction: affordability evaluation itself visits cost reducers".into(),
            });
        }
    }

    /// Only new cost payloads receive this complete semantic/binding walk.
    /// Trying each typed object also visits predicate children and comparator
    /// formulas omitted by the legacy visitors, without broadening their
    /// behavior for unrelated cards.
    fn cost_formula_tree(&mut self, value: &Value, path: &str, scope: &BTreeSet<String>) {
        match value {
            Value::Object(fields) => {
                if let Ok(predicate) = serde_json::from_value::<PredicateSpec>(value.clone()) {
                    validate_predicate(&predicate, path, self.card_id, self.ctx, self.errors);
                    validate_predicate_binding_scope(
                        &predicate,
                        path,
                        self.card_id,
                        scope,
                        self.errors,
                    );
                }
                if let Ok(formula) =
                    serde_json::from_value::<crate::formula::FormulaSpec>(value.clone())
                {
                    validate_formula_binding_scope(
                        &formula,
                        path,
                        self.card_id,
                        scope,
                        self.errors,
                    );
                }
                for (key, child) in fields {
                    // The existing formula visitor reports compound operands
                    // as cost.reduce[i], without the operator in the path.
                    let child_path = if matches!(
                        key.as_str(),
                        "max" | "min" | "floor_div" | "subtract" | "multiply"
                    ) {
                        path.to_owned()
                    } else {
                        field_path(path, key)
                    };
                    self.cost_formula_tree(child, &child_path, scope);
                }
            }
            Value::Array(values) => {
                for (index, child) in values.iter().enumerate() {
                    self.cost_formula_tree(child, &format!("{path}[{index}]"), scope);
                }
            }
            _ => {}
        }
    }

    fn steps(
        &mut self,
        steps: &[StepSpec],
        path: &str,
        scope: &mut BTreeSet<String>,
        reducer: bool,
    ) {
        for (index, step) in steps.iter().enumerate() {
            let path = format!("{path}[{index}]");
            let value = serde_json::to_value(step).expect("typed step serializes");
            let (verb, args) = value
                .as_object()
                .expect("step object")
                .iter()
                .next()
                .expect("step verb");
            if let Value::Object(fields) = args {
                let mut child_scope = scope.clone();
                for declaration in [
                    "bind_as",
                    "bind_left_as",
                    "bind_right_as",
                    "bind_placed_as",
                    "bind_count_as",
                ] {
                    if let Some(name) = fields.get(declaration).and_then(Value::as_str) {
                        child_scope.insert(name.into());
                    }
                }
                if verb == "raw_rust" {
                    if let Some(Value::Array(names)) = fields.get("binds") {
                        child_scope
                            .extend(names.iter().filter_map(Value::as_str).map(str::to_owned));
                    }
                }
                for (key, child) in fields {
                    let child_path = field_path(&path, key);
                    if let Some(steps) = step_list(child) {
                        // Each branch starts from the same incoming scope;
                        // declarations in one sibling cannot legalize another.
                        let mut branch_scope =
                            if verb == "search_own_security_stack" && key == "on_no_match" {
                                scope.clone()
                            } else {
                                child_scope.clone()
                            };
                        self.steps(&steps, &child_path, &mut branch_scope, reducer);
                    } else {
                        self.walk(child, &child_path, scope, reducer);
                    }
                }
            } else if let Some(steps) = step_list(args) {
                self.steps(
                    &steps,
                    &field_path(&path, verb),
                    &mut scope.clone(),
                    reducer,
                );
            } else {
                self.walk(args, &path, scope, reducer);
            }
            // Preserve the established declaration/branch-merge rules. This
            // scratch call advances scope only; the legacy entry point already
            // owns its unrelated diagnostics. In particular, scheduled or
            // granted bodies never export future declarations to this tail.
            validate_step_binding_scope(step, &path, self.card_id, scope, &mut Vec::new());
        }
    }
}

fn step_list(value: &Value) -> Option<Vec<StepSpec>> {
    value.as_array()?;
    serde_json::from_value(value.clone()).ok()
}
