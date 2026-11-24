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
fn printed_text_pick_kind_requires_a_single_selection_and_card_noun() {
    for (text, filter, expected) in [
        (
            "Add 1 Digimon card to your hand.",
            "{}",
            vec!["kind_mismatch"],
        ),
        (
            "Add 1 Tamer card to your hand.",
            "{kind: digimon}",
            vec!["kind_mismatch"],
        ),
        ("Add 1 Option card to your hand.", "{kind: option}", vec![]),
        (
            "Add 1 yellow card with the [Vaccine] trait.",
            "{kind: digimon, color_is: yellow, trait_has: Vaccine}",
            vec!["kind_mismatch"],
        ),
        (
            "Add 1 yellow card with the [Vaccine] trait.",
            "{color_is: yellow, trait_has: Vaccine}",
            vec![],
        ),
        (
            "Add 1 Digimon card to your hand.",
            "{all_of: [{kind: digimon}, {trait_has: Vaccine}]}",
            vec![],
        ),
        (
            "Add 1 Digimon card to your hand.",
            "{any_of: [{kind: digimon}, {kind: tamer}]}",
            vec!["kind_mismatch"],
        ),
        (
            "Add 1 card to your hand.",
            "{any_of: [{kind: digimon}, {kind: tamer}]}",
            vec!["kind_mismatch"],
        ),
        (
            "Add 1 card to your hand.",
            "{any_of: [{kind: digimon}, {trait_has: Vaccine}]}",
            vec![],
        ),
        ("Add 1 Digimon or Tamer card.", "{}", vec![]),
        ("Play 1 card.", "{any_of: [{kind: digimon}, {kind: tamer}]}", vec![]),
        ("Play 1 card.", "{kind: digimon}", vec!["kind_mismatch"]),
        ("Play 1 card.", "{kind: tamer}", vec!["kind_mismatch"]),
        ("Play 1 level 3 card.", "{kind: digimon, level_eq: 3}", vec![]),
        ("Play 1 card with 5000 DP or less.", "{kind: digimon, dp_lte: 5000}", vec![]),
        ("Play 1 card.", "{none_of: [{kind: tamer}]}", vec!["kind_mismatch"]),
        ("Play 1 card.", "{not: {kind: option}}", vec!["kind_mismatch"]),
        ("Play 1 card.", "{none_of: [{kind: option}]}", vec!["kind_mismatch"]),
        ("Play 1 card.", "{any_of: [{not: {kind: option}}, {kind: dual}]}", vec![]),
        ("Play 1 card.", "{not: {kind: tamer}}", vec!["kind_mismatch"]),
        ("Play 1 card.", "{not: {not: {any_of: [{kind: digimon}, {kind: tamer}]}}}", vec![]),
        ("Play 1 card.", "{not: {all_of: [{kind: option}, {trait_has: Vaccine}]}}", vec![]),
        ("Play 1 card with 5000 DP or less.", "{any_of: [{kind: digimon, dp_lte: 5000}, {kind: tamer}]}", vec!["kind_mismatch"]),
        ("Play 1 card with 5000 DP or less.", "{any_of: [{dp_lte: 5000}, {kind: tamer, dp_lte: 5000}]}", vec![]),
        ("Play 1 card with 5000 DP or less.", "{dp_lte: 5000}", vec![]),
        ("Play 1 level 3 card.", "{any_of: [{level_eq: 3}, {kind: tamer}]}", vec!["kind_mismatch"]),
        ("Add 1 Digimon card.", "{all_of: [{not: {kind: option}}, {not: {kind: tamer}}]}", vec!["kind_mismatch"]),
        ("Add 1 Tamer card.", "{any_of: [{kind: tamer}, {kind: dual}]}", vec!["kind_mismatch"]),
        ("Add 1 Digimon card.", "{kind: digimon}", vec![]),
        ("Add 1 Option card.", "{kind: option}", vec![]),
        ("[On Play] Use 1 Option card.", "{kind: option}", vec![]),
        ("[On Play] Use 1 Option card.", "{kind: digimon}", vec!["kind_mismatch"]),
        ("[On Play] Add 1 card to your hand.", "{any_of: [{kind: digimon}, {kind: tamer}]}", vec!["kind_mismatch"]),
        ("[On Play] Add 1 card to your hand.", "{}", vec![]),
        ("[On Play] Use 1 card.", "{kind: option}", vec![]),
        ("[On Play] Play or use 1 white card.", "{kind: option}", vec!["kind_mismatch"]),
        ("[On Play] Use or play 1 white card.", "{kind: option}", vec!["kind_mismatch"]),
        ("[On Play] Play or use 1 white card.", "{color_is: white}", vec![]),
        ("[On Play] Use or play 1 white card.", "{any_of: [{kind: digimon}, {kind: tamer}, {kind: option}]}", vec![]),
        ("Add 1 Digimon card. Trash 1 Tamer card.", "{}", vec![]),
        ("Choose 1 Digimon.", "{}", vec![]),
        ("You may play 1 [Hiroko Sagisaka] or 1 [TS] trait Digimon card with 5000 DP or less.", "{any_of: [{name_is: Hiroko Sagisaka}, {kind: digimon, trait_has: TS, dp_lte: 5000}]}", vec!["may_without_optional"]),
        ("Play 1 [Arisa Kinosaki] or 1 [Puppet] trait Digimon card from your hand.", "{any_of: [{kind: tamer, name_is: Arisa Kinosaki}, {kind: digimon, trait_has: Puppet}]}", vec![]),
    ] {
        let clause = format!("- when: on_play\n  text: '{text}'\n  process:\n    - select_hand: {{of: you, optional: false, filter: {filter}, bind_as: pick, prompt: Choose}}");
        assert_eq!(check(&clause, text), expected, "{text}, {filter}");
    }
    let text = "Choose 1 Digimon card.";
    let clause = format!("- when: on_play\n  text: '{text}'\n  process:\n    - select_hand: {{of: you, optional: false, filter: {{}}, bind_as: a, prompt: Choose}}\n    - select_trash: {{of: you, optional: false, filter: {{}}, bind_as: b, prompt: Choose}}");
    assert!(check(&clause, text).is_empty());
    assert!(check("- when: on_play\n  text: Draw 1.", "Draw 1.").is_empty());
}

#[test]
fn printed_text_name_match_modes() {
    for (text, filter, expected) in [
        (
            "Add 1 card with [Agumon] in its name.",
            "{name_is: Agumon}",
            vec!["name_match_mismatch"],
        ),
        (
            "Add 1 card with [Agumon] in its name.",
            "{name_contains: Agumon}",
            vec![],
        ),
        (
            "Add cards with [Agumon] in their names.",
            "{name_is: Agumon}",
            vec!["name_match_mismatch"],
        ),
        (
            "Add 1 [Agumon] from your trash.",
            "{name_contains: Agumon}",
            vec!["name_match_mismatch"],
        ),
        (
            "Add 1 [Agumon] from your trash.",
            "{name_is: Agumon}",
            vec![],
        ),
        (
            "Add 1 card with the [Agumon] trait.",
            "{name_contains: Agumon}",
            vec![],
        ),
        (
            "Add 1 card with [Agumon] in its text.",
            "{name_contains: Agumon}",
            vec![],
        ),
        (
            "Add 1 card with [Agumon] in its name or the [Agumon] trait.",
            "{name_is: Agumon}",
            vec![],
        ),
    ] {
        let clause = format!("- when: on_play\n  text: '{text}'\n  process:\n    - select_trash: {{of: you, optional: false, filter: {filter}, bind_as: pick, prompt: Choose}}");
        assert_eq!(check(&clause, text), expected, "{text}, {filter}");
    }
}

#[test]
fn printed_text_trait_match_modes() {
    for (text, filter, expected) in [
        (
            "Add 1 card with the [Dragon] trait.",
            "{trait_contains: Dragon}",
            vec!["trait_match_mismatch"],
        ),
        (
            "Add 1 card with the [Dragon] trait.",
            "{trait_has: Dragon}",
            vec![],
        ),
        (
            "Add 1 card with [Dragon] in its traits.",
            "{trait_has: Dragon}",
            vec!["trait_match_mismatch"],
        ),
        (
            "Add 1 card with [Dragon] in its type.",
            "{trait_has: Dragon}",
            vec!["trait_match_mismatch"],
        ),
        (
            "Add 1 card with [Dragon] in its traits.",
            "{trait_contains: Dragon}",
            vec![],
        ),
        (
            "Add 1 card with [Dragon] in its name.",
            "{trait_contains: Dragon}",
            vec![],
        ),
    ] {
        let clause = format!("- when: on_play\n  text: '{text}'\n  process:\n    - select_hand: {{of: you, optional: false, filter: {filter}, bind_as: pick, prompt: Choose}}");
        assert_eq!(check(&clause, text), expected, "{text}, {filter}");
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

#[test]
fn printed_text_option_main_and_delay_timings() {
    let text = "[Main] Draw 1.";
    assert!(check(&format!("- when: main_from_hand\n  text: '{text}'"), text).is_empty());
    let text = "[Main] ＜Delay＞ Gain 2 memory.";
    assert!(check(
        &format!("- kind: delay\n  trigger: delayed\n  text: '{text}'"),
        text
    )
    .is_empty());
    assert_eq!(
        check(
            "- when: on_play\n  text: '[Main] Draw 1.'",
            "[Main] Draw 1."
        ),
        vec!["timing_tag_mismatch"]
    );
}

#[test]
fn printed_text_timing_tags_after_zone_icons_and_angle_prefixes() {
    for prefix in [
        "{Hand}",
        "{Trash} {Hand}",
        "＜Use Req. ([Seven Code] trait)＞",
        "<Use Req. ([Seven Code] trait)>",
    ] {
        let text = format!("{prefix} [Main] Draw 1.");
        assert_eq!(
            check(&format!("- when: on_play\n  text: '{text}'"), &text),
            vec!["timing_tag_mismatch"]
        );
        assert!(check(&format!("- when: main_from_hand\n  text: '{text}'"), &text).is_empty());
    }
    for text in ["＜Main＞ Draw 1.", "<Main> Draw 1."] {
        assert!(check(&format!("- when: main_from_hand\n  text: '{text}'"), text).is_empty());
        assert_eq!(
            check(&format!("- when: on_security\n  text: '{text}'"), text),
            vec!["timing_tag_mismatch"]
        );
    }
    assert!(check(
        "- kind: grant_keyword\n  keyword: Blocker\n  text: '＜Blocker＞'",
        "＜Blocker＞"
    )
    .is_empty());
    for text in ["{Hand", "＜Main", "<Main", ""] {
        let expected = if text.is_empty() {
            vec!["text_not_printed"]
        } else {
            vec![]
        };
        assert_eq!(
            check(&format!("- when: on_play\n  text: '{text}'"), text),
            expected
        );
    }
}

#[test]
fn printed_text_main_timing_preserves_the_activation_zone() {
    let text = "[Main] Draw 1.";
    assert!(check(&format!("- when: main_on_field\n  text: '{text}'"), text).is_empty());
    assert_eq!(
        check(&format!("- when: main_from_trash\n  text: '{text}'"), text),
        vec!["timing_tag_mismatch"]
    );
    for (zone, timing) in [
        ("{Hand}", "main_from_hand"),
        ("[Hand]", "main_from_hand"),
        ("{Trash}", "main_from_trash"),
        ("[Trash]", "main_from_trash"),
        ("{Battle Area}", "main_on_field"),
        ("{Field}", "main_on_field"),
    ] {
        let text = format!("{zone} [Main] Draw 1.");
        for authored in ["main_from_hand", "main_from_trash", "main_on_field", "main"] {
            let expected = if authored == timing {
                vec![]
            } else {
                vec!["timing_tag_mismatch"]
            };
            assert_eq!(
                check(&format!("- when: {authored}\n  text: '{text}'"), &text),
                expected,
                "{zone} with {authored}"
            );
        }
    }
    let text = "{Trash}[Main] Draw 1.";
    assert!(check(&format!("- when: main_from_trash\n  text: '{text}'"), text).is_empty());
    assert_eq!(
        check(&format!("- when: main_from_hand\n  text: '{text}'"), text),
        vec!["timing_tag_mismatch"]
    );
    let text = "{Hand} {Trash} [Main] Draw 1.";
    assert!(check(
        &format!("- when: [main_from_hand, main_from_trash]\n  text: '{text}'"),
        text
    )
    .is_empty());
    let text = "{Breeding} [Start of Your Main Phase] Draw 1.";
    assert!(check(
        &format!("- when: start_of_your_main_phase\n  text: '{text}'"),
        text
    )
    .is_empty());
    let text = "{Security} [Main] Draw 1.";
    assert!(check(&format!("- when: main\n  text: '{text}'"), text).is_empty());
    assert_eq!(
        check(&format!("- when: main_from_hand\n  text: '{text}'"), text),
        vec!["timing_tag_mismatch"]
    );
}

#[test]
fn printed_text_by_payment_requires_declinable_dsl() {
    let text = "[Start of Your Main Phase] By placing 1 Digimon card from your hand under this Tamer, draw 1.";
    let clause = format!("- when: start_of_your_main_phase\n  text: '{text}'\n  process:\n    - select_hand: {{of: you, filter: {{kind: digimon}}, bind_as: stash, prompt: Place}}\n    - place_selected_card_under_tamer: {{card: stash, tamer: source}}\n    - draw: {{of: you, count: 1}}");
    assert_eq!(check(&clause, text), vec!["may_without_optional"]);
    assert!(check(
        &clause.replace("bind_as: stash", "optional: true, bind_as: stash"),
        text
    )
    .is_empty());
    assert_eq!(check("- when: on_play\n  text: By suspending this Tamer, gain 1 memory.\n  process:\n    - activation_cost: {suspend_self: true}\n    - gain_memory: 1", "By suspending this Tamer, gain 1 memory."), vec!["may_without_optional"]);
    let text = "[Main] ＜Delay＞ Gain 2 memory.";
    assert!(check(
        &format!("- kind: delay\n  trigger: delayed\n  text: '{text}'"),
        text
    )
    .is_empty());
}

#[test]
fn printed_text_zero_minimum_reveal_buckets_are_declinable() {
    let text = "[On Deletion] Reveal 3. You may play 1 card. Trash the rest.";
    for min in ["min: 0, ", ""] {
        let clause = format!("- when: on_deletion\n  text: '{text}'\n  process:\n    - reveal_top_deck: {{of: you, count: 3, bind_as: revealed}}\n    - if:\n        condition: {{your_turn: true}}\n        then:\n          - select_reveal_buckets: {{from: revealed, buckets: [{{bind_as: pick, {min}max: 1}}]}}");
        assert!(check(&clause, text).is_empty());
    }
    for buckets in [
        "[]",
        "[{bind_as: pick, min: 1, max: 1}]",
        "[{bind_as: pick, min: 0, max: 0}]",
    ] {
        let clause = format!("- when: on_deletion\n  text: '{text}'\n  process:\n    - select_reveal_buckets: {{from: revealed, buckets: {buckets}}}");
        assert_eq!(check(&clause, text), vec!["may_without_optional"]);
    }
}

#[test]
fn printed_text_chosen_activation_declines_payment_but_not_body_choices() {
    for timing in ["main_from_hand", "main_on_field"] {
        let text = "By placing 1 card from your trash under this Digimon, draw 1.";
        assert!(check(&format!("- when: {timing}\n  text: '{text}'"), text).is_empty());
        let text = "By placing 1 card from your trash under this Digimon, you may draw 1.";
        assert_eq!(
            check(&format!("- when: {timing}\n  text: '{text}'"), text),
            vec!["may_without_optional"]
        );
        assert_eq!(
            check(
                &format!("- when: {timing}\n  text: You may draw 1."),
                "You may draw 1."
            ),
            vec!["may_without_optional"]
        );
    }
    for timing in ["main", "on_play", "[main_from_hand, on_play]"] {
        let text = "By placing 1 card, draw 1.";
        assert_eq!(
            check(&format!("- when: {timing}\n  text: '{text}'"), text),
            vec!["may_without_optional"]
        );
    }
}

#[test]
fn printed_text_zero_minimum_source_selections_are_declinable() {
    let text = "You may play 1 Digimon from under your Tamers.";
    for step in ["select_under_tamer_sources", "select_own_sources"] {
        for min in [0, 1] {
            let clause = format!("- when: on_play\n  text: '{text}'\n  process:\n    - {step}: {{min: {min}, max: 1, bind_as: pick}}");
            assert_eq!(
                check(&clause, text),
                if min == 0 {
                    vec![]
                } else {
                    vec!["may_without_optional"]
                }
            );
        }
    }
}

#[test]
fn printed_text_optionality_wording_exemptions() {
    for (clause, printed, expected) in [
        // Parenthesized reminder text describes keyword machinery.
        ("- when: on_play\n  text: 'Trigger <Blitz>. (This Digimon may attack.)'", "Trigger <Blitz>. (This Digimon may attack.)", vec![]),
        // Use-requirement flood gates are play permissions, not prompts.
        ("- kind: flood_gate\n  text: You may ignore this card's color requirements.", "You may ignore this card's color requirements.", vec![]),
        // "up to" already permits zero.
        ("- when: on_play\n  text: Delete up to 2 Digimon.\n  optional: true", "Delete up to 2 Digimon.", vec![]),
        ("- when: on_play\n  text: By adding your top security card to the hand, draw 1.\n  optional: true", "By adding your top security card to the hand, draw 1.", vec![]),
        ("- when: on_play\n  text: By unsuspending this Digimon, draw 1.\n  optional: true", "By unsuspending this Digimon, draw 1.", vec![]),
        // Delegating clauses inherit the referenced effect's optionality.
        ("- when: on_security\n  text: \"[Security] Activate this card's [Main] effects.\"\n  optional: true", "[Security] Activate this card's [Main] effects.", vec![]),
        ("- when: on_play\n  text: You may link 1 card.\n  process:\n    - link_cards: {from: [trash], to: own_digimon, count: {up_to: 1}, cost: free}", "You may link 1 card.", vec![]),
        ("- when: on_play\n  text: You may add your top security card to the hand.\n  process:\n    - may_add_top_security_to_hand: {of: you}", "You may add your top security card to the hand.", vec![]),
        ("- when: on_play\n  text: You may draw 1.\n  process:\n    - draw: {of: you, count: 1}", "You may draw 1.", vec!["may_without_optional"]),
    ] {
        assert_eq!(check(clause, printed), expected, "{clause}");
    }
}
