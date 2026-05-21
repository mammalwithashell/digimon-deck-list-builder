//! Tasks A3.1 / A3.2 — source-subject predicate leaves (`is_face_down`,
//! `is_bottom_source`).
//!
//! A `select_own_sources` step with `filter: { is_face_down: true }` must
//! expose ONLY face-down digivolution-stack sources as selection candidates
//! and exclude face-up ones. A `filter: { is_bottom_source: true }` must
//! expose ONLY the `card_sources` index-0 source (the bottom of the
//! digivolution stack). These prove the `PredicateSubject::Source` variant
//! carries source-stack metadata (the `face_down` flag and the stack index)
//! into the predicate evaluator.
//!
//! Pattern mirrors `phase2g_select_sources.rs`
//! (`select_own_sources_filters_cards_from_source_carrier_only`): build a
//! carrier stack, run a `SelectOwnSources` step, and inspect the parked
//! selection's `valid_action_ids` for the per-source action IDs.
//!
//! This file will also host A3.3/A3.4 tests later.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledPredicate, CompiledStep};
use digimon_engine::action::space::encode_source_select;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::{run_steps, RunOutcome};
use digimon_engine::effect_context::EffectContext;

#[test]
fn select_own_sources_is_face_down_filter_offers_only_face_down_sources() {
    // Carrier stack: bottom source FACE-DOWN, second source FACE-UP, top card.
    // The `is_face_down: true` filter must offer only the bottom source.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("FD-SRC", "Face Down Source"))
        .add_card(make_test_card("FU-SRC", "Face Up Source"))
        .add_card(make_test_card("TOP-CARD", "Top Card"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let carrier = runner.place_stack(0, &["FD-SRC", "FU-SRC", "TOP-CARD"]);
    let source_card = runner.game.players[0].hand[0].handle();

    // Mark the bottom source (card_sources[0]) face-down; leave the others up.
    {
        let perm = &mut runner.game.players[0].battle_area[carrier.index as usize];
        perm.card_sources[0].face_down = true;
        assert!(!perm.card_sources[1].face_down, "second source stays face-up");
        assert!(!perm.card_sources[2].face_down, "top card stays face-up");
    }

    let mut filter = CompiledPredicate::default();
    filter.is_face_down = Some(true);

    let steps = vec![CompiledStep::SelectOwnSources {
        target: Some(CompiledBindingRef::Source),
        filter,
        min: 1,
        max: 1,
        bind_as: Some("chosen".to_string()),
        prompt: "Choose a face-down source".to_string(),
        then: vec![CompiledStep::GainMemory(1)],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(carrier), 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Parked);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("source selection should be pending");

    // `select_own_sources` only offers digivolution-stack sources (indices
    // `0..len-1`); the top card (index 2) is the permanent itself and is
    // never a candidate.
    let fd_action = encode_source_select(carrier.index as u16, 0).expect("face-down source action");
    let fu_action = encode_source_select(carrier.index as u16, 1).expect("face-up source action");

    assert!(
        pending.valid_action_ids.contains(&fd_action),
        "the face-down source must be a candidate under `is_face_down: true`"
    );
    assert!(
        !pending.valid_action_ids.contains(&fu_action),
        "a face-up source must NOT be a candidate under `is_face_down: true`"
    );
}

#[test]
fn select_own_sources_is_face_down_false_filter_offers_only_face_up_sources() {
    // Inverse: `is_face_down: false` must offer only face-up sources.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("FD-SRC", "Face Down Source"))
        .add_card(make_test_card("FU-SRC", "Face Up Source"))
        .add_card(make_test_card("TOP-CARD", "Top Card"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let carrier = runner.place_stack(0, &["FD-SRC", "FU-SRC", "TOP-CARD"]);
    let source_card = runner.game.players[0].hand[0].handle();

    {
        let perm = &mut runner.game.players[0].battle_area[carrier.index as usize];
        perm.card_sources[0].face_down = true;
    }

    let mut filter = CompiledPredicate::default();
    filter.is_face_down = Some(false);

    let steps = vec![CompiledStep::SelectOwnSources {
        target: Some(CompiledBindingRef::Source),
        filter,
        min: 1,
        max: 1,
        bind_as: Some("chosen".to_string()),
        prompt: "Choose a face-up source".to_string(),
        then: vec![CompiledStep::GainMemory(1)],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(carrier), 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Parked);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("source selection should be pending");

    // `select_own_sources` only offers digivolution-stack sources (indices
    // `0..len-1`); the top card (index 2) is the permanent itself and is
    // never a candidate regardless of filter.
    let fd_action = encode_source_select(carrier.index as u16, 0).expect("face-down source action");
    let fu_action = encode_source_select(carrier.index as u16, 1).expect("face-up source action");

    assert!(
        !pending.valid_action_ids.contains(&fd_action),
        "a face-down source must NOT be a candidate under `is_face_down: false`"
    );
    assert!(
        pending.valid_action_ids.contains(&fu_action),
        "the face-up source must be a candidate under `is_face_down: false`"
    );
}

#[test]
fn select_own_sources_is_face_down_inside_all_of_combinator() {
    // Regression for the degrade-to-Card bug: an `is_face_down` leaf nested
    // inside an `all_of` combinator must still see the `Source` subject's
    // source-stack metadata. With the bug, the combinator recursed with the
    // already-degraded `Card` subject, so the nested `is_face_down` leaf
    // unconditionally returned false and the filter offered ZERO sources.
    // After the fix, `all_of: [{ is_face_down: true }]` offers exactly the
    // face-down source.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("FD-SRC", "Face Down Source"))
        .add_card(make_test_card("FU-SRC", "Face Up Source"))
        .add_card(make_test_card("TOP-CARD", "Top Card"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let carrier = runner.place_stack(0, &["FD-SRC", "FU-SRC", "TOP-CARD"]);
    let source_card = runner.game.players[0].hand[0].handle();

    {
        let perm = &mut runner.game.players[0].battle_area[carrier.index as usize];
        perm.card_sources[0].face_down = true;
    }

    // `all_of: [{ is_face_down: true }]` — the source leaf is nested one
    // combinator level deep, so it only evaluates correctly if the
    // recursion preserves the `Source` subject.
    let mut nested = CompiledPredicate::default();
    nested.is_face_down = Some(true);
    let mut filter = CompiledPredicate::default();
    filter.all_of = vec![nested];

    let steps = vec![CompiledStep::SelectOwnSources {
        target: Some(CompiledBindingRef::Source),
        filter,
        min: 1,
        max: 1,
        bind_as: Some("chosen".to_string()),
        prompt: "Choose a face-down source".to_string(),
        then: vec![CompiledStep::GainMemory(1)],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(carrier), 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Parked);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("source selection should be pending");

    let fd_action = encode_source_select(carrier.index as u16, 0).expect("face-down source action");
    let fu_action = encode_source_select(carrier.index as u16, 1).expect("face-up source action");

    assert!(
        pending.valid_action_ids.contains(&fd_action),
        "the face-down source must be a candidate under nested `all_of: [{{ is_face_down: true }}]`"
    );
    assert!(
        !pending.valid_action_ids.contains(&fu_action),
        "a face-up source must NOT be a candidate under nested `all_of: [{{ is_face_down: true }}]`"
    );
}

#[test]
fn select_own_sources_is_face_down_inside_any_of_combinator() {
    // Companion regression: an `any_of` of two `is_face_down` arms covering
    // both polarities must offer BOTH sources. With the degrade-to-Card bug
    // each nested `is_face_down` leaf saw a `Card` subject and returned
    // false, so the OR never matched and the filter offered ZERO sources.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("FD-SRC", "Face Down Source"))
        .add_card(make_test_card("FU-SRC", "Face Up Source"))
        .add_card(make_test_card("TOP-CARD", "Top Card"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let carrier = runner.place_stack(0, &["FD-SRC", "FU-SRC", "TOP-CARD"]);
    let source_card = runner.game.players[0].hand[0].handle();

    {
        let perm = &mut runner.game.players[0].battle_area[carrier.index as usize];
        perm.card_sources[0].face_down = true;
    }

    // `any_of: [{ is_face_down: true }, { is_face_down: false }]` — both
    // arms are source leaves nested one combinator level deep; the OR
    // matches every source regardless of polarity.
    let mut down_arm = CompiledPredicate::default();
    down_arm.is_face_down = Some(true);
    let mut up_arm = CompiledPredicate::default();
    up_arm.is_face_down = Some(false);
    let mut filter = CompiledPredicate::default();
    filter.any_of = vec![down_arm, up_arm];

    let steps = vec![CompiledStep::SelectOwnSources {
        target: Some(CompiledBindingRef::Source),
        filter,
        min: 1,
        max: 1,
        bind_as: Some("chosen".to_string()),
        prompt: "Choose a source".to_string(),
        then: vec![CompiledStep::GainMemory(1)],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(carrier), 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Parked);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("source selection should be pending");

    let fd_action = encode_source_select(carrier.index as u16, 0).expect("face-down source action");
    let fu_action = encode_source_select(carrier.index as u16, 1).expect("face-up source action");

    assert!(
        pending.valid_action_ids.contains(&fd_action),
        "the face-down source must be a candidate under nested `any_of` of both polarities"
    );
    assert!(
        pending.valid_action_ids.contains(&fu_action),
        "the face-up source must be a candidate under nested `any_of` of both polarities"
    );
}

// --- Task A3.2 — `is_bottom_source` source-subject predicate leaf ---------

#[test]
fn select_own_sources_is_bottom_source_filter_offers_only_index_zero() {
    // Carrier stack with three digivolution-stack sources plus the top card.
    // `card_sources` index 0 is the bottom; the `is_bottom_source: true`
    // filter must offer ONLY that source.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BOTTOM", "Bottom Source"))
        .add_card(make_test_card("MID", "Mid Source"))
        .add_card(make_test_card("UPPER", "Upper Source"))
        .add_card(make_test_card("TOP-CARD", "Top Card"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let carrier = runner.place_stack(0, &["BOTTOM", "MID", "UPPER", "TOP-CARD"]);
    let source_card = runner.game.players[0].hand[0].handle();

    let mut filter = CompiledPredicate::default();
    filter.is_bottom_source = Some(true);

    let steps = vec![CompiledStep::SelectOwnSources {
        target: Some(CompiledBindingRef::Source),
        filter,
        min: 1,
        max: 1,
        bind_as: Some("chosen".to_string()),
        prompt: "Choose the bottom source".to_string(),
        then: vec![CompiledStep::GainMemory(1)],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(carrier), 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Parked);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("source selection should be pending");

    // `select_own_sources` only offers digivolution-stack sources (indices
    // `0..len-1`); the top card (index 3) is the permanent itself and is
    // never a candidate.
    let bottom_action =
        encode_source_select(carrier.index as u16, 0).expect("bottom source action");
    let mid_action = encode_source_select(carrier.index as u16, 1).expect("mid source action");
    let upper_action = encode_source_select(carrier.index as u16, 2).expect("upper source action");

    assert!(
        pending.valid_action_ids.contains(&bottom_action),
        "the index-0 source must be a candidate under `is_bottom_source: true`"
    );
    assert!(
        !pending.valid_action_ids.contains(&mid_action),
        "a non-bottom source must NOT be a candidate under `is_bottom_source: true`"
    );
    assert!(
        !pending.valid_action_ids.contains(&upper_action),
        "a non-bottom source must NOT be a candidate under `is_bottom_source: true`"
    );
}

#[test]
fn select_own_sources_is_bottom_source_false_filter_offers_only_non_bottom() {
    // Inverse: `is_bottom_source: false` must offer the non-bottom
    // digivolution-stack sources (indices 1, 2) but NOT the index-0 source.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BOTTOM", "Bottom Source"))
        .add_card(make_test_card("MID", "Mid Source"))
        .add_card(make_test_card("UPPER", "Upper Source"))
        .add_card(make_test_card("TOP-CARD", "Top Card"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let carrier = runner.place_stack(0, &["BOTTOM", "MID", "UPPER", "TOP-CARD"]);
    let source_card = runner.game.players[0].hand[0].handle();

    let mut filter = CompiledPredicate::default();
    filter.is_bottom_source = Some(false);

    let steps = vec![CompiledStep::SelectOwnSources {
        target: Some(CompiledBindingRef::Source),
        filter,
        min: 1,
        max: 1,
        bind_as: Some("chosen".to_string()),
        prompt: "Choose a non-bottom source".to_string(),
        then: vec![CompiledStep::GainMemory(1)],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(carrier), 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Parked);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("source selection should be pending");

    // `select_own_sources` only offers digivolution-stack sources (indices
    // `0..len-1`); the top card (index 3) is never a candidate regardless of
    // filter.
    let bottom_action =
        encode_source_select(carrier.index as u16, 0).expect("bottom source action");
    let mid_action = encode_source_select(carrier.index as u16, 1).expect("mid source action");
    let upper_action = encode_source_select(carrier.index as u16, 2).expect("upper source action");

    assert!(
        !pending.valid_action_ids.contains(&bottom_action),
        "the index-0 source must NOT be a candidate under `is_bottom_source: false`"
    );
    assert!(
        pending.valid_action_ids.contains(&mid_action),
        "a non-bottom source must be a candidate under `is_bottom_source: false`"
    );
    assert!(
        pending.valid_action_ids.contains(&upper_action),
        "a non-bottom source must be a candidate under `is_bottom_source: false`"
    );
}

#[test]
fn select_own_sources_face_down_and_bottom_all_of_offers_only_that_source() {
    // The exact predicate shape the BEATBREAK cost uses: pay by trashing
    // "the bottom face-down card". `all_of: [{ is_face_down: true },
    // { is_bottom_source: true }]` must offer ONLY the source that is BOTH
    // face-down AND at `card_sources` index 0.
    //
    // Stack layout (bottom → top):
    //   index 0 (BOTTOM)  — face-down  + bottom   → MATCH
    //   index 1 (MID)     — face-down  + NOT bottom
    //   index 2 (UPPER)   — face-up    + NOT bottom
    //   index 3 (TOP-CARD)— the permanent itself, never a candidate
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BOTTOM", "Bottom Source"))
        .add_card(make_test_card("MID", "Mid Source"))
        .add_card(make_test_card("UPPER", "Upper Source"))
        .add_card(make_test_card("TOP-CARD", "Top Card"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let carrier = runner.place_stack(0, &["BOTTOM", "MID", "UPPER", "TOP-CARD"]);
    let source_card = runner.game.players[0].hand[0].handle();

    // Make index 0 and index 1 face-down; leave index 2 face-up. Only index
    // 0 is BOTH face-down and bottom.
    {
        let perm = &mut runner.game.players[0].battle_area[carrier.index as usize];
        perm.card_sources[0].face_down = true;
        perm.card_sources[1].face_down = true;
        assert!(!perm.card_sources[2].face_down, "upper source stays face-up");
    }

    let mut face_down_arm = CompiledPredicate::default();
    face_down_arm.is_face_down = Some(true);
    let mut bottom_arm = CompiledPredicate::default();
    bottom_arm.is_bottom_source = Some(true);
    let mut filter = CompiledPredicate::default();
    filter.all_of = vec![face_down_arm, bottom_arm];

    let steps = vec![CompiledStep::SelectOwnSources {
        target: Some(CompiledBindingRef::Source),
        filter,
        min: 1,
        max: 1,
        bind_as: Some("chosen".to_string()),
        prompt: "Choose the bottom face-down source".to_string(),
        then: vec![CompiledStep::GainMemory(1)],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(carrier), 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Parked);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("source selection should be pending");

    let bottom_action =
        encode_source_select(carrier.index as u16, 0).expect("bottom source action");
    let mid_action = encode_source_select(carrier.index as u16, 1).expect("mid source action");
    let upper_action = encode_source_select(carrier.index as u16, 2).expect("upper source action");

    assert!(
        pending.valid_action_ids.contains(&bottom_action),
        "the face-down bottom source must be the only candidate under `all_of`"
    );
    assert!(
        !pending.valid_action_ids.contains(&mid_action),
        "a face-down NON-bottom source must NOT be a candidate under `all_of`"
    );
    assert!(
        !pending.valid_action_ids.contains(&upper_action),
        "a face-up non-bottom source must NOT be a candidate under `all_of`"
    );
}
