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
    assert!(eval_predicate(
        &pred_digimon,
        &rctx,
        PredicateSubject::Card(card)
    ));

    // Tamer predicate should not match a Digimon card.
    let pred_tamer = CompiledPredicate {
        kind: Some(CompiledCardKind::Tamer),
        ..Default::default()
    };
    assert!(!eval_predicate(
        &pred_tamer,
        &rctx,
        PredicateSubject::Card(card)
    ));
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

// ── Task 3: combinators + existentials ────────────────────────────────

use digimon_dsl::compiled::{CompiledExistential, CompiledPlayerRef};

#[test]
fn all_of_combinator_ands_children() {
    let runner = fresh_runner();
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    // make_test_card produces Digimon with level 3 — both sub-predicates
    // should hold, so the outer all_of should pass.
    let pred = CompiledPredicate {
        all_of: vec![
            CompiledPredicate {
                kind: Some(CompiledCardKind::Digimon),
                ..Default::default()
            },
            CompiledPredicate {
                level_gte: Some(1),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    assert!(eval_predicate(&pred, &rctx, PredicateSubject::Card(card)));
}

#[test]
fn all_of_combinator_short_circuits_on_false() {
    let runner = fresh_runner();
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    // Tamer predicate fails → all_of should fail even though second is fine.
    let pred = CompiledPredicate {
        all_of: vec![
            CompiledPredicate {
                kind: Some(CompiledCardKind::Tamer),
                ..Default::default()
            },
            CompiledPredicate {
                level_gte: Some(1),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    assert!(!eval_predicate(&pred, &rctx, PredicateSubject::Card(card)));
}

#[test]
fn any_of_combinator_ors_children() {
    let runner = fresh_runner();
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    // First child is wrong (Tamer), second is correct (Digimon) — should pass.
    let pred = CompiledPredicate {
        any_of: vec![
            CompiledPredicate {
                kind: Some(CompiledCardKind::Tamer),
                ..Default::default()
            },
            CompiledPredicate {
                kind: Some(CompiledCardKind::Digimon),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    assert!(eval_predicate(&pred, &rctx, PredicateSubject::Card(card)));
}

#[test]
fn none_of_combinator_inverts_any_of() {
    let runner = fresh_runner();
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    // Card is Digimon, not Tamer — none_of[Tamer] should pass.
    let pred = CompiledPredicate {
        none_of: vec![CompiledPredicate {
            kind: Some(CompiledCardKind::Tamer),
            ..Default::default()
        }],
        ..Default::default()
    };
    assert!(eval_predicate(&pred, &rctx, PredicateSubject::Card(card)));
}

#[test]
fn none_of_fails_when_child_matches() {
    let runner = fresh_runner();
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    // Card IS Digimon — none_of[Digimon] should fail.
    let pred = CompiledPredicate {
        none_of: vec![CompiledPredicate {
            kind: Some(CompiledCardKind::Digimon),
            ..Default::default()
        }],
        ..Default::default()
    };
    assert!(!eval_predicate(&pred, &rctx, PredicateSubject::Card(card)));
}

#[test]
fn not_inverts_single_child() {
    let runner = fresh_runner();
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    // Card is Digimon, not Tamer — not(Tamer) should pass.
    let pred = CompiledPredicate {
        not: Some(Box::new(CompiledPredicate {
            kind: Some(CompiledCardKind::Tamer),
            ..Default::default()
        })),
        ..Default::default()
    };
    assert!(eval_predicate(&pred, &rctx, PredicateSubject::Card(card)));
}

#[test]
fn not_fails_when_child_matches() {
    let runner = fresh_runner();
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    // Card IS Digimon — not(Digimon) should fail.
    let pred = CompiledPredicate {
        not: Some(Box::new(CompiledPredicate {
            kind: Some(CompiledCardKind::Digimon),
            ..Default::default()
        })),
        ..Default::default()
    };
    assert!(!eval_predicate(&pred, &rctx, PredicateSubject::Card(card)));
}

#[test]
fn any_permanent_matches_if_any_battle_area_perm_matches() {
    let mut runner = fresh_runner();
    // Place FIXT-DIGI (a Digimon) on player 0's field using the built-in helper.
    runner.place_on_field(0, "TEST-A", Some(0));

    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    let pred = CompiledPredicate {
        any_permanent: Some(Box::new(CompiledExistential {
            of: CompiledPlayerRef::You,
            predicate: CompiledPredicate {
                kind: Some(CompiledCardKind::Digimon),
                ..Default::default()
            },
        })),
        ..Default::default()
    };
    assert!(eval_predicate(&pred, &rctx, PredicateSubject::None));
}

#[test]
fn no_permanent_passes_when_no_match() {
    let runner = fresh_runner();
    // Player 0 has nothing on the field.
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    let pred = CompiledPredicate {
        no_permanent: Some(Box::new(CompiledExistential {
            of: CompiledPlayerRef::You,
            predicate: CompiledPredicate {
                kind: Some(CompiledCardKind::Tamer),
                ..Default::default()
            },
        })),
        ..Default::default()
    };
    assert!(eval_predicate(&pred, &rctx, PredicateSubject::None));
}

#[test]
fn no_permanent_fails_when_match_exists() {
    let mut runner = fresh_runner();
    runner.place_on_field(0, "TEST-A", Some(0));
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    let pred = CompiledPredicate {
        no_permanent: Some(Box::new(CompiledExistential {
            of: CompiledPlayerRef::You,
            predicate: CompiledPredicate {
                kind: Some(CompiledCardKind::Digimon),
                ..Default::default()
            },
        })),
        ..Default::default()
    };
    assert!(!eval_predicate(&pred, &rctx, PredicateSubject::None));
}

#[test]
fn all_permanents_passes_when_all_match() {
    let mut runner = fresh_runner();
    runner.place_on_field(0, "TEST-A", Some(0));
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    // All of player 0's field is Digimon.
    let pred = CompiledPredicate {
        all_permanents: Some(Box::new(CompiledExistential {
            of: CompiledPlayerRef::You,
            predicate: CompiledPredicate {
                kind: Some(CompiledCardKind::Digimon),
                ..Default::default()
            },
        })),
        ..Default::default()
    };
    assert!(eval_predicate(&pred, &rctx, PredicateSubject::None));
}

#[test]
fn all_permanents_fails_when_none_exist() {
    let runner = fresh_runner();
    // No permanents on field — all_permanents requires at least one.
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    let pred = CompiledPredicate {
        all_permanents: Some(Box::new(CompiledExistential {
            of: CompiledPlayerRef::You,
            predicate: CompiledPredicate {
                kind: Some(CompiledCardKind::Digimon),
                ..Default::default()
            },
        })),
        ..Default::default()
    };
    assert!(!eval_predicate(&pred, &rctx, PredicateSubject::None));
}
