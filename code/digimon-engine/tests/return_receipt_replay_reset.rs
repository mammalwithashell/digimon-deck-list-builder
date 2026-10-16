//! Replay-reset discriminator: actual grouped return allocates a live receipt.
use digimon_engine::{action::build_action_mask,debug_runner::{DebugRunner,make_test_card},enums::GamePhase};
const PRODUCER:&str=r#"
card: RESET-RETURN
name: Reset Return Probe
kind: digimon
level: 3
color: [blue]
cost: 3
dp: 1000
effects:
  - when: on_play
    process:
      - return_all_to_deck:
          over: {all_of: [{of: opponent}, {kind: digimon}]}
          position: bottom
"#;
fn empty_replay_runner()->DebugRunner {
    let mut r=DebugRunner::builder().from_dsl_yaml(PRODUCER).unwrap()
        .add_card(make_test_card("VICTIM-A","Victim A"))
        .add_card(make_test_card("VICTIM-B","Victim B")).start();
    r.game.reset_for_replay();r
}
fn lay_and_park(r:&mut DebugRunner) {
    r.game.current_phase=GamePhase::Main;r.game.set_memory(3);
    r.place_on_field(0,"RESET-RETURN",None);
    r.place_on_field(1,"VICTIM-A",None);r.place_on_field(1,"VICTIM-B",None);
    r.fire_on_play(0,0);
    assert!(r.game.pending_selection.is_some(),"real grouped bottom-deck order decision must park");
    assert_eq!(r.game.players[1].battle_area.len(),2);
}
fn finish(r:&mut DebugRunner) {
    for _ in 0..4 {
        let Some(s)=r.game.pending_selection.as_ref() else {return;};
        let actor=s.selecting_player;let action=s.valid_action_ids[0];
        assert!(build_action_mask(&r.game,actor)[action as usize]>0.5);
        r.game.resolve_selection(actor,action).unwrap();
    }
    panic!("grouped return did not finish in bounded decisions");
}
#[test]
fn replay_reset_reused_grouped_return_matches_fresh_parked_digest_and_clone() {
    let mut reused=empty_replay_runner();lay_and_park(&mut reused);finish(&mut reused);
    assert!(reused.game.players[1].battle_area.is_empty(),"warmup actually committed");
    reused.game.reset_for_replay();
    let mut fresh=empty_replay_runner();
    assert_eq!(reused.game.verification_digest(),fresh.game.verification_digest(),"empty baseline equivalent");
    lay_and_park(&mut reused);lay_and_park(&mut fresh);
    assert_eq!(build_action_mask(&reused.game,0),build_action_mask(&fresh.game,0));
    assert_eq!(reused.game.verification_digest(),fresh.game.verification_digest(),
        "same replay history after reset must assign same live return receipt");
    let mut cloned=DebugRunner::wrap(reused.game.clone());
    finish(&mut reused);finish(&mut fresh);finish(&mut cloned);
    assert_eq!(reused.game.verification_digest(),fresh.game.verification_digest());
    assert_eq!(reused.game.verification_digest(),cloned.game.verification_digest());
}
