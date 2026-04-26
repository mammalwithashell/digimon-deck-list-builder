//! Task 5 integration tests: reveal-pool consumer steps and MarkSecurityFaceUp.
//!
//! One test per new CompiledStep arm:
//!   1. AddToHandFromReveal
//!   2. TrashFromReveal
//!   3. ReturnToDeckFromReveal
//!   4. RevealTopDeck (produces a named binding)
//!   5. PlaceRemainderOnDeck (installs an ordered-permutation selection)
//!   6. TrashFromHandByIndex
//!   7. MarkSecurityFaceUp

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledPlayerRef, CompiledStackPosition, CompiledStep,
};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Reveal the top card of player 0's deck into `game.revealed_cards` by direct
/// push (avoids depending on a specific DebugRunner API). Returns the handle.
fn push_deck_card_to_reveal(runner: &mut DebugRunner) -> CardHandle {
    let card = runner.game.players[0]
        .deck
        .pop()
        .expect("deck must be non-empty to reveal");
    let handle = card.handle();
    runner.game.revealed_cards.push(card);
    handle
}

/// Push a fresh CardSource with the given data_id directly into player 0's
/// security zone. Returns its handle.
fn push_to_security(runner: &mut DebugRunner, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_security: unknown card_id {}", card_id));
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, 0, card_index);
    let handle = card.handle();
    runner.game.players[0].security.push(card);
    handle
}

// ── 1. AddToHandFromReveal ────────────────────────────────────────────────────

#[test]
fn add_to_hand_from_reveal_consumes_card_binding() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("REV", "REV"))
        .hand(0, &["SRC"])
        .deck(0, &["REV"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();

    // Seed the reveal pool by popping the deck card.
    let revealed = push_deck_card_to_reveal(&mut runner);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        bindings.insert_card("top", revealed);

        run_steps(
            &[CompiledStep::AddToHandFromReveal {
                of: CompiledPlayerRef::You,
                card: CompiledBindingRef::Named("top".to_string()),
            }],
            &mut ctx,
            &mut bindings,
        );
    }

    assert!(
        runner.game.revealed_cards.is_empty(),
        "reveal pool should be drained after AddToHandFromReveal"
    );
    assert!(
        runner.game.players[0].hand.iter().any(|cs| cs.handle() == revealed),
        "revealed card should have landed in hand"
    );
}

// ── 2. TrashFromReveal ────────────────────────────────────────────────────────

#[test]
fn trash_from_reveal_moves_card_to_trash() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("REV", "REV"))
        .hand(0, &["SRC"])
        .deck(0, &["REV"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();
    let revealed = push_deck_card_to_reveal(&mut runner);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        bindings.insert_card("top", revealed);

        run_steps(
            &[CompiledStep::TrashFromReveal {
                of: CompiledPlayerRef::You,
                card: CompiledBindingRef::Named("top".to_string()),
            }],
            &mut ctx,
            &mut bindings,
        );
    }

    assert!(
        runner.game.revealed_cards.is_empty(),
        "reveal pool should be drained after TrashFromReveal"
    );
    assert!(
        runner.game.players[0].trash.iter().any(|cs| cs.handle() == revealed),
        "revealed card should be in trash"
    );
}

// ── 3. ReturnToDeckFromReveal ─────────────────────────────────────────────────

#[test]
fn return_to_deck_from_reveal_places_card_on_top() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("REV", "REV"))
        .hand(0, &["SRC"])
        .deck(0, &["REV"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();
    let revealed = push_deck_card_to_reveal(&mut runner);

    // Deck is empty now (we popped the only card into reveal pool).
    assert!(runner.game.players[0].deck.is_empty());

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        bindings.insert_card("top", revealed);

        run_steps(
            &[CompiledStep::ReturnToDeckFromReveal {
                of: CompiledPlayerRef::You,
                card: CompiledBindingRef::Named("top".to_string()),
                position: CompiledStackPosition::Top,
            }],
            &mut ctx,
            &mut bindings,
        );
    }

    assert!(
        runner.game.revealed_cards.is_empty(),
        "reveal pool should be empty after ReturnToDeckFromReveal"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        1,
        "card should be back in deck"
    );
    assert_eq!(
        runner.game.players[0].deck.last().unwrap().handle(),
        revealed,
        "card should be at the top of the deck (last element)"
    );
}

// ── 4. RevealTopDeck → produces a named binding ───────────────────────────────

#[test]
fn reveal_top_deck_binds_single_card_by_name() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("TOP", "TOP"))
        .hand(0, &["SRC"])
        .deck(0, &["TOP"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();
    // Record the handle of the top deck card before revealing.
    let deck_top_handle = runner.game.players[0].deck.last().unwrap().handle();

    let mut bindings = Bindings::new();
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);

        run_steps(
            &[CompiledStep::RevealTopDeck {
                of: CompiledPlayerRef::You,
                count: 1,
                zone: None,
                bind_as: Some("revealed_card".to_string()),
            }],
            &mut ctx,
            &mut bindings,
        );
    }

    // Deck should now be empty; reveal pool has one card.
    assert!(runner.game.players[0].deck.is_empty(), "deck top was revealed");
    assert_eq!(runner.game.revealed_cards.len(), 1, "one card in reveal pool");
    assert_eq!(runner.game.revealed_cards[0].handle(), deck_top_handle);

    // The binding should be set.
    let bound = bindings
        .get_card("revealed_card")
        .expect("RevealTopDeck should have set 'revealed_card' binding");
    assert_eq!(bound, deck_top_handle, "binding should match the revealed card");
}

// ── 5. PlaceRemainderOnDeck ───────────────────────────────────────────────────

#[test]
fn place_remainder_on_deck_installs_permutation_selection_and_resolves() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("REM", "REM"))
        .hand(0, &["SRC"])
        .deck(0, &["REM"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();
    let rem_handle = push_deck_card_to_reveal(&mut runner);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();

        run_steps(
            &[CompiledStep::PlaceRemainderOnDeck {
                of: CompiledPlayerRef::You,
                position: CompiledStackPosition::Top,
            }],
            &mut ctx,
            &mut bindings,
        );
    }

    // PlaceRemainderOnDeck installs an OrderedPermutation selection — resolve it.
    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("PlaceRemainderOnDeck should install a PendingSelection for the permutation");
    assert_eq!(sel.valid_action_ids.len(), 1, "one card → one action");
    let action_id = sel.valid_action_ids[0];
    let selecting_player = sel.selecting_player;

    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve PlaceRemainderOnDeck permutation");

    assert!(runner.game.pending_selection.is_none(), "selection cleared");
    assert!(runner.game.revealed_cards.is_empty(), "reveal pool drained");
    assert_eq!(runner.game.players[0].deck.len(), 1, "card returned to deck");
    assert_eq!(
        runner.game.players[0].deck.last().unwrap().handle(),
        rem_handle,
        "card at top of deck"
    );
}

// ── 6. TrashFromHandByIndex ───────────────────────────────────────────────────

#[test]
fn trash_from_hand_by_index_moves_card_from_hand_to_trash() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("TGT", "TGT"))
        .hand(0, &["SRC", "TGT"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();
    let tgt_handle = runner.game.players[0].hand[1].handle();

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        // Hand index 1 = TGT (the card we want to trash).
        bindings.insert_hand_index("idx", 0, 1);

        run_steps(
            &[CompiledStep::TrashFromHandByIndex {
                of: CompiledPlayerRef::You,
                hand_index: CompiledBindingRef::Named("idx".to_string()),
            }],
            &mut ctx,
            &mut bindings,
        );
    }

    assert_eq!(runner.game.players[0].hand.len(), 1, "one card should remain in hand");
    assert!(
        runner.game.players[0].trash.iter().any(|cs| cs.handle() == tgt_handle),
        "TGT should be in trash"
    );
}

// ── 7. MarkSecurityFaceUp ─────────────────────────────────────────────────────

#[test]
fn mark_security_face_up_sets_face_up_bit_for_security_card() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("SEC", "SEC"))
        .hand(0, &["SRC"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();
    // Place SEC directly into player 0's security zone.
    let sec_handle = push_to_security(&mut runner, "SEC");

    // Confirm the card starts face-down (not in face_up_security).
    assert!(
        !runner.game.players[0].face_up_security.contains(&sec_handle.0),
        "security card should start face-down"
    );

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        bindings.insert_card("sec", sec_handle);

        run_steps(
            &[CompiledStep::MarkSecurityFaceUp {
                of: CompiledPlayerRef::You,
                card: CompiledBindingRef::Named("sec".to_string()),
            }],
            &mut ctx,
            &mut bindings,
        );
    }

    assert!(
        runner.game.players[0].face_up_security.contains(&sec_handle.0),
        "security card should now be marked face-up"
    );
}
