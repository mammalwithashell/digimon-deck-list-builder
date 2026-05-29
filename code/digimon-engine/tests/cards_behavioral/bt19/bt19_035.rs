use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

#[test]
fn bt19_035_when_own_xros_heart_digimon_is_played_debuffs_one_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT19-035")
        .expect("BT19-035 YAML loads")
        .dsl_card("BT19-008")
        .expect("BT19-008 YAML loads")
        .add_card(opponent_body())
        .hand(0, &["BT19-008"])
        .memory(10)
        .start();
    runner.place_on_field(0, "BT19-035", Some(0));
    let target = runner.place_on_field(1, "OPP", Some(0));

    runner.play(0, 0).expect("play Xros Heart Digimon");
    runner.game.drain_effect_queue();
    choose_opponent_permanent(&mut runner, target, "select BT19-035 debuff target");
    runner.auto_resolve().expect("finish BT19-035 effect");

    assert_eq!(runner.game.effective_dp(target), Some(3000));
    assert_eq!(
        runner
            .modifiers()
            .sum(target, ModifierType::SecurityAttackChange),
        -1
    );
}

#[test]
fn bt19_035_ignores_non_xros_heart_digimon_play_events() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT19-035")
        .expect("BT19-035 YAML loads")
        .add_card(non_xros_body())
        .add_card(opponent_body())
        .hand(0, &["NON-XROS"])
        .memory(10)
        .start();
    runner.place_on_field(0, "BT19-035", Some(0));
    let target = runner.place_on_field(1, "OPP", Some(0));

    runner.play(0, 0).expect("play non-Xros Digimon");
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "non-Xros Heart play must not install a target prompt"
    );
    assert_eq!(runner.game.effective_dp(target), Some(6000));
    assert_eq!(
        runner
            .modifiers()
            .sum(target, ModifierType::SecurityAttackChange),
        0
    );
}

fn choose_opponent_permanent(runner: &mut DebugRunner, handle: PermanentHandle, label: &str) {
    if runner.pending_kind() == Some(SelectionKind::TriggerOrder) {
        let view = runner.pending_selection_view().expect(label);
        let choice = view
            .effect_choices
            .as_ref()
            .and_then(|choices| {
                choices
                    .iter()
                    .find(|choice| choice.label.starts_with("BT19-035"))
            })
            .expect("BT19-035 trigger choice");
        runner
            .execute_action(view.selecting_player, choice.action_id)
            .expect("choose BT19-035 trigger");
    }

    let view = runner.pending_selection_view().expect(label);
    assert_eq!(view.kind, SelectionKind::OppField);
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

fn opponent_body() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("OPP", "Opponent");
    card.card_kind = CardKind::Digimon;
    card.dp = Some(6000);
    card
}

fn non_xros_body() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("NON-XROS", "Non Xros");
    card.card_kind = CardKind::Digimon;
    card.traits = vec!["Plain".to_string()];
    card.dp = Some(3000);
    card
}
