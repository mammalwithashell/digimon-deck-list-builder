use digimon_dsl::compiled::{CompiledStep, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{EffectTiming, GamePhase};
use digimon_engine::selection::SelectionKind;

fn runner() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("TARGET", "Target"))
        .hand(0, &["SRC"])
        .build();
    runner.game.turn_order = vec![0, 1];
    runner.game.turn_player_idx = 0;
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;
    runner.game.memory = 0;
    runner.place_on_field(0, "TARGET", None);
    runner
}

fn schedule(runner: &mut DebugRunner, steps: Vec<CompiledStep>) {
    let src = runner.game.players[0].hand[0].handle();
    let mut ctx = EffectContext::new(&mut runner.game, src, None, 0);
    let mut bindings = Bindings::new();
    run_steps(&steps, &mut ctx, &mut bindings);
}

/// Two pending processings due at the same timing are one simultaneous pool
/// (§18-1-2), ordered by the turn player. The first body parks on a choice;
/// once it resolves, the body finishes and the pool continues with the
/// second entry, each exactly once.
#[test]
fn scheduled_drain_resumes_after_selection_and_continues_remaining_effects() {
    let mut runner = runner();
    schedule(
        &mut runner,
        vec![
            CompiledStep::ScheduleDelayed {
                when: CompiledTiming::EndOfYourTurn,
                body: vec![
                    CompiledStep::SelectEffectChoice {
                        // Two labels: a one-label choice auto-resolves without parking
                        // (G-DSL-EFFECT-CHOICE-BRANCH-LEGALITY), and this test needs a park.
                        labels: vec!["first".to_string(), "second".to_string()],
                        legal_when: None,
                        bind_as: None,
                        prompt: "Choose one".to_string(),
                        prompt_key: None,
                    },
                    CompiledStep::GainMemory(1),
                ],
            },
            CompiledStep::ScheduleDelayed {
                when: CompiledTiming::EndOfYourTurn,
                body: vec![CompiledStep::GainMemory(2)],
            },
        ],
    );

    assert_eq!(runner.game.scheduled_effects.len(), 2);
    digimon_engine::scheduled_effects::fire_scheduled_for_timing(
        &mut runner.game,
        EffectTiming::EndOfYourTurn,
    );

    // §18-1-2 / §15-4-3-5-1: the turn player orders the two.
    let (player, first) = {
        let pending = runner.game.pending_selection.as_ref().expect("an ordering choice");
        assert_eq!(pending.kind, SelectionKind::TriggerOrder, "{pending:?}");
        assert_eq!(pending.valid_action_ids.len(), 2, "{pending:?}");
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    assert!(runner.game.scheduled_effects.is_empty(), "both joined the pool");
    assert_eq!(runner.game.memory, 0);
    runner
        .game
        .resolve_selection(player, first)
        .expect("order the choice-bearing body first");

    let (selecting_player, action_id) = {
        let pending = runner.game.pending_selection.as_ref().expect("the body's choice");
        assert_eq!(pending.kind, SelectionKind::EffectChoice, "{pending:?}");
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    assert_eq!(runner.game.memory, 0);
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve scheduled selection");

    assert!(runner.game.pending_selection.is_none());
    assert!(runner.game.effect_queue.is_empty());
    assert!(runner.game.scheduled_effects.is_empty());
    assert_eq!(runner.game.memory, 3);
}

/// Pending processing scheduled WHILE its timing's pool resolves is not
/// added to that pool: it waits for the next matching boundary.
#[test]
fn scheduled_during_the_pool_waits_for_the_next_boundary() {
    let mut runner = runner();
    schedule(
        &mut runner,
        vec![CompiledStep::ScheduleDelayed {
            when: CompiledTiming::EndOfYourTurn,
            body: vec![
                CompiledStep::ScheduleDelayed {
                    when: CompiledTiming::EndOfYourTurn,
                    body: vec![CompiledStep::GainMemory(4)],
                },
                CompiledStep::GainMemory(1),
            ],
        }],
    );

    digimon_engine::scheduled_effects::fire_scheduled_for_timing(
        &mut runner.game,
        EffectTiming::EndOfYourTurn,
    );
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.memory, 1, "only the outer body ran");
    assert_eq!(runner.game.scheduled_effects.len(), 1, "the new one waits");

    digimon_engine::scheduled_effects::fire_scheduled_for_timing(
        &mut runner.game,
        EffectTiming::EndOfYourTurn,
    );
    assert_eq!(runner.game.memory, 5, "it ran at the next boundary");
    assert!(runner.game.scheduled_effects.is_empty());
}
