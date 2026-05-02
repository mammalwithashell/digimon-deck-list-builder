//! Continuation dispatcher: a step slice with no selection steps runs
//! straight through. (Selection-step parking test lands in Task 4 once
//! the first selection handler exists.)

use digimon_dsl::compiled::CompiledStep;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

#[test]
fn run_steps_with_no_selections_executes_all_steps_inline() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build();
    let card = runner.game.players[0].hand[0].handle();
    let before = runner.game.memory;
    {
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        let mut b = Bindings::new();
        run_steps(
            &[CompiledStep::GainMemory(1), CompiledStep::GainMemory(2)],
            &mut ctx,
            &mut b,
        );
    }
    assert_eq!(runner.game.memory, before + 3);
}
