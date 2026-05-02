//! Phase 2d Task 7: a select inside an `if` body must defer the steps
//! AFTER the `if` until the inner select resolves.
//!
//! Pattern: if (your_turn) { select_opponent_permanent; delete it } then
//! gain_memory(5). The +5 memory must NOT happen until after the delete.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledPredicate, CompiledStep};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

#[test]
fn outer_steps_wait_for_inner_park() {
    // P0 turn player, P1 has one field permanent.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("TGT", "TGT"))
        .hand(0, &["SRC"])
        .hand(1, &["TGT"])
        .build();
    runner.place_on_field(1, "TGT", None);

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![
        CompiledStep::If {
            condition: CompiledPredicate {
                your_turn: Some(true),
                ..CompiledPredicate::default()
            },
            then: vec![
                CompiledStep::SelectOpponentPermanent {
                    filter: CompiledPredicate::default(),
                    bind_as: Some("tgt".to_string()),
                    selector: None,
                    prompt: "Pick".to_string(),
                    prompt_key: None,
                    optional: false,
                },
                CompiledStep::DeletePermanent {
                    target: CompiledBindingRef::Named("tgt".to_string()),
                },
            ],
            else_branch: vec![],
        },
        CompiledStep::GainMemory(5),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // BEFORE the player resolves the selection: memory unchanged, target alive.
    assert_eq!(
        runner.game.memory, memory_before,
        "gain_memory(5) must not run until inner selection resolves"
    );
    assert_eq!(runner.game.players[1].battle_area.len(), 1, "target alive");

    // Resolve the field-target selection on P1's only permanent.
    let (action_id, selecting_player) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("inner select_opponent_permanent should have parked");
        (pending.valid_action_ids[0], pending.selecting_player)
    };
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve_selection should succeed");

    // AFTER resolution: target deleted AND memory bumped.
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        0,
        "DeletePermanent should have fired in the inner tail"
    );
    assert_eq!(
        runner.game.memory,
        memory_before + 5,
        "outer tail (gain_memory(5)) should have run after inner resolved"
    );
}
