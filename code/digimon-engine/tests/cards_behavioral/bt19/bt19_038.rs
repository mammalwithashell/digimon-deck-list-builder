use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

#[test]
fn bt19_038_suspends_one_digimon_and_locks_one_when_digivolving_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT19-038")
        .expect("BT19-038 YAML loads")
        .add_card(make_test_card("OPP-A", "Opponent A"))
        .add_card(make_test_card("OPP-B", "Opponent B"))
        .memory(10)
        .start();
    let source = runner.place_on_field(0, "BT19-038", Some(0));
    let suspend_target = runner.place_on_field(1, "OPP-A", Some(0));
    let lock_target = runner.place_on_field(1, "OPP-B", Some(0));

    fire_when_digivolving(&mut runner, source);
    choose_permanent(&mut runner, suspend_target, "select suspend target");
    choose_permanent(&mut runner, lock_target, "select lock target");
    runner.auto_resolve().expect("finish BT19-038 effect");

    assert!(runner.game.players[1].battle_area[suspend_target.index as usize].is_suspended);
    assert!(
        runner.modifiers().has(
            lock_target,
            ModifierType::CannotActivateWhenDigivolvingEffects
        ),
        "chosen lock target must suppress When Digivolving effects"
    );
    assert!(
        runner
            .modifiers()
            .has(lock_target, ModifierType::CannotUnsuspend),
        "chosen lock target must be unable to unsuspend"
    );
    assert!(
        !runner.modifiers().has(
            suspend_target,
            ModifierType::CannotActivateWhenDigivolvingEffects
        ),
        "the merely suspended target is unaffected by the timing lock"
    );
}

#[test]
fn bt19_038_lockout_expires_after_opponents_turn_ends() {
    let filler = vec!["FILL"; 12];
    let mut runner = DebugRunner::builder()
        .dsl_card("BT19-038")
        .expect("BT19-038 YAML loads")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    let source = runner.place_on_field(0, "BT19-038", Some(0));
    let opp = runner.place_on_field(1, "BT19-038", Some(0));

    fire_when_digivolving(&mut runner, source);
    choose_permanent(&mut runner, opp, "select suspend target");
    choose_permanent(&mut runner, opp, "select lock target");
    runner.auto_resolve().expect("finish BT19-038 effect");

    assert!(runner.modifiers().has(opp, ModifierType::CannotUnsuspend));
    assert!(runner
        .modifiers()
        .has(opp, ModifierType::CannotActivateWhenDigivolvingEffects));

    let installer = 0u8;
    for _ in 0..4 {
        let ending = runner.turn_player();
        runner.pass_turn();
        if ending != installer {
            break;
        }
    }

    assert!(!runner.modifiers().has(opp, ModifierType::CannotUnsuspend));
    assert!(!runner
        .modifiers()
        .has(opp, ModifierType::CannotActivateWhenDigivolvingEffects));
}

fn fire_when_digivolving(runner: &mut DebugRunner, source: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}

fn choose_permanent(runner: &mut DebugRunner, handle: PermanentHandle, label: &str) {
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

#[allow(dead_code)]
fn tamer(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}
