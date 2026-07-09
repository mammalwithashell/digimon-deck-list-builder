//! G-DSL-PLAY-OR-USE-FROM-SOURCES — source-origin sibling of
//! `play_or_use_from_hand`.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledClause, CompiledStep};
use digimon_engine::action::space::HAND_EFFECT_START;
use digimon_engine::card_data::{CardData, DualCardData, DualDigimonFace, DualOptionFace};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const SELECTOR_YAML: &str = r#"
card: TEST-POU-SOURCES
name: Play Or Use From Sources
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_play
    process:
      - select_own_sources:
          min: 1
          max: 1
          bind_as: picked
          filter: { play_or_use_cost_lte: 10 }
          prompt: "Choose a source card to play or use"
          then:
            - play_or_use_from_sources:
                of: you
                card: picked
                cost: free
"#;

const REDUCED_SELECTOR_YAML: &str = r#"
card: TEST-POU-SOURCES
name: Play Or Use From Sources
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_play
    process:
      - select_own_sources:
          min: 1
          max: 1
          bind_as: picked
          filter: { play_or_use_cost_lte: 10 }
          prompt: "Choose a source card to play or use"
          then:
            - play_or_use_from_sources:
                of: you
                card: picked
                cost: { reduce: 2 }
"#;

const OPPONENT_OF_SELECTOR_YAML: &str = r#"
card: TEST-POU-SOURCES
name: Play Or Use From Sources
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_play
    process:
      - select_own_sources:
          min: 1
          max: 1
          bind_as: picked
          filter: { play_or_use_cost_lte: 10 }
          prompt: "Choose a source card to play or use"
          then:
            - play_or_use_from_sources:
                of: opponent
                card: picked
                cost: free
"#;

struct OptionMainDraw;

impl CardEffect for OptionMainDraw {
    fn effects(&self, card: digimon_engine::card_source::CardHandle) -> Vec<Effect> {
        vec![Effect::when_attacking(card)
            .option_main()
            .name("OptionMain draw 1")
            .process(|ctx: &mut EffectContext| {
                ctx.draw(ctx.player, 1);
            })
            .build()]
    }
}

fn digimon(id: &str, cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = cost;
    card
}

fn option(id: &str, cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.level = None;
    card.dp = None;
    card.play_cost = cost;
    card
}

fn dual(id: &str, play_cost: u16, use_cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Dual;
    card.play_cost = play_cost;
    card.dual = Some(DualCardData {
        digimon: DualDigimonFace {
            level: 4,
            dp: 4000,
            colors: vec![],
            traits: vec![],
            evo_costs: vec![],
            effect_text: String::new(),
            inherited_text: String::new(),
            keywords: vec![],
        },
        option: DualOptionFace {
            use_cost,
            colors: vec![],
            effect_text: String::new(),
            security_text: String::new(),
            keywords: vec![],
        },
    });
    card
}

fn base() -> DebugRunner {
    base_with_selector(SELECTOR_YAML, None)
}

fn base_with_selector(selector_yaml: &str, memory: Option<i16>) -> DebugRunner {
    let mut builder = DebugRunner::builder()
        .from_dsl_yaml(selector_yaml)
        .expect("selector YAML compiles")
        .add_card(make_test_card("DECK-PAD", "Deck Pad"))
        .add_card(digimon("HOST", 3))
        .add_card(digimon("SRC-DIGI", 5))
        .add_card(option("SRC-OPT", 5))
        .add_card(dual("SRC-DUAL", 5, 5))
        .deck(0, &["DECK-PAD"; 5]);
    if let Some(memory) = memory {
        builder = builder.memory(memory);
    }
    builder.start()
}

fn fire_selector(runner: &mut DebugRunner) {
    let selector = runner.place_on_field(0, "TEST-POU-SOURCES", Some(0));
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(selector));
    runner.game.drain_effect_queue();
}

fn choose_first_source(runner: &mut DebugRunner) {
    let selection = runner
        .game
        .pending_selection
        .as_ref()
        .expect("source selection is pending");
    assert!(
        matches!(selection.kind, SelectionKind::SourceMulti { .. }),
        "expected source-card selection, got {:?}",
        selection.kind
    );
    let selecting_player = selection.selecting_player;
    let action = selection.valid_action_ids[0];
    runner
        .game
        .resolve_selection(selecting_player, action)
        .expect("select source card");
}

#[test]
fn play_or_use_from_sources_compiles_from_yaml() {
    let runner = base();
    let card = runner
        .compiled_card("TEST-POU-SOURCES")
        .expect("compiled selector");
    let triggered = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("triggered clause exists");
    let CompiledStep::SelectOwnSources { then, .. } = &triggered.process[0] else {
        panic!("first step should be select_own_sources");
    };
    assert!(matches!(then[0], CompiledStep::PlayOrUseFromSources { .. }));
}

#[test]
fn play_or_use_from_sources_plays_selected_digimon_source_free() {
    let mut runner = base();
    let host = runner.place_stack(0, &["SRC-DIGI", "HOST"]);

    fire_selector(&mut runner);
    choose_first_source(&mut runner);

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "SRC-DIGI"),
        "selected Digimon source should be played as a new permanent"
    );
    assert_eq!(
        runner.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        1,
        "selected source should leave the original stack"
    );
}

#[test]
fn play_or_use_from_sources_plays_selected_digimon_source_with_reduced_cost() {
    let mut runner = base_with_selector(REDUCED_SELECTOR_YAML, Some(10));
    runner.place_stack(0, &["SRC-DIGI", "HOST"]);
    let memory_before = runner.memory();

    fire_selector(&mut runner);
    choose_first_source(&mut runner);

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "SRC-DIGI"),
        "selected Digimon source should be played as a new permanent"
    );
    assert_eq!(
        runner.memory(),
        memory_before - 3,
        "play cost 5 reduced by 2 should pay 3 memory"
    );
}

#[test]
fn play_or_use_from_sources_honors_of_player_for_source_owner() {
    let mut runner = base_with_selector(OPPONENT_OF_SELECTOR_YAML, Some(10));
    let host = runner.place_stack(0, &["SRC-DIGI", "HOST"]);
    let memory_before = runner.memory();

    fire_selector(&mut runner);
    choose_first_source(&mut runner);

    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "SRC-DIGI"),
        "`of: opponent` must not play a source selected from your own stack"
    );
    assert_eq!(
        runner.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        2,
        "rejected source play should leave the original stack intact"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "rejected source play should not pay memory"
    );
}

#[test]
fn play_or_use_from_sources_uses_selected_option_source_free() {
    let mut runner = base();
    runner.register_effect("SRC-OPT", Arc::new(OptionMainDraw));
    let host = runner.place_stack(0, &["SRC-OPT", "HOST"]);
    let hand_before = runner.game.players[0].hand.len();

    fire_selector(&mut runner);
    choose_first_source(&mut runner);

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "Option source [Main] body should resolve"
    );
    assert_eq!(
        runner.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        1,
        "used Option should leave the original source stack"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "SRC-OPT"),
        "used Option should dispose to trash"
    );
}

#[test]
fn play_or_use_from_sources_dual_source_exposes_face_choice() {
    let mut runner = base();
    runner.place_stack(0, &["SRC-DUAL", "HOST"]);

    fire_selector(&mut runner);
    choose_first_source(&mut runner);

    let choice = runner
        .game
        .pending_selection
        .as_ref()
        .expect("DUAL face choice is pending");
    assert_eq!(choice.kind, SelectionKind::EffectChoice);
    let labels: Vec<_> = choice
        .effect_choices
        .as_ref()
        .expect("effect-choice entries")
        .iter()
        .map(|entry| entry.label.as_str())
        .collect();
    assert_eq!(labels, ["Play as Digimon", "Use as Option"]);
    let selecting_player = choice.selecting_player;
    runner
        .game
        .resolve_selection(selecting_player, HAND_EFFECT_START)
        .expect("choose play face");
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "SRC-DUAL"),
        "choosing the play face should play the DUAL source as a permanent"
    );
}
