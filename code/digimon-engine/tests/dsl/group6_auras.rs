use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::ModifierType;
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
