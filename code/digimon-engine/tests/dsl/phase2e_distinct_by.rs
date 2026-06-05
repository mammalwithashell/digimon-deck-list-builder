//! Phase 2e Task 8: `distinct_by: card_number` removes other zone indices
//! that share the picked card's printed card_id from the next-step
//! candidate list.

use digimon_dsl::compiled::{
    CompiledDistinctBy, CompiledPlayerRef, CompiledPredicate, CompiledStep, CompiledZone,
};
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
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, card_index));
}

#[test]
fn distinct_by_card_number_filters_duplicates_after_pick() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("DUP", "DUP"))
        .add_card(make_test_card("UNIQ", "UNIQ"))
        .hand(0, &["SRC"])
        .build();

    // Two copies of "DUP" + one "UNIQ" in opponent's trash.
    push_to_trash(&mut runner, 1, "DUP");
    push_to_trash(&mut runner, 1, "DUP");
    push_to_trash(&mut runner, 1, "UNIQ");

    let src_card = runner.game.players[0].hand[0].handle();

    let steps = vec![CompiledStep::SelectCountCappedMulti {
        clamp_to_available: false,
        of: CompiledPlayerRef::Opponent,
        zone: CompiledZone::Trash,
        min: 0,
        max: digimon_dsl::compiled::CompiledCountBound::Literal(3),
        filter: CompiledPredicate::default(),
        bind_as: Some("picks".to_string()),
        prompt: "Pick distinct".to_string(),
        prompt_key: None,
        optional_zero: false,
        distinct_by: Some(CompiledDistinctBy::CardNumber),
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Step 1: 3 candidates (the two DUPs + UNIQ).
    let pending = runner.game.pending_selection.as_ref().unwrap();
    assert_eq!(pending.valid_action_ids.len(), 3);
    let (action_id, selecting_player) = (pending.valid_action_ids[0], pending.selecting_player);
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("first pick");

    // Step 2: only UNIQ should remain — both DUP indices are filtered out
    // because the picked DUP shares its card_id with the other DUP.
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("pending must re-arm after first pick");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "after picking a DUP, the other DUP must be filtered by distinct_by=card_number"
    );

    let (action_id, selecting_player) = (pending.valid_action_ids[0], pending.selecting_player);
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("second pick");

    // No more candidates: trampoline auto-commits.
    assert!(runner.game.pending_selection.is_none());
}
