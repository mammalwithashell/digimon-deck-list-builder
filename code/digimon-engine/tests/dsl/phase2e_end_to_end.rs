//! Phase 2e end-to-end: SelectReveal → SelectEffectChoice → GainMemory.
//! Confirms two parking selections compose and the final tail runs only
//! after both resolve.

use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate, CompiledStep};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

fn push_to_revealed(runner: &mut DebugRunner, owner: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let card_index = runner.game.next_card_index();
    runner
        .game
        .revealed_cards
        .push(CardSource::new(data_idx, owner, card_index));
}

#[test]
fn select_reveal_then_effect_choice_then_gain_memory() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("R0", "R0"))
        .add_card(make_test_card("R1", "R1"))
        .hand(0, &["SRC"])
        .build();

    push_to_revealed(&mut runner, 0, "R0");
    push_to_revealed(&mut runner, 0, "R1");

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![
        CompiledStep::SelectReveal {
            then: vec![],
            of: CompiledPlayerRef::You,
            filter: CompiledPredicate::default(),
            bind_as: Some("picked".to_string()),
            prompt: "pick".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::SelectEffectChoice {
            labels: vec!["A".to_string(), "B".to_string()],
            bind_as: Some("branch".to_string()),
            prompt: "choose".to_string(),
            prompt_key: None,
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // First parked: SelectReveal.
    assert!(runner.game.pending_selection.is_some());
    assert_eq!(runner.game.memory, memory_before, "tail must not have run");

    let (a, p) = {
        let s = runner.game.pending_selection.as_ref().unwrap();
        (s.valid_action_ids[0], s.selecting_player)
    };
    runner.game.resolve_selection(p, a).expect("reveal pick");

    // Second parked: SelectEffectChoice. The reveal callback installed it
    // before its tail's gain_memory could fire.
    assert!(
        runner.game.pending_selection.is_some(),
        "after resolving SelectReveal, SelectEffectChoice should be installed"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "GainMemory must wait for SelectEffectChoice to resolve"
    );

    let (a, p) = {
        let s = runner.game.pending_selection.as_ref().unwrap();
        (s.valid_action_ids[0], s.selecting_player)
    };
    runner.game.resolve_selection(p, a).expect("choice pick");

    // Both resolved → final tail ran.
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.memory, memory_before + 1);
}
