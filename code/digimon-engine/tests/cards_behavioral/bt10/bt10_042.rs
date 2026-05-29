//! BT10-042 Venusmon - Digimon, Lv.6, Yellow, DP 12000, Cost 13.
//!
//! # Card Text
//!
//! ```text
//! [When Digivolving] All of your opponent's Digimon gain <Security A. -1>
//! until the end of your opponent's turn.
//! [Opponent's Turn] Your opponent's Digimon with <Security A.> can't attack
//! this Digimon and can't activate [When Attacking] and [When Digivolving]
//! effects.
//! ```

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledModifierValue, CompiledStep,
};
use digimon_engine::action::{build_action_mask, encode_attack};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, GamePhase, ModifierType};
use digimon_engine::selection::TriggerSource;

const TIMING_ATTACKER_YAML: &str = r#"
card: TEST-BT10-TIMING-ATTACKER
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

fn venusmon_runner() -> DebugRunner {
    let mut other_target = make_test_card("TEST-BT10-OTHER-TARGET", "Other Target");
    other_target.dp = Some(12000);
    let mut attacker = make_test_card("TEST-BT10-ATTACKER", "Attacker");
    attacker.dp = Some(7000);

    DebugRunner::builder()
        .dsl_card("BT10-042")
        .expect("BT10-042 found in embedded DSL pack")
        .from_dsl_yaml(TIMING_ATTACKER_YAML)
        .expect("register timing attacker")
        .add_card(other_target)
        .add_card(attacker)
        .memory(10)
        .build()
}

fn fire_venusmon_when_digivolving(
    runner: &mut DebugRunner,
    venusmon: digimon_engine::permanent::PermanentHandle,
) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(venusmon),
    );
    runner.game.drain_effect_queue();
}

fn opponent_turn_main(runner: &mut DebugRunner) {
    runner.game.turn_player_idx = 1;
    runner.game.current_phase = GamePhase::Main;
}

#[test]
fn bt10_042_has_metadata_evolution_and_expected_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("BT10-042")
        .expect("BT10-042 found in embedded DSL pack")
        .build();
    let compiled = runner
        .compiled_card("BT10-042")
        .expect("BT10-042 compiled card present");

    assert_eq!(compiled.name, "Venusmon");
    assert_eq!(compiled.level, Some(6));
    assert_eq!(compiled.cost, Some(13));
    assert_eq!(compiled.dp, Some(12000));
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.traits, vec!["Shaman", "Olympos XII"]);
    assert_eq!(compiled.attribute.as_deref(), Some("Vaccine"));
    assert!(compiled.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && matches!(path.cost, Some(CompiledCost::Literal(4)))
            && path.from.as_ref().is_some_and(|from| {
                from.level_eq == Some(5) && from.color_is == Some(CompiledColor::Yellow)
            })
    }));

    let has_security_attack_debuff = compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Triggered(triggered)
                if triggered.process.iter().any(|step| matches!(
                    step,
                    CompiledStep::AddModifier { modifier, value, .. }
                        if modifier == "SecurityAttackChange"
                            && *value == CompiledModifierValue::Literal(-1)
                ))
        )
    });
    assert!(
        has_security_attack_debuff,
        "When Digivolving must apply SecurityAttackChange -1"
    );

    let lock_modifiers: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { modifier, .. }) => {
                modifier.as_deref()
            }
            _ => None,
        })
        .collect();
    assert!(lock_modifiers.contains(&"CannotAttackSource"));
    assert!(lock_modifiers.contains(&"CannotActivateWhenAttackingEffects"));
    assert!(lock_modifiers.contains(&"CannotActivateWhenDigivolvingEffects"));
}

#[test]
fn bt10_042_when_digivolving_applies_security_attack_minus_one_to_all_opponent_digimon() {
    let mut runner = venusmon_runner();
    let venusmon = runner.place_on_field(0, "BT10-042", Some(0));
    let opponent_a = runner.place_on_field(1, "TEST-BT10-ATTACKER", Some(0));
    let opponent_b = runner.place_on_field(1, "TEST-BT10-TIMING-ATTACKER", Some(0));

    fire_venusmon_when_digivolving(&mut runner, venusmon);

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opponent_a, ModifierType::SecurityAttackChange),
        -1
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opponent_b, ModifierType::SecurityAttackChange),
        -1
    );
}

#[test]
fn bt10_042_opponent_turn_lock_blocks_matching_attacker_and_suppresses_timings() {
    let mut runner = venusmon_runner();
    let venusmon = runner.place_on_field(0, "BT10-042", Some(0));
    let other_target = runner.place_on_field(0, "TEST-BT10-OTHER-TARGET", Some(0));
    let matching_attacker = runner.place_on_field(1, "TEST-BT10-TIMING-ATTACKER", Some(0));
    fire_venusmon_when_digivolving(&mut runner, venusmon);
    let nonmatching_attacker = runner.place_on_field(1, "TEST-BT10-TIMING-ATTACKER", Some(0));

    runner.game.players[0].battle_area[venusmon.index as usize].is_suspended = true;
    runner.game.players[0].battle_area[other_target.index as usize].is_suspended = true;
    opponent_turn_main(&mut runner);
    runner.game.tick_declarative_effects();

    let mask = build_action_mask(&runner.game, 1);
    assert_eq!(
        mask[encode_attack(matching_attacker.index as u16, venusmon.index as u16) as usize],
        0.0,
        "Digimon with Security Attack must not be able to attack Venusmon"
    );
    assert_eq!(
        mask[encode_attack(matching_attacker.index as u16, other_target.index as u16) as usize],
        1.0,
        "Venusmon's lock must not stop that Digimon from attacking other legal targets"
    );
    assert_eq!(
        mask[encode_attack(nonmatching_attacker.index as u16, venusmon.index as u16) as usize],
        1.0,
        "Digimon without Security Attack must still be able to attack Venusmon"
    );

    let before = runner.memory();
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(matching_attacker),
    );
    assert!(
        runner.game.effect_queue.is_empty(),
        "matching opponent Digimon must not queue WhenAttacking"
    );

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(nonmatching_attacker),
    );
    assert_eq!(runner.game.effect_queue.len(), 1);
    runner.game.drain_effect_queue();
    assert_eq!(runner.memory(), before + 1);

    let before_digivolving = runner.memory();
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(matching_attacker),
    );
    assert!(
        runner.game.effect_queue.is_empty(),
        "matching opponent Digimon must not queue WhenDigivolving"
    );
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(nonmatching_attacker),
    );
    assert_eq!(runner.game.effect_queue.len(), 1);
    runner.game.drain_effect_queue();
    assert_eq!(runner.memory(), before_digivolving + 1);
}
