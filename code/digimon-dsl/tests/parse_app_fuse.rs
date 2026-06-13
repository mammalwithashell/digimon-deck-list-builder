//! Parse/serde + lowering coverage for the `app_fuse` step
//! (effect-initiated App Fuse).

use digimon_dsl::step::{AppFuseZone, StepSpec};

fn parse_one(yaml: &str) -> StepSpec {
    serde_yml::from_str(yaml).expect("app_fuse step parses")
}

#[test]
fn app_fuse_defaults_to_hand_nonoptional() {
    // Bare `app_fuse: {}` → hand zone, no filter, optional defaults false
    // (codebase convention: every other step's `optional` defaults false;
    // the shipped riders set `optional: true` explicitly).
    let s = parse_one("app_fuse: {}");
    match s {
        StepSpec::AppFuse(a) => {
            assert_eq!(a.from, AppFuseZone::Hand, "default zone is hand");
            assert!(!a.optional, "optional defaults false");
            assert!(a.result_filter.is_none(), "no filter by default");
        }
        _ => panic!("expected StepSpec::AppFuse"),
    }
}

#[test]
fn app_fuse_parses_hand_optional() {
    let s = parse_one("app_fuse: { from: hand, optional: true }");
    match s {
        StepSpec::AppFuse(a) => {
            assert_eq!(a.from, AppFuseZone::Hand);
            assert!(a.optional);
        }
        _ => panic!("expected StepSpec::AppFuse"),
    }
}

#[test]
fn app_fuse_parses_trash_with_filter() {
    let yaml = r#"
app_fuse:
  from: trash
  optional: true
  result_filter:
    any_of:
      - trait_has: System
      - trait_has: Life
      - trait_has: Transmutation
"#;
    match parse_one(yaml) {
        StepSpec::AppFuse(a) => {
            assert_eq!(a.from, AppFuseZone::Trash);
            assert!(a.optional);
            assert!(a.result_filter.is_some(), "filter present");
        }
        _ => panic!("expected StepSpec::AppFuse"),
    }
}

#[test]
fn app_fuse_lowers_to_compiled_step() {
    use digimon_dsl::compile::compile;
    use digimon_dsl::compiled::{CompiledAppFuseZone, CompiledClause, CompiledStep};
    use digimon_dsl::spec::CardSpec;

    let yaml = r#"
card: DSL-APPFUSE-001
name: Test App Fuse
kind: tamer
color: [green]
cost: 3
effects:
  - when: end_of_your_turn
    optional: true
    process:
      - app_fuse: { from: trash, optional: true }
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("card parses");
    let compiled = compile(&spec).expect("card compiles");
    let found = compiled.effects.iter().any(|clause| {
        let CompiledClause::Triggered(t) = clause else {
            return false;
        };
        t.process.iter().any(|s| {
            matches!(
                s,
                CompiledStep::AppFuse {
                    from_zone: CompiledAppFuseZone::Trash,
                    optional: true,
                    ..
                }
            )
        })
    });
    assert!(found, "compiled card contains CompiledStep::AppFuse(Trash, optional)");
}
