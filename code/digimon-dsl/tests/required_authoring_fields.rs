use digimon_dsl::authoring_fields::{check, OPTIONAL_STEPS};

fn errors(raw: &str) -> Vec<digimon_dsl::ValidationError> {
    check(&serde_yml::from_str::<serde_json::Value>(raw).unwrap())
}

#[test]
fn every_optional_step_requires_an_explicit_boolean_including_nested_branches() {
    for verb in OPTIONAL_STEPS {
        for optional in ["", "optional: null", "optional: 'false'"] {
            let yaml =
                format!("effects: [{{process: [{{if: {{then: [{{{verb}: {{{optional}}}}}]}}}}]}}]");
            let findings = errors(&yaml);
            assert_eq!(findings.len(), 1, "{verb}: {optional}");
            assert!(findings[0]
                .message
                .contains(&format!("step `{verb}` must declare `optional:`")));
        }
        for optional in ["true", "false"] {
            assert!(errors(&format!(
                "effects: [{{process: [{{{verb}: {{optional: {optional}}}}}]}}]"
            ))
            .is_empty());
        }
    }
    assert!(errors("effects: [{process: []}]").is_empty());
    assert!(errors("effects: [{optional: true, process: [{select_hand: {}}]}]").len() == 1);
}

#[test]
fn reveal_search_buckets_require_optional_but_reveal_selection_minimums_do_not() {
    assert_eq!(
        errors("process: [{reveal_search: {buckets: [{}, {optional: true}]}}]").len(),
        1
    );
    assert!(errors("process: [{reveal_search: {buckets: [{optional: false}]}}]").is_empty());
    assert!(errors("process: [{select_reveal_buckets: {buckets: [{min: 0}]}}]").is_empty());
}

#[test]
fn level_gated_source_digivolution_requires_explicit_color_on_every_alternative() {
    for from in [
        "{level_eq: 3}",
        "{level: {eq: 3}}",
        "{level_eq: 3, any_color: false}",
        "{level_eq: 3, any_of: [{color_is: red}, {trait_has: X}]}",
    ] {
        let findings = errors(&format!("alt_paths: [{{kind: digivolve, from: {from}}}]"));
        assert_eq!(findings.len(), 1, "{from}");
        assert!(findings[0].message.contains("any_color: true"));
    }
    for from in [
        "{level_eq: 3, color_is: red}",
        "{level_eq: 3, any_color: true}",
        "{level_eq: 3, any_of: [{color_is: red}, {color_is: blue}]}",
        "{all_of: [{level_eq: 3}, {color_is: red}]}",
        "{name_is: X}",
    ] {
        assert!(
            errors(&format!(
                "dual: {{option: {{alt_paths: [{{kind: digivolve, from: {from}}}]}}}}"
            ))
            .is_empty(),
            "{from}"
        );
    }
    assert!(
        errors("alt_paths: [{kind: digivolve, direction: into, from: {level_eq: 6}}]").is_empty()
    );
    assert!(errors("alt_paths: [{kind: burst_digivolve, from: {level_eq: 6}}]").is_empty());
    assert!(errors("alt_paths: []").is_empty());
}

#[test]
fn explicit_choices_survive_formatting_and_do_not_change_compilation() {
    let predicate: digimon_dsl::predicate::PredicateSpec =
        serde_yml::from_str("any_color: true").unwrap();
    assert!(predicate.is_empty());
    let raw = "card: X-1\nname: X\nkind: digimon\ncolor: [red]\ncost: 3\nalt_paths: [{kind: digivolve, from: {level_eq: 3, any_color: true}, cost: 1}]\neffects: [{when: on_play, process: [{select_hand: {of: you, optional: false, filter: {}, bind_as: pick, prompt: Choose}}]}]";
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(raw).unwrap();
    let formatted = digimon_dsl::pretty::format_spec(&spec);
    assert!(formatted.contains("optional: false"));
    assert!(errors(&formatted).is_empty());
    let mut before = spec.clone();
    before.alt_paths[0].from.as_mut().unwrap().any_color = false;
    assert_eq!(
        digimon_dsl::compile::compile(&before).unwrap(),
        digimon_dsl::compile::compile(&spec).unwrap()
    );
}

#[test]
fn file_loader_rejects_omission() {
    let path = std::env::temp_dir().join(format!("required-fields-{}.yaml", std::process::id()));
    std::fs::write(&path, "card: X-1\nname: X\nkind: digimon\ncolor: [red]\neffects: [{when: on_play, process: [{select_hand: {}}]}]").unwrap();
    let result = digimon_dsl::loader::load_file(&path);
    std::fs::remove_file(path).unwrap();
    assert!(
        matches!(result, Err(digimon_dsl::DslError::Validation(errors)) if errors[0].message.contains("step `select_hand` must declare `optional:`"))
    );
}

#[test]
fn migration_snapshot_accepts_omissions_but_verification_requires_explicit_fields() {
    let root = std::env::temp_dir().join(format!("authoring-snapshot-{}", std::process::id()));
    std::fs::create_dir_all(root.join("bt99")).unwrap();
    let path = root.join("bt99/BT99-001.yaml");
    let snapshot = root.join("before.json");
    let raw = "card: BT99-001\nname: Snapshot\nkind: digimon\ncolor: [red]\nalt_paths: [{kind: digivolve, from: {level_eq: 3}, cost: 1}]\neffects: [{when: on_play, process: [{select_hand: {of: you, filter: {}, bind_as: pick, prompt: Choose}}]}]";
    std::fs::write(&path, raw).unwrap();
    let run = |mode| {
        std::process::Command::new(env!("CARGO_BIN_EXE_verify_authoring_fields"))
            .arg(mode)
            .arg(&root)
            .arg(&snapshot)
            .output()
            .unwrap()
    };
    let output = run("snapshot");
    assert!(output.status.success(), "{:?}", output);
    assert!(!run("verify").status.success());
    std::fs::write(
        &path,
        raw.replace("level_eq: 3", "any_color: true, level_eq: 3")
            .replace("of: you", "optional: false, of: you"),
    )
    .unwrap();
    let output = run("verify");
    assert!(output.status.success(), "{:?}", output);
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("1 identical compiled cards; 0 differences"));
    std::fs::write(&path, "card: [").unwrap();
    assert!(!run("snapshot").status.success());
    std::fs::remove_file(&path).unwrap();
    assert!(!run("snapshot").status.success());
    std::fs::remove_dir_all(root).unwrap();
}
