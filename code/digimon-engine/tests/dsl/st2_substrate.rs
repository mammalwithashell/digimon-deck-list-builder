//! ST-2 Cocytus Blue substrate coverage.
//!
//! These tests pin the reusable engine/DSL primitives needed by early blue
//! cards: deterministic bottom-source trash and battle-context predicates.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledClause, CompiledPredicate, CompiledStep};
use digimon_engine::action::space::encode_source_select;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::selection::AttackTarget;
use digimon_engine::AttackResult;

fn source_ids(runner: &DebugRunner, player: u8, index: usize) -> Vec<String> {
    runner.game.player(player).battle_area[index]
        .card_sources
        .iter()
        .map(|source| source.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn run_trash_bottom_sources(
    runner: &mut DebugRunner,
    target: digimon_engine::PermanentHandle,
    count: u8,
) {
    let source_card = runner
        .game
        .player(target.player)
        .battle_area
        .get(target.index as usize)
        .map(|permanent| permanent.top_card().handle())
        .unwrap_or(digimon_engine::card_source::CardHandle(0));
    let mut bindings = Bindings::new();
    bindings.insert_permanent("target", target);
    let steps = vec![CompiledStep::TrashBottomSources {
        target: CompiledBindingRef::Named("target".to_string()),
        count,
    }];

    let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
    run_steps(&steps, &mut ctx, &mut bindings);
}

#[test]
fn trash_bottom_sources_count_one_trashes_bottom_source_without_prompt() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BOTTOM", "Bottom Source"))
        .add_card(make_test_card("MIDDLE", "Middle Source"))
        .add_card(make_test_card("TOP", "Top Card"))
        .start();
    let target = runner.place_stack(1, &["BOTTOM", "MIDDLE", "TOP"]);

    run_trash_bottom_sources(&mut runner, target, 1);

    assert!(
        runner.game.pending_selection.is_none(),
        "deterministic bottom-source trash must not expose a source-selection prompt"
    );
    assert_eq!(source_ids(&runner, 1, 0), vec!["MIDDLE", "TOP"]);
    assert_eq!(
        runner.game.player(1).trash[0].card_id(&runner.game.card_data),
        "BOTTOM"
    );
}

#[test]
fn trash_bottom_sources_count_two_trashes_in_bottom_to_top_order() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BOTTOM", "Bottom Source"))
        .add_card(make_test_card("MIDDLE", "Middle Source"))
        .add_card(make_test_card("TOP", "Top Card"))
        .start();
    let target = runner.place_stack(1, &["BOTTOM", "MIDDLE", "TOP"]);

    run_trash_bottom_sources(&mut runner, target, 2);

    assert_eq!(source_ids(&runner, 1, 0), vec!["TOP"]);
    let trash_ids: Vec<_> = runner
        .game
        .player(1)
        .trash
        .iter()
        .map(|source| source.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(trash_ids, vec!["BOTTOM", "MIDDLE"]);
}

#[test]
fn trash_bottom_sources_soft_fails_when_count_exceeds_available_sources() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("ONLY", "Only Source"))
        .add_card(make_test_card("TOP", "Top Card"))
        .start();
    let target = runner.place_stack(1, &["ONLY", "TOP"]);

    run_trash_bottom_sources(&mut runner, target, 2);

    assert_eq!(
        source_ids(&runner, 1, 0),
        vec!["TOP"],
        "top card must remain even when fewer sources exist than requested"
    );
    assert_eq!(runner.game.player(1).trash.len(), 1);
}

#[test]
fn trash_bottom_sources_soft_fails_for_no_sources_and_stale_target() {
    let mut no_sources = DebugRunner::builder()
        .add_card(make_test_card("TOP", "Top Card"))
        .start();
    let target = no_sources.place_on_field(1, "TOP", None);
    run_trash_bottom_sources(&mut no_sources, target, 1);
    assert_eq!(source_ids(&no_sources, 1, 0), vec!["TOP"]);
    assert!(no_sources.game.player(1).trash.is_empty());

    let mut stale = DebugRunner::builder()
        .add_card(make_test_card("BOTTOM", "Bottom Source"))
        .add_card(make_test_card("TOP", "Top Card"))
        .start();
    let stale_target = stale.place_stack(1, &["BOTTOM", "TOP"]);
    stale.game.player_mut(1).battle_area.clear();
    run_trash_bottom_sources(&mut stale, stale_target, 1);
    assert!(stale.game.player(1).trash.is_empty());
}

#[test]
fn trash_bottom_sources_yaml_lowers_to_compiled_step() {
    let yaml = r#"
card: DSL-ST2-BOTTOM-SOURCE
name: Bottom Source Trash
kind: digimon
level: 3
color: [blue]
cost: 3
dp: 2000
effects:
  - when: when_attacking
    process:
      - trash_bottom_sources: { target: source, count: 2 }
"#;

    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("YAML compiles");
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };

    assert_eq!(
        triggered.process[0],
        CompiledStep::TrashBottomSources {
            target: CompiledBindingRef::Source,
            count: 2,
        }
    );
}

#[test]
fn battle_opponent_no_sources_predicate_matches_only_opposing_combatant() {
    let yaml = r#"
card: DSL-ST2-BATTLE-AURA
name: Battle Aura Source
kind: digi_egg
level: 2
color: [blue]
cost: 0
traits: [Lesser]
effects:
  - scope: inherited
    kind: aura
    active_when: { battle_opponent_no_sources: true }
    target: {}
    dp_modifier: 1000
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("battle predicate YAML loads")
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("DEF-SRC", "Defender Source"))
        .add_card(make_test_card("DEFENDER", "Defender"))
        .start();
    runner.game.card_data.iter_mut().for_each(|card| {
        if card.card_id == "ATTACKER" {
            card.dp = Some(1000);
        } else if card.card_id == "DEFENDER" {
            card.dp = Some(2000);
        }
    });

    let attacker = runner.place_stack(0, &["DSL-ST2-BATTLE-AURA", "ATTACKER"]);
    let defender = runner.place_on_field(1, "DEFENDER", None);
    runner.game.tick_declarative_effects();

    let result = runner
        .game
        .begin_attack(attacker, AttackTarget::Digimon(defender), false);

    assert_eq!(result, AttackResult::MutualDestruction);
    assert!(
        runner.game.player(0).battle_area.is_empty(),
        "1000 attacker should tie 2000 defender only while the battle predicate grants +1000"
    );
    assert!(
        runner.game.player(1).battle_area.is_empty(),
        "defender with zero sources should be deleted in the tied battle"
    );

    let mut with_source = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("battle predicate YAML loads")
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("DEF-SRC", "Defender Source"))
        .add_card(make_test_card("DEFENDER", "Defender"))
        .start();
    with_source.game.card_data.iter_mut().for_each(|card| {
        if card.card_id == "ATTACKER" {
            card.dp = Some(1000);
        } else if card.card_id == "DEFENDER" {
            card.dp = Some(2000);
        }
    });
    let attacker = with_source.place_stack(0, &["DSL-ST2-BATTLE-AURA", "ATTACKER"]);
    let defender = with_source.place_stack(1, &["DEF-SRC", "DEFENDER"]);
    with_source.game.tick_declarative_effects();

    let result = with_source
        .game
        .begin_attack(attacker, AttackTarget::Digimon(defender), false);

    assert_eq!(result, AttackResult::DefenderWins);
    assert!(
        with_source.game.player(0).battle_area.is_empty(),
        "predicate must fail when the opposing battled Digimon has a source"
    );
    assert_eq!(with_source.game.player(1).battle_area.len(), 1);
}

#[test]
fn battle_opponent_no_sources_yaml_lowers_predicate_leaf() {
    let yaml = r#"
card: DSL-ST2-BATTLE-PRED-LOWER
name: Battle Predicate Lowering
kind: digi_egg
level: 2
color: [blue]
cost: 0
effects:
  - scope: inherited
    kind: aura
    active_when: { battle_opponent_no_sources: true }
    target: {}
    dp_modifier: 1000
"#;

    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("YAML compiles");
    let CompiledClause::Declarative(digimon_dsl::compiled::CompiledDeclarativeClause::Aura {
        active_when:
            Some(CompiledPredicate {
                battle_opponent_no_sources: Some(true),
                ..
            }),
        ..
    }) = &compiled.effects[0]
    else {
        panic!("expected inherited aura active_when battle_opponent_no_sources leaf");
    };
}

#[test]
fn kaiser_nail_shape_selects_one_source_then_plays_it_for_free() {
    let source_yaml = r#"
card: DSL-ST2-NAIL-SOURCE
name: Kaiser Nail Source
kind: digimon
level: 3
color: [blue]
cost: 3
dp: 1000
effects:
  - when: on_play
    process:
      - gain_memory: 1
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(source_yaml)
        .expect("source YAML loads")
        .add_card(make_test_card("OTHER-SOURCE", "Other Source"))
        .add_card(make_test_card("HOST", "Host"))
        .memory(0)
        .start();
    let host = runner.place_stack(0, &["DSL-ST2-NAIL-SOURCE", "OTHER-SOURCE", "HOST"]);
    let source_card = runner.game.player(0).battle_area[host.index as usize]
        .top_card()
        .handle();
    let battle_before = runner.game.player(0).battle_area.len();

    let mut bindings = Bindings::new();
    bindings.insert_permanent("host", host);
    let steps = vec![
        CompiledStep::SelectMaterial {
            of_permanent: CompiledBindingRef::Named("host".to_string()),
            filter: CompiledPredicate::default(),
            bind_as: Some("picked_source".to_string()),
            prompt: "Choose 1 digivolution card to play".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::PlayFromMaterials {
            target: CompiledBindingRef::Named("host".to_string()),
            source_index: CompiledBindingRef::Named("picked_source".to_string()),
            cost_delta: Some(digimon_dsl::compiled::CompiledCostDelta::Free),
            suppress_on_play: false,
            bind_as: Some("played".to_string()),
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(host), 0);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let first_source = encode_source_select(host.index as u16, 0).expect("source action");
    {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("Kaiser Nail must ask which source to play");
        assert_eq!(
            pending.valid_action_ids.len(),
            2,
            "both real sources are candidates; the host top card is not"
        );
        assert!(pending.valid_action_ids.contains(&first_source));
    }

    runner
        .game
        .resolve_selection(0, first_source)
        .expect("select source to play");
    runner.game.drain_effect_queue();

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        source_ids(&runner, 0, host.index as usize),
        vec!["OTHER-SOURCE", "HOST"],
        "selected source leaves the original stack"
    );
    assert_eq!(
        runner.game.player(0).battle_area.len(),
        battle_before + 1,
        "selected source enters battle area as a fresh permanent"
    );
    let played = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|permanent| {
            permanent.top_card().card_id(&runner.game.card_data) == "DSL-ST2-NAIL-SOURCE"
        })
        .expect("selected source should be played");
    assert_eq!(
        played.top_card().owner,
        0,
        "played source keeps its card ownership"
    );
    assert_eq!(
        runner.game.memory, 1,
        "free source play should still fire the selected Digimon's On Play"
    );
}
