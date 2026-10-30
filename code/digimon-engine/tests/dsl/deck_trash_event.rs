//! Exact-card effect-caused deck-trash timing; revealed cards are not deck cards.
use digimon_engine::action::{build_action_mask, space::PLAY_HAND_START};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::GamePhase;

const TRIGGER: &str = r#"
card: DECK-TRIGGER
name: Deck Trigger
kind: digimon
level: 3
color: [purple]
cost: 0
dp: 1000
effects:
  - when: on_discard_deck
    scope: trash
    process:
      - gain_memory: 1
"#;
fn fixture(owner: u8, count: u8) -> DebugRunner { fixture_parked(owner, count, false) }
fn fixture_parked(owner: u8, count: u8, park: bool) -> DebugRunner {
    fixture_with_trigger(owner, count, park, TRIGGER)
}
fn fixture_with_trigger(owner: u8, count: u8, park: bool, trigger: &str) -> DebugRunner {
    let miller = format!(r#"
card: MILLER
name: Miller
kind: digimon
level: 3
color: [purple]
cost: 0
dp: 1000
effects:
  - when: on_play
    process:
      - trash_from_top: {{ of: {}, count: {} }}
"#, if owner == 0 { "you" } else { "opponent" }, count);
    let miller = if park { format!("{miller}\n      - select_effect_choice:\n          bind_as: finish\n          labels: [Finish, Finish also]\n          prompt: Finish the causing effect\n") } else { miller };
    let mut r = DebugRunner::builder().from_dsl_yaml(trigger).expect("self deck-trash timing")
        .from_dsl_yaml(&miller).unwrap().add_card(make_test_card("PAD", "Pad"))
        .hand(0, &["MILLER"]).deck(0, &["PAD"; 5]).deck(1, &["PAD"; 5])
        .deck(owner, &["PAD", "PAD", "DECK-TRIGGER", "DECK-TRIGGER", "DECK-TRIGGER"])
        .memory(5).start();
    r.game.current_phase = GamePhase::Main;
    let unrelated = r.game.players[owner as usize].deck.pop().unwrap();
    r.game.players[owner as usize].put_in_trash(unrelated);

    r
}
#[test]
fn on_discard_deck_fires_for_the_milled_physical_card_of_either_owner() {
    for owner in [0, 1] {
        let mut r = fixture(owner, 1);
        let card = r.game.players[owner as usize].deck.last().unwrap().handle();
        assert!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize] > 0.0);
        r.game.decode_action(PLAY_HAND_START, 0);
        r.auto_resolve().unwrap();
        assert!(r.game.players[owner as usize].trash.iter().any(|c| c.handle() == card));
        assert_eq!(r.game.memory, if owner == 0 { 6 } else { 4 });
    }
}
#[test]
fn on_discard_deck_does_not_fire_for_an_unrelated_existing_trash_card() {
    let mut r = fixture(0, 1);
    for _ in 0..2 {
        let card = r.game.players[0].deck.pop().unwrap();
        r.game.players[0].put_in_trash(card);
    }
    r.game.decode_action(PLAY_HAND_START, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.memory, 5);
}

fn start_parked() -> (DebugRunner, digimon_engine::card_source::CardHandle) {
    let mut r = fixture_parked(0, 1, true);
    let card = r.game.players[0].deck.last().unwrap().handle();
    assert!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize] > 0.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    assert!(r.pending_selection_view().is_some());
    assert_eq!(r.game.memory, 5, "self trigger waits for the causing effect");
    (r, card)
}
fn finish(r: &mut DebugRunner) {
    let view = r.pending_selection_view().unwrap();
    let action = view.valid_action_ids[0];
    assert!(build_action_mask(&r.game, view.selecting_player)[action as usize] > 0.0);
    r.execute_action(view.selecting_player, action).unwrap();
    r.auto_resolve().unwrap();
}
#[test]
fn parked_deck_trash_source_leaving_then_reentering_is_a_new_card() {
    let (mut r, card) = start_parked();
    let index = r.game.players[0].trash.iter().position(|c| c.handle() == card).unwrap();
    let moved = r.game.players[0].trash.remove(index);
    r.game.players[0].put_in_hand(moved);
    let moved = r.game.players[0].hand.pop().unwrap();
    r.game.players[0].put_in_trash(moved);
    finish(&mut r);
    assert_eq!(r.game.memory, 5, "the old pending incarnation cannot revive");
}
#[test]
fn rollback_storage_insertion_preserves_the_pending_trash_incarnation() {
    let (mut r, card) = start_parked();
    let index = r.game.players[0].trash.iter().position(|c| c.handle() == card).unwrap();
    let moved = r.game.players[0].trash.remove(index);
    r.game.players[0].insert_in_trash(index, moved);
    finish(&mut r);
    assert_eq!(r.game.memory, 6, "failed tentative extraction is not a new area entry");
}
#[test]
fn cloning_preserves_pending_card_identity_and_branch_specific_departure() {
    let (mut r, card) = start_parked();
    let mut other = fixture_parked(0, 1, true);
    other.game = r.game.clone();
    let index = other.game.players[0].trash.iter().position(|c| c.handle() == card).unwrap();
    let moved = other.game.players[0].trash.remove(index);
    other.game.players[0].put_in_hand(moved);
    finish(&mut r); finish(&mut other);
    assert_eq!(r.game.memory, 6);
    assert_eq!(other.game.memory, 5);
}
#[test]
fn verification_distinguishes_reentered_pending_source_with_identical_visible_zones() {
    let (r, card) = start_parked();
    let mut other = fixture_parked(0, 1, true); other.game = r.game.clone();
    let index = other.game.players[0].trash.iter().position(|c| c.handle() == card).unwrap();
    let moved = other.game.players[0].trash.remove(index);
    other.game.players[0].put_in_hand(moved);
    let moved = other.game.players[0].hand.pop().unwrap(); other.game.players[0].put_in_trash(moved);
    assert_ne!(r.game.verification_digest(), other.game.verification_digest());
}

#[test]
fn multiple_physical_deck_copies_each_trigger_exactly_once() {
    for owner in [0, 1] {
        let mut r = fixture(owner, 2);
        r.game.decode_action(PLAY_HAND_START, 0);
        r.auto_resolve().unwrap();
        assert_eq!(r.game.memory, if owner == 0 { 7 } else { 3 });
        assert_eq!(r.game.players[owner as usize].trash.len(), 3);
    }
}
#[test]
fn revealed_cards_trashed_by_effect_are_not_trashed_from_deck() {
    let mut r = fixture_parked(0, 0, true);
    let source = r.game.players[0].hand[0].handle();
    let card = r.game.players[0].deck.pop().unwrap();
    let handle = card.handle();
    r.game.revealed_cards.push(card);
    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(&mut r.game, source, None, 0);
        assert!(ctx.trash_from_reveal(0, handle));
    }
    r.game.decode_action(PLAY_HAND_START, 0);
    finish(&mut r);
    assert_eq!(r.game.memory, 5);
    assert!(r.game.players[0].trash.iter().any(|c| c.handle() == handle));
}

#[test]
fn already_enqueued_deck_trash_source_cannot_revive_after_reentry() {
    let trigger = TRIGGER.replace("      - gain_memory: 1", "      - select_effect_choice:\n          bind_as: finish\n          labels: [Finish, Finish also]\n          prompt: Finish this deck-trash trigger\n      - gain_memory: 1");
    let mut r = fixture_with_trigger(0, 2, false, &trigger);
    let causing_card = r.game.players[0].hand[0].handle();
    r.game.decode_action(PLAY_HAND_START, 0);
    // The first sibling parks inside its body, after both deck-trash events
    // have been released. The remaining queued sibling is the subject here.
    let sibling = r.game.effect_queue.iter()
        .find(|q| q.timing == digimon_engine::enums::EffectTiming::OnDiscardDeck)
        .expect("second physical copy already enqueued");
    let queued = sibling.source_card;
    let event = sibling.trigger_context.as_ref().unwrap();
    let cause = event.event_cause_effect.unwrap();
    assert_eq!(cause.controller, 0);
    assert_eq!(cause.source_card, Some(causing_card), "retain the miller attribution while a sibling resolves");
    assert_eq!(event.source_effect.unwrap().source_card, Some(causing_card));
    assert_eq!(event.event_card, Some(queued));
    assert!(r.pending_selection_view().is_some());
    assert_eq!(r.game.memory, 5);
    let index = r.game.players[0].trash.iter().position(|c| c.handle() == queued).unwrap();
    let card = r.game.players[0].trash.remove(index);
    r.game.players[0].put_in_hand(card);
    let card = r.game.players[0].hand.pop().unwrap();
    r.game.players[0].put_in_trash(card);
    finish(&mut r);
    assert_eq!(r.game.memory, 6, "only the still-live first sibling resolves");
}

#[test]
fn unrelated_trash_arrival_history_does_not_change_verification_state() {
    let (r, _) = start_parked();
    let mut other = fixture_parked(0, 1, true);
    other.game = r.game.clone();
    // This is the preexisting copy, never an event subject. Restore the exact
    // zone order while changing only its irrelevant arrival history.
    let card = other.game.players[0].trash.remove(0);
    other.game.players[0].put_in_hand(card);
    let card = other.game.players[0].hand.pop().unwrap();
    other.game.players[0].put_in_trash(card);
    let card = other.game.players[0].trash.pop().unwrap();
    other.game.players[0].trash.insert(0, card);
    assert_eq!(r.game.verification_digest(), other.game.verification_digest());
}
