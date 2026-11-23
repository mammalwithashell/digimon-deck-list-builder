use digimon_dsl::printed_text_lint::check_printed_text;
use digimon_dsl::security_icon_lint::PrintedSection;

fn check(clause: &str, printed: &str) -> Vec<&'static str> {
    let yaml = format!("card: X-1\nname: X\nkind: digimon\nlevel: 3\ncolor: [yellow]\ncost: 3\ndp: 1000\neffects:\n{clause}\n");
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(&yaml).unwrap_or_else(|e| panic!("{clause}: {e}"));
    check_printed_text(
        &spec,
        &[PrintedSection {
            label: "Effect".into(),
            text: printed.into(),
        }],
    )
    .into_iter()
    .map(|f| f.kind)
    .collect()
}

#[test]
fn printed_text_findings_and_clean_cases() {
    for (clause, printed, expected) in [
        ("- when: on_play", "", vec!["missing_text"]),
        ("- when: on_play\n  text: Delete it.", "Draw 1.", vec!["text_not_printed"]),
        ("- when: on_play\n  text: You may draw 1.", "You may draw 1.", vec!["may_without_optional"]),
        ("- when: on_play\n  text: Draw 1.\n  optional: true", "Draw 1.", vec!["optional_without_may"]),
        ("- when: on_play\n  text: '[On Deletion] Draw 1.'", "[On Deletion] Draw 1.", vec!["timing_tag_mismatch"]),
        ("- when: on_play\n  text: '[Once Per Turn] Draw 1.'", "[Once Per Turn] Draw 1.", vec!["once_per_turn_mismatch"]),
        ("- when: on_play\n  text: Draw 1.\n  once_per_turn: true", "Draw 1.", vec!["once_per_turn_mismatch"]),
        ("- when: on_play\n  text: '[On Play] Draw 1.'", "[On Play] Draw 1.", vec![]),
        ("- when: on_play\n  text: You may not draw. You can't attack.", "You may not draw. You can’t attack.", vec![]),
        ("- when: on_play\n  text: '[Once Per Turn] Draw 1.'\n  max_per_turn: 1", "[Once Per Turn] Draw 1.", vec![]),
        ("- when: on_play\n  text: '[Your Turn] Draw 1.'\n  active_when: {your_turn: true}", "[Your Turn] Draw 1.", vec![]),
        ("- when: on_play\n  text: '[Your Turn] Draw 1.'", "[Your Turn] Draw 1.", vec!["timing_tag_mismatch"]),
        ("- when: on_play\n  text: You may draw 1.\n  process:\n    - if:\n        condition: {your_turn: true}\n        then: []\n        else:\n          - optional:\n              - draw: {of: you, count: 1}", "You may draw 1.", vec![]),
        ("- when: on_play\n  text: You may choose.\n  process:\n    - select_hand: {of: you, bind_as: pick, optional: true, filter: {}, prompt: Choose}", "You may choose.", vec![]),
        ("- kind: cost_reduction\n  text: You may reduce the cost.\n  optional: true\n  amount: 1", "You may reduce the cost.", vec![]),
        ("- when: on_play\n  text: 'Draw — 1.'", "DRAW –   1.", vec![]),
    ] {
        assert_eq!(check(clause, printed), expected, "{clause}");
    }
}

#[test]
fn printed_text_round_trip_and_schema() {
    let yaml = "card: X-1\nname: X\nkind: digimon\nlevel: 3\ncolor: [yellow]\ncost: 3\ndp: 1000\neffects:\n- when: on_play\n  text: '[On Play] Draw 1.'\n  process: [{draw: {of: you, count: 1}}]\n- kind: cost_reduction\n  text: You may reduce the cost.\n  optional: true\n  amount: 1\n";
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).unwrap();
    let formatted = digimon_dsl::pretty::format_spec(&spec);
    assert_eq!(spec, serde_yml::from_str(&formatted).unwrap());
    let digimon_dsl::clause::ClauseSpec::Declarative(c) = &spec.effects[1] else {
        panic!()
    };
    assert!(!c.body.contains_key("text"));
    let schema: serde_json::Value =
        serde_json::from_str(&digimon_dsl::schema::export_json_schema()).unwrap();
    for name in ["TriggeredClause", "DeclarativeClause"] {
        assert!(schema["definitions"][name]["properties"]
            .get("text")
            .is_some());
    }
    let mut without = spec.clone();
    for clause in &mut without.effects {
        match clause {
            digimon_dsl::clause::ClauseSpec::Triggered(c) => c.text = None,
            digimon_dsl::clause::ClauseSpec::Declarative(c) => c.text = None,
        }
    }
    assert_eq!(
        digimon_dsl::compile::compile(&spec).unwrap(),
        digimon_dsl::compile::compile(&without).unwrap()
    );
}

#[test]
fn printed_text_keyword_reminder_choices_belong_to_keyword_machinery() {
    let text = "＜Raid＞ (When attacking, you may change the target.)";
    assert!(check(
        &format!("- kind: grant_keyword\n  keyword: Raid\n  text: '{text}'"),
        text
    )
    .is_empty());
    assert_eq!(
        check(
            "- when: when_attacking\n  text: You may change the target.",
            "You may change the target."
        ),
        vec!["may_without_optional"]
    );
}

#[test]
fn printed_text_by_payment_is_optional_without_the_word_may() {
    let text = "[On Play] By trashing 1 card, draw 2.";
    assert!(check(
        &format!("- when: on_play\n  optional: true\n  text: '{text}'"),
        text
    )
    .is_empty());
    assert_eq!(
        check(
            "- when: on_play\n  optional: true\n  text: Reduce the cost by 3.",
            "Reduce the cost by 3."
        ),
        vec!["optional_without_may"]
    );
}

#[test]
fn printed_text_zero_floor_and_optional_zero_are_declinable() {
    for fields in ["min: 0", "min: 2, optional_zero: true"] {
        assert!(check(&format!("- when: on_play\n  text: You may place cards.\n  process:\n    - select_zone_cards: {{zones: [hand, trash], filter: {{}}, bind_as: picks, prompt: Choose, {fields}}}"), "You may place cards.").is_empty());
    }
    assert_eq!(check("- when: on_play\n  text: You may place cards.\n  process:\n    - select_zone_cards: {zones: [hand], filter: {}, min: 2, prompt: Choose}", "You may place cards."), vec!["may_without_optional"]);
}

#[test]
fn printed_text_explicit_decline_branch_is_optional() {
    let text = "[Security] You may play a Tamer. Then, add this card to the hand.";
    assert!(check(&format!("- when: on_security\n  text: '{text}'\n  process:\n    - select_effect_choice: {{labels: [Play Tamer, Decline]}}"), text).is_empty());
    assert_eq!(check(&format!("- when: on_security\n  text: '{text}'\n  process:\n    - select_effect_choice: {{labels: [Play Tamer, Draw]}}"), text), vec!["may_without_optional"]);
}

#[test]
fn printed_text_may_requires_word_boundaries() {
    assert!(check(
        "- when: on_play\n  text: Names may1 and may_name are not permissions.",
        "Names may1 and may_name are not permissions."
    )
    .is_empty());
}
