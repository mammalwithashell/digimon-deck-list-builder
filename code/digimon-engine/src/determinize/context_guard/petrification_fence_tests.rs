use crate::{
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    effect::{CardEffect, Effect},
    effect_context::EffectContext,
    enums::GamePhase,
    trigger_context::TriggerContext,
};
use std::{
    collections::HashSet,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{atomic::{AtomicUsize, Ordering}, Arc},
};

#[test]
fn petrification_fence_preserves_native_play_and_deletion_for_each_controller() {
    for seat in [0, 1] {
        for cloned in [false, true] {
            for fenced in [false, true] {
                for unavailable in [false, true] {
                    for security_count in [0, 2] {
                        let mut r = DebugRunner::builder()
                            .add_card(make_test_card("TOKEN-FENCE", "Token fixture"))
                            .hand(seat, &["TOKEN-FENCE"])
                            .security(seat, &vec!["TOKEN-FENCE"; security_count])
                            .security(1 - seat, &["TOKEN-FENCE", "TOKEN-FENCE"])
                            .memory(5).start();
                        r.skip_mulligan();
                        r.set_first_player(seat);
                        r.set_phase(GamePhase::Main);
                        let source = r.game.players[seat as usize].hand[0].handle();
                        if unavailable {
                            let mut context = TriggerContext::default();
                            context.event_card = Some(CardHandle(65000)).into();
                            assert!(context.event_card.project_if_unentitled(&HashSet::new()));
                            r.game.current_trigger_context = Some(context);
                        }
                        if fenced { r.game.require_event_card_read_fence_for_test(); }
                        let mut game = if cloned { r.game.clone() } else { r.game };
                        let token = EffectContext::new(&mut game, source, None, seat)
                            .play_token(seat, "petrification").expect("token must be played");
                        let physical = game.players[seat as usize].battle_area[token.index as usize]
                            .top_card().handle();
                        let effects = game.registry_effects("TOKEN_PETRIFICATION", physical).unwrap();
                        assert_eq!(effects.len(), 1);
                        assert_eq!(effects[0].source_card, physical);
                        game.delete_permanent_with_effects(token);
                        assert!(game.players[seat as usize].battle_area.is_empty());
                        assert_eq!(game.players[seat as usize].security.len(), security_count.saturating_sub(1));
                        assert_eq!(game.players[seat as usize].trash.len(), usize::from(security_count != 0));
                        assert_eq!(game.players[(1 - seat) as usize].security.len(), 2);
                        assert!(game.players[(1 - seat) as usize].trash.is_empty());
                        assert!(game.pending_selection.is_none());
                        assert_eq!(game.projected_event_card_read_fence(), fenced);
                    }
                }
            }
        }
    }
}

struct OpaqueToken(Arc<AtomicUsize>);
impl CardEffect for OpaqueToken {
    fn effects(&self, _: CardHandle) -> Vec<Effect> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Vec::new()
    }
}

#[test]
fn petrification_registration_cannot_authorize_an_opaque_replacement() {
    for cloned in [false, true] {
        let mut game = DebugRunner::builder().start().game;
        let factory = game.effect_registry.get("TOKEN_PETRIFICATION").unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        game.effect_registry.insert("TOKEN_PETRIFICATION", Arc::new(OpaqueToken(calls.clone())));
        let mut game = if cloned { game.clone() } else { game };
        game.require_event_card_read_fence_for_test();
        assert!(!game.effect_registry.read_fence_constructible("TOKEN_PETRIFICATION"));
        let result = catch_unwind(AssertUnwindSafe(|| game.registry_effects("TOKEN_PETRIFICATION", CardHandle(42))));
        let error = result.expect_err("same card ID cannot authorize an opaque factory");
        let message = error.downcast_ref::<String>().unwrap();
        assert!(message.contains("event-reference fence reached opaque factory"));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        // A retained fixed factory does not acquire the replacement's body.
        assert_eq!(factory.effects(CardHandle(42)).len(), 1);
    }
}
