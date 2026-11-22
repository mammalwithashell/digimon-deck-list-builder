//! BT27-099 Surtr Brander. Official Bandai English image/list, 2026-11-22.
//! Q7956: resolve the whole effect before zero-DP rule checks.
//! Q7957: effect-trash activation requires direct removal from security.
//! Patterns: mandatory target choice, turn-scoped DP, security exchange,
//! security-discard trigger. DCGO unavailable in this fresh cloud checkout.
use digimon_engine::action::space::{PLAY_HAND_START, ATTACK_START};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, GamePhase};

fn base() -> DebugRunnerBuilder {
    let mut body = make_test_card("BODY", "Body");
    body.colors = vec![CardColor::Yellow]; body.dp = Some(10000);
    let mut low = body.clone(); low.card_id = "LOW".into(); low.dp = Some(7000);
    DebugRunner::builder().dsl_card("BT27-099").expect("Surtr Brander compiles")
        .add_card(body).add_card(low)
        .from_dsl_yaml("card: TRASHER\nname: Trasher\nkind: digimon\nlevel: 3\ncolor: [yellow]\ncost: 0\ndp: 2000\neffects:\n  - when: on_play\n    process:\n      - trash_top_security: { of: you }\n").unwrap()
        .deck(0, &["BODY"; 12]).deck(1, &["BODY"; 12]).memory(10)
}
fn main_phase(mut r: DebugRunner) -> DebugRunner {
    r.game.turn_player_idx=0; r.game.memory_pair=(0,1); r.game.current_phase=GamePhase::Main;
    r.game.turn_count=3; r
}
fn sec(r: &DebugRunner, p: usize) -> Vec<String> {
    r.game.players[p].security.iter().map(|c|c.card_id(&r.game.card_data).to_string()).collect()
}
#[test]
fn bt27_099_main_reduces_chosen_target_and_exchanges_security() {
    let mut r=main_phase(base().hand(0,&["BT27-099"]).security(0,&["LOW"]).start());
    r.place_on_field(0,"BODY",Some(0));
    let a=r.place_on_field(1,"BODY",Some(0)); let b=r.place_on_field(1,"LOW",Some(0));
    assert!(digimon_engine::action::build_action_mask(&r.game,0)[PLAY_HAND_START as usize] > 0.0); r.game.decode_action(PLAY_HAND_START,0);
    let v=r.pending_selection_view().unwrap();
    assert_eq!(v.valid_action_ids.len(),2); assert!(!r.pending_is_optional());
    r.execute_action(0, ATTACK_START+a.index as u16).unwrap();
    assert_eq!(r.effective_dp(a),Some(4000)); assert_eq!(r.effective_dp(b),Some(7000));
    assert_eq!(sec(&r,0),vec!["BT27-099"]);
    assert!(r.game.players[0].hand.iter().any(|c|c.card_id(&r.game.card_data)=="LOW"));
    r.end_turn(); assert_eq!(r.effective_dp(a),Some(10000));
}
#[test]
fn bt27_099_main_empty_opponent_and_security_still_places_option() {
    let mut r=main_phase(base().hand(0,&["BT27-099"]).security(0,&[]).start());
    r.place_on_field(0,"BODY",Some(0)); assert!(digimon_engine::action::build_action_mask(&r.game,0)[PLAY_HAND_START as usize] > 0.0); r.game.decode_action(PLAY_HAND_START,0);
    assert!(r.pending_selection().is_none()); assert_eq!(sec(&r,0),vec!["BT27-099"]);
}
#[test]
fn bt27_099_effect_trash_offers_only_tied_lowest_dp_digimon() {
    let mut r=main_phase(base().hand(0,&["TRASHER"]).security(0,&["BT27-099"]).start());
    let a=r.place_on_field(1,"LOW",Some(0)); let b=r.place_on_field(1,"LOW",Some(0));
    r.place_on_field(1,"BODY",Some(0));
    assert!(digimon_engine::action::build_action_mask(&r.game,0)[PLAY_HAND_START as usize] > 0.0); r.game.decode_action(PLAY_HAND_START,0);
    let v=r.pending_selection_view().unwrap();
    assert_eq!(v.valid_action_ids,vec![ATTACK_START+a.index as u16,ATTACK_START+b.index as u16]);
    assert!(!r.pending_is_optional());
    r.execute_action(0, ATTACK_START+b.index as u16).unwrap();
    assert_eq!(sec(&r,1).last().map(String::as_str),Some("LOW"));
    assert_eq!(r.game.players[1].battle_area.len(),2);
    assert!(r.game.players[0].trash.iter().any(|c|c.card_id(&r.game.card_data)=="BT27-099"));
}
#[test]
fn bt27_099_effect_trash_empty_opponent_has_no_selection() {
    let mut r=main_phase(base().hand(0,&["TRASHER"]).security(0,&["BT27-099"]).start());
    assert!(digimon_engine::action::build_action_mask(&r.game,0)[PLAY_HAND_START as usize] > 0.0); r.game.decode_action(PLAY_HAND_START,0); assert!(r.pending_selection().is_none());
    assert!(sec(&r,0).is_empty()); assert!(sec(&r,1).is_empty());
}
#[test]
fn bt27_099_security_check_moves_lowest_opponent_to_security() {
    let mut r=main_phase(base().security(1,&["BT27-099"]).start());
    let attacker=r.place_on_field(0,"BODY",Some(0)); let low=r.place_on_field(0,"LOW",Some(0));
    r.attack_player(attacker,1,false);
    let v=r.pending_selection_view().expect("security target choice");
    assert_eq!(v.valid_action_ids,vec![ATTACK_START+low.index as u16]);
    r.execute_action(v.selecting_player, v.valid_action_ids[0]).unwrap();
    assert_eq!(sec(&r,0).last().map(String::as_str),Some("LOW"));
}
