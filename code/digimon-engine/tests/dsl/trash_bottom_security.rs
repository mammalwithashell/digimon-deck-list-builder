//! Behavioral coverage of the shared security-bottom lowering, independent of cards.
use digimon_dsl::compiled::{CompiledPlayerRef, CompiledStep};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::{bindings::Bindings, step::run_step};
use digimon_engine::effect_context::EffectContext;

#[test]
fn trash_bottom_security_moves_only_bottom_physical_card_of_referenced_player() {
    for (of, owner) in [
        (CompiledPlayerRef::You, 0),
        (CompiledPlayerRef::Opponent, 1),
    ] {
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("SOURCE", "Source"))
            .add_card(make_test_card("BOTTOM", "Bottom"))
            .add_card(make_test_card("TOP", "Top"))
            .hand(0, &["SOURCE"])
            .security(0, &["BOTTOM", "TOP"])
            .security(1, &["BOTTOM", "TOP"])
            .build();
        let source = runner.game.players[0].hand[0].handle();
        let bottom = runner.game.players[owner].security[0].handle();
        let top = runner.game.players[owner].security[1].handle();
        let other: Vec<_> = runner.game.players[1 - owner]
            .security
            .iter()
            .map(|c| c.handle())
            .collect();
        run_step(
            &CompiledStep::TrashBottomSecurity { of },
            &mut EffectContext::new(&mut runner.game, source, None, 0),
            &mut Bindings::new(),
        );
        assert_eq!(runner.game.players[owner].security.len(), 1);
        assert_eq!(runner.game.players[owner].security[0].handle(), top);
        assert_eq!(
            runner.game.players[owner].trash.last().unwrap().handle(),
            bottom
        );
        assert_eq!(
            runner.game.players[1 - owner]
                .security
                .iter()
                .map(|c| c.handle())
                .collect::<Vec<_>>(),
            other
        );
    }
}

#[test]
fn trash_bottom_security_empty_stack_does_not_move_another_zone() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SOURCE", "Source"))
        .hand(0, &["SOURCE"])
        .deck(0, &["SOURCE"])
        .build();
    let source = runner.game.players[0].hand[0].handle();
    run_step(
        &CompiledStep::TrashBottomSecurity {
            of: CompiledPlayerRef::You,
        },
        &mut EffectContext::new(&mut runner.game, source, None, 0),
        &mut Bindings::new(),
    );
    assert!(runner.game.players[0].security.is_empty());
    assert!(runner.game.players[0].trash.is_empty());
    assert_eq!(runner.game.players[0].hand.len(), 1);
    assert_eq!(runner.game.players[0].deck.len(), 1);
}
