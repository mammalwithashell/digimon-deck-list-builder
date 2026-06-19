//! Phase 2e Task 3: SelectReveal installs a parking selection over
//! `Game::revealed_cards`; its callback resolves the picked index into a
//! `CardHandle` and writes it into Bindings.

use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate, CompiledStep};
use digimon_engine::card_source::{CardSource, RevealOverlay};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::{EffectContext, EffectReadContext};

fn push_to_revealed(runner: &mut DebugRunner, owner: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, owner, card_index);
    runner.game.revealed_cards.push(card);
}

fn push_to_revealed_with_overlay(
    runner: &mut DebugRunner,
    owner: u8,
    card_id: &str,
    overlay_name: &str,
) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    let mut card = CardSource::new(data_idx, owner, card_index);
    card.reveal_overlay = Some(RevealOverlay {
        name: Some(overlay_name.to_string()),
        kind: None,
    });
    runner.game.revealed_cards.push(card);
}

#[test]
fn select_reveal_binds_picked_card_handle() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("R0", "R0"))
        .add_card(make_test_card("R1", "R1"))
        .hand(0, &["SRC"])
        .build();

    push_to_revealed(&mut runner, 0, "R0");
    push_to_revealed(&mut runner, 0, "R1");
    let target_handle = runner.game.revealed_cards[1].handle();
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![
        CompiledStep::SelectReveal {
            then: vec![],
            of: CompiledPlayerRef::You,
            filter: CompiledPredicate::default(),
            bind_as: Some("picked".to_string()),
            prompt: "Pick a revealed card".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert!(runner.game.pending_selection.is_some());

    // Pick the second revealed card (index 1).
    let (action_id, selecting_player) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        (pending.valid_action_ids[1], pending.selecting_player)
    };
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "tail must run after resolution"
    );
    // Binding visibility across the callback is exercised in the end-to-end
    // test in Task 9; here we just assert the install + resolve plumbing.
    let _ = target_handle; // tracked for the e2e test
}

#[test]
fn revealed_card_overlay_affects_reveal_predicates_only() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("R0", "Plainmon"))
        .add_card(make_test_card("R1", "AlsoPlainmon"))
        .hand(0, &["SRC"])
        .build();

    push_to_revealed(&mut runner, 0, "R0");
    push_to_revealed_with_overlay(&mut runner, 0, "R1", "Overlaymon");
    let overlay_handle = runner.game.revealed_cards[1].handle();
    let src_card = runner.game.players[0].hand[0].handle();

    let steps = vec![CompiledStep::SelectReveal {
        then: vec![],
        of: CompiledPlayerRef::You,
        filter: CompiledPredicate {
            name_is: Some("Overlaymon".to_string()),
            zone: vec![digimon_dsl::compiled::CompiledZone::Reveal],
            ..Default::default()
        },
        bind_as: Some("picked".to_string()),
        prompt: "Pick an overlay card".to_string(),
        prompt_key: None,
        optional: false,
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut Bindings::new());
    }

    let pending = runner.game.pending_selection.as_ref().unwrap();
    assert_eq!(
        pending.valid_action_ids,
        vec![digimon_engine::action::space::SEL_REVEAL_START + 1],
        "only the revealed source with overlay identity should be selectable"
    );

    assert!(runner.game.add_to_hand_from_reveal(0, overlay_handle));
    let moved = runner.game.players[0].hand.last().unwrap();
    assert!(
        moved.reveal_overlay.is_none(),
        "moving out of reveal must clear transient overlay metadata"
    );

    let read = EffectReadContext::new(&runner.game, src_card, None, 0);
    assert!(
        !eval_predicate(
            &CompiledPredicate {
                name_is: Some("Overlaymon".to_string()),
                ..Default::default()
            },
            &read,
            PredicateSubject::Card(moved.handle())
        ),
        "ordinary card predicates after movement must not see reveal overlays"
    );
}

#[test]
fn reveal_overlay_survives_select_reveal_until_destination_move() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("R0", "Plainmon"))
        .add_card(make_test_card("R1", "AlsoPlainmon"))
        .hand(0, &["SRC"])
        .build();

    push_to_revealed(&mut runner, 0, "R0");
    push_to_revealed_with_overlay(&mut runner, 0, "R1", "Overlaymon");
    let src_card = runner.game.players[0].hand[0].handle();

    let steps = vec![
        CompiledStep::SelectReveal {
            then: vec![],
            of: CompiledPlayerRef::You,
            filter: CompiledPredicate {
                name_is: Some("Overlaymon".to_string()),
                zone: vec![digimon_dsl::compiled::CompiledZone::Reveal],
                ..Default::default()
            },
            bind_as: Some("picked".to_string()),
            prompt: "Pick an overlay card".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::AddToHandFromReveal {
            of: CompiledPlayerRef::You,
            card: digimon_dsl::compiled::CompiledBindingRef::Named("picked".to_string()),
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut Bindings::new());
    }

    let (action_id, selecting_player) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        assert_eq!(
            pending.valid_action_ids,
            vec![digimon_engine::action::space::SEL_REVEAL_START + 1]
        );
        (pending.valid_action_ids[0], pending.selecting_player)
    };

    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.revealed_cards.len(),
        1,
        "selected card should have moved out of reveal"
    );
    assert_eq!(
        runner.game.players[0]
            .hand
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "R1"
    );
    assert!(
        runner.game.players[0]
            .hand
            .last()
            .unwrap()
            .reveal_overlay
            .is_none(),
        "destination card should not retain reveal-only overlay"
    );
}
