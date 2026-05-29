use digimon_engine::action::{build_action_mask, encode_attack};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const LOCK_SOURCE_YAML: &str = r#"
card: TEST-VENUS-LOCK
name: Venus Lock Source
kind: digimon
level: 6
color: [yellow]
cost: 13
dp: 12000
traits: [Shaman]
attribute: Vaccine
effects:
  - kind: aura
    active_when: { opponents_turn: true }
    target:
      of: opponent
      zone: [battle_area]
      kind: digimon
      security_attack_changed: true
    modifier: CannotAttackSource
  - kind: aura
    active_when: { opponents_turn: true }
    target:
      of: opponent
      zone: [battle_area]
      kind: digimon
      security_attack_changed: true
    modifier: CannotActivateWhenAttackingEffects
  - kind: aura
    active_when: { opponents_turn: true }
    target:
      of: opponent
      zone: [battle_area]
      kind: digimon
      security_attack_changed: true
    modifier: CannotActivateWhenDigivolvingEffects
"#;

const TIMING_ATTACKER_YAML: &str = r#"
card: TEST-TIMING-ATTACKER
name: Timing Attacker
kind: digimon
level: 4
color: [purple]
cost: 4
dp: 5000
traits: [Wizard]
attribute: Virus
effects:
  - when: when_attacking
    process:
      - gain_memory: 1
  - when: when_digivolving
    process:
      - gain_memory: 1
"#;

fn runner_with_lock_source() -> DebugRunner {
    let mut other_target = make_test_card("TEST-OTHER-TARGET", "Other Target");
    other_target.dp = Some(12000);
    let mut security_attacker = make_test_card("TEST-SA-ATTACKER", "Security Attacker");
    security_attacker.dp = Some(7000);
    let mut plain_attacker = make_test_card("TEST-PLAIN-ATTACKER", "Plain Attacker");
    plain_attacker.dp = Some(7000);

    DebugRunner::builder()
        .from_dsl_yaml(LOCK_SOURCE_YAML)
        .expect("register lock source")
        .add_card(other_target)
        .add_card(security_attacker)
        .add_card(plain_attacker)
        .build()
}

fn opponent_turn_main(runner: &mut DebugRunner) {
    runner.game.turn_player_idx = 1;
    runner.game.current_phase = GamePhase::Main;
}

fn grant_security_attack_change(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.modifiers.add(
        handle,
        ModifierEntry::simple(ModifierType::SecurityAttackChange, 1, Expiry::Permanent, 1),
    );
}

#[test]
fn conditional_attack_lock_masks_only_security_attack_digimon_against_source() {
    let mut runner = runner_with_lock_source();
    let source = runner.place_on_field(0, "TEST-VENUS-LOCK", Some(0));
    let other_target = runner.place_on_field(0, "TEST-OTHER-TARGET", Some(0));
    let security_attacker = runner.place_on_field(1, "TEST-SA-ATTACKER", Some(0));
    let plain_attacker = runner.place_on_field(1, "TEST-PLAIN-ATTACKER", Some(0));
    runner.game.players[0].battle_area[source.index as usize].is_suspended = true;
    runner.game.players[0].battle_area[other_target.index as usize].is_suspended = true;
    opponent_turn_main(&mut runner);
    grant_security_attack_change(&mut runner, security_attacker);
    runner.game.tick_declarative_effects();

    let mask = build_action_mask(&runner.game, 1);

    assert_eq!(
        mask[encode_attack(security_attacker.index as u16, source.index as u16) as usize],
        0.0,
        "matching opponent Digimon with Security Attack must not be able to attack the lock source"
    );
    assert_eq!(
        mask[encode_attack(security_attacker.index as u16, other_target.index as u16) as usize],
        1.0,
        "the same attacker must still be able to attack other legal suspended Digimon"
    );
    assert_eq!(
        mask[encode_attack(plain_attacker.index as u16, source.index as u16) as usize],
        1.0,
        "opponent Digimon without Security Attack must not be suppressed by the lock"
    );
}

#[test]
fn conditional_timing_lock_suppresses_only_security_attack_digimon_timings() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LOCK_SOURCE_YAML)
        .expect("register lock source")
        .from_dsl_yaml(TIMING_ATTACKER_YAML)
        .expect("register timing attacker")
        .build();
    let _source = runner.place_on_field(0, "TEST-VENUS-LOCK", Some(0));
    let security_attacker = runner.place_on_field(1, "TEST-TIMING-ATTACKER", Some(0));
    let plain_attacker = runner.place_on_field(1, "TEST-TIMING-ATTACKER", Some(0));
    opponent_turn_main(&mut runner);
    grant_security_attack_change(&mut runner, security_attacker);
    runner.game.tick_declarative_effects();

    let before = runner.memory();
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(security_attacker),
    );
    assert!(
        runner.game.effect_queue.is_empty(),
        "matching opponent Digimon with Security Attack must not queue WhenAttacking"
    );
    runner.game.drain_effect_queue();
    assert_eq!(runner.memory(), before);

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(plain_attacker),
    );
    assert_eq!(
        runner.game.effect_queue.len(),
        1,
        "non-matching opponent Digimon must still queue WhenAttacking"
    );
    runner.game.drain_effect_queue();
    assert_eq!(
        runner.memory(),
        before + 1,
        "non-matching WhenAttacking effect should resolve normally"
    );

    let before_digivolve = runner.memory();
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(security_attacker),
    );
    assert!(
        runner.game.effect_queue.is_empty(),
        "matching opponent Digimon with Security Attack must not queue WhenDigivolving"
    );

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(plain_attacker),
    );
    assert_eq!(
        runner.game.effect_queue.len(),
        1,
        "non-matching opponent Digimon must still queue WhenDigivolving"
    );
    runner.game.drain_effect_queue();
    assert_eq!(
        runner.memory(),
        before_digivolve + 1,
        "non-matching WhenDigivolving effect should resolve normally"
    );
}
