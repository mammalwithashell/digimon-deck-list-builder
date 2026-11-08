//! BT18-098 Dragon's Roar: actual printed face, official bundle and DCGO inspected.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_attack, PASS, PLAY_HAND_START},
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;
const ID: &str = "BT18-098";
const TRASHER: &str = r#"
card: ROAR-TRASHER
name: Security Trasher
kind: digimon
color: [blue]
level: 3
cost: 0
dp: 1000
effects:
  - when: on_play
    process:
      - trash_top_security: { of: you }
"#;
fn creature(id: &str, color: CardColor, traits: &[&str], dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 0;
    c.traits = traits.iter().map(|s| s.to_string()).collect();
    c
}
fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT18-098 production YAML")
        .from_dsl_yaml(TRASHER)
        .unwrap()
        .add_card(creature("DATA", CardColor::Yellow, &["Data"], 8000))
        .add_card(creature("WITCH", CardColor::Yellow, &["Witchelny"], 8000))
        .add_card(creature(
            "PURPLE-WITCH",
            CardColor::Purple,
            &["Witchelny"],
            8000,
        ))
        .add_card(creature("PLAIN", CardColor::Yellow, &["Vaccine"], 8000))
        .add_card(creature("RED", CardColor::Red, &[], 8000))
        .add_card(creature("LOW", CardColor::Blue, &[], 6000))
        .add_card(creature("HIGH", CardColor::Blue, &[], 9000))
        .add_card(creature("PAD", CardColor::Blue, &[], 2000))
        .deck(0, &["PAD"; 20])
        .deck(1, &["PAD"; 20])
}
fn main_runner(security: usize) -> DebugRunner {
    let sec = vec!["PAD"; security];
    let mut r = builder()
        .hand(0, &[ID])
        .security(0, &sec)
        .memory(10)
        .start();
    r.skip_mulligan();
    r.place_on_field(0, "DATA", Some(0));
    r.game.enter_main_phase();
    r
}
fn begin_main(r: &mut DebugRunner) {
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
}
fn accept_payment(r: &mut DebugRunner) {
    let p = r
        .pending_selection_view()
        .expect("optional security payment");
    assert_eq!(p.kind, SelectionKind::Replacement);
    assert!(p.is_optional);
    let mask = build_action_mask(&r.game, p.selecting_player);
    assert_eq!(mask[PASS as usize], 1.0);
    for a in p.valid_action_ids {
        assert_eq!(mask[a as usize], 1.0);
    }
    r.accept_optional_trigger().unwrap();
}
fn pick(r: &mut DebugRunner, h: PermanentHandle) {
    let p = r
        .pending_selection_view()
        .expect("mandatory opponent Digimon choice");
    assert!(!p.is_optional);
    let a = encode_attack(0, h.index as u16);
    assert!(
        p.valid_action_ids.contains(&a),
        "target action {a} in {p:?}"
    );
    let m = build_action_mask(&r.game, p.selecting_player);
    assert_eq!(m[a as usize], 1.0);
    assert_eq!(m[PASS as usize], 0.0);
    r.execute_action(p.selecting_player, a).unwrap();
}
fn in_security(r: &DebugRunner, id: &str) -> bool {
    r.game.players[0]
        .security
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == id)
}
fn in_trash(r: &DebugRunner, id: &str) -> bool {
    r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == id)
}
#[test]
fn bt18_098_color_waiver_requires_own_yellow_data_or_witchelny_digimon() {
    for (ally, expected) in [
        ("DATA", true),
        ("WITCH", true),
        ("PURPLE-WITCH", false),
        ("PLAIN", false),
    ] {
        let mut r = builder()
            .hand(0, &[ID])
            .security(0, &["PAD"])
            .memory(10)
            .start();
        r.skip_mulligan();
        r.place_on_field(0, ally, Some(0));
        r.game.enter_main_phase();
        assert_eq!(
            build_action_mask(&r.game, 0)[PLAY_HAND_START as usize],
            if expected { 1.0 } else { 0.0 },
            "{ally}"
        );
    }
    let mut r = builder()
        .hand(0, &[ID])
        .security(0, &["PAD"])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.place_on_field(1, "DATA", Some(0));
    r.game.enter_main_phase();
    assert_eq!(
        build_action_mask(&r.game, 0)[PLAY_HAND_START as usize],
        0.0,
        "opponent's yellow Data is insufficient"
    );
}
#[test]
fn bt18_098_normal_two_color_requirement_is_still_available() {
    let mut r = builder().hand(0, &[ID]).memory(10).start();
    r.skip_mulligan();
    r.place_on_field(0, "PLAIN", Some(0));
    r.place_on_field(0, "RED", Some(0));
    r.game.enter_main_phase();
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
}
#[test]
fn bt18_098_main_pays_top_security_then_debuffs_and_places_self_on_bottom() {
    let mut r = main_runner(3);
    let target = r.place_on_field(1, "HIGH", Some(0));
    begin_main(&mut r);
    assert_eq!(r.game.players[0].security.len(), 3, "cost waits for accept");
    accept_payment(&mut r);
    assert_eq!(r.game.players[0].security.len(), 2);
    pick(&mut r, target);
    r.auto_resolve().unwrap();
    assert_eq!(r.effective_dp(target), Some(3000));
    assert_eq!(r.game.players[0].security.len(), 3);
    assert_eq!(r.game.players[0].security[0].card_id(&r.game.card_data), ID);
    assert!(!in_trash(&r, ID));
    assert_eq!(r.game.memory, 5, "printed Option use cost is paid");
}
#[test]
fn bt18_098_main_above_two_remaining_security_disposes_to_trash() {
    let mut r = main_runner(4);
    let target = r.place_on_field(1, "HIGH", Some(0));
    begin_main(&mut r);
    accept_payment(&mut r);
    pick(&mut r, target);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].security.len(), 3);
    assert!(!in_security(&r, ID));
    assert!(in_trash(&r, ID));
}
#[test]
fn bt18_098_main_decline_preserves_security_and_does_not_debuff_or_place_self() {
    let mut r = main_runner(2);
    let target = r.place_on_field(1, "HIGH", Some(0));
    begin_main(&mut r);
    let p = r.pending_selection_view().unwrap();
    assert!(p.is_optional);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
    r.decline_optional_trigger().unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].security.len(), 2);
    assert_eq!(r.effective_dp(target), Some(9000));
    assert!(!in_security(&r, ID));
    assert!(in_trash(&r, ID));
}
#[test]
fn bt18_098_main_empty_security_gives_no_debuff_or_self_placement() {
    let mut r = main_runner(0);
    let target = r.place_on_field(1, "HIGH", Some(0));
    begin_main(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(r.effective_dp(target), Some(9000));
    assert!(!in_security(&r, ID));
    assert!(in_trash(&r, ID));
}
#[test]
fn bt18_098_main_without_opponent_targets_still_places_self_after_payment() {
    let mut r = main_runner(2);
    begin_main(&mut r);
    accept_payment(&mut r);
    r.auto_resolve().unwrap();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.game.players[0].security.len(), 2);
    assert!(in_security(&r, ID));
}
#[test]
fn bt18_098_main_dp_debuff_lasts_through_opponents_turn_then_expires() {
    let mut r = main_runner(4);
    let target = r.place_on_field(1, "HIGH", Some(0));
    begin_main(&mut r);
    accept_payment(&mut r);
    pick(&mut r, target);
    r.auto_resolve().unwrap();
    r.end_turn();
    r.auto_resolve().unwrap();
    assert_eq!(r.effective_dp(target), Some(3000));
    r.end_turn();
    r.auto_resolve().unwrap();
    assert_eq!(r.effective_dp(target), Some(9000));
}
#[test]
fn bt18_098_security_trash_deletes_only_eligible_opponent_digimon_then_recovers() {
    let mut r = builder().security(0, &[ID]).start();
    r.skip_mulligan();
    let low = r.place_on_field(1, "LOW", Some(0));
    let high = r.place_on_field(1, "HIGH", Some(0));
    let own = r.place_on_field(0, "LOW", Some(0));
    let trasher = r.place_on_field(0, "ROAR-TRASHER", Some(0));
    r.fire_on_play(0, trasher.index as usize);
    let p = r
        .pending_selection_view()
        .expect("security deletion prompt");
    assert_eq!(p.valid_action_ids.len(), 1);
    assert!(p
        .valid_action_ids
        .contains(&encode_attack(0, low.index as u16)));
    assert!(!p
        .valid_action_ids
        .contains(&encode_attack(0, high.index as u16)));
    pick(&mut r, low);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "HIGH"
    );
    assert_eq!(r.effective_dp(own), Some(6000));
    assert_eq!(r.game.players[0].security.len(), 1);
    assert!(in_trash(&r, ID));
}
#[test]
fn bt18_098_security_trash_with_remaining_security_does_not_recover() {
    let mut r = builder().security(0, &["PAD", ID]).start();
    r.skip_mulligan();
    let low = r.place_on_field(1, "LOW", Some(0));
    let trasher = r.place_on_field(0, "ROAR-TRASHER", Some(0));
    let deck = r.game.players[0].deck.len();
    r.fire_on_play(0, trasher.index as usize);
    pick(&mut r, low);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].security.len(), 1);
    assert_eq!(r.game.players[0].deck.len(), deck);
}
#[test]
fn bt18_098_security_without_targets_still_recovers_from_zero() {
    let mut r = builder().security(0, &[ID]).start();
    r.skip_mulligan();
    let trasher = r.place_on_field(0, "ROAR-TRASHER", Some(0));
    r.fire_on_play(0, trasher.index as usize);
    r.auto_resolve().unwrap();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.game.players[0].security.len(), 1);
    assert!(in_trash(&r, ID));
}
#[test]
fn bt18_098_normal_security_check_activates_security_body_once() {
    let mut r = builder().security(1, &[ID]).memory(0).start();
    r.skip_mulligan();
    let attacker = r.place_on_field(0, "HIGH", Some(0));
    let low = r.place_on_field(0, "LOW", Some(0));
    r.place_on_field(0, "LOW", Some(0));
    assert_ne!(
        r.attack_player(attacker, 1, false),
        digimon_engine::combat::AttackResult::Invalid
    );
    pick(&mut r, low);
    r.auto_resolve().unwrap();
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "a checked card's disposal must not reactivate its security-trash clause"
    );
    assert_eq!(r.game.players[1].security.len(), 1);
    assert!(r.game.players[1]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == ID));
}
