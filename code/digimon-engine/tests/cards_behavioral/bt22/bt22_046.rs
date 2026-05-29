//! BT22-046 Gargomon.
//!
//! [When Digivolving] If you have 1 or fewer Tamers, you may play 1 Tamer
//! card with the [CS] trait from your hand without paying the cost.
//! Inherited: [All Turns] This Digimon gets +1000 DP.

use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn bt22_046_when_digivolving_plays_cs_tamer_from_hand_for_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-046")
        .expect("BT22-046 YAML loads")
        .add_card(cs_tamer("CS-TAMER"))
        .add_card(tamer("PLAIN-TAMER"))
        .hand(0, &["CS-TAMER", "PLAIN-TAMER"])
        .memory(3)
        .start();
    let gargomon = runner.place_on_field(0, "BT22-046", Some(0));
    let field_before = runner.battle_area_size(0);
    let memory_before = runner.memory();

    fire_when_digivolving(&mut runner, gargomon);
    let view = runner
        .pending_selection_view()
        .expect("CS Tamer hand selection");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        runner.pending_is_optional(),
        "printed 'may' can be declined"
    );
    assert!(view.valid_action_ids.contains(&PLAY_HAND_START));
    assert!(!view.valid_action_ids.contains(&(PLAY_HAND_START + 1)));

    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("select CS Tamer");
    runner.auto_resolve().expect("play selected Tamer");

    assert_eq!(runner.battle_area_size(0), field_before + 1);
    assert_eq!(runner.memory(), memory_before, "free play spends no memory");
}

#[test]
fn bt22_046_when_digivolving_is_blocked_with_two_tamers() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-046")
        .expect("BT22-046 YAML loads")
        .add_card(cs_tamer("CS-TAMER"))
        .add_card(tamer("TAMER-A"))
        .add_card(tamer("TAMER-B"))
        .hand(0, &["CS-TAMER"])
        .start();
    let gargomon = runner.place_on_field(0, "BT22-046", Some(0));
    runner.place_on_field(0, "TAMER-A", Some(0));
    runner.place_on_field(0, "TAMER-B", Some(0));

    fire_when_digivolving(&mut runner, gargomon);

    assert!(
        runner.pending_selection_view().is_none(),
        "2 Tamers must fail the '1 or fewer Tamers' gate"
    );
}

#[test]
fn bt22_046_when_digivolving_can_be_declined() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-046")
        .expect("BT22-046 YAML loads")
        .add_card(cs_tamer("CS-TAMER"))
        .hand(0, &["CS-TAMER"])
        .start();
    let gargomon = runner.place_on_field(0, "BT22-046", Some(0));
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    fire_when_digivolving(&mut runner, gargomon);
    runner.execute_action(0, PASS).expect("decline free play");
    runner.auto_resolve().expect("finish decline");

    assert_eq!(runner.hand_size(0), hand_before);
    assert_eq!(runner.battle_area_size(0), field_before);
}

fn fire_when_digivolving(
    runner: &mut DebugRunner,
    source: digimon_engine::permanent::PermanentHandle,
) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}

fn cs_tamer(id: &str) -> CardData {
    let mut card = tamer(id);
    card.traits = vec!["CS".to_string()];
    card.colors = vec![CardColor::Yellow];
    card
}

fn tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}
