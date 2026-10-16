use digimon_dsl::{raw_rust_registry::StubRegistry, validator::{validate, ValidationContext}, CardSpec};

fn spec(source: &str, target: &str) -> CardSpec {
    serde_yml::from_str(&format!(r#"
card: TOP-OF-VALIDATION
name: Top Binding
kind: digimon
level: 4
color: [black]
cost: 4
dp: 3000
effects:
  - when: when_attacking
    process:
      - select_own_permanent:
          filter: {{kind: digimon}}
          bind_as: donor
          prompt: Choose donor
      - place_as_bottom_source:
          source: {source}
          target: {target}
          bind_placed_as: paid
"#)).unwrap()
}

#[test]
fn top_of_rejects_undeclared_donor() {
    let card = spec("{top_of: typo}", "source");
    let registry = StubRegistry::empty();
    let errors = validate(&card, &ValidationContext { raw_rust: &registry }).unwrap_err();
    assert!(errors.iter().any(|e| e.path.ends_with(".source.top_of") && e.message.contains("undeclared binding")));
}

#[test]
fn bottom_source_rejects_ambiguous_structured_references_on_both_sides() {
    let card = spec("{top_of: donor, binding: donor}", "{permanent: donor, binding: donor}");
    let registry = StubRegistry::empty();
    let errors = validate(&card, &ValidationContext { raw_rust: &registry }).unwrap_err();
    for side in [".source", ".target"] {
        assert!(errors.iter().any(|e| e.path.ends_with(side) && e.message.contains("only one binding field")));
    }
}

#[test]
fn top_of_declared_donor_compiles_to_exact_top_reference() {
    let card = spec("{top_of: donor}", "source");
    let registry = StubRegistry::empty();
    validate(&card, &ValidationContext { raw_rust: &registry }).unwrap();
    let compiled = digimon_dsl::compile::compile(&card).unwrap();
    let digimon_dsl::compiled::CompiledClause::Triggered(clause) = &compiled.effects[0] else { panic!("trigger") };
    assert!(matches!(&clause.process[1], digimon_dsl::compiled::CompiledStep::PlaceAsBottomSource { source: digimon_dsl::compiled::CompiledBindingRef::TopOf(name), .. } if name == "donor"));
}
