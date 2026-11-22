//! EX4-019 official Bandai entry and printed image checked 2026-11-22.
//! Separate exact-Mirage-deck prerequisite; behavioral coverage is not admission.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_digivolve, PASS},
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
use digimon_engine::permanent::PermanentHandle;
const ID: &str = "EX4-019";
fn card(id: &str, lv: u8, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(lv);
    c.colors = vec![color];
    c.dp = Some(if id == "TOP" { 12000 } else { 1000 });
    c
}
fn setup() -> DebugRunner {
    let mut t = make_test_card("TAMER", "Tamer");
    t.card_kind = CardKind::Tamer;
    t.level = Some(3);
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("EX4-019 production DSL")
        .add_card(card("FILL", 3, CardColor::Blue))
        .add_card(card("FOUR", 4, CardColor::Blue))
        .add_card(card("RED", 4, CardColor::Red))
        .add_card(card("TOP", 6, CardColor::Blue))
        .add_card(t)
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .security(0, &["FILL"; 5])
        .security(1, &["FILL"; 5])
        .build();
    r.set_first_player(0);
    r.set_turn(2);
    r.set_phase(GamePhase::Main);
    r.game.set_memory(10);
    r
}
fn act(r: &mut DebugRunner, p: u8, a: u16) {
    assert_eq!(build_action_mask(&r.game, p)[a as usize], 1.0);
    r.game.decode_action(a, p);
}
fn evolve(r: &mut DebugRunner) {
    let h = r.place_on_field(0, "FOUR", Some(0));
    r.add_to_hand(0, ID);
    act(r, 0, encode_digivolve(0, h.index as u16));
}
fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}
fn fill_hand(r: &mut DebugRunner, p: u8, n: usize) {
    for _ in 0..n {
        r.add_to_hand(p, "FILL");
    }
}
#[test]
fn ex4_019_identity_and_standard_evolution_cost() {
    let mut r = setup();
    let c = r.compiled_card(ID).unwrap();
    assert_eq!(c.level, Some(5));
    assert_eq!(c.dp, Some(7000));
    assert_eq!(c.cost, Some(7));
    let d = r.deck_size(0);
    evolve(&mut r);
    assert_eq!(r.memory(), 7);
    assert_eq!(r.deck_size(0), d - 1);
    assert!(r.pending_selection().is_none());
}
#[test]
fn ex4_019_actual_evolution_mandatory_choice_filters_and_bounces() {
    for pick in 0..2 {
        let mut r = setup();
        r.place_on_field(1, "FILL", Some(0));
        r.place_stack(1, &["FILL", "FOUR"]);
        r.place_on_field(1, "TOP", Some(2));
        r.place_on_field(1, "TAMER", Some(3));
        r.place_on_field(0, "FILL", Some(1));
        r.place_in_breeding(1, "FILL");
        evolve(&mut r);
        let v = r.pending_selection_view().expect("mandatory bounce choice");
        assert!(!v.is_optional);
        assert_eq!(v.valid_action_ids.len(), 2);
        assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
        act(&mut r, 0, v.valid_action_ids[pick]);
        assert_eq!(r.hand_size(1), 1);
        assert_eq!(r.battle_area_size(1), 3);
        assert_eq!(r.battle_area_size(0), 2);
        assert_eq!(
            r.game.players[1].hand[0].card_id(&r.game.card_data),
            if pick == 0 { "FILL" } else { "FOUR" }
        );
        assert_eq!(r.game.players[1].trash.len(), pick);
    }
}
#[test]
fn ex4_019_no_eligible_target_and_no_on_play_bounce() {
    let mut r = setup();
    r.place_on_field(1, "TOP", Some(0));
    r.place_on_field(1, "TAMER", Some(1));
    r.place_on_field(0, "FILL", Some(1));
    evolve(&mut r);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(1), 0);
    let mut r = setup();
    r.place_on_field(1, "FILL", Some(0));
    r.add_to_hand(0, ID);
    r.play(0, 0);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(1), 1);
}
#[test]
fn ex4_019_wrong_level_or_color_cannot_evolve() {
    for id in ["RED", "FILL", "TOP"] {
        let mut r = setup();
        r.place_on_field(0, id, Some(0));
        r.add_to_hand(0, ID);
        assert_eq!(
            build_action_mask(&r.game, 0)[encode_digivolve(0, 0) as usize],
            0.0
        );
    }
}
#[test]
fn ex4_019_inherited_real_attack_hand_seven_eight_nine() {
    for n in [7, 8, 9] {
        let mut r = setup();
        fill_hand(&mut r, 1, n);
        let h = r.place_stack(0, &[ID, "TOP"]);
        r.attack_player(h, 1, false);
        assert_eq!(suspended(&r, h), n < 8, "hand={n}");
        assert_eq!(r.security_count(1), 4);
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn ex4_019_own_hand_does_not_satisfy_condition() {
    let mut r = setup();
    fill_hand(&mut r, 0, 9);
    fill_hand(&mut r, 1, 7);
    let h = r.place_stack(0, &[ID, "TOP"]);
    r.attack_player(h, 1, false);
    assert!(suspended(&r, h));
}
#[test]
fn ex4_019_top_face_and_other_attacker_do_not_use_inherited() {
    for face in [true, false] {
        let mut r = setup();
        fill_hand(&mut r, 1, 8);
        let h = if face {
            r.place_on_field(0, ID, Some(0))
        } else {
            r.place_stack(0, &[ID, "TOP"]);
            r.place_on_field(0, "TOP", Some(1))
        };
        r.attack_player(h, 1, false);
        assert!(suspended(&r, h));
    }
}
#[test]
fn ex4_019_inherited_once_per_turn_and_resets() {
    let mut r = setup();
    fill_hand(&mut r, 1, 8);
    let h = r.place_stack(0, &[ID, "TOP"]);
    r.attack_player(h, 1, false);
    assert!(!suspended(&r, h));
    r.attack_player(h, 1, false);
    assert!(suspended(&r, h));
    r.end_turn();
    r.end_turn();
    r.set_phase(GamePhase::Main);
    r.attack_player(h, 1, false);
    assert!(!suspended(&r, h));
    assert_eq!(r.security_count(1), 2);
}
#[test]
fn ex4_019_inherited_uses_opponent_relative_to_controller() {
    let mut r = setup();
    r.set_first_player(1);
    fill_hand(&mut r, 0, 8);
    let h = r.place_stack(1, &[ID, "TOP"]);
    r.attack_player(h, 0, false);
    assert!(!suspended(&r, h));
    assert_eq!(r.security_count(0), 4);
}
#[test]
fn ex4_019_failed_hand_condition_does_not_consume_opt() {
    let mut r = setup();
    fill_hand(&mut r, 1, 7);
    let h = r.place_stack(0, &[ID, "TOP"]);
    r.attack_player(h, 1, false);
    assert!(suspended(&r, h));
    r.game.players[0].battle_area[h.index as usize].is_suspended = false;
    r.add_to_hand(1, "FILL");
    r.attack_player(h, 1, false);
    assert!(!suspended(&r, h));
}
