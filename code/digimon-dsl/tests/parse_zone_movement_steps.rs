use digimon_dsl::compile::compile;
use digimon_dsl::compiled::{
    CompiledClause, CompiledPlayerRef, CompiledStackPosition, CompiledStep,
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
