//! Install after v1 primitive declarations; preserve the captured OnPlay
//! context at host-to-count and through cloned continuations.
//! OnPlay captures target_permanent, not event_host_permanent. Both the
//! host filter and tail require its Tamer target; absent context fails closed.
use digimon_engine::{
    action::{build_action_mask, space::PLAY_HAND_START},
    debug_runner::{make_test_card, DebugRunner},
};
const SOURCE: &str = r#"
card: PREFIX-TRIGGER
name: Context carrier
kind: tamer
color: [white]
cost: 1
effects:
  - when: on_play
    process:
      - pay_source_prefix:
          host_filter:
            all_of:
              - name_is: Context host
              - event_target_kind: tamer
          minimum: 1
          optional: true
      - if:
          condition: {event_target_kind: tamer}
          then:
            - draw: {of: you, count: 3}
"#;
fn choose(r: &mut DebugRunner, a: u16) {
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.execute_action(0, a).unwrap();
}
#[test]
fn prefix_revalidates_and_resumes_under_original_trigger_context_in_clones() {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(SOURCE)
        .unwrap()
        .add_card(make_test_card("PREFIX-HOST", "Context host"))
        .add_card(make_test_card("PREFIX-MATERIAL", "Material"))
        .hand(0, &["PREFIX-TRIGGER"])
        .deck(0, &["PREFIX-MATERIAL"; 12])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.place_stack(0, &["PREFIX-MATERIAL", "PREFIX-HOST"]);
    r.game.enter_main_phase();
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    assert!(r.game.pending_selection.is_some());
    let host_saved = r.game.clone();
    for _ in 0..2 {
        r.game = host_saved.clone();
        choose(&mut r, 100);
        assert!(
            r.game
                .pending_selection
                .as_ref()
                .and_then(|p| p.effect_choices.as_ref())
                .is_some(),
            "original event filter remains true in count prompt"
        );
        let count_saved = r.game.clone();
        for _ in 0..2 {
            r.game = count_saved.clone();
            choose(&mut r, 30);
            assert_eq!(r.game.players[0].trash.len(), 1);
            assert_eq!(
                r.game.players[0].hand.len(),
                3,
                "event-dependent continuation uses original OnPlay context"
            );
            assert!(r.game.pending_selection.is_none());
        }
    }
}
