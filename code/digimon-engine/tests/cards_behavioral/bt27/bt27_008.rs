use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::selection::TriggerSource;

#[test]
fn bt27_008_declining_the_chronicle_cost_does_not_draw_or_gain_memory() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT27-008")
        .unwrap()
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .hand(0, &["BT27-008"])
        .memory(4)
        .start();
    r.place_on_field(0, "BT27-008", Some(0));
    r.game.enter_main_phase();
    assert!(r.pending_is_optional());
    r.decline_prompt().unwrap();
    assert_eq!(r.game.memory, 4);
    assert_eq!(r.hand_size(0), 1);
    assert!(r.game.players[0].trash.is_empty());
}

#[test]
fn bt27_008_trashes_only_chronicle_to_draw_and_gain_memory() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT27-008")
        .unwrap()
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .hand(0, &["BT27-008", "FILLER"])
        .memory(4)
        .start();
    let ryudamon = r.place_on_field(0, "BT27-008", Some(0));
    let before = r.game.players[0].hand.len();
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(ryudamon),
    );
    r.game.drain_effect_queue();
    let pick = r.pending_selection_view().expect("Choose a Chronicle card");
    assert!(pick.is_optional);
    assert_eq!(pick.valid_action_ids.len(), 1);
    r.execute_action(0, pick.valid_action_ids[0]).unwrap();
    assert_eq!(r.game.memory, 5);
    assert_eq!(r.game.players[0].hand.len(), before);
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|card| card.card_id(&r.game.card_data) == "BT27-008"));

    let stack = r.place_stack(0, &["BT27-008", "FILLER"]);
    assert!(r.game.has_keyword(stack, Keyword::Raid));
}
