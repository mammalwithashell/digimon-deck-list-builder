//! BT23-027 Angemon.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn bt23_027_on_play_has_barrier_and_draws_one() {
    let mut runner = angemon_runner().deck(0, &["DRAW-CARD"]).start();
    let angemon = runner.place_on_field(0, "BT23-027", Some(0));

    assert!(runner.game.has_keyword(angemon, Keyword::Barrier));
    fire_on_play(&mut runner, angemon);
    runner.auto_resolve().expect("finish draw");

    assert_eq!(runner.hand_size(0), 1);
}

#[test]
fn bt23_027_inherited_barrier_is_available_from_stack() {
    let mut runner = angemon_runner().start();
    let carrier = runner.place_stack(0, &["BT23-027", "CARRIER"]);

    assert!(runner.game.has_keyword(carrier, Keyword::Barrier));
}

#[test]
fn bt23_027_when_digivolving_your_turn_may_dna_into_shakkoumon_from_hand() {
    let mut runner = angemon_runner()
        .hand(0, &["SHAKKOUMON"])
        .deck(0, &["DRAW-CARD"])
        .memory(10)
        .start();
    let angemon = runner.place_on_field(0, "BT23-027", Some(0));
    runner.place_on_field(0, "DNA-PARTNER", Some(0));

    fire_when_digivolving(&mut runner, angemon);
    let shakkoumon = runner
        .pending_selection_view()
        .expect("your-turn branch should offer Shakkoumon from hand");
    assert_eq!(shakkoumon.kind, SelectionKind::Hand);
    runner
        .execute_action(shakkoumon.selecting_player, shakkoumon.valid_action_ids[0])
        .expect("choose Shakkoumon");

    let pair = runner
        .pending_selection_view()
        .expect("Shakkoumon choice should ask for DNA materials");
    assert_eq!(pair.kind, SelectionKind::Target);
    assert!(
        pair.valid_action_ids.len() >= 2,
        "both own Digimon should be available as DNA materials"
    );
}

fn angemon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT23-027")
        .expect("BT23-027 YAML loads")
        .add_card(digimon_named("SHAKKOUMON", "Shakkoumon", 5, 8000))
        .add_card(digimon_named("DNA-PARTNER", "DNA Partner", 4, 5000))
        .add_card(digimon_named("CARRIER", "Carrier", 5, 7000))
        .add_card(digimon_named("DRAW-CARD", "Draw Card", 3, 1000))
}

fn fire_on_play(runner: &mut DebugRunner, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn fire_when_digivolving(runner: &mut DebugRunner, source: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}

fn digimon_named(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}
