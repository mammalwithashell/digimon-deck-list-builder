use digimon_dsl::compiled::{CompiledCardKind, CompiledPredicate};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use digimon_engine::effect_context::EffectReadContext;

fn fresh_runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(make_test_card("TEST-A", "Test"))
        .add_card(make_test_card("TEST-B", "Test"))
        .add_card(make_test_card("TEST-C", "Test"))
        // Put TEST-A in player 0's hand so we can grab a valid handle.
        .hand(0, &["TEST-A"])
        .build()
}

fn any_card_handle(runner: &DebugRunner) -> CardHandle {
    // Player 0's hand is populated in fresh_runner().
    runner.game.players[0].hand[0].handle()
}

#[test]
fn empty_predicate_matches_anything() {
    let runner = fresh_runner();
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    let pred = CompiledPredicate::default();
    assert!(eval_predicate(&pred, &rctx, PredicateSubject::None));
}

#[test]
fn kind_predicate_matches_kind_on_subject_card() {
    let runner = fresh_runner();
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);

    // make_test_card produces Digimon — should match.
    let pred_digimon = CompiledPredicate {
        kind: Some(CompiledCardKind::Digimon),
        ..Default::default()
    };
    assert!(eval_predicate(&pred_digimon, &rctx, PredicateSubject::Card(card)));

    // Tamer predicate should not match a Digimon card.
    let pred_tamer = CompiledPredicate {
        kind: Some(CompiledCardKind::Tamer),
        ..Default::default()
    };
    assert!(!eval_predicate(&pred_tamer, &rctx, PredicateSubject::Card(card)));
}

#[test]
fn your_turn_predicate_reads_game_state() {
    let runner = fresh_runner();
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let tp = game.turn_player();
    let rctx = EffectReadContext::new(game, card, None, tp);
    let pred = CompiledPredicate {
        your_turn: Some(true),
        ..Default::default()
    };
    assert!(eval_predicate(&pred, &rctx, PredicateSubject::None));
}
