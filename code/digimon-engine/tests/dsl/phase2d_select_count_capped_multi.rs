//! Phase 2d Task 5: SelectCountCappedMulti installs a parking selection,
//! its callback writes the picks into Bindings as CardList, and a
//! follow-on PerSelected step iterates the picks.

use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate, CompiledStep, CompiledZone};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::{run_steps, RunOutcome};
use digimon_engine::effect_context::EffectContext;

use crate::phase2d_helpers::resolve_count_capped_picks;

/// Push `card_id` into `player`'s trash, looked up via `game.card_data`.
fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    runner.game.players[player as usize].trash.push(card);
}

#[test]
fn multi_pick_binds_card_list_for_per_selected() {
    // P0 owns the effect. P1 has 3 cards in trash; we pick up to 2 and
    // gain 1 memory per pick.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("T1", "T1"))
        .add_card(make_test_card("T2", "T2"))
        .add_card(make_test_card("T3", "T3"))
        .hand(0, &["SRC"])
        .build();

    push_to_trash(&mut runner, 1, "T1");
    push_to_trash(&mut runner, 1, "T2");
    push_to_trash(&mut runner, 1, "T3");
    assert_eq!(runner.game.players[1].trash.len(), 3);

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![
        CompiledStep::SelectCountCappedMulti {
            of: CompiledPlayerRef::Opponent,
            zone: CompiledZone::Trash,
            max: digimon_dsl::compiled::CompiledCountBound::Literal(2),
            filter: CompiledPredicate::default(),
            bind_as: Some("picks".to_string()),
            prompt: "Pick up to 2".to_string(),
            prompt_key: None,
            optional_zero: false,
            distinct_by: None,
        },
        CompiledStep::PerSelected {
            selection: "picks".to_string(),
            bind_as: "p".to_string(),
            body: vec![CompiledStep::GainMemory(1)],
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Pick the first remaining candidate twice — auto-commits at max.
    resolve_count_capped_picks(&mut runner, &[0, 0]);

    assert!(
        runner.game.pending_selection.is_none(),
        "no selection must remain after auto-commit at max"
    );
    assert_eq!(
        runner.game.memory,
        memory_before + 2,
        "PerSelected over the 2 picks should have run gain_memory(1) twice"
    );
}

#[test]
fn empty_multi_pick_runs_tail_once_synchronously() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .hand(0, &["SRC"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;
    let steps = vec![
        CompiledStep::SelectCountCappedMulti {
            of: CompiledPlayerRef::Opponent,
            zone: CompiledZone::Trash,
            max: digimon_dsl::compiled::CompiledCountBound::Literal(2),
            filter: CompiledPredicate::default(),
            bind_as: Some("picks".to_string()),
            prompt: "Pick up to 2".to_string(),
            prompt_key: None,
            optional_zero: false,
            distinct_by: None,
        },
        CompiledStep::GainMemory(1),
    ];

    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Synchronous);
    assert!(
        runner.game.pending_selection.is_none(),
        "empty count-capped selection should not park"
    );
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "empty count-capped selection should run its tail exactly once"
    );
}
