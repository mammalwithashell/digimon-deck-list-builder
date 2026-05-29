//! ST10-14 Chaos Degradation.
//!
//! [Main] Place 1 opponent Digimon face down at the top or bottom of
//! opponent's security stack. If you do, trash the top card of opponent's
//! security stack.
//! [Security] You may place 1 opponent Digimon face down at the top or bottom
//! of its owner's security stack.

use digimon_engine::action::space::encode_attack;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::selection::SelectionKind;

#[test]
fn st10_14_main_places_opponent_digimon_bottom_security_then_trashes_top() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST10-14")
        .expect("ST10-14 YAML loads")
        .add_card(digimon("OPP"))
        .add_card(make_test_card("OPP-SEC", "Opponent Security"))
        .hand(0, &["ST10-14"])
        .security(1, &["OPP-SEC"])
        .memory(10)
        .start();
    let opp = runner.place_on_field(1, "OPP", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));
    choose_permanent(&mut runner, opp);
    choose_effect_choice(&mut runner, 1);
    runner.auto_resolve().expect("finish main effect");

    assert_eq!(runner.security_count(1), 1);
    assert_eq!(
        runner.game.players[1].security[0].card_id(&runner.game.card_data),
        "OPP",
        "placing bottom then trashing top leaves the chosen Digimon in security"
    );
    assert_eq!(runner.battle_area_size(1), 0);
}

#[test]
fn st10_14_main_top_choice_trashes_the_placed_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST10-14")
        .expect("ST10-14 YAML loads")
        .add_card(digimon("OPP"))
        .add_card(make_test_card("OPP-SEC", "Opponent Security"))
        .hand(0, &["ST10-14"])
        .security(1, &["OPP-SEC"])
        .memory(10)
        .start();
    let opp = runner.place_on_field(1, "OPP", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));
    choose_permanent(&mut runner, opp);
    choose_effect_choice(&mut runner, 0);
    runner.auto_resolve().expect("finish main effect");

    assert_eq!(runner.security_count(1), 1);
    assert_eq!(runner.game.players[1].trash.len(), 1);
    assert_eq!(
        runner.game.players[1].trash[0].card_id(&runner.game.card_data),
        "OPP"
    );
}

#[test]
fn st10_14_security_effect_is_optional_and_does_not_trash_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST10-14")
        .expect("ST10-14 YAML loads")
        .add_card(digimon("ATK"))
        .add_card(digimon("OPP"))
        .security(1, &["ST10-14"])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATK", Some(0));
    let opp = runner.place_on_field(0, "OPP", Some(0));

    let result = runner.attack_player(attacker, 1, false);
    assert_eq!(result, AttackResult::InProgress);

    let pick = runner
        .pending_selection_view()
        .expect("security effect target prompt");
    assert!(pick.is_optional, "security effect says 'you may'");
    assert!(pick.valid_action_ids.contains(&encode_permanent(opp)));
    choose_permanent(&mut runner, opp);
    choose_effect_choice(&mut runner, 1);
    runner.auto_resolve().expect("finish security effect");

    assert_eq!(runner.security_count(0), 1);
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        "OPP"
    );
    assert!(runner.game.players[0].trash.is_empty());
}

fn choose_permanent(runner: &mut DebugRunner, target: digimon_engine::permanent::PermanentHandle) {
    let view = runner
        .pending_selection_view()
        .expect("permanent selection prompt");
    runner
        .execute_action(view.selecting_player, encode_permanent(target))
        .expect("select permanent");
}

fn encode_permanent(handle: digimon_engine::permanent::PermanentHandle) -> u16 {
    encode_attack(0, handle.index as u16)
}

fn choose_effect_choice(runner: &mut DebugRunner, index: usize) {
    let view = runner
        .pending_selection_view()
        .expect("top/bottom choice prompt");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[index])
        .expect("choose top/bottom");
}

fn digimon(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card
}
