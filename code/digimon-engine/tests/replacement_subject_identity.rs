use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use std::sync::Arc;

struct Rider;
impl CardEffect for Rider {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("Identity rider")
            .optional()
            .replacement_process(|rctx| {
                rctx.effect
                    .select_own_permanent("Continue", true, |_g, _h| true, |_ctx, _pick| {});
            })
            .build()]
    }
}

#[test]
fn deferred_replacement_retains_original_subject_across_field_shift() {
    for accept in [false, true] {
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("TARGET", "Target"))
            .add_card(make_test_card("BYSTANDER", "Bystander"))
            .start();
        runner.register_effect("TARGET", Arc::new(Rider));
        let target = runner.place_on_field(0, "TARGET", None);
        runner.place_on_field(0, "BYSTANDER", None);
        runner.game.delete_permanent_with_effects(target);
        assert!(runner.game.pending_selection.is_some());
        if accept {
            runner
                .game
                .resolve_selection(0, REPLACEMENT_ACCEPT)
                .unwrap();
        }
        runner.game.players[0].battle_area.swap(0, 1);
        let mut cloned = runner.game.clone();
        for game in [&mut runner.game, &mut cloned] {
            game.resolve_selection(0, PASS).unwrap();
            let remaining: Vec<_> = game.players[0]
                .battle_area
                .iter()
                .map(|p| p.top_card().card_id(&game.card_data))
                .collect();
            assert_eq!(remaining, vec!["BYSTANDER"], "accept={accept}");
            assert!(game.pending_selection.is_none());
        }
    }
}
