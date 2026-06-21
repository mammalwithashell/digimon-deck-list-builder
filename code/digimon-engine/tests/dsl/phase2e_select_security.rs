//! Phase 2e Task 4: SelectSecurity installs a parking selection over
//! `Game::player(of).security`; the callback resolves the picked index
//! into a CardHandle.

use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate, CompiledStep};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

fn push_to_security(runner: &mut DebugRunner, owner: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, owner, card_index);
    runner.game.players[owner as usize].security.push(card);
}

#[test]
fn select_security_opponent_binds_picked_handle() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("S0", "S0"))
        .add_card(make_test_card("S1", "S1"))
        .hand(0, &["SRC"])
        .build();

    push_to_security(&mut runner, 1, "S0");
    push_to_security(&mut runner, 1, "S1");
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![
        CompiledStep::SelectSecurity {
            then: vec![],
            of: CompiledPlayerRef::Opponent,
            filter: CompiledPredicate::default(),
            bind_as: Some("sec_pick".to_string()),
            prompt: "Pick an opponent security".to_string(),
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

    let (action_id, selecting_player) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        (pending.valid_action_ids[0], pending.selecting_player)
    };
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.memory, memory_before + 1);
}
