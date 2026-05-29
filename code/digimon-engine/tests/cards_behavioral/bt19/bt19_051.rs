use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

#[test]
fn bt19_051_protects_one_own_digimon_and_gives_dp_until_opponents_turn_ends() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT19-051")
        .expect("BT19-051 YAML loads")
        .memory(10)
        .start();
    let source = runner.place_on_field(0, "BT19-051", Some(0));
    let target = runner.place_on_field(0, "BT19-051", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
    choose_own_permanent(&mut runner, target, "select protected target");
    runner.auto_resolve().expect("finish BT19-051 effect");

    assert!(runner
        .modifiers()
        .has(target, ModifierType::CannotBeReturnedToHand));
    assert!(runner
        .modifiers()
        .has(target, ModifierType::CannotBeReturnedToDeck));
    assert_eq!(runner.game.effective_dp(target), Some(10000));
}

fn choose_own_permanent(runner: &mut DebugRunner, handle: PermanentHandle, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = encode_attack(0, handle.index as u16);
    assert!(
        view.valid_action_ids.contains(&action),
        "{label}: expected action {action}, got {:?}",
        view.valid_action_ids
    );
    runner
        .execute_action(view.selecting_player, action)
        .expect(label);
}
