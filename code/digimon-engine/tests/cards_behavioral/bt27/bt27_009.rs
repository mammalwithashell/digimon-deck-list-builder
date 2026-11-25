use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::owned_card_choice::CardChoiceZone;
use digimon_engine::selection::TriggerSource;

#[test]
fn bt27_009_attack_offers_chronicle_digivolution_from_hand_or_trash() {
    let mut r = super::board("BT27-009");
    let attacker = r.place_on_field(0, "BT27-009", Some(0));
    r.add_to_hand(0, "BT27-013");
    r.add_to_hand(0, "LEVEL4");
    r.inject_trash(0, "BT27-013");
    r.set_turn(2);
    r.attack_player(attacker, 1, false);
    let prompt = r
        .pending_selection_view()
        .expect("Choose a digivolution zone");
    assert!(prompt.is_optional);
    let rows = prompt
        .card_choice
        .expect("Hand and trash appear in the union picker")
        .rows;
    let candidates: Vec<_> = rows
        .iter()
        .filter(|row| row.card_id.as_deref() == Some("BT27-013"))
        .collect();
    assert_eq!(candidates.len(), 2);
    assert!(candidates
        .iter()
        .any(|row| matches!(row.zone, Some(CardChoiceZone::Hand))));
    assert!(candidates
        .iter()
        .any(|row| matches!(row.zone, Some(CardChoiceZone::Trash))));
    r.decline_prompt().unwrap();
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
    assert_eq!(r.security_count(1), 0);
}

#[test]
fn bt27_009_inherited_dp_only_applies_on_your_turn() {
    let mut r = super::board("BT27-009");
    let carrier = r.place_stack(0, &["BT27-009", "LEVEL4"]);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(carrier), Some(9000));
    r.set_first_player(1);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(carrier), Some(7000));
}

#[test]
fn bt27_009_boosts_the_chosen_digimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT27-009")
        .unwrap()
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .start();
    let ginryumon = r.place_on_field(0, "BT27-009", Some(0));
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(ginryumon));
    r.game.drain_effect_queue();
    let pick = r.pending_selection_view().expect("Choose an own Digimon");
    assert!(!pick.is_optional);
    assert_eq!(pick.valid_action_ids.len(), 1);
    r.execute_action(0, pick.valid_action_ids[0]).unwrap();
    assert_eq!(r.effective_dp(ginryumon), Some(8000));
}
