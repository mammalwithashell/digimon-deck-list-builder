//! Task 4 integration tests: SelectTrash → AddToHandFromTrash and
//! SelectHand → no-op (selection parking) round-trips.

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledPlayerRef, CompiledPredicate, CompiledStep,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

/// Helper: build a runner with player 0 having `src_id` in hand and
/// `tgt_id` in trash. The trash is populated via direct push since
/// `DebugRunnerBuilder` has no `.trash()` method.
fn make_runner_with_trash(src_id: &str, tgt_id: &str) -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card(src_id, src_id))
        .add_card(make_test_card(tgt_id, tgt_id))
        .hand(0, &[src_id])
        .build();

    // Populate trash manually: find the data index for tgt_id and push a
    // CardSource directly onto player 0's trash. Use next_card_index to
    // keep card indices disjoint from those already assigned to hand.
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == tgt_id)
        .expect("tgt_id must be in card_data");
    let card_index = runner.game.next_card_index();
    let card = digimon_engine::card_source::CardSource::new(data_idx, 0, card_index);
    runner.game.players[0].trash.push(card);

    runner
}

#[test]
fn select_trash_then_add_to_hand_from_trash_round_trip() {
    let mut runner = make_runner_with_trash("SRC", "TGT");

    let card = runner.game.players[0].hand[0].handle();
    let tgt_handle = runner.game.players[0].trash[0].handle();

    // Drive run_steps — installs a SelectTrash with tail = [AddToHandFromTrash].
    {
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        let mut bindings = Bindings::new();
        let steps = vec![
            CompiledStep::SelectTrash {
                of: CompiledPlayerRef::You,
                filter: CompiledPredicate::default(),
                bind_as: Some("pick".to_string()),
                prompt: "Pick a card from trash".to_string(),
                prompt_key: None,
                optional: false,
            },
            CompiledStep::AddToHandFromTrash {
                of: CompiledPlayerRef::You,
                card: CompiledBindingRef::Named("pick".to_string()),
            },
        ];
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // A pending selection should be installed now, with the target trash card
    // in its valid action set. Resolve it by picking the only valid action.
    let (action_id, selecting_player) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("SelectTrash installed a PendingSelection");
        assert_eq!(
            pending.valid_action_ids.len(),
            1,
            "exactly one trash card should be selectable"
        );
        (pending.valid_action_ids[0], pending.selecting_player)
    };

    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve SelectTrash");

    // Callback fired: the trashed card is now in hand, trash is empty.
    assert!(
        runner.game.pending_selection.is_none(),
        "pending_selection should be cleared after resolution"
    );
    assert!(
        runner.game.players[0].trash.is_empty(),
        "TGT should have left the trash"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|cs| cs.handle() == tgt_handle),
        "TGT should have landed in hand"
    );
}

#[test]
fn select_hand_parks_selection_and_fires_callback() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("TGT", "TGT"))
        .hand(0, &["SRC", "TGT"])
        .build();

    let src_handle = runner.game.players[0].hand[0].handle();

    // A SelectHand with no tail — just installs a selection; the callback
    // fires with the chosen index (no further steps to run).
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        let mut bindings = Bindings::new();
        let steps = vec![CompiledStep::SelectHand {
            of: CompiledPlayerRef::You,
            filter: CompiledPredicate::default(),
            bind_as: Some("chosen".to_string()),
            prompt: "Pick a card from hand".to_string(),
            prompt_key: None,
            optional: false,
        }];
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Selection parked — game is not yet resolved.
    assert!(
        runner.game.pending_selection.is_some(),
        "SelectHand should park a PendingSelection"
    );
    let pending = runner.game.pending_selection.as_ref().unwrap();
    // Both hand cards (indices 0 and 1) are selectable.
    assert_eq!(pending.valid_action_ids.len(), 2);

    // Pick the second card (index 1 = TGT). PLAY_HAND_START == 0 so
    // action_id for hand index 1 is 0 + 1 = 1.
    let action_id = pending.valid_action_ids[1];
    let selecting_player = pending.selecting_player;
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve SelectHand");

    assert!(
        runner.game.pending_selection.is_none(),
        "pending_selection should be cleared after resolution"
    );
    // Both hand cards should still be present (we only selected, didn't move).
    assert_eq!(
        runner.game.players[0].hand.len(),
        2,
        "hand should be unmodified (SelectHand alone doesn't move cards)"
    );
}

#[test]
fn select_own_permanent_parks_selection() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("DIG", "DIG"))
        .hand(0, &["DIG"])
        .build();

    let _h = runner.place_on_field(0, "DIG", None);
    let card = runner.game.players[0].battle_area[0].top_card().handle();

    {
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        let mut bindings = Bindings::new();
        let steps = vec![CompiledStep::SelectOwnPermanent {
            filter: CompiledPredicate::default(),
            bind_as: Some("tgt".to_string()),
            prompt: "Choose your Digimon".to_string(),
            prompt_key: None,
            optional: false,
        }];
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert!(
        runner.game.pending_selection.is_some(),
        "SelectOwnPermanent should install a PendingSelection"
    );
    assert_eq!(
        runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .len(),
        1,
        "one permanent on field → one valid action"
    );
}

#[test]
fn select_opponent_permanent_parks_selection() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("DIG", "DIG"))
        .hand(0, &["DIG"])
        .hand(1, &["DIG"])
        .build();

    // Place a Digimon on player 1's field.
    let _h = runner.place_on_field(1, "DIG", None);
    // Source card is player 0's hand card.
    let card = runner.game.players[0].hand[0].handle();

    {
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        let mut bindings = Bindings::new();
        let steps = vec![CompiledStep::SelectOpponentPermanent {
            filter: CompiledPredicate::default(),
            bind_as: Some("tgt".to_string()),
            prompt: "Choose opponent Digimon".to_string(),
            prompt_key: None,
            optional: false,
        }];
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert!(
        runner.game.pending_selection.is_some(),
        "SelectOpponentPermanent should install a PendingSelection"
    );
    assert_eq!(
        runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .len(),
        1,
        "one permanent on opponent's field → one valid action"
    );
}
