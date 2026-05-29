//! EX6-016 Salamon
//!
//! [Start of Your Main Phase] If you have a purple Digimon or Tamer, gain
//! 1 memory.
//!
//! Inherited Effect [When Attacking] [Once Per Turn] 1 of your opponent's
//! Digimon gets -2000 DP for the turn.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

#[test]
fn ex6_016_start_of_main_gains_memory_with_own_purple_digimon_or_tamer() {
    for (id, kind) in [
        ("PURPLE-DIGIMON", CardKind::Digimon),
        ("PURPLE-TAMER", CardKind::Tamer),
    ] {
        let mut card = make_test_card(id, id);
        card.card_kind = kind;
        card.colors = vec![CardColor::Purple];

        let mut runner = DebugRunner::builder()
            .dsl_card("EX6-016")
            .expect("EX6-016 YAML loads")
            .add_card(card)
            .memory(0)
            .start();
        let salamon = runner.place_on_field(0, "EX6-016", Some(0));
        runner.place_on_field(0, id, Some(0));

        fire_start_main(&mut runner, salamon);

        assert_eq!(
            runner.memory(),
            1,
            "{id} should satisfy the purple board gate"
        );
    }
}

#[test]
fn ex6_016_start_of_main_requires_own_purple_permanent() {
    let mut yellow = make_test_card("YELLOW-DIGIMON", "Yellow Digimon");
    yellow.colors = vec![CardColor::Yellow];
    let mut purple = make_test_card("OPP-PURPLE-DIGIMON", "Opp Purple");
    purple.colors = vec![CardColor::Purple];

    let mut runner = DebugRunner::builder()
        .dsl_card("EX6-016")
        .expect("EX6-016 YAML loads")
        .add_card(yellow)
        .add_card(purple)
        .memory(0)
        .start();
    let salamon = runner.place_on_field(0, "EX6-016", Some(0));
    runner.place_on_field(0, "YELLOW-DIGIMON", Some(0));
    runner.place_on_field(1, "OPP-PURPLE-DIGIMON", Some(0));

    fire_start_main(&mut runner, salamon);

    assert_eq!(runner.memory(), 0);
}

#[test]
fn ex6_016_inherited_when_attacking_debuffs_one_opponent_digimon_once() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX6-016")
        .expect("EX6-016 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(digimon_with_dp("OPP", 3000))
        .start();
    let carrier = runner.place_stack(0, &["EX6-016", "CARRIER"]);
    let opp = runner.place_on_field(1, "OPP", Some(0));

    fire_when_attacking(&mut runner, carrier);
    let view = runner
        .pending_selection_view()
        .expect("inherited debuff should select an opponent Digimon");
    let action = view.valid_action_ids[0];
    runner
        .execute_action(0, action)
        .expect("resolve debuff target");

    assert_eq!(
        runner.dp_of(opp),
        Some(1000),
        "3000 DP test card gets -2000 DP"
    );

    fire_when_attacking(&mut runner, carrier);
    assert!(
        runner.pending_selection_view().is_none(),
        "once-per-turn should block a second same-turn debuff"
    );
}

fn fire_start_main(runner: &mut DebugRunner, source: digimon_engine::permanent::PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}

fn fire_when_attacking(
    runner: &mut DebugRunner,
    source: digimon_engine::permanent::PermanentHandle,
) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}

fn digimon_with_dp(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.dp = Some(dp);
    card
}
