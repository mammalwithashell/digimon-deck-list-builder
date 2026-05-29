//! BT14-102 Angemon.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

#[test]
fn bt14_102_inherited_on_deletion_places_yellow_vaccine_from_hand_into_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT14-102")
        .expect("BT14-102 loads")
        .add_card(digimon("CARRIER", CardColor::Yellow, &[]))
        .add_card(digimon("HAND-VACCINE", CardColor::Yellow, &["Vaccine"]))
        .hand(0, &["HAND-VACCINE"])
        .memory(5)
        .start();
    let carrier = runner.place_stack(0, &["BT14-102", "CARRIER"]);

    runner.game.delete_permanent_with_effects(carrier);
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect("hand security target");
    runner
        .execute_action(view.selecting_player, first_non_pass(&view.valid_action_ids))
        .expect("select yellow Vaccine");
    runner.auto_resolve().expect("finish inherited effect");

    assert_eq!(runner.game.players[0].hand.len(), 0);
    assert_eq!(runner.game.players[0].security.len(), 1);
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        "HAND-VACCINE"
    );
}

#[test]
fn bt14_102_has_attack_choice_and_self_security_on_deletion_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("BT14-102")
        .expect("BT14-102 loads")
        .start();
    let card = runner.compiled_card("BT14-102").expect("compiled card");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        digimon_dsl::compiled::CompiledClause::Triggered(t)
            if t.when.contains(&digimon_dsl::compiled::CompiledTiming::WhenAttacking)
                && t.optional
    )));
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        digimon_dsl::compiled::CompiledClause::Triggered(t)
            if t.when.contains(&digimon_dsl::compiled::CompiledTiming::OnDeletion)
                && t.scope == digimon_dsl::compiled::CompiledScope::FaceUp
    )));
}

fn first_non_pass(ids: &[u16]) -> u16 {
    ids.iter()
        .copied()
        .find(|id| *id != digimon_engine::action::space::PASS)
        .expect("non-pass action")
}

fn digimon(
    id: &str,
    color: CardColor,
    traits: &[&str],
) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.colors = vec![color];
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}
