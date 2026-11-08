use super::*;
use crate::{
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    effect::{CardEffect, Effect},
};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

struct CapturedResource {
    calls: Arc<AtomicUsize>,
    captured: Option<CardHandle>,
}
impl CardEffect for CapturedResource {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut effect = Effect::on_play(card).build();
        effect.resource_flow = self.captured.is_some();
        vec![effect]
    }
}

fn verify<T: PartialEq + std::fmt::Debug>(reject: bool, run: impl FnOnce() -> T, expected: T) {
    let result = catch_unwind(AssertUnwindSafe(run));
    if reject {
        let error = result.expect_err("greedy must reject before reading factory metadata");
        let message = error
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied())
            .unwrap_or("");
        assert!(
            message.contains("closed-list projected world reached unproven factory"),
            "{message}"
        );
    } else {
        assert_eq!(result.unwrap(), expected);
    }
}

#[test]
fn greedy_resource_metadata_checks_world_before_factory_on_real_legal_mask() {
    for seat in 0..2 {
        for answer in [false, true] {
            for reject in [false, true] {
                for cloned in [false, true] {
                    let mut game = DebugRunner::builder()
                        .add_card(make_test_card("GREEDY-CAPTURE", "Plain rookie"))
                        .hand(seat, &["GREEDY-CAPTURE"])
                        .memory(5)
                        .start()
                        .game;
                    game.current_phase = GamePhase::Main;
                    game.turn_player_idx = seat as usize;
                    game.memory_pair = (seat, 1 - seat);
                    let calls = Arc::new(AtomicUsize::new(0));
                    game.effect_registry.insert(
                        "GREEDY-CAPTURE",
                        Arc::new(CapturedResource {
                            calls: calls.clone(),
                            captured: answer.then_some(CardHandle(42)),
                        }),
                    );
                    let mask = crate::action::build_action_mask(&game, seat);
                    assert!(
                        mask[PLAY_HAND_START as usize] > 0.0,
                        "fixture must expose an actual legal play"
                    );
                    if reject {
                        game.require_projected_world_read_guard_for_test();
                    }
                    if cloned {
                        game = game.clone();
                    }
                    game.current_trigger_context = None;
                    let before = calls.load(Ordering::SeqCst);
                    verify(
                        reject,
                        || card_has_resource_flow(&game, &game.players[seat as usize].hand[0]),
                        answer,
                    );
                    verify(reject, || greedy_action(&game, &mask), PLAY_HAND_START);
                    assert_eq!(
                        calls.load(Ordering::SeqCst) - before,
                        if reject { 0 } else { 2 }
                    );
                    assert_eq!(game.memory, 5);
                    assert_eq!(game.players[seat as usize].hand.len(), 1);
                    assert!(game.pending_selection.is_none());
                }
            }
        }
    }
}

#[test]
fn greedy_printed_resource_fast_path_still_avoids_factory_execution() {
    for seat in 0..2 {
        for guarded in [false, true] {
            let mut card = make_test_card("GREEDY-PRINTED", "Printed draw");
            card.effect_text = "Draw 1.".into();
            let mut game = DebugRunner::builder()
                .add_card(card)
                .hand(seat, &["GREEDY-PRINTED"])
                .start()
                .game;
            let calls = Arc::new(AtomicUsize::new(0));
            game.effect_registry.insert(
                "GREEDY-PRINTED",
                Arc::new(CapturedResource {
                    calls: calls.clone(),
                    captured: None,
                }),
            );
            if guarded {
                game.require_projected_world_read_guard_for_test();
            }
            assert!(card_has_resource_flow(
                &game,
                &game.players[seat as usize].hand[0]
            ));
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        }
    }
}
