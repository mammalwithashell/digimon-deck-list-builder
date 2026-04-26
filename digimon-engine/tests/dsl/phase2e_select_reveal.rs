//! Phase 2e Task 3: SelectReveal installs a parking selection over
//! `Game::revealed_cards`; its callback resolves the picked index into a
//! `CardHandle` and writes it into Bindings.

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
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, owner, card_index);
    runner.game.revealed_cards.push(card);
}

#[test]
fn select_reveal_binds_picked_card_handle() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("R0", "R0"))
        .add_card(make_test_card("R1", "R1"))
        .hand(0, &["SRC"])
        .build();

    push_to_revealed(&mut runner, 0, "R0");
    push_to_revealed(&mut runner, 0, "R1");
    let target_handle = runner.game.revealed_cards[1].handle();
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![
        CompiledStep::SelectReveal {
            of: CompiledPlayerRef::You,
            filter: CompiledPredicate::default(),
            bind_as: Some("picked".to_string()),
            prompt: "Pick a revealed card".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert!(runner.game.pending_selection.is_some());

    // Pick the second revealed card (index 1).
    let (action_id, selecting_player) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        (pending.valid_action_ids[1], pending.selecting_player)
    };
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "tail must run after resolution"
    );
    // Binding visibility across the callback is exercised in the end-to-end
    // test in Task 9; here we just assert the install + resolve plumbing.
    let _ = target_handle; // tracked for the e2e test
}
