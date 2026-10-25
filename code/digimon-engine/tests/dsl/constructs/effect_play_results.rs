//! Actual play results are scoped to the resolving effect, including Tamer plays.
use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::owned_card_choice::CardChoiceOperation;
use digimon_engine::{EffectTiming, TriggerSource};

const PROBE: &str = r#"
card: PLAY-RESULT-PROBE
name: Play Result Probe
kind: digimon
level: 3
color: [yellow]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - search_own_security_stack:
          bind_as: selected
          optional: true
          filter: {}
          on_select: []
          prompt: Choose a card to play
      - play_security_card: { of: you, card: selected }
      - if:
          condition: { effect_played_any_digimon: true }
          then: [{ gain_memory: 3 }]
          else: [{ gain_memory: 1 }]
  - when: when_attacking
    process:
      - if:
          condition: { effect_played_any_digimon: true }
          then: [{ gain_memory: 3 }]
          else: [{ gain_memory: 1 }]
"#;

fn setup(actor: u8, kind: CardKind) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut target = make_test_card("RESULT-TARGET", "Result Target");
    target.card_kind = kind;
    if kind == CardKind::Tamer {
        target.level = None;
        target.dp = None;
    }
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(PROBE).unwrap()
        .add_card(target)
        .add_card(make_test_card("RESULT-PAD", "Pad"))
        .security(actor, &["RESULT-TARGET"])
        .deck(0, &["RESULT-PAD"; 5])
        .deck(1, &["RESULT-PAD"; 5])
        .memory(4)
        .start();
    r.skip_mulligan();
    r.game.turn_player_idx = actor as usize;
    r.game.enter_main_phase();
    let host = r.place_on_field(actor, "PLAY-RESULT-PROBE", None);
    r.fire_on_play(actor, host.index as usize);
    (r, host)
}

fn commit(r: &DebugRunner, actor: u8) -> u16 {
    let pending = r.game.pending_selection.as_ref().expect("owned security choice");
    let page = pending.card_choice.as_ref().unwrap().view(&r.game, actor);
    let action = page.rows.iter()
        .find(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
        .expect("the physical selected card is offered").action_id;
    assert_eq!(build_action_mask(&r.game, actor)[action as usize], 1.0);
    action
}

#[test]
fn played_result_credits_completed_digimon_and_tamer_plays() {
    for actor in [0, 1] {
        for kind in [CardKind::Digimon, CardKind::Tamer] {
            let (mut r, _) = setup(actor, kind);
            let before = r.game.memory;
            let action = commit(&r, actor);
            r.execute_action(actor, action).unwrap();
            assert!(r.game.pending_selection.is_none());
            assert_eq!(r.game.player(actor).battle_area.len(), 2);
            assert!(r.game.player(actor).security.is_empty());
            assert_eq!(r.game.memory, before + 3);
        }
    }
}

#[test]
fn declining_selection_does_not_publish_a_played_result() {
    for actor in [0, 1] {
        let (mut r, _) = setup(actor, CardKind::Tamer);
        let before = r.game.memory;
        assert_eq!(build_action_mask(&r.game, actor)[PASS as usize], 1.0);
        r.execute_action(actor, PASS).unwrap();
        assert_eq!(r.game.player(actor).battle_area.len(), 1);
        assert_eq!(r.game.player(actor).security.len(), 1);
        assert_eq!(r.game.memory, before + 1);
    }
}

#[test]
fn a_late_play_restriction_does_not_publish_a_played_result() {
    for actor in [0, 1] {
        let (mut r, _) = setup(actor, CardKind::Tamer);
        let before = r.game.memory;
        let action = commit(&r, actor);
        r.game.modifiers.add_player_modifier(actor, PlayerModifierEntry::simple(
            ModifierType::CannotPlayTamerByEffect, 0, Expiry::Permanent, None, actor));
        r.execute_action(actor, action).unwrap();
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.game.player(actor).battle_area.len(), 1);
        assert_eq!(r.game.player(actor).security.len(), 1);
        assert_eq!(r.game.memory, before + 1);
    }
}

#[test]
fn a_previous_effects_completed_play_does_not_leak_into_a_new_effect() {
    for actor in [0, 1] {
        let (mut r, host) = setup(actor, CardKind::Tamer);
        let action = commit(&r, actor);
        r.execute_action(actor, action).unwrap();
        assert_eq!(r.game.player(actor).battle_area.len(), 2);
        let before = r.game.memory;
        r.game.enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(host));
        r.game.drain_effect_queue();
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.game.memory, before + 1);
    }
}
