use digimon_engine::dsl::spec::CardSpec;
use digimon_engine::dsl::step::{BindingRef, RawRustStep, StepSpec};

fn parse_single_step(yaml_body: &str) -> StepSpec {
    let yaml = format!(
        r#"
card: X-1
name: Test
kind: option
color: [red]
cost: 0
effects:
  - when: main_from_hand
    process:
      - {body}
"#,
        body = yaml_body
    );
    let spec: CardSpec = serde_yml::from_str(&yaml).unwrap();
    spec.effects[0].as_triggered().unwrap().process[0].clone()
}

#[test]
fn parse_gain_memory() {
    let step = parse_single_step("gain_memory: 1");
    assert!(matches!(step, StepSpec::GainMemory(1)));
}

#[test]
fn parse_draw() {
    let step = parse_single_step("draw: { of: you, count: 2 }");
    match step {
        StepSpec::Draw(d) => assert_eq!(d.count, 2),
        _ => panic!("expected Draw"),
    }
}

#[test]
fn parse_select_trash_with_binding() {
    let yaml_body = r#"select_trash: { of: you, bind_as: pick, filter: { name_contains: Greymon }, prompt: "Return" }"#;
    let step = parse_single_step(yaml_body);
    match step {
        StepSpec::SelectTrash(s) => {
            assert_eq!(s.bind_as.as_deref(), Some("pick"));
            assert_eq!(s.prompt, "Return");
        }
        _ => panic!("expected SelectTrash"),
    }
}

/// The `if` step nests `condition`, `then`, and `else` inside the `if:` map.
/// A flat form (`if: <pred>\nthen: [...]`) would place `then`/`else` as
/// siblings of `if:` in the step map, which is incompatible with serde's
/// external-tag representation (expects exactly one discriminant key per
/// variant). See the module-level doc comment in `step.rs` for the full
/// rationale.
///
/// Note: this test uses its own full YAML string rather than `parse_single_step`
/// because the multi-line `if:` block requires specific indentation that
/// `parse_single_step`'s `format!("- {body}")` template cannot provide for
/// multi-line bodies.
#[test]
fn parse_if_then_else() {
    let yaml = r#"
card: X-1
name: Test
kind: option
color: [red]
cost: 0
effects:
  - when: main_from_hand
    process:
      - if:
          condition: { equals: [branch, 0] }
          then:
            - gain_memory: 1
          else:
            - gain_memory: 2
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let step = spec.effects[0].as_triggered().unwrap().process[0].clone();
    match step {
        StepSpec::If(i) => {
            assert_eq!(i.then.len(), 1);
            assert_eq!(i.else_.as_ref().map(|v| v.len()), Some(1));
        }
        _ => panic!("expected If"),
    }
}

#[test]
fn parse_raw_rust_step() {
    let step = parse_single_step(
        "raw_rust: { fn: my_fn, consumes: [target], binds: [output] }",
    );
    match step {
        StepSpec::RawRust(RawRustStep { fn_name, consumes, binds }) => {
            assert_eq!(fn_name, "my_fn");
            assert_eq!(consumes, vec!["target".to_string()]);
            assert_eq!(binds, vec!["output".to_string()]);
        }
        _ => panic!("expected RawRust"),
    }
}

#[test]
fn parse_delete_permanent_with_binding_ref() {
    let step = parse_single_step("delete_permanent: { target: tgt }");
    match step {
        StepSpec::DeletePermanent(d) => {
            assert_eq!(d.target, BindingRef::Named("tgt".to_string()));
        }
        _ => panic!("expected DeletePermanent"),
    }
}
