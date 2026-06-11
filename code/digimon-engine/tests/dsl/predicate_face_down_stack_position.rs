//! Tasks A3.1–A3.4 — Phase A3 predicate leaves: `is_face_down` (A3.1),
//! `is_bottom_source` (A3.2), `host_kind_is` (A3.3), and
//! `has_face_down_source` (A3.4).
//!
//! The first three are source-subject leaves: they filter
//! `select_own_sources` candidates. A `select_own_sources` step with
//! `filter: { is_face_down: true }` must expose ONLY face-down
//! digivolution-stack sources as selection candidates and exclude face-up
//! ones. A `filter: { is_bottom_source: true }` must expose ONLY the
//! `card_sources` index-0 source (the bottom of the digivolution stack). A
//! `filter: { host_kind_is: ... }` must expose ONLY sources whose host
//! permanent matches the given card kind. These prove the
//! `PredicateSubject::Source` variant carries source-stack metadata (the
//! `face_down` flag, the stack index, and the host's kind) into the
//! predicate evaluator.
//!
//! The last is a permanent-subject leaf: it filters `select_own_permanent`
//! candidates. A `filter: { has_face_down_source: true }` must expose ONLY
//! permanents that carry at least one face-down source in their digivolution
//! stack.
//!
//! Pattern mirrors `phase2g_select_sources.rs`
//! (`select_own_sources_filters_cards_from_source_carrier_only`): build a
//! carrier stack, run a `SelectOwnSources` step, and inspect the parked
//! selection's `valid_action_ids` for the per-source action IDs.

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledCardKind, CompiledPredicate, CompiledStep,
};
use digimon_engine::action::space::{encode_attack, encode_source_select};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::{run_steps, RunOutcome};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;

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
        assert!(
            !perm.card_sources[1].face_down,
            "second source stays face-up"
        );
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
        assert!(
            !perm.card_sources[2].face_down,
            "upper source stays face-up"
        );
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

// --- Task A3.3 — `host_kind_is` source-subject predicate leaf -------------

/// Build a `CardData` like `make_test_card` but with the given `CardKind` so
/// a carrier's TOP card can be a Tamer rather than the default Digimon.
fn make_test_card_kind(
    card_id: &str,
    card_name: &str,
    kind: CardKind,
) -> digimon_engine::card_data::CardData {
    let mut d = make_test_card(card_id, card_name);
    d.card_kind = kind;
    d
}

#[test]
fn select_own_sources_host_kind_is_tamer_offers_only_tamer_hosted_source() {
    // Two own permanents:
    //   - a Tamer carrier with one digivolution source (TAMER-SRC)
    //   - a Digimon carrier with one digivolution source (DIGI-SRC)
    // `host_kind_is: tamer` must offer ONLY the Tamer-hosted source.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TAMER-SRC", "Tamer Stash Source"))
        .add_card(make_test_card_kind(
            "TAMER-TOP",
            "Hosting Tamer",
            CardKind::Tamer,
        ))
        .add_card(make_test_card("DIGI-SRC", "Digimon Stash Source"))
        .add_card(make_test_card("DIGI-TOP", "Hosting Digimon"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let tamer_carrier = runner.place_stack(0, &["TAMER-SRC", "TAMER-TOP"]);
    let digi_carrier = runner.place_stack(0, &["DIGI-SRC", "DIGI-TOP"]);
    let source_card = runner.game.players[0].hand[0].handle();

    let mut filter = CompiledPredicate::default();
    filter.host_kind_is = Some(CompiledCardKind::Tamer);

    let steps = vec![CompiledStep::SelectOwnSources {
        target: None,
        filter,
        min: 1,
        max: 1,
        bind_as: Some("chosen".to_string()),
        prompt: "Choose a Tamer-hosted source".to_string(),
        then: vec![CompiledStep::GainMemory(1)],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Parked);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("source selection should be pending");

    // `select_own_sources` only offers digivolution-stack sources (index 0 of
    // each two-card carrier); the top card is the permanent itself.
    let tamer_source_action =
        encode_source_select(tamer_carrier.index as u16, 0).expect("tamer-hosted source action");
    let digi_source_action =
        encode_source_select(digi_carrier.index as u16, 0).expect("digimon-hosted source action");

    assert!(
        pending.valid_action_ids.contains(&tamer_source_action),
        "the Tamer-hosted source must be a candidate under `host_kind_is: tamer`"
    );
    assert!(
        !pending.valid_action_ids.contains(&digi_source_action),
        "a Digimon-hosted source must NOT be a candidate under `host_kind_is: tamer`"
    );
}

#[test]
fn select_own_sources_host_kind_is_digimon_offers_only_digimon_hosted_source() {
    // Inverse: `host_kind_is: digimon` must offer ONLY the Digimon-hosted
    // source and exclude the one stashed under a Tamer.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TAMER-SRC", "Tamer Stash Source"))
        .add_card(make_test_card_kind(
            "TAMER-TOP",
            "Hosting Tamer",
            CardKind::Tamer,
        ))
        .add_card(make_test_card("DIGI-SRC", "Digimon Stash Source"))
        .add_card(make_test_card("DIGI-TOP", "Hosting Digimon"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let tamer_carrier = runner.place_stack(0, &["TAMER-SRC", "TAMER-TOP"]);
    let digi_carrier = runner.place_stack(0, &["DIGI-SRC", "DIGI-TOP"]);
    let source_card = runner.game.players[0].hand[0].handle();

    let mut filter = CompiledPredicate::default();
    filter.host_kind_is = Some(CompiledCardKind::Digimon);

    let steps = vec![CompiledStep::SelectOwnSources {
        target: None,
        filter,
        min: 1,
        max: 1,
        bind_as: Some("chosen".to_string()),
        prompt: "Choose a Digimon-hosted source".to_string(),
        then: vec![CompiledStep::GainMemory(1)],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Parked);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("source selection should be pending");

    let tamer_source_action =
        encode_source_select(tamer_carrier.index as u16, 0).expect("tamer-hosted source action");
    let digi_source_action =
        encode_source_select(digi_carrier.index as u16, 0).expect("digimon-hosted source action");

    assert!(
        pending.valid_action_ids.contains(&digi_source_action),
        "the Digimon-hosted source must be a candidate under `host_kind_is: digimon`"
    );
    assert!(
        !pending.valid_action_ids.contains(&tamer_source_action),
        "a Tamer-hosted source must NOT be a candidate under `host_kind_is: digimon`"
    );
}

#[test]
fn select_own_sources_host_kind_is_inside_all_of_combinator() {
    // Regression for the degrade-to-Card path: a `host_kind_is` leaf nested
    // inside an `all_of` combinator must still see the `Source` subject's
    // host-permanent metadata. With the degrade bug it would recurse with the
    // already-degraded `Card` subject and unconditionally fail.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TAMER-SRC", "Tamer Stash Source"))
        .add_card(make_test_card_kind(
            "TAMER-TOP",
            "Hosting Tamer",
            CardKind::Tamer,
        ))
        .add_card(make_test_card("DIGI-SRC", "Digimon Stash Source"))
        .add_card(make_test_card("DIGI-TOP", "Hosting Digimon"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let tamer_carrier = runner.place_stack(0, &["TAMER-SRC", "TAMER-TOP"]);
    let digi_carrier = runner.place_stack(0, &["DIGI-SRC", "DIGI-TOP"]);
    let source_card = runner.game.players[0].hand[0].handle();

    let mut nested = CompiledPredicate::default();
    nested.host_kind_is = Some(CompiledCardKind::Tamer);
    let mut filter = CompiledPredicate::default();
    filter.all_of = vec![nested];

    let steps = vec![CompiledStep::SelectOwnSources {
        target: None,
        filter,
        min: 1,
        max: 1,
        bind_as: Some("chosen".to_string()),
        prompt: "Choose a Tamer-hosted source".to_string(),
        then: vec![CompiledStep::GainMemory(1)],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Parked);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("source selection should be pending");

    let tamer_source_action =
        encode_source_select(tamer_carrier.index as u16, 0).expect("tamer-hosted source action");
    let digi_source_action =
        encode_source_select(digi_carrier.index as u16, 0).expect("digimon-hosted source action");

    assert!(
        pending.valid_action_ids.contains(&tamer_source_action),
        "the Tamer-hosted source must be a candidate under nested `all_of: [{{ host_kind_is: tamer }}]`"
    );
    assert!(
        !pending.valid_action_ids.contains(&digi_source_action),
        "a Digimon-hosted source must NOT be a candidate under nested `all_of: [{{ host_kind_is: tamer }}]`"
    );
}

// --- Task A3.4 — `has_face_down_source` permanent-subject predicate leaf ---

#[test]
fn select_own_permanent_has_face_down_source_offers_only_tamer_with_face_down_stash() {
    // Two own Tamers:
    //   - one with a digivolution source marked face-down (a stash)
    //   - one with a digivolution source that is purely face-up
    // `select_own_permanent { kind: tamer, has_face_down_source: true }`
    // must offer ONLY the Tamer carrying the face-down source.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("STASH-SRC", "Stash Source"))
        .add_card(make_test_card_kind(
            "TAMER-FD",
            "Tamer With Stash",
            CardKind::Tamer,
        ))
        .add_card(make_test_card("PLAIN-SRC", "Plain Source"))
        .add_card(make_test_card_kind(
            "TAMER-FU",
            "Tamer Without Stash",
            CardKind::Tamer,
        ))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let tamer_fd = runner.place_stack(0, &["STASH-SRC", "TAMER-FD"]);
    let tamer_fu = runner.place_stack(0, &["PLAIN-SRC", "TAMER-FU"]);
    let source_card = runner.game.players[0].hand[0].handle();

    // Mark the first Tamer's bottom source face-down; leave the second's up.
    {
        let perm = &mut runner.game.players[0].battle_area[tamer_fd.index as usize];
        perm.card_sources[0].face_down = true;
    }
    {
        let perm = &mut runner.game.players[0].battle_area[tamer_fu.index as usize];
        assert!(
            !perm.card_sources[0].face_down,
            "the other Tamer's source stays face-up"
        );
    }

    let mut filter = CompiledPredicate::default();
    filter.kind = Some(CompiledCardKind::Tamer);
    filter.has_face_down_source = Some(true);

    let steps = vec![CompiledStep::SelectOwnPermanent {
        filter,
        bind_as: Some("chosen".to_string()),
        selector: None,
        prompt: "Choose a Tamer with a face-down stash".to_string(),
        prompt_key: None,
        optional: false,
        continue_on_decline: false,
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Parked);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("permanent selection should be pending");

    // `select_own_permanent` action IDs are `encode_attack(0, field_index)`.
    let fd_action = encode_attack(0, tamer_fd.index as u16);
    let fu_action = encode_attack(0, tamer_fu.index as u16);

    assert!(
        pending.valid_action_ids.contains(&fd_action),
        "the Tamer with a face-down source must be a candidate under `has_face_down_source: true`"
    );
    assert!(
        !pending.valid_action_ids.contains(&fu_action),
        "a Tamer with only face-up sources must NOT be a candidate under `has_face_down_source: true`"
    );
}

#[test]
fn select_own_permanent_has_face_down_source_excludes_tamer_with_only_face_up_sources() {
    // A single own Tamer whose only digivolution source is face-up. A
    // `has_face_down_source: true` filter must offer ZERO permanents (the
    // selection short-circuits without parking).
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("PLAIN-SRC", "Plain Source"))
        .add_card(make_test_card_kind(
            "TAMER-FU",
            "Tamer Without Stash",
            CardKind::Tamer,
        ))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let tamer_fu = runner.place_stack(0, &["PLAIN-SRC", "TAMER-FU"]);
    let source_card = runner.game.players[0].hand[0].handle();

    {
        let perm = &mut runner.game.players[0].battle_area[tamer_fu.index as usize];
        assert!(
            !perm.card_sources[0].face_down,
            "the Tamer's source is face-up"
        );
    }

    let mut filter = CompiledPredicate::default();
    filter.kind = Some(CompiledCardKind::Tamer);
    filter.has_face_down_source = Some(true);

    let steps = vec![CompiledStep::SelectOwnPermanent {
        filter,
        bind_as: Some("chosen".to_string()),
        selector: None,
        prompt: "Choose a Tamer with a face-down stash".to_string(),
        prompt_key: None,
        optional: false,
        continue_on_decline: false,
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    // No candidate → `select_own_permanent` no-ops without parking.
    assert_eq!(
        outcome,
        RunOutcome::Synchronous,
        "a Tamer with only face-up sources is not a candidate, so the step does not park"
    );
    assert!(
        runner.game.pending_selection.is_none(),
        "no selection should be installed when `has_face_down_source: true` matches nothing"
    );
}

#[test]
fn select_own_permanent_has_face_down_source_false_offers_only_tamer_without_stash() {
    // Polarity check: `has_face_down_source: false` must offer ONLY the Tamer
    // whose stack carries no face-down source.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("STASH-SRC", "Stash Source"))
        .add_card(make_test_card_kind(
            "TAMER-FD",
            "Tamer With Stash",
            CardKind::Tamer,
        ))
        .add_card(make_test_card("PLAIN-SRC", "Plain Source"))
        .add_card(make_test_card_kind(
            "TAMER-FU",
            "Tamer Without Stash",
            CardKind::Tamer,
        ))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let tamer_fd = runner.place_stack(0, &["STASH-SRC", "TAMER-FD"]);
    let tamer_fu = runner.place_stack(0, &["PLAIN-SRC", "TAMER-FU"]);
    let source_card = runner.game.players[0].hand[0].handle();

    {
        let perm = &mut runner.game.players[0].battle_area[tamer_fd.index as usize];
        perm.card_sources[0].face_down = true;
    }

    let mut filter = CompiledPredicate::default();
    filter.kind = Some(CompiledCardKind::Tamer);
    filter.has_face_down_source = Some(false);

    let steps = vec![CompiledStep::SelectOwnPermanent {
        filter,
        bind_as: Some("chosen".to_string()),
        selector: None,
        prompt: "Choose a Tamer without a face-down stash".to_string(),
        prompt_key: None,
        optional: false,
        continue_on_decline: false,
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Parked);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("permanent selection should be pending");

    let fd_action = encode_attack(0, tamer_fd.index as u16);
    let fu_action = encode_attack(0, tamer_fu.index as u16);

    assert!(
        !pending.valid_action_ids.contains(&fd_action),
        "a Tamer with a face-down source must NOT be a candidate under `has_face_down_source: false`"
    );
    assert!(
        pending.valid_action_ids.contains(&fu_action),
        "the Tamer with only face-up sources must be a candidate under `has_face_down_source: false`"
    );
}
