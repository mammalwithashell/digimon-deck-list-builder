//! Independent behavioral witnesses for own-carrier effect immunity exclusion.
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{EffectTiming, Expiry, Keyword, ModifierType};
use digimon_engine::selection::TriggerSource;
const SHIELD: &str = r#"
card: OWN-SHIELD
name: Own Shield
kind: digimon
level: 6
color: [yellow]
cost: 12
dp: 12000
effects:
  - when: when_digivolving
    process:
      - grant_effect_immunity:
          target: self
          source_kind: digimon
          source_controller: any
          exclude_self: true
          expiry: end_of_opponents_turn
"#;
fn fixture() -> (DebugRunner, digimon_engine::permanent::PermanentHandle, digimon_engine::permanent::PermanentHandle) {
    let mut r = DebugRunner::builder().from_dsl_yaml(SHIELD).unwrap()
        .add_card(make_test_card("OTHER", "Other"))
        .deck(0, &["OTHER"; 10]).deck(1, &["OTHER"; 10]).start();
    let h = r.place_on_field(0, "OWN-SHIELD", None);
    let other = r.place_on_field(0, "OTHER", None);
    (r, h, other)
}
fn shield(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle) {
    r.game.enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
#[test]
fn own_one_shot_suspends_but_other_friendly_effect_cannot() {
    let (mut r, h, other) = fixture(); shield(&mut r, h);
    let other_card = r.game.players[0].battle_area[other.index as usize].top_card().handle();
    EffectContext::new(&mut r.game, other_card, Some(other), 0).suspend(h);
    assert!(!r.game.players[0].battle_area[h.index as usize].is_suspended);
    let own_card = r.game.players[0].battle_area[h.index as usize].top_card().handle();
    EffectContext::new(&mut r.game, own_card, Some(h), 0).suspend(h);
    assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
}
#[test]
fn preexisting_other_friendly_modifier_is_suppressed_but_own_modifier_remains() {
    let (mut r, h, other) = fixture();
    let other_card = r.game.players[0].battle_area[other.index as usize].top_card().handle();
    EffectContext::new(&mut r.game, other_card, Some(other), 0)
        .add_modifier(h, ModifierType::ChangeDp, -4000, Expiry::EndOfTurn);
    let own_card = r.game.players[0].battle_area[h.index as usize].top_card().handle();
    EffectContext::new(&mut r.game, own_card, Some(h), 0)
        .add_modifier(h, ModifierType::ChangeDp, 2000, Expiry::EndOfTurn);
    assert_eq!(r.game.effective_dp(h), Some(10000));
    shield(&mut r, h);
    assert_eq!(r.game.effective_dp(h), Some(14000));
}
#[test]
fn granted_keywords_distinguish_own_from_other_friendly_carrier() {
    let (mut r, h, other) = fixture();
    let own_card = r.game.players[0].battle_area[h.index as usize].top_card().handle();
    let other_card = r.game.players[0].battle_area[other.index as usize].top_card().handle();
    EffectContext::new(&mut r.game, own_card, Some(h), 0).grant_keyword(h, Keyword::Blocker, Expiry::EndOfTurn);
    EffectContext::new(&mut r.game, other_card, Some(other), 0).grant_keyword(h, Keyword::Rush, Expiry::EndOfTurn);
    assert!(r.game.has_keyword(h, Keyword::Blocker));
    assert!(r.game.has_keyword(h, Keyword::Rush));
    shield(&mut r, h);
    assert!(r.game.has_keyword(h, Keyword::Blocker));
    assert!(!r.game.has_keyword(h, Keyword::Rush));
}

#[test]
fn parked_own_effect_keeps_carrier_identity_after_lower_slot_departure() {
    let parked = format!("{}{}", SHIELD, r#"
  - when: on_play
    process:
      - select_any_permanent:
          bind_as: lower
          filter: { name_is: Lower }
          prompt: Delete lower slot
      - delete_permanent: { target: lower }
      - select_effect_choice:
          bind_as: confirm
          labels: [Continue, Continue also]
          prompt: Finish own effect
      - add_modifier:
          target: self
          modifier: ChangeDp
          value: 2000
          expiry: end_of_turn
"#);
    let mut r = DebugRunner::builder().from_dsl_yaml(&parked).unwrap()
        .add_card(make_test_card("LOWER", "Lower"))
        .deck(0, &["LOWER"; 10]).deck(1, &["LOWER"; 10]).start();
    r.place_on_field(0, "LOWER", None);
    let h = r.place_on_field(0, "OWN-SHIELD", None);
    shield(&mut r, h);
    r.game.enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    let initial = r.pending_selection_view().expect("select the lower slot");
    assert_eq!(initial.valid_action_ids.len(), 1);
    r.execute_action(initial.selecting_player, initial.valid_action_ids[0]).unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 1, "the causing effect really deletes the lower slot");
    let v = r.pending_selection_view().expect("own effect parks after lower departure");
    let action = v.valid_action_ids[0];
    assert!(digimon_engine::action::build_action_mask(&r.game, v.selecting_player)[action as usize] > 0.0);
    r.execute_action(v.selecting_player, action).unwrap();
    r.auto_resolve().unwrap();
    let shifted = digimon_engine::permanent::PermanentHandle { player: 0, index: 0 };
    assert_eq!(r.game.effective_dp(shifted), Some(14000));
}
