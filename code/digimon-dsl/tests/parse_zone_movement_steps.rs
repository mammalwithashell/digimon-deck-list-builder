use digimon_dsl::compile::compile;
use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledClause, CompiledPlayerRef, CompiledStackPosition, CompiledStep,
};
use digimon_dsl::spec::CardSpec;
use digimon_dsl::step::{SecurityFace, StepSpec};

fn compile_steps(yaml_steps: &str) -> Vec<CompiledStep> {
    let yaml = format!(
        r#"
card: X-ZONE
name: Zone Verb Test
kind: digimon
level: 6
color: [blue]
cost: 10
dp: 11000
effects:
  - when: main_on_field
    process:
{yaml_steps}
"#
    );
    let spec: CardSpec = serde_yml::from_str(&yaml).expect("card YAML parses");
    let compiled = compile(&spec).expect("card compiles");
    match &compiled.effects[0] {
        CompiledClause::Triggered(t) => t.process.clone(),
        other => panic!("expected triggered clause, got {other:?}"),
    }
}

fn parse_first_step(yaml_steps: &str) -> StepSpec {
    let yaml = format!(
        r#"
card: X-ZONE
name: Zone Verb Test
kind: digimon
level: 6
color: [blue]
cost: 10
dp: 11000
effects:
  - when: main_on_field
    process:
{yaml_steps}
"#
    );
    let spec: CardSpec = serde_yml::from_str(&yaml).expect("card YAML parses");
    match &spec.effects[0] {
        digimon_dsl::clause::ClauseSpec::Triggered(t) => t.process[0].clone(),
        other => panic!("expected triggered clause, got {other:?}"),
    }
}

#[test]
fn bounce_self_parses_and_compiles() {
    assert!(matches!(
        parse_first_step("      - bounce_self: {}\n"),
        StepSpec::BounceSelf(_)
    ));
    assert_eq!(
        compile_steps("      - bounce_self: {}\n")[0],
        CompiledStep::BounceSelf
    );
}

#[test]
fn place_self_security_verbs_parse_face_axis_and_compile() {
    assert!(matches!(
        parse_first_step("      - place_self_at_security:\n          position: top\n          face: up\n"),
        StepSpec::PlaceSelfAtSecurity(args)
            if args.position == digimon_dsl::step::StackPosition::Top
                && args.face == SecurityFace::Up
    ));
    assert_eq!(
        compile_steps(
            "      - place_self_at_security:\n          position: top\n          face: up\n"
        )[0],
        CompiledStep::PlaceSelfAtSecurity {
            position: CompiledStackPosition::Top,
            face_up: true,
        }
    );
    assert_eq!(
        compile_steps("      - place_self_option_at_security:\n          position: bottom\n          face: down\n")[0],
        CompiledStep::PlaceSelfOptionAtSecurity {
            position: CompiledStackPosition::Bottom,
            face_up: false,
        }
    );
}

#[test]
fn permanent_and_stacked_security_verbs_compile() {
    let steps = compile_steps(
        r#"      - place_permanent_on_security_observed:
          target: source
          position: random
          face: up
          include_sources: true
      - security_place_stacked_card:
          carrier: source
          source_index_from_top: 0
          of: you
          position: top
          face: down
      - security_place_top_stacked_card:
          carrier: source
          of: opponent
          position: bottom
          face: up
"#,
    );
    assert_eq!(
        steps[0],
        CompiledStep::PlacePermanentOnSecurityObserved {
            of: CompiledPlayerRef::You,
            target: digimon_dsl::compiled::CompiledBindingRef::Source,
            position: CompiledStackPosition::Random,
            face_up: true,
            include_sources: true,
        }
    );
    assert!(matches!(
        &steps[1],
        CompiledStep::SecurityPlaceStackedCard {
            source_index_from_top: Some(0),
            ..
        }
    ));
    assert_eq!(
        steps[2],
        CompiledStep::SecurityPlaceTopStackedCard {
            carrier: digimon_dsl::compiled::CompiledBindingRef::Source,
            of: CompiledPlayerRef::Opponent,
            position: CompiledStackPosition::Bottom,
            face_up: true,
        }
    );
}

#[test]
fn gain_memory_fn_parses_and_compiles() {
    // Phase 2 Track F (G-DSL-GAIN-MEMORY-FN): formula-valued memory gain.
    // EX1-021 shape — "Gain 1 memory for every 4 cards in your hand."
    // The DSL formula uses BasePerDelta (base/per/delta) wrapped in
    // `floor_div` to express N / 4.
    let yaml = r#"      - gain_memory_fn:
          formula:
            floor_div:
              - base: 0
                per:
                  card_count_in_zone: { of: you, zone: hand }
                delta: 1
              - 4
"#;
    let steps = compile_steps(yaml);
    assert!(matches!(&steps[0], CompiledStep::GainMemoryFn { .. }));

    let spec = parse_first_step(yaml);
    assert!(matches!(spec, StepSpec::GainMemoryFn(_)));
}

#[test]
fn lose_memory_fn_parses_and_compiles() {
    let steps = compile_steps(
        r#"      - lose_memory_fn:
          formula: 3
"#,
    );
    assert!(matches!(&steps[0], CompiledStep::LoseMemoryFn { .. }));
}

#[test]
fn place_top_source_as_bottom_parses_and_compiles() {
    // Phase 2 Track F (G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM): the verb takes
    // a single `target:` permanent binding and lowers to the deterministic
    // engine helper (no player choice exposed).
    let steps = compile_steps(
        r#"      - place_top_source_as_bottom:
          target: source
"#,
    );
    assert_eq!(
        steps[0],
        CompiledStep::PlaceTopSourceAsBottom {
            target: digimon_dsl::compiled::CompiledBindingRef::Source,
        }
    );

    // Round-trip through StepSpec preserves identity.
    let spec = parse_first_step(
        r#"      - place_top_source_as_bottom:
          target: source
"#,
    );
    assert!(matches!(spec, StepSpec::PlaceTopSourceAsBottom(_)));
}

#[test]
fn bulk_trash_and_hand_verbs_compile_formulas() {
    let steps = compile_steps(
        r#"      - return_all_trash_to_deck_bottom: { of: you }
      - trash_top_n_digivolution_cards_of_each:
          of: opponent
          n: 2
      - trash_opponent_hand_to_count:
          opponent: opponent
          target_count: 3
"#,
    );
    assert_eq!(
        steps[0],
        CompiledStep::ReturnAllTrashToDeckBottom {
            of: CompiledPlayerRef::You
        }
    );
    assert!(matches!(
        &steps[1],
        CompiledStep::TrashTopNDigivolutionCardsOfEach {
            of: CompiledPlayerRef::Opponent,
            ..
        }
    ));
    assert!(matches!(
        &steps[2],
        CompiledStep::TrashOpponentHandToCount {
            opponent: CompiledPlayerRef::Opponent,
            ..
        }
    ));
}

#[test]
fn move_trash_card_to_deck_top_compiles_with_select_trash_binding() {
    // G-ZONE-SELECTED-TRASH-TO-DECK-TOP — LM-030 clause B shape: a select_trash
    // step binds one card, then move_trash_card_to_deck_top places it on top.
    let steps = compile_steps(
        r#"      - select_trash:
          of: you
          bind_as: to_return
          filter: { kind: digimon }
          prompt: "Return 1 Digimon from trash to the top of the deck"
      - move_trash_card_to_deck_top:
          of: you
          card: to_return
"#,
    );
    let move_step = steps
        .iter()
        .find(|s| matches!(s, CompiledStep::MoveTrashCardToDeckTop { .. }))
        .expect("move_trash_card_to_deck_top must lower into a CompiledStep");
    match move_step {
        CompiledStep::MoveTrashCardToDeckTop { of, card } => {
            assert_eq!(*of, CompiledPlayerRef::You);
            assert_eq!(*card, CompiledBindingRef::Named("to_return".into()));
        }
        other => panic!("expected MoveTrashCardToDeckTop, got {other:?}"),
    }
}

#[test]
fn move_trash_card_to_deck_top_rejects_unknown_field() {
    let yaml = r#"
card: BAD-TOP
name: Bad Deck Top
kind: digimon
level: 6
color: [blue]
cost: 10
dp: 11000
effects:
  - when: main_on_field
    process:
      - move_trash_card_to_deck_top:
          of: you
          card: picked
          position: top
"#;
    let spec: Result<CardSpec, _> = serde_yml::from_str(yaml);
    assert!(
        spec.is_err(),
        "move_trash_card_to_deck_top must reject the unknown `position` field"
    );
}

#[test]
fn search_own_security_stack_compiles_nested_selection_body() {
    let steps = compile_steps(
        r#"      - search_own_security_stack:
          filter: { trait_has: Olympos XII }
          prompt: "Choose a card in your security"
          bind_as: picked_security
          on_select:
            - add_to_hand_from_security:
                of: you
                card: picked_security
          on_no_match:
            - gain_memory: 1
"#,
    );
    match &steps[0] {
        CompiledStep::SearchOwnSecurityStack {
            prompt,
            bind_as,
            on_select,
            on_no_match,
            ..
        } => {
            assert_eq!(prompt, "Choose a card in your security");
            assert_eq!(bind_as.as_deref(), Some("picked_security"));
            assert_eq!(on_select.len(), 1);
            assert_eq!(on_no_match.as_ref().expect("on_no_match").len(), 1);
        }
        other => panic!("expected SearchOwnSecurityStack, got {other:?}"),
    }
}

#[test]
fn malformed_zone_movement_steps_fail_to_parse() {
    let bad_face = r#"
card: BAD-FACE
name: Bad
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: main_on_field
    process:
      - place_self_at_security:
          position: top
          face: sideways
"#;
    assert!(serde_yml::from_str::<CardSpec>(bad_face).is_err());

    let missing_source = r#"
card: BAD-STACKED
name: Bad
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: main_on_field
    process:
      - security_place_stacked_card:
          carrier: source
          position: top
          face: up
"#;
    let spec: CardSpec = serde_yml::from_str(missing_source).expect("schema parses");
    let err = compile(&spec).expect_err("validator rejects missing source selector");
    assert!(
        err.iter().any(|e| e
            .message
            .contains("requires source or source_index_from_top")),
        "unexpected errors: {err:?}"
    );
}

// ─── G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME ───────────────────────────────
//
// `return_selected_sources_to_hand` mirrors `trash_selected_sources` but routes
// each `select_own_sources`-bound digivolution source card to its owner's hand
// (BT12-031 Imperialdramon: Dragon Mode alt-cost).

#[test]
fn return_selected_sources_to_hand_parses_as_step_spec() {
    let step = parse_first_step(
        "      - return_selected_sources_to_hand:\n          source_refs: dragon_mode_source\n",
    );
    match step {
        StepSpec::ReturnSelectedSourcesToHand(args) => {
            assert_eq!(args.source_refs, "dragon_mode_source");
        }
        other => panic!("expected ReturnSelectedSourcesToHand, got {other:?}"),
    }
}

#[test]
fn return_selected_sources_to_hand_lowers_inside_select_own_sources() {
    // The verb only ever appears in a `select_own_sources` `then:` tail, since
    // it consumes a source-refs binding.
    let yaml = r#"
card: X-RET-SRC
name: Return Source Picker
kind: digimon
level: 6
color: [blue]
cost: 10
dp: 11000
effects:
  - when: when_digivolving
    process:
      - select_own_sources:
          from: source
          min: 0
          max: 1
          bind_as: picked_sources
          then:
            - return_selected_sources_to_hand:
                source_refs: picked_sources
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("card YAML parses");
    let compiled = compile(&spec).expect("card compiles");
    let process = match &compiled.effects[0] {
        CompiledClause::Triggered(t) => &t.process,
        other => panic!("expected triggered clause, got {other:?}"),
    };
    match &process[0] {
        CompiledStep::SelectOwnSources { then, .. } => {
            assert_eq!(
                then,
                &vec![CompiledStep::ReturnSelectedSourcesToHand {
                    source_refs: "picked_sources".to_string(),
                }]
            );
        }
        other => panic!("expected SelectOwnSources, got {other:?}"),
    }
}

#[test]
fn return_selected_sources_to_hand_rejects_unknown_field() {
    let bad = r#"
card: BAD-RET-SRC
name: Bad
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: main_on_field
    process:
      - return_selected_sources_to_hand:
          source_refs: picked
          count: 1
"#;
    assert!(
        serde_yml::from_str::<CardSpec>(bad).is_err(),
        "return_selected_sources_to_hand must reject the unknown `count` field"
    );
}
