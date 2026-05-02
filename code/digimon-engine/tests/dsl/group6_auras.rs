use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause};
use digimon_engine::action::PLAY_HAND_START;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{GamePhase, ModifierType};
use digimon_engine::permanent::PermanentHandle;

#[test]
fn aura_other_predicate_excludes_source_permanent() {
    let yaml = r#"
card: TEST-AURA-OTHER
name: Aura Source
kind: digimon
color: [blue]
level: 3
cost: 3
dp: 1000
traits: [Gaossmon]
effects:
  - kind: aura
    target: { owner: you, trait: Gaossmon, other: true }
    dp_modifier: 3000
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile");
    assert_eq!(
        compiled
            .effects
            .iter()
            .filter(|effect| matches!(
                effect,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura { .. })
            ))
            .count(),
        1
    );

    let mut ally = make_test_card("TEST-GAOSSMON", "Gaossmon");
    ally.traits.push("Gaossmon".to_string());

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(ally)
        .build();
    runner.place_on_field(0, "TEST-AURA-OTHER", None);
    runner.place_on_field(0, "TEST-GAOSSMON", None);

    runner.game.tick_declarative_effects();

    let source = PermanentHandle {
        player: 0,
        index: 0,
    };
    let ally = PermanentHandle {
        player: 0,
        index: 1,
    };
    assert_eq!(runner.game.modifiers.sum(source, ModifierType::ChangeDp), 0);
    assert_eq!(
        runner.game.modifiers.sum(ally, ModifierType::ChangeDp),
        3000
    );
}

#[test]
fn decode_action_refreshes_static_declarative_aura_after_play() {
    let yaml = r#"
card: TEST-AURA-DECODE
name: Aura Decode Source
kind: digimon
color: [blue]
level: 3
cost: 0
dp: 1000
traits: []
effects:
  - kind: aura
    target: { owner: you, trait: Gaossmon }
    dp_modifier: 3000
"#;

    let mut ally = make_test_card("TEST-GAOSSMON-DECODE", "Gaossmon");
    ally.traits.push("Gaossmon".to_string());

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(ally)
        .hand(0, &["TEST-AURA-DECODE"])
        .memory(5)
        .build();
    runner.place_on_field(0, "TEST-GAOSSMON-DECODE", None);
    runner.game.current_phase = GamePhase::Main;

    runner.game.decode_action(PLAY_HAND_START, 0);

    let ally = PermanentHandle {
        player: 0,
        index: 0,
    };
    assert_eq!(
        runner.game.modifiers.sum(ally, ModifierType::ChangeDp),
        3000
    );
}

#[test]
fn aura_can_install_player_scoped_modifier_from_static_field_effect() {
    let yaml = r#"
card: TEST-PLAYER-AURA
name: Player Aura Source
kind: digimon
color: [black]
level: 3
cost: 3
dp: 1000
traits: []
effects:
  - kind: aura
    target_player: opponent
    modifier: CannotReduceDigivolveCost
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile");
    assert_eq!(
        compiled
            .effects
            .iter()
            .filter(|effect| matches!(
                effect,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura { .. })
            ))
            .count(),
        1
    );

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .build();
    runner.place_on_field(0, "TEST-PLAYER-AURA", None);

    runner.game.tick_declarative_effects();

    assert!(runner
        .game
        .modifiers
        .player_has(1, ModifierType::CannotReduceDigivolveCost));
    assert!(!runner
        .game
        .modifiers
        .player_has(0, ModifierType::CannotReduceDigivolveCost));
}

#[test]
fn aura_tick_refresh_does_not_stack_dp_modifier() {
    let yaml = r#"
card: TEST-AURA-REFRESH
name: Aura Refresh Source
kind: digimon
color: [blue]
level: 3
cost: 3
dp: 1000
traits: []
effects:
  - kind: aura
    target: { owner: you, trait: Gaossmon }
    dp_modifier: 3000
"#;

    let mut ally = make_test_card("TEST-GAOSSMON-REFRESH", "Gaossmon");
    ally.traits.push("Gaossmon".to_string());

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(ally)
        .build();
    runner.place_on_field(0, "TEST-AURA-REFRESH", None);
    runner.place_on_field(0, "TEST-GAOSSMON-REFRESH", None);

    runner.game.tick_declarative_effects();
    runner.game.tick_declarative_effects();

    let ally = PermanentHandle {
        player: 0,
        index: 1,
    };
    assert_eq!(
        runner.game.modifiers.sum(ally, ModifierType::ChangeDp),
        3000
    );
}

#[test]
fn aura_tick_refresh_removes_materialized_modifier_after_source_leaves() {
    let yaml = r#"
card: TEST-AURA-LEAVES
name: Aura Leaves Source
kind: digimon
color: [blue]
level: 3
cost: 3
dp: 1000
traits: []
effects:
  - kind: aura
    target: { owner: you, trait: Gaossmon }
    dp_modifier: 3000
"#;

    let mut ally = make_test_card("TEST-GAOSSMON-LEAVES", "Gaossmon");
    ally.traits.push("Gaossmon".to_string());

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(ally)
        .build();
    runner.place_on_field(0, "TEST-AURA-LEAVES", None);
    runner.place_on_field(0, "TEST-GAOSSMON-LEAVES", None);

    runner.game.tick_declarative_effects();
    let old_ally = PermanentHandle {
        player: 0,
        index: 1,
    };
    assert_eq!(
        runner.game.modifiers.sum(old_ally, ModifierType::ChangeDp),
        3000
    );

    runner.game.players[0].battle_area.remove(0);
    runner.game.tick_declarative_effects();

    let new_ally = PermanentHandle {
        player: 0,
        index: 0,
    };
    assert_eq!(
        runner.game.modifiers.sum(old_ally, ModifierType::ChangeDp),
        0
    );
    assert_eq!(
        runner.game.modifiers.sum(new_ally, ModifierType::ChangeDp),
        0
    );
}

#[test]
fn player_scoped_aura_tick_refresh_does_not_duplicate_modifier() {
    let yaml = r#"
card: TEST-PLAYER-AURA-REFRESH
name: Player Aura Refresh Source
kind: digimon
color: [black]
level: 3
cost: 3
dp: 1000
traits: []
effects:
  - kind: aura
    target_player: opponent
    modifier: CannotReduceDigivolveCost
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .build();
    runner.place_on_field(0, "TEST-PLAYER-AURA-REFRESH", None);

    runner.game.tick_declarative_effects();
    runner.game.tick_declarative_effects();

    let installed = runner
        .game
        .modifiers
        .player_modifiers_iter(1)
        .filter(|entry| entry.modifier == ModifierType::CannotReduceDigivolveCost)
        .count();
    assert_eq!(installed, 1);
}
