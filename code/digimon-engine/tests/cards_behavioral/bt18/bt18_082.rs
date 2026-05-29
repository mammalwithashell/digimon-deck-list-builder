//! BT18-082 Lucemon: Chaos Mode.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn bt18_082_opponent_declines_delete_then_recover_and_trash_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-082")
        .expect("BT18-082 loads")
        .add_card(make_test_card("SEC", "Security"))
        .add_card(digimon("OPP-DIGI"))
        .deck(0, &["SEC"])
        .security(0, &["SEC"])
        .security(1, &["SEC", "SEC"])
        .memory(5)
        .start();
    let luce = runner.place_on_field(0, "BT18-082", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DIGI", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(luce));
    runner.game.drain_effect_queue();

    let view = runner.pending_selection_view().expect("opponent delete choice");
    assert_eq!(view.selecting_player, 1);
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("opponent declines");
    runner.auto_resolve().expect("finish fallback");

    assert_eq!(runner.game.players[0].security.len(), 2);
    assert_eq!(runner.game.players[1].security.len(), 1);
}

fn digimon(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card
}

#[test]
fn bt18_082_replacement_trashes_bottom_security_to_prevent_leave() {
    let runner = DebugRunner::builder()
        .dsl_card("BT18-082")
        .expect("BT18-082 loads")
        .start();
    let card = runner.compiled_card("BT18-082").expect("compiled card");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            digimon_dsl::compiled::CompiledClause::Declarative(
                digimon_dsl::compiled::CompiledDeclarativeClause::Replacement { once_per_turn, .. }
            ) if *once_per_turn
        )),
        "Lucemon must have OPT leave-field replacement"
    );
}
