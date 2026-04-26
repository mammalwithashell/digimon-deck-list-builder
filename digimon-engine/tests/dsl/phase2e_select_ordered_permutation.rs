//! Phase 2e Task 7: SelectOrderedPermutation resolves `items` to a
//! CardList, drives the multi-step permutation trampoline, and binds the
//! ordered result as a CardList.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledStep};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, card_index));
}

#[test]
fn select_ordered_permutation_orders_input_list() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("A", "A"))
        .add_card(make_test_card("B", "B"))
        .hand(0, &["SRC"])
        .build();

    push_to_trash(&mut runner, 0, "A");
    push_to_trash(&mut runner, 0, "B");
    let a = runner.game.players[0].trash[0].handle();
    let b = runner.game.players[0].trash[1].handle();
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let mut bindings = Bindings::new();
    bindings.insert_card_list("input", vec![a, b]);

    let steps = vec![
        CompiledStep::SelectOrderedPermutation {
            items: CompiledBindingRef::Named("input".to_string()),
            bind_as: Some("ordered".to_string()),
            prompt: "Order them".to_string(),
            prompt_key: None,
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // A 2-item permutation prompts twice. Pick offset 1 first (B), then 0 (A).
    let (action_id, selecting_player) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        // Two items remaining: action_ids are SEL_REVEAL_START + 0 and +1.
        (pending.valid_action_ids[1], pending.selecting_player)
    };
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("first pick");

    // Second pick: only one candidate remains; pick it.
    let (action_id, selecting_player) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        (pending.valid_action_ids[0], pending.selecting_player)
    };
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("second pick");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.memory, memory_before + 1);
}

#[test]
fn select_ordered_permutation_empty_runs_tail_synchronously() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .hand(0, &["SRC"])
        .build();
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let mut bindings = Bindings::new();
    bindings.insert_card_list("input", vec![]);

    let steps = vec![
        CompiledStep::SelectOrderedPermutation {
            items: CompiledBindingRef::Named("input".to_string()),
            bind_as: Some("ordered".to_string()),
            prompt: "Order".to_string(),
            prompt_key: None,
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Empty items: engine fires the final callback immediately. Tail runs
    // synchronously — no selection installed.
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.memory, memory_before + 1);
}
