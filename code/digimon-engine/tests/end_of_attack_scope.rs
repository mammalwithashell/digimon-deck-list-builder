//! CR 4.3 11-1-5, 11-5-1-4, 11-6-2 and 15-16-15-1.
//! Printed [End of Attack] is carrier-scoped; native broad observers and
//! scheduled processing retain the broad end-of-an-attack timing.

use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, PASS, SECURITY_TARGET},
    },
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    effect::{CardEffect, Effect, EffectBuilder},
    enums::{EffectTiming, GamePhase},
    permanent::PermanentHandle,
    selection::AttackState,
};
use std::sync::Arc;

const SCOPED: &str = r#"
card: EOA-SCOPED
name: Scoped
kind: digimon
level: 4
color: [red]
cost: 5
dp: 6000
effects:
  - when: end_of_attack
    optional: true
    process: [{gain_memory: 1}]
"#;

const LEAVER: &str = r#"
card: EOA-LEAVER
name: Leaver
kind: digimon
level: 4
color: [red]
cost: 5
dp: 6000
effects:
  - when: when_attacking
    process: [{delete_permanent: {target: source}}]
  - when: end_of_attack
    process: [{gain_memory: 8}]
"#;

const SCHEDULER: &str = r#"
card: EOA-SCHEDULER
name: Scheduler
kind: digimon
level: 4
color: [red]
cost: 5
dp: 6000
effects:
  - when: on_play
    process:
      - schedule_delayed:
          when: end_of_attack
          body: [{gain_memory: 2}]
"#;

const REMOVE_TARGET: &str = r#"
card: EOA-REMOVE-TARGET
name: Remove target
kind: digimon
level: 4
color: [red]
cost: 5
dp: 6000
effects:
  - when: when_attacking
    process:
      - select_opponent_permanent: {bind_as: target, filter: {kind: digimon}, prompt: "Delete the attack target"}
      - delete_permanent: {target: target}
  - when: end_of_attack
    optional: true
    process: [{gain_memory: 1}]
"#;

fn board() -> DebugRunner {
    let mut fill = make_test_card("FILL", "Filler");
    fill.dp = Some(1000);
    fill.level = Some(3);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(SCOPED)
        .unwrap()
        .from_dsl_yaml(LEAVER)
        .unwrap()
        .from_dsl_yaml(SCHEDULER)
        .unwrap()
        .from_dsl_yaml(REMOVE_TARGET)
        .unwrap()
        .add_card(fill)
        .add_card(make_test_card("OBSERVER", "Broad observer"))
        .add_card(make_test_card("REMOVER", "Self-removing observer"))
        .security(1, &["FILL"; 3])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(3)
        .start();
    r.set_first_player(0);
    r.set_phase(GamePhase::Main);
    r
}

fn declare(r: &mut DebugRunner, h: PermanentHandle) {
    let action = encode_attack(h.index as u16, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, h.player)[action as usize], 1.0);
    r.game.decode_action(action, h.player);
}

fn finish(r: &mut DebugRunner) {
    for _ in 0..16 {
        let Some(v) = r.pending_selection_view() else {
            return;
        };
        let action = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .unwrap_or(PASS);
        assert_eq!(
            build_action_mask(&r.game, v.selecting_player)[action as usize],
            1.0
        );
        r.execute_action(v.selecting_player, action).unwrap();
    }
    panic!("unexpected prompt loop");
}

struct BroadObserver;
impl CardEffect for BroadObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_attack(card)
            .name("At the end of an attack, gain 2 memory")
            .process(|ctx| ctx.gain_memory(2))
            .build()]
    }
}

struct CancelDeclaration;
impl CardEffect for CancelDeclaration {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            EffectBuilder::new(card, EffectTiming::WhenWouldBeAttackTarget)
                .replacement_process(|ctx| ctx.cancel())
                .build(),
        ]
    }
}

struct RemoveEarlierCarrier;
impl CardEffect for RemoveEarlierCarrier {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_attack(card)
            .name("Remove earlier carrier")
            .process(|ctx| {
                ctx.bounce_self();
            })
            .build()]
    }
}

#[test]
fn printed_carrier_scope_preserves_native_broad_observers_and_scheduled_processing() {
    let mut r = board();
    r.register_effect("OBSERVER", Arc::new(BroadObserver));
    let attacker = r.place_on_field(0, "FILL", Some(0));
    r.place_on_field(0, "OBSERVER", Some(0));
    let scheduler = r.place_on_field(0, "EOA-SCHEDULER", Some(0));
    r.fire_on_play(0, scheduler.index as usize);
    // The unrelated printed carrier must NOT add its own +1.
    r.place_on_field(0, "EOA-SCOPED", Some(0));
    let before = r.memory();
    declare(&mut r, attacker);
    finish(&mut r);
    assert_eq!(
        r.memory(),
        before + 4,
        "native observer + scheduled body each fire once"
    );
    assert!(r.game.scheduled_effects.is_empty());
    assert!(r.game.pending_attack.is_none());
}

#[test]
fn printed_end_of_attack_qualifier_survives_cloning_and_carrier_compaction() {
    let mut r = board();
    r.register_effect("REMOVER", Arc::new(RemoveEarlierCarrier));
    let earlier = r.place_on_field(0, "REMOVER", Some(0));
    let earlier_card = r.top_card(earlier);
    let attacker = r.place_on_field(0, "EOA-SCOPED", Some(0));
    declare(&mut r, attacker);
    assert!(matches!(
        r.game.pending_attack.as_ref().unwrap().state,
        AttackState::EndOfAttackDrain(_)
    ));
    // Resolve the earlier simultaneous observer first, using its offered
    // physical source identity. The printed carrier then compacts into slot0.
    let view = r
        .pending_selection_view()
        .expect("simultaneous order choice");
    let action = view
        .effect_choices
        .as_ref()
        .expect("trigger choices")
        .iter()
        .find(|choice| choice.source_card == Some(earlier_card))
        .unwrap()
        .action_id;
    assert_eq!(
        build_action_mask(&r.game, view.selecting_player)[action as usize],
        1.0
    );
    r.execute_action(view.selecting_player, action).unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    let before = r.memory();
    let mut fork = DebugRunner::wrap(r.game.clone());
    finish(&mut fork);
    assert_eq!(fork.memory(), before + 1);
    assert!(fork.game.pending_attack.is_none());
    assert!(
        r.game.pending_selection.is_some(),
        "original remains parked"
    );
    finish(&mut r);
    assert_eq!(r.memory(), before + 1);
    assert!(r.game.pending_attack.is_none());
}

#[test]
fn departed_attacker_does_not_transfer_printed_end_of_attack_to_compacted_bystander() {
    let mut r = board();
    r.register_effect("OBSERVER", Arc::new(BroadObserver));
    let attacker = r.place_on_field(0, "EOA-LEAVER", Some(0));
    r.place_on_field(0, "EOA-SCOPED", Some(0));
    r.place_on_field(0, "OBSERVER", Some(0));
    let before = r.memory();
    declare(&mut r, attacker);
    finish(&mut r);
    assert_eq!(
        r.memory(),
        before + 2,
        "11-1-5 timing still occurs; only broad observer fires"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "EOA-SCOPED"
    );
    assert_eq!(
        r.security_count(1),
        3,
        "departed attacker performs no security check"
    );
    assert!(r.game.pending_attack.is_none());
}

#[test]
fn cancelled_before_declaration_cannot_trigger_printed_end_of_attack() {
    let mut r = board();
    r.register_effect("OBSERVER", Arc::new(BroadObserver));
    let attacker = r.place_on_field(0, "EOA-SCOPED", Some(0));
    let defender = r.place_on_field(1, "FILL", Some(0));
    r.game.players[1].battle_area[defender.index as usize].is_suspended = true;
    r.register_effect("FILL", Arc::new(CancelDeclaration));
    r.place_on_field(0, "OBSERVER", Some(0));
    let before = r.memory();
    let action = encode_attack(attacker.index as u16, defender.index as u16);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    finish(&mut r);
    assert_eq!(
        r.memory(),
        before + 2,
        "broad cleanup observer remains; no attack was performed by printed carrier"
    );
    assert!(!r.game.players[0].battle_area[attacker.index as usize].is_suspended);
    assert!(r.game.pending_attack.is_none());
}

#[test]
fn unsuccessful_declared_attack_still_triggers_its_live_carriers_end_of_attack() {
    let mut r = board();
    let attacker = r.place_on_field(0, "EOA-REMOVE-TARGET", Some(0));
    let defender = r.place_on_field(1, "FILL", Some(0));
    r.game.players[1].battle_area[defender.index as usize].is_suspended = true;
    let before = r.memory();
    let action = encode_attack(attacker.index as u16, defender.index as u16);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    finish(&mut r);
    assert_eq!(
        r.memory(),
        before + 1,
        "11-5-1-4: target removal makes the attack unsuccessful, not undeclared"
    );
    assert!(r.game.players[1].battle_area.is_empty());
    assert!(r.game.pending_attack.is_none());
}
