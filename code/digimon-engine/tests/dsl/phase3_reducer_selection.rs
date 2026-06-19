use digimon_dsl::compiled::{CompiledBindingRef, CompiledClause, CompiledPredicate, CompiledStep};
use digimon_dsl::spec::CardSpec;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::selection::SelectionKind;

#[test]
fn select_any_permanent_and_select_dna_pair_compile() {
    let yaml = r#"
card: TEST-SELECT
name: Selection Reducer
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
effects:
  - when: [on_play]
    process:
      - select_any_permanent:
          filter: { kind: digimon }
          bind_as: target
          prompt: Pick anything
      - select_dna_pair:
          left_filter: { kind: digimon }
          right_filter: { kind: digimon }
          bind_left_as: left
          bind_right_as: right
          prompt: Pick DNA pair
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile");
    match &compiled.effects[0] {
        CompiledClause::Triggered(triggered) => {
            assert!(matches!(
                triggered.process[0],
                CompiledStep::SelectAnyPermanent { .. }
            ));
            assert!(matches!(
                triggered.process[1],
                CompiledStep::SelectDnaPair { .. }
            ));
        }
        other => panic!("expected triggered clause, got {other:?}"),
    }
}

#[test]
fn select_any_permanent_can_bind_either_players_field() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("D1", "D1"))
        .add_card(make_test_card("D2", "D2"))
        .hand(0, &["SRC"])
        .build();
    runner.place_on_field(0, "D1", None);
    runner.place_on_field(1, "D2", None);

    let src_card = runner.game.players[0].hand[0].handle();
    let steps = vec![
        CompiledStep::SelectAnyPermanent {
            then: vec![],
            filter: CompiledPredicate::default(),
            bind_as: Some("target".to_string()),
            selector: None,
            prompt: "Pick anything".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::Suspend {
            target: CompiledBindingRef::Named("target".to_string()),
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let (selecting_player, opponent_action) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("select_any_permanent should park a selection");
        // AnyField (not Target): select_any_permanent spans BOTH battle areas and
        // encodes the engine player per id (encode_attack(player, index)), so the
        // UI can decode the side — see SelectionKind::AnyField.
        assert_eq!(pending.kind, SelectionKind::AnyField);
        assert_eq!(pending.valid_action_ids.len(), 2);
        let action = *pending
            .valid_action_ids
            .iter()
            .find(|action| **action >= digimon_engine::action::space::ATTACK_START + 15)
            .expect("opponent permanent should have a distinct action id");
        (pending.selecting_player, action)
    };

    runner
        .game
        .resolve_selection(selecting_player, opponent_action)
        .expect("resolve select_any_permanent");

    assert!(
        runner.game.players[1].battle_area[0].is_suspended,
        "the opponent permanent selected through select_any_permanent should be suspended"
    );
    assert!(
        !runner.game.players[0].battle_area[0].is_suspended,
        "the own permanent should not have been selected"
    );
}

#[test]
fn select_dna_pair_chains_two_distinct_permanent_picks() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("L", "L"))
        .add_card(make_test_card("R", "R"))
        .hand(0, &["SRC"])
        .build();
    runner.place_on_field(0, "L", None);
    runner.place_on_field(0, "R", None);

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;
    let steps = vec![
        CompiledStep::SelectDnaPair {
            left_filter: CompiledPredicate::default(),
            right_filter: CompiledPredicate::default(),
            bind_left_as: "left".to_string(),
            bind_right_as: "right".to_string(),
            prompt: "Pick DNA pair".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::Suspend {
            target: CompiledBindingRef::Named("right".to_string()),
        },
        CompiledStep::GainMemory(3),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let (selecting_player, first_action) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("first DNA pick should park");
        assert_eq!(pending.valid_action_ids.len(), 2);
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .game
        .resolve_selection(selecting_player, first_action)
        .expect("resolve first DNA pick");

    let (selecting_player, second_action) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("second DNA pick should park");
        assert_eq!(
            pending.valid_action_ids.len(),
            1,
            "second DNA pick should exclude the first permanent"
        );
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .game
        .resolve_selection(selecting_player, second_action)
        .expect("resolve second DNA pick");

    assert_eq!(runner.game.memory, memory_before + 3);
    assert!(
        runner.game.players[0].battle_area[1].is_suspended,
        "the right-side binding should be available to the tail"
    );
}
