//! Capability RED tests: official general rules 4-15-2 (PDF p9), EX2-007 Q3276.
//! Printed metadata only; this is not a partial Mother D-Reaper implementation.
use digimon_engine::action::{mask::build_action_mask, space::{MOVE_FROM_BREEDING, PASS}};
use digimon_engine::debug_runner::{DebugRunner, make_test_card, make_test_egg};
use digimon_engine::enums::{CardColor, GamePhase};
fn runner(id: &str) -> DebugRunner {
    let mut mother = make_test_egg("EX2-007", "Mother D-Reaper");
    mother.level = None; mother.dp = Some(15000); mother.colors = vec![CardColor::White];
    mother.traits = vec!["D-Reaper".into(), "Ability Synthesis Agent".into()];
    let mut r = DebugRunner::builder().add_card(mother)
        .add_card(make_test_egg("PROBE-EGG", "No-DP level2 egg"))
        .add_card(make_test_card("PROBE-ROOKIE", "DP-bearing rookie"))
        .deck(0,&["PROBE-ROOKIE";8]).deck(1,&["PROBE-ROOKIE";8]).memory(3).start();
    r.skip_mulligan(); r.place_in_breeding(0,id); r.set_phase(GamePhase::Breeding); r
}
#[test] fn level_less_dp_digimon_has_legal_breeding_move() {
    let r=runner("EX2-007");
    assert_eq!(build_action_mask(&r.game,0)[MOVE_FROM_BREEDING as usize],1.0,"Q3276: Mother has DP despite no level");
}
#[test] fn level_less_dp_digimon_passes_shared_movement_validator() {
    let mut r=runner("EX2-007"); assert!(r.game.move_from_breeding(0));
    assert!(r.game.players[0].breeding_area.is_none());
    assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"EX2-007");
}
#[test] fn legal_breeding_action_preserves_card_identity_and_orientation() {
    let mut r=runner("EX2-007");
    let before=r.game.players[0].breeding_area.as_ref().unwrap().top_card().handle();
    r.game.players[0].breeding_area.as_mut().unwrap().is_suspended=true;
    assert_eq!(build_action_mask(&r.game,0)[MOVE_FROM_BREEDING as usize],1.0);
    r.game.decode_action(MOVE_FROM_BREEDING,0);
    assert!(r.game.players[0].breeding_area.is_none());
    let p=&r.game.players[0].battle_area[0];assert_eq!(p.top_card().handle(),before);assert!(p.is_suspended);
    assert_eq!(r.current_phase(),GamePhase::Main);
}
#[test] fn cloned_breeding_move_and_pass_are_independent() {
    let mut r=runner("EX2-007");let saved=r.game.clone();
    assert_eq!(build_action_mask(&r.game,0)[MOVE_FROM_BREEDING as usize],1.0);
    r.game.decode_action(MOVE_FROM_BREEDING,0);assert_eq!(r.game.players[0].battle_area.len(),1);
    r.game=saved;r.game.decode_action(PASS,0);
    assert!(r.game.players[0].battle_area.is_empty());assert!(r.game.players[0].breeding_area.is_some());
}
#[test] fn ordinary_no_dp_egg_stays_in_breeding() {
    let mut r=runner("PROBE-EGG");assert_eq!(build_action_mask(&r.game,0)[MOVE_FROM_BREEDING as usize],0.0);
    assert!(!r.game.move_from_breeding(0));assert!(r.game.players[0].breeding_area.is_some());
}
#[test] fn ordinary_dp_rookie_still_moves() {
    let mut r=runner("PROBE-ROOKIE");assert_eq!(build_action_mask(&r.game,0)[MOVE_FROM_BREEDING as usize],1.0);
    r.game.decode_action(MOVE_FROM_BREEDING,0);assert!(r.game.players[0].breeding_area.is_none());
}
#[test] fn full_battle_area_prevents_level_less_dp_move() {
    let mut r=runner("EX2-007");for _ in 0..r.game.rules.field_slots {r.place_on_field(0,"PROBE-ROOKIE",Some(0));}
    assert_eq!(build_action_mask(&r.game,0)[MOVE_FROM_BREEDING as usize],0.0);
    assert!(!r.game.move_from_breeding(0));assert!(r.game.players[0].breeding_area.is_some());
}

// Rule 4-15-2 / Mother Q3276: turn entry must preserve the legal DP-based action.
#[test]
fn next_turn_keeps_level_less_dp_breeding_move_available() {
    let mut r = runner("EX2-007");
    r.game.decode_action(PASS, 0);
    for _ in 0..2 {
        let actor = r.game.turn_player();
        assert_eq!(build_action_mask(&r.game, actor)[PASS as usize], 1.0);
        r.game.decode_action(PASS, actor);
    }
    assert_eq!(r.game.turn_player(), 0);
    assert_eq!(r.current_phase(), GamePhase::Breeding);
    assert_eq!(
        build_action_mask(&r.game, 0)[MOVE_FROM_BREEDING as usize],
        1.0
    );
    let saved = r.game.clone();
    for _ in 0..2 {
        r.game = saved.clone();
        r.game.decode_action(MOVE_FROM_BREEDING, 0);
        assert!(r.game.players[0].breeding_area.is_none());
        assert_eq!(r.current_phase(), GamePhase::Main);
    }
}
#[test]
fn next_turn_still_skips_breeding_for_no_dp_egg() {
    let mut r = runner("PROBE-EGG");
    r.game.decode_action(PASS, 0);
    for _ in 0..2 {
        let actor = r.game.turn_player();
        assert_eq!(build_action_mask(&r.game, actor)[PASS as usize], 1.0);
        r.game.decode_action(PASS, actor);
    }
    assert_eq!(r.game.turn_player(), 0);
    assert_eq!(r.current_phase(), GamePhase::Main);
    assert!(r.game.players[0].breeding_area.is_some());
}
