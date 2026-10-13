//! Supplemental strict correspondence only. Existing opaque visitor output is untouched.
use digimon_dsl::compiled::CompiledCard;
use serde::Serialize;
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Branch {
    Then,
    Else,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IfLocation {
    pub effect: usize,
    pub ancestors: Vec<(usize, Branch)>,
    pub step: usize,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EqualsWitness {
    pub version: u32,
    pub card_id: String,
    pub location: IfLocation,
    pub original_source_bytes: Vec<u8>,
    pub original_condition: serde_yml::Value,
    pub compiled_card: CompiledCard,
    pub binding: String,
    pub integer: i64,
    pub original_opaque_paths: Vec<String>,
    pub compiled_operand_paths: Vec<String>,
    pub original_opaque_nodes_preserved: bool,
    pub reference_closure_complete: bool,
    pub runtime_admission: bool,
    pub source_authority: bool,
}

/// Prove one exact occurrence. This does not validate unrelated predicates or
/// discharge the original visitor's opaque/closure/engine obligations.
pub fn witness_equals_if(
    source: &[u8],
    card_id: &str,
    location: &IfLocation,
    supplied: &CompiledCard,
) -> Result<EqualsWitness, String> {
    use digimon_dsl::compiled::{
        CompiledBindingCompare, CompiledClause, CompiledPredicate, CompiledStep,
    };
    use digimon_dsl::{clause::ClauseSpec, step::StepSpec, CardSpec};
    use serde_yml::Value;
    let spec: CardSpec = serde_yml::from_slice(source).map_err(|e| format!("source parse: {e}"))?;
    if spec.card != card_id || spec.dual.is_some() {
        return Err("owner mismatch or unsupported Dual location".into());
    }
    let replay =
        digimon_dsl::compile::compile(&spec).map_err(|e| format!("source compile: {e:?}"))?;
    if &replay != supplied {
        return Err("whole compiled card differs from original source replay".into());
    }
    let Some(ClauseSpec::Triggered(authored)) = spec.effects.get(location.effect) else {
        return Err("authored triggered clause required".into());
    };
    let Some(CompiledClause::Triggered(compiled)) = replay.effects.get(location.effect) else {
        return Err("compiled triggered clause required".into());
    };
    if spec.effects.len() != replay.effects.len() {
        return Err("clause correspondence unsupported".into());
    }
    let mut a = authored.process.as_slice();
    let mut c = compiled.process.as_slice();
    let mut ap = format!("authored.effects[{}]@Triggered.0.process", location.effect);
    let mut cp = format!("compiled.effects[{}]@Triggered.0.process", location.effect);
    for (index, branch) in &location.ancestors {
        if a.len() != c.len() {
            return Err("ordered step correspondence differs".into());
        }
        let Some(StepSpec::If(ai)) = a.get(*index) else {
            return Err("authored ancestor If required".into());
        };
        let Some(CompiledStep::If {
            then, else_branch, ..
        }) = c.get(*index)
        else {
            return Err("compiled ancestor If required".into());
        };
        match branch {
            Branch::Then => {
                a = ai.then.as_slice();
                c = then.as_slice();
                ap = format!("{ap}[{index}]@If.0.then");
                cp = format!("{cp}[{index}]@If.then");
            }
            Branch::Else => {
                a = ai.else_.as_deref().ok_or("authored else branch absent")?;
                c = else_branch.as_slice();
                ap = format!("{ap}[{index}]@If.0.else_");
                cp = format!("{cp}[{index}]@If.else_branch");
            }
        }
    }
    if a.len() != c.len() {
        return Err("ordered target step correspondence differs".into());
    }
    let index = location.step;
    let Some(StepSpec::If(ai)) = a.get(index) else {
        return Err("authored target If required".into());
    };
    let Some(CompiledStep::If { condition, .. }) = c.get(index) else {
        return Err("compiled target If required".into());
    };
    let Value::Mapping(map) = &ai.condition else {
        return Err("literal condition mapping required".into());
    };
    if map.len() != 1 {
        return Err("exact equals condition without sibling fields required".into());
    }
    let Some(Value::Sequence(operands)) = map.get(Value::String("equals".into())) else {
        return Err("literal equals sequence required".into());
    };
    if operands.len() != 2 {
        return Err("exactly two equals operands required".into());
    }
    let Value::String(binding) = &operands[0] else {
        return Err("first operand must be a binding string".into());
    };
    if binding.is_empty() {
        return Err("empty binding unsupported".into());
    }
    let Value::Number(number) = &operands[1] else {
        return Err("second operand must be an integer number".into());
    };
    let integer = number
        .as_i64()
        .ok_or("strict i64 operand required; no compiler fallback")?;
    let expected = CompiledPredicate {
        equals: Some(vec![
            CompiledBindingCompare::Binding(binding.clone()),
            CompiledBindingCompare::Literal(integer),
        ]),
        ..Default::default()
    };
    if condition != &expected {
        return Err("complete compiled predicate differs from strict ordered operands".into());
    }
    let ap = format!("{ap}[{index}]@If.0.condition.equals");
    let cp = format!("{cp}[{index}]@If.condition.equals");
    Ok(EqualsWitness {
        version: 1,
        card_id: card_id.into(),
        location: location.clone(),
        original_source_bytes: source.to_vec(),
        original_condition: ai.condition.clone(),
        compiled_card: replay.clone(),
        binding: binding.clone(),
        integer,
        original_opaque_paths: vec![format!("{ap}[0]"), format!("{ap}[1]")],
        compiled_operand_paths: vec![format!("{cp}[0]@Binding.0"), format!("{cp}[1]@Literal.0")],
        original_opaque_nodes_preserved: true,
        reference_closure_complete: false,
        runtime_admission: false,
        source_authority: false,
    })
}
