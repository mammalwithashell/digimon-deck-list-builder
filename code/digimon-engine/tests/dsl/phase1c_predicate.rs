use digimon_dsl::compiled::{
    CompiledBindingCompare, CompiledBindingOwnerPredicate, CompiledCardKind, CompiledPlayerRef,
    CompiledPredicate,
};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, make_test_egg, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::predicate::{
    eval_predicate, eval_predicate_with_bindings, PredicateSubject,
};
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

#[test]
fn can_hatch_predicate_requires_empty_breeding_and_digitama() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TEST-A", "Test"))
        .add_card(make_test_egg("EGG-A", "Egg"))
        .hand(0, &["TEST-A"])
        .digitama(0, &["EGG-A"])
        .build();
    let source = any_card_handle(&runner);
    let pred = CompiledPredicate {
        can_hatch: Some(digimon_dsl::compiled::CompiledPlayerRef::You),
        ..Default::default()
    };

    let rctx = EffectReadContext::new(&runner.game, source, None, 0);
    assert!(
        eval_predicate(&pred, &rctx, PredicateSubject::None),
        "fresh DebugRunner should have an empty breeding area and egg deck"
    );

    assert!(runner.game.hatch(0), "precondition: hatch succeeds");
    let rctx = EffectReadContext::new(&runner.game, source, None, 0);
    assert!(
        !eval_predicate(&pred, &rctx, PredicateSubject::None),
        "occupied breeding area must make can_hatch false"
    );

    runner.game.players[0].breeding_area = None;
    runner.game.players[0].digitama_deck.clear();
    let rctx = EffectReadContext::new(&runner.game, source, None, 0);
    assert!(
        !eval_predicate(&pred, &rctx, PredicateSubject::None),
        "empty digitama deck must make can_hatch false"
    );
}

fn any_card_handle(runner: &DebugRunner) -> CardHandle {
    // Player 0's hand is populated in fresh_runner().
    runner.game.players[0].hand[0].handle()
}

fn compare_binding(name: &str) -> CompiledBindingCompare {
    CompiledBindingCompare::Binding(name.to_string())
}

fn compare_literal(value: i64) -> CompiledBindingCompare {
    CompiledBindingCompare::Literal(value)
}

fn equals(values: Vec<CompiledBindingCompare>) -> CompiledPredicate {
    CompiledPredicate {
        equals: Some(values),
        ..Default::default()
    }
}

fn not_equals(values: Vec<CompiledBindingCompare>) -> CompiledPredicate {
    CompiledPredicate {
        not_equals: Some(values),
        ..Default::default()
    }
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

#[test]
fn equals_passes_when_all_referenced_values_match_and_fails_when_any_differ() {
    let runner = fresh_runner();
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    let pred = equals(vec![
        compare_binding("branch"),
        compare_binding("expected"),
        compare_literal(1),
    ]);
    let mut bindings = Bindings::new();
    bindings.insert_literal("branch", 1);
    bindings.insert_literal("expected", 1);

    assert!(eval_predicate_with_bindings(
        &pred,
        &rctx,
        PredicateSubject::None,
        Some(&bindings),
    ));

    bindings.insert_literal("expected", 2);
    assert!(!eval_predicate_with_bindings(
        &pred,
        &rctx,
        PredicateSubject::None,
        Some(&bindings),
    ));
}

#[test]
fn not_equals_is_inverse_for_two_present_values() {
    let runner = fresh_runner();
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    let pred_equals = equals(vec![compare_binding("branch"), compare_literal(1)]);
    let pred_not_equals = not_equals(vec![compare_binding("branch"), compare_literal(1)]);
    let mut bindings = Bindings::new();

    bindings.insert_literal("branch", 1);
    assert!(eval_predicate_with_bindings(
        &pred_equals,
        &rctx,
        PredicateSubject::None,
        Some(&bindings),
    ));
    assert!(!eval_predicate_with_bindings(
        &pred_not_equals,
        &rctx,
        PredicateSubject::None,
        Some(&bindings),
    ));

    bindings.insert_literal("branch", 0);
    assert!(!eval_predicate_with_bindings(
        &pred_equals,
        &rctx,
        PredicateSubject::None,
        Some(&bindings),
    ));
    assert!(eval_predicate_with_bindings(
        &pred_not_equals,
        &rctx,
        PredicateSubject::None,
        Some(&bindings),
    ));
}

#[test]
fn missing_binding_makes_equals_and_not_equals_false() {
    let runner = fresh_runner();
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    let pred_equals = equals(vec![compare_binding("branch"), compare_binding("expected")]);
    let pred_not_equals = not_equals(vec![compare_binding("branch"), compare_binding("expected")]);
    let mut bindings = Bindings::new();
    bindings.insert_literal("branch", 1);

    assert!(!eval_predicate_with_bindings(
        &pred_equals,
        &rctx,
        PredicateSubject::None,
        Some(&bindings),
    ));
    assert!(!eval_predicate_with_bindings(
        &pred_not_equals,
        &rctx,
        PredicateSubject::None,
        Some(&bindings),
    ));
}

#[test]
fn binding_owner_predicate_matches_bound_permanent_controller() {
    let mut runner = fresh_runner();
    let mine = runner.place_on_field(0, "TEST-A", Some(0));
    let opponent = runner.place_on_field(1, "TEST-B", Some(0));
    let source = any_card_handle(&runner);
    let rctx = EffectReadContext::new(&runner.game, source, None, 0);
    let pred = CompiledPredicate {
        binding_owner: Some(CompiledBindingOwnerPredicate {
            binding: "picked".to_string(),
            of: CompiledPlayerRef::You,
        }),
        ..Default::default()
    };

    let mut bindings = Bindings::new();
    bindings.insert_permanent("picked", mine);
    assert!(
        eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::None, Some(&bindings)),
        "a permanent bound to the effect controller should satisfy binding_owner: you"
    );

    bindings.insert_permanent("picked", opponent);
    assert!(
        !eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::None, Some(&bindings)),
        "an opponent permanent should not satisfy binding_owner: you"
    );
}

#[test]
fn binding_comparisons_flow_through_compound_predicates() {
    let runner = fresh_runner();
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    let equal_branch = equals(vec![compare_binding("branch"), compare_literal(1)]);
    let different_mode = not_equals(vec![compare_binding("mode"), compare_literal(0)]);
    let all_pred = CompiledPredicate {
        all_of: vec![equal_branch.clone(), different_mode.clone()],
        ..Default::default()
    };
    let any_pred = CompiledPredicate {
        any_of: vec![equal_branch.clone(), different_mode.clone()],
        ..Default::default()
    };
    let none_pred = CompiledPredicate {
        none_of: vec![different_mode],
        ..Default::default()
    };
    let mut bindings = Bindings::new();
    bindings.insert_literal("branch", 1);
    bindings.insert_literal("mode", 2);

    assert!(eval_predicate_with_bindings(
        &all_pred,
        &rctx,
        PredicateSubject::None,
        Some(&bindings),
    ));
    assert!(eval_predicate_with_bindings(
        &any_pred,
        &rctx,
        PredicateSubject::None,
        Some(&bindings),
    ));

    bindings.insert_literal("mode", 0);
    assert!(!eval_predicate_with_bindings(
        &all_pred,
        &rctx,
        PredicateSubject::None,
        Some(&bindings),
    ));
    assert!(eval_predicate_with_bindings(
        &any_pred,
        &rctx,
        PredicateSubject::None,
        Some(&bindings),
    ));
    assert!(eval_predicate_with_bindings(
        &none_pred,
        &rctx,
        PredicateSubject::None,
        Some(&bindings),
    ));
}

// ── Task 3: combinators + existentials ────────────────────────────────

use digimon_dsl::compiled::CompiledExistential;

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
                level_gte: Some(digimon_dsl::compiled::CompiledDpConstraint::Literal(1)),
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
                level_gte: Some(digimon_dsl::compiled::CompiledDpConstraint::Literal(1)),
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
fn binding_comparisons_flow_through_permanent_existentials() {
    let mut runner = fresh_runner();
    runner.place_on_field(0, "TEST-A", Some(0));
    runner.place_on_field(0, "TEST-B", Some(0));
    let card = any_card_handle(&runner);
    let game = &runner.game;
    let rctx = EffectReadContext::new(game, card, None, 0);
    let inner = CompiledPredicate {
        all_of: vec![
            CompiledPredicate {
                kind: Some(CompiledCardKind::Digimon),
                ..Default::default()
            },
            equals(vec![compare_binding("branch"), compare_literal(1)]),
        ],
        ..Default::default()
    };
    let any_pred = CompiledPredicate {
        any_permanent: Some(Box::new(CompiledExistential {
            of: CompiledPlayerRef::You,
            predicate: inner.clone(),
        })),
        ..Default::default()
    };
    let all_pred = CompiledPredicate {
        all_permanents: Some(Box::new(CompiledExistential {
            of: CompiledPlayerRef::You,
            predicate: inner,
        })),
        ..Default::default()
    };
    let mut bindings = Bindings::new();
    bindings.insert_literal("branch", 1);

    assert!(eval_predicate_with_bindings(
        &any_pred,
        &rctx,
        PredicateSubject::None,
        Some(&bindings),
    ));
    assert!(eval_predicate_with_bindings(
        &all_pred,
        &rctx,
        PredicateSubject::None,
        Some(&bindings),
    ));

    bindings.insert_literal("branch", 0);
    assert!(!eval_predicate_with_bindings(
        &any_pred,
        &rctx,
        PredicateSubject::None,
        Some(&bindings),
    ));
    assert!(!eval_predicate_with_bindings(
        &all_pred,
        &rctx,
        PredicateSubject::None,
        Some(&bindings),
    ));
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
