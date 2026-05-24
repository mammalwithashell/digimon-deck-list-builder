//! BT10-042 Venusmon.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::encode_attack;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::selection::TriggerSource;
use digimon_engine::PermanentHandle;

const YAML: &str = include_str!("../../../cards/bt10/BT10-042.yaml");

const EFFECT_DIGIMON_YAML: &str = r#"
card: TEST-EFFECT-DIGIMON
name: Test Effect Digimon
kind: digimon
level: 4
color: [yellow]
cost: 4
dp: 5000
traits: [Test]
effects:
  - when: when_attacking
    process:
      - draw: { of: you, count: 1 }
  - when: when_digivolving
    process:
      - draw: { of: you, count: 1 }
"#;

#[test]
fn bt10_042_has_printed_metadata_and_security_attack_change_auras() {
    let runner = venusmon_runner().start();
    let compiled = runner
        .compiled_card("BT10-042")
        .expect("BT10-042 compiled card present");

    assert_eq!(compiled.card, "BT10-042");
    assert_eq!(compiled.name, "Venusmon");
    assert_eq!(compiled.level, Some(6));
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.cost, Some(13));
    assert_eq!(compiled.dp, Some(12000));
    assert_eq!(compiled.attribute.as_deref(), Some("Vaccine"));
    assert!(compiled.traits.iter().any(|t| t == "Shaman"));
    assert!(compiled.traits.iter().any(|t| t == "Olympos XII"));

    let when_digivolving = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::WhenDigivolving] => {
                Some(t)
            }
            _ => None,
        })
        .expect("When Digivolving clause");
    assert!(when_digivolving.process.iter().any(|step| matches!(
        step,
        CompiledStep::AddModifier { modifier, .. } if modifier == "SecurityAttackChange"
    )));

    assert!(
        compiled.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                active_when: Some(active),
                modifier: Some(modifier),
                ..
            }) if active.opponents_turn == Some(true)
                && modifier == "CannotBeAttackedBySecurityAttackChanged"
        )),
        "Venusmon must carry the opponent-turn attack-target restriction"
    );

    for modifier_name in [
        "CannotActivateWhenAttackingEffects",
        "CannotActivateWhenDigivolvingEffects",
    ] {
        assert!(
            compiled.effects.iter().any(|clause| matches!(
                clause,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                    active_when: Some(active),
                    target,
                    modifier: Some(modifier),
                    ..
                }) if active.opponents_turn == Some(true)
                    && target.kind == Some(CompiledCardKind::Digimon)
                    && target.has_security_attack_change == Some(true)
                    && modifier == modifier_name
            )),
            "missing Security-A-changed target aura for {modifier_name}"
        );
    }
}

#[test]
fn bt10_042_when_digivolving_gives_all_opponent_digimon_security_attack_minus_one() {
    let mut runner = venusmon_runner().start();
    let venusmon = runner.place_on_field(0, "BT10-042", Some(0));
    let first = runner.place_on_field(1, "VANILLA", Some(0));
    let second = runner.place_on_field(1, "TEST-EFFECT-DIGIMON", Some(0));
    runner.place_on_field(1, "TEST-TAMER", Some(0));

    fire_when_digivolving(&mut runner, venusmon);

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(first, ModifierType::SecurityAttackChange),
        -1
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(second, ModifierType::SecurityAttackChange),
        -1
    );
    assert_eq!(
        runner.game.has_security_attack_change(first),
        true,
        "temporary SecurityAttackChange must satisfy the Security A. predicate"
    );
}

#[test]
fn bt10_042_only_security_attack_changed_digimon_cannot_attack_venusmon() {
    let mut runner = venusmon_runner().start();
    let venusmon = runner.place_on_field(0, "BT10-042", Some(0));
    let changed_attacker = runner.place_on_field(1, "TEST-EFFECT-DIGIMON", Some(0));
    let plain_attacker = runner.place_on_field(1, "VANILLA", Some(0));
    runner.end_turn();
    runner.game.current_phase = GamePhase::Main;
    runner.game.players[0].battle_area[venusmon.index as usize].is_suspended = true;
    runner.game.modifiers.add(
        changed_attacker,
        ModifierEntry::simple(
            ModifierType::SecurityAttackChange,
            1,
            Expiry::Permanent,
            changed_attacker.player,
        ),
    );
    for handle in [changed_attacker, plain_attacker] {
        runner.game.modifiers.add(
            handle,
            ModifierEntry::simple(ModifierType::GrantRush, 0, Expiry::Permanent, handle.player),
        );
    }
    runner.game.tick_declarative_effects();

    let mask = build_action_mask(&runner.game, 1);
    assert_eq!(
        mask[encode_attack(changed_attacker.index as u16, venusmon.index as u16) as usize],
        0.0,
        "Security-A-changed Digimon must not see Venusmon as an attack target"
    );
    assert_eq!(
        mask[encode_attack(plain_attacker.index as u16, venusmon.index as u16) as usize],
        1.0,
        "plain Digimon may still attack the suspended Venusmon"
    );
    assert_eq!(
        runner.attack_digimon(changed_attacker, venusmon, false),
        AttackResult::Invalid
    );
    assert_ne!(
        runner.attack_digimon(plain_attacker, venusmon, false),
        AttackResult::Invalid,
        "the restriction must not become a blanket CannotAttackTarget"
    );
}

#[test]
fn bt10_042_security_attack_changed_opponent_digimon_cannot_activate_attack_or_digivolve_effects() {
    let mut runner = venusmon_runner().deck(1, &["DRAWN-A", "DRAWN-B"]).start();
    runner.place_on_field(0, "BT10-042", Some(0));
    let attacker = runner.place_on_field(1, "TEST-EFFECT-DIGIMON", Some(0));
    runner.end_turn();
    runner.game.modifiers.add(
        attacker,
        ModifierEntry::simple(
            ModifierType::SecurityAttackChange,
            1,
            Expiry::Permanent,
            attacker.player,
        ),
    );
    runner.game.tick_declarative_effects();

    let hand_before = runner.hand_size(1);
    let deck_before = runner.deck_size(1);

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(attacker),
    );
    runner.game.drain_effect_queue();
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(attacker),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.hand_size(1),
        hand_before,
        "When Attacking and When Digivolving draw effects must both be suppressed"
    );
    assert_eq!(runner.deck_size(1), deck_before);
}

fn venusmon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT10-042 YAML loads")
        .from_dsl_yaml(EFFECT_DIGIMON_YAML)
        .expect("test effect Digimon YAML loads")
        .add_card(make_digimon("VANILLA"))
        .add_card(make_tamer("TEST-TAMER"))
        .add_card(make_draw_card("DRAWN-A"))
        .add_card(make_draw_card("DRAWN-B"))
}

fn make_digimon(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card
}

fn make_tamer(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card
}

fn make_draw_card(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(1000);
    card
}

fn fire_when_digivolving(runner: &mut DebugRunner, source: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}
