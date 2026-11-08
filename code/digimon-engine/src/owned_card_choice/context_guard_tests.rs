use super::*;
use crate::{debug_runner::{make_test_card, DebugRunner}, trigger_context::TriggerContext};
use std::{panic::{catch_unwind, AssertUnwindSafe}, sync::{Arc, atomic::{AtomicUsize, Ordering}}};

fn fixture(player: u8) -> (Game, OwnedCardChoice) {
    let mut game = DebugRunner::builder().add_card(make_test_card("GUARD-CARD", "Guard"))
        .security(player, &["GUARD-CARD"]).memory(5).start().game;
    let choice = OwnedCardChoice::security(&mut game, player, vec![0], false).unwrap();
    (game, choice)
}
fn mark(game: &mut Game, marked: bool, nested: bool) {
    let mut context = TriggerContext::default();
    context.projected_reference_reads_guarded = marked;
    if nested {
        let mut parent = TriggerContext::default();
        parent.simultaneous_play_members.push(context);
        context = parent;
    }
    game.current_trigger_context = Some(context);
}

#[test]
fn raw_security_filter_rejects_before_body_through_live_choice() {
    for player in [0, 1] { for nested in [false, true] { for marked in [false, true] {
        let (mut game, mut choice) = fixture(player);
        let calls = Arc::new(AtomicUsize::new(0)); let seen = calls.clone();
        choice.set_security_filter(move |game, index| {
            seen.fetch_add(1, Ordering::SeqCst);
            assert_eq!(index, 0); assert_eq!(game.player(player).security.len(), 1); true
        });
        let clone = choice.clone();
        assert_eq!(clone, choice, "cloning retains shared filter identity");
        mark(&mut game, marked, nested);
        let token = clone.token();
        let result = catch_unwind(AssertUnwindSafe(|| clone.valid_action_ids(&game)));
        if marked {
            assert!(result.is_err(), "raw security predicate must reject before body");
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        } else {
            assert!(result.unwrap().contains(&crate::card_choice_page::CARD_CHOICE_START));
            assert!(calls.load(Ordering::SeqCst) > 0);
        }
        assert_eq!(clone.token(), token); assert_eq!(game.memory, 5);
    } } }
}

#[test]
fn source_completion_recovery_checks_before_none_candidate_body() {
    for nested in [false, true] { for marked in [false, true] { for allowed in [false, true] {
        let (mut game, mut choice) = fixture(0);
        let calls = Arc::new(AtomicUsize::new(0)); let seen = calls.clone();
        choice.set_source_completion_filter(move |_, candidate| {
            seen.fetch_add(1, Ordering::SeqCst); assert!(candidate.is_none()); allowed
        });
        let clone = choice.clone(); mark(&mut game, marked, nested);
        let result = catch_unwind(AssertUnwindSafe(|| clone.recovery(&game)));
        if marked {
            assert!(result.is_err()); assert_eq!(calls.load(Ordering::SeqCst), 0);
        } else {
            assert_eq!(result.unwrap(), if allowed { CardChoiceRecovery::Current } else { CardChoiceRecovery::Impossible });
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        }
    } } }
}

#[test]
fn stored_source_completion_checks_some_candidate_even_when_prefetched() {
    for nested in [false, true] { for marked in [false, true] { for allowed in [false, true] {
        let (mut game, mut choice) = fixture(0);
        let expected = CardChoicePin::Security { owner: 0, card: game.players[0].security[0].handle() };
        let calls = Arc::new(AtomicUsize::new(0)); let seen = calls.clone();
        choice.set_source_completion_filter(move |_, candidate| {
            seen.fetch_add(1, Ordering::SeqCst); assert_eq!(candidate, Some(expected)); allowed
        });
        let prefetched = choice.source_completion_filter.as_ref().unwrap().0.clone();
        mark(&mut game, marked, nested);
        let result = catch_unwind(AssertUnwindSafe(|| prefetched(&game, Some(expected))));
        if marked {
            assert!(result.is_err()); assert_eq!(calls.load(Ordering::SeqCst), 0);
        } else {
            assert_eq!(result.unwrap(), allowed); assert_eq!(calls.load(Ordering::SeqCst), 1);
        }
    } } }
}
