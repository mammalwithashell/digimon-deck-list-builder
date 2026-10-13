#[path = "../src/bin/implementation_inventory/equals_witness.rs"]
mod witness;
use digimon_dsl::{compile::compile, compiled::*, CardSpec};
use witness::{witness_equals_if, Branch, IfLocation};
fn source(condition: &str) -> Vec<u8> {
    format!("card: TEST-01\nname: Test\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 2\ndp: 3000\neffects:\n  - when: on_play\n    process:\n      - if:\n          condition: {condition}\n          then: [{{gain_memory: 1}}]\n").into_bytes()
}
fn compiled(s: &[u8]) -> CompiledCard {
    compile(&serde_yml::from_slice::<CardSpec>(s).unwrap()).unwrap()
}
fn loc() -> IfLocation {
    IfLocation {
        effect: 0,
        ancestors: vec![],
        step: 0,
    }
}
#[test]
fn strict_positive_card_shaped_and_numeric_binding_strings() {
    for bind in ["branch", "ST1-02", "0"] {
        for n in [i64::MIN, 0, 1, i64::MAX] {
            let raw = source(&format!("{{equals: ['{bind}', {n}]}}"));
            let c = compiled(&raw);
            let w = witness_equals_if(&raw, "TEST-01", &loc(), &c).unwrap();
            assert_eq!(w.binding, bind);
            assert_eq!(w.integer, n);
            assert_eq!(w.original_source_bytes, raw);
            assert!(!w.reference_closure_complete && !w.runtime_admission && !w.source_authority);
        }
    }
}
#[test]
fn compiler_zero_fallback_is_not_a_witness() {
    for value in [
        "true",
        "false",
        "null",
        "1.5",
        "18446744073709551615",
        "{}",
        "[]",
    ] {
        let raw = source(&format!("{{equals: [branch, {value}]}}"));
        let c = compiled(&raw);
        let CompiledClause::Triggered(t) = &c.effects[0] else {
            panic!()
        };
        let CompiledStep::If { condition, .. } = &t.process[0] else {
            panic!()
        };
        assert_eq!(
            condition.equals.as_ref().unwrap()[1],
            CompiledBindingCompare::Literal(0)
        );
        assert!(
            witness_equals_if(&raw, "TEST-01", &loc(), &c).is_err(),
            "{value}"
        );
    }
}
#[test]
fn malformed_shapes_refuse() {
    for condition in [
        "{equals: []}",
        "{equals: [branch]}",
        "{equals: [branch,0,1]}",
        "{equals: [0,branch]}",
        "{equals: ['',0]}",
        "{equals: [branch,'0']}",
        "{not_equals: [branch,0]}",
        "{equals: [branch,0], binding_present: branch}",
        "{equals: [branch,0], future_unknown: 1}",
        "{equals: [branch, !test 0]}",
        "{equals: [branch,0], equals: [other,1]}",
    ] {
        let raw = source(condition);
        let parsed = serde_yml::from_slice::<CardSpec>(&raw);
        if let Ok(spec) = parsed {
            if let Ok(c) = compile(&spec) {
                assert!(
                    witness_equals_if(&raw, "TEST-01", &loc(), &c).is_err(),
                    "{condition}"
                );
            }
        }
    }
}
#[test]
fn foreign_compiled_owner_binding_and_path_refuse() {
    let raw = source("{equals: [branch,0]}");
    let c = compiled(&raw);
    let mut foreign = c.clone();
    foreign.card = "OTHER".into();
    assert!(witness_equals_if(&raw, "TEST-01", &loc(), &foreign).is_err());
    let mut foreign = c.clone();
    let CompiledClause::Triggered(t) = &mut foreign.effects[0] else {
        panic!()
    };
    let CompiledStep::If { condition, .. } = &mut t.process[0] else {
        panic!()
    };
    condition.equals = Some(vec![
        CompiledBindingCompare::Binding("foreign".into()),
        CompiledBindingCompare::Literal(0),
    ]);
    assert!(witness_equals_if(&raw, "TEST-01", &loc(), &foreign).is_err());
    assert!(witness_equals_if(&raw, "OTHER", &loc(), &c).is_err());
    for bad in [
        IfLocation { effect: 1, ..loc() },
        IfLocation { step: 1, ..loc() },
        IfLocation {
            ancestors: vec![(0, Branch::Then)],
            ..loc()
        },
    ] {
        assert!(witness_equals_if(&raw, "TEST-01", &bad, &c).is_err());
    }
}
#[test]
fn actual_eight_pairs_keep_distinct_occurrences() {
    let cases: [(&[u8], &str, Vec<IfLocation>); 3] = [
        (
            include_bytes!("../../digimon-engine/cards/bt25/BT25-078.yaml"),
            "BT25-078",
            vec![
                IfLocation { step: 2, ..loc() },
                IfLocation { step: 3, ..loc() },
            ],
        ),
        (
            include_bytes!("../../digimon-engine/cards/bt26/BT26-073.yaml"),
            "BT26-073",
            vec![
                IfLocation {
                    ancestors: vec![(0, Branch::Then)],
                    step: 1,
                    ..loc()
                },
                IfLocation {
                    ancestors: vec![(0, Branch::Then)],
                    step: 2,
                    ..loc()
                },
                IfLocation {
                    ancestors: vec![(0, Branch::Else)],
                    step: 1,
                    ..loc()
                },
            ],
        ),
        (
            include_bytes!("../../digimon-engine/cards/p/P-156.yaml"),
            "P-156",
            vec![
                IfLocation { step: 2, ..loc() },
                IfLocation { step: 3, ..loc() },
                IfLocation {
                    effect: 1,
                    ancestors: vec![(0, Branch::Then)],
                    step: 1,
                },
            ],
        ),
    ];
    let mut witnesses = vec![];
    let mut count = 0;
    for (raw, id, locations) in cases {
        let c = compiled(raw);
        for location in locations {
            let w = witness_equals_if(raw, id, &location, &c).unwrap();
            assert_eq!(w.location, location);
            assert!(w.original_opaque_nodes_preserved);
            witnesses.push(w);
            count += 1;
        }
    }
    assert_eq!(count, 8);
    if let Ok(path) = std::env::var("EQUALS_WITNESS_OUTPUT") {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        f.write_all(&serde_json::to_vec_pretty(&witnesses).unwrap())
            .unwrap();
    }
}
