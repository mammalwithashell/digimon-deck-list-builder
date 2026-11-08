use digimon_engine::action::space::{HAND_EFFECT_START, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::card_source::CardHandle;
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::TriggerSource;
use std::sync::Arc;

fn setup(cards: &[&str]) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-006", "TestSix"))
        .add_card(make_test_card("TEST-008", "TestEight"))
        .add_card(make_test_card("OPTIONAL", "Optional"))
        .memory(0).start();
    r.register_effect("OPTIONAL", Arc::new(OptionalWithGate));
    for id in cards { r.place_on_field(0, id, Some(0)); }
    r.game.enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::PlayerBattleArea(0));
    r.game.drain_effect_queue();
    r
}

struct OptionalWithGate;
impl CardEffect for OptionalWithGate {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_your_turn(card).optional().needs_outer_optional_prompt()
            .name("Gain 3 memory").process(|ctx| ctx.gain_memory(3)).build()]
    }
}

#[test]
fn stable_ids_survive_reindexing_cloning_and_wire_serialization() {
    let mut r = setup(&["TEST-006", "TEST-008", "TEST-006"]);
    let before = r.game.trigger_choices();
    assert_eq!(before.len(), 3);
    assert_ne!(before[0].trigger_id, before[2].trigger_id);
    assert_eq!(r.game.clone().trigger_choices()[2].trigger_id, before[2].trigger_id);
    let json = digimon_engine::serialization::to_ui_json(&r.game);
    assert_eq!(json["pendingSelection"]["triggerChoices"][0]["triggerId"], before[0].trigger_id);
    r.game.resolve_selection(0, HAND_EFFECT_START + 1).unwrap();
    let after = r.game.trigger_choices();
    assert_eq!(after[0].trigger_id, before[0].trigger_id);
    assert_eq!(after[1].trigger_id, before[2].trigger_id);
    assert_eq!(after[1].action_id, HAND_EFFECT_START + 1);
}

#[test]
fn optional_activation_metadata_follows_current_engine_gate() {
    let mut r = setup(&["OPTIONAL", "TEST-006"]);
    let before = r.game.trigger_choices();
    assert!(before[0].activation_preset_available);
    r.game.resolve_selection(0, HAND_EFFECT_START).unwrap();
    let gate = r.game.trigger_choices();
    assert_eq!(gate[0].trigger_id, before[0].trigger_id);
    assert!(gate[0].activation_prompt);
    assert_eq!(gate[0].generation, before[0].generation);
    r.game.resolve_selection(0, PASS).unwrap();
    assert_eq!(r.memory(), 5);
}

#[test]
fn repeated_events_from_one_card_get_distinct_ids_and_change_generation() {
    let mut r = setup(&["TEST-006", "TEST-008"]);
    let before = r.game.trigger_choices();
    let old_generation = before[0].generation.clone();
    // Stage another occurrence of the same effect while the original prompt is parked.
    r.game.enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::PlayerBattleArea(0));
    r.game.resolve_selection(0, HAND_EFFECT_START).unwrap();
    let after = r.game.trigger_choices();
    assert!(after.iter().all(|choice| !before.iter().any(|old| old.trigger_id == choice.trigger_id)));
    assert_ne!(after[0].generation, old_generation);
    assert!(r.game.waiting_trigger_choices().iter().any(|choice| choice.trigger_id == before[1].trigger_id));
}
