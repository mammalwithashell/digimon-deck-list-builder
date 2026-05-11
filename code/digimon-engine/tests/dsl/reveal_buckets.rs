use digimon_dsl::compile::compile;
use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledPlayerRef, CompiledPredicate, CompiledRevealBucket, CompiledStep,
};
use digimon_dsl::spec::CardSpec;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::{run_steps, RunOutcome};
use digimon_engine::effect_context::EffectContext;

fn push_to_revealed(runner: &mut DebugRunner, owner: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, owner, card_index);
    runner.game.revealed_cards.push(card);
}

fn compile_steps(yaml: &str) -> Vec<CompiledStep> {
    let spec: CardSpec = serde_yml::from_str(yaml).expect("card yaml parses");
    let compiled = compile(&spec).expect("card yaml compiles");
    let digimon_dsl::compiled::CompiledClause::Triggered(effect) = &compiled.effects[0] else {
        panic!("expected triggered effect");
    };
    effect.process.clone()
}

#[test]
fn select_reveal_buckets_compiles_named_buckets() {
    let steps = compile_steps(
        r#"
card: TEST-REVEAL-BUCKETS
name: Reveal Buckets Test
kind: option
color: [red]
cost: 2
effects:
  - when: main
    process:
      - reveal_top_deck: { of: you, count: 3, bind_as: revealed }
      - select_reveal_buckets:
          from: revealed
          buckets:
            - bind_as: hybrid
              filter: { trait_has: Hybrid }
              min: 0
              max: 1
            - bind_as: tamer
              filter: { kind: tamer }
              min: 0
              max: 1
          no_duplicate_cards: true
          prompt: "Choose cards to add"
      - add_to_hand_from_reveal: { of: you, card: hybrid }
      - add_to_hand_from_reveal: { of: you, card: tamer }
"#,
    );

    assert!(
        matches!(steps[1], CompiledStep::SelectRevealBuckets { .. }),
        "second step should lower to CompiledStep::SelectRevealBuckets: {steps:#?}"
    );
}

#[test]
fn select_reveal_buckets_singleton_bucket_can_add_to_hand_from_reveal() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("R0", "Hybrid Candidate"))
        .add_card(make_test_card("R1", "Other Candidate"))
        .hand(0, &["SRC"])
        .build();

    push_to_revealed(&mut runner, 0, "R0");
    push_to_revealed(&mut runner, 0, "R1");
    let first = runner.game.revealed_cards[0].handle();
    let second = runner.game.revealed_cards[1].handle();
    let src_card = runner.game.players[0].hand[0].handle();

    let steps = vec![
        CompiledStep::SelectRevealBuckets {
            from: "revealed".to_string(),
            buckets: vec![
                CompiledRevealBucket {
                    bind_as: "hybrid".to_string(),
                    filter: None,
                    min: 0,
                    max: 1,
                },
                CompiledRevealBucket {
                    bind_as: "tamer".to_string(),
                    filter: None,
                    min: 0,
                    max: 1,
                },
            ],
            no_duplicate_cards: true,
            prompt: Some("Choose cards to add".to_string()),
        },
        CompiledStep::AddToHandFromReveal {
            of: CompiledPlayerRef::You,
            card: CompiledBindingRef::Named("hybrid".to_string()),
        },
        CompiledStep::AddToHandFromReveal {
            of: CompiledPlayerRef::You,
            card: CompiledBindingRef::Named("tamer".to_string()),
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        bindings.insert_card_list("revealed", vec![first, second]);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let first_action = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        pending.valid_action_ids[0]
    };
    runner.game.resolve_selection(0, first_action).unwrap();
    runner
        .game
        .resolve_selection(0, digimon_engine::action::space::PASS)
        .unwrap();

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.players[0]
            .hand
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "R0"
    );
    assert_eq!(
        runner.game.revealed_cards.len(),
        1,
        "singleton bucket binding should feed add_to_hand_from_reveal"
    );
    assert_eq!(runner.game.revealed_cards[0].handle(), second);
}

#[test]
fn select_reveal_buckets_empty_nested_body_runs_tail_synchronously() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .hand(0, &["SRC"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();
    let steps = vec![
        CompiledStep::If {
            condition: CompiledPredicate {
                memory_gte: Some(digimon_dsl::compiled::CompiledDpConstraint::Literal(0)),
                ..Default::default()
            },
            then: vec![
                CompiledStep::SelectRevealBuckets {
                    from: "revealed".to_string(),
                    buckets: vec![CompiledRevealBucket {
                        bind_as: "hybrid".to_string(),
                        filter: None,
                        min: 1,
                        max: 1,
                    }],
                    no_duplicate_cards: true,
                    prompt: Some("Choose cards to add".to_string()),
                },
                CompiledStep::GainMemory(1),
            ],
            else_branch: Vec::new(),
        },
        CompiledStep::GainMemory(2),
    ];

    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        bindings.insert_card_list("revealed", Vec::new());
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Synchronous);
    assert!(
        runner.game.pending_selection.is_none(),
        "empty reveal buckets should not report a parked selection"
    );
    assert!(
        runner.game.dsl_outer_tail.is_none(),
        "outer tail should not be parked without a pending selection"
    );
    assert_eq!(
        runner.game.memory, 3,
        "synchronous bucket callback should run inner and outer tail steps"
    );
}
