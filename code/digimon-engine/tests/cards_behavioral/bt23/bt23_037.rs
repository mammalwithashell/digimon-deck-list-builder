//! BT23-037 Tentomon.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, ModifierType};
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn bt23_037_inherited_attack_plays_hudie_and_blocks_digivolving() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-037")
        .expect("BT23-037 loads")
        .add_card(carrier())
        .add_card(hudie("HUDIE"))
        .hand(0, &["HUDIE"])
        .memory(5)
        .start();
    let carrier = runner.place_stack(0, &["BT23-037", "CARRIER"]);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();

    let view = runner.pending_selection_view().expect("Hudie hand pick");
    assert_eq!(view.kind, SelectionKind::Hand);
    runner
        .execute_action(view.selecting_player, first_non_pass(&view.valid_action_ids))
        .expect("pick Hudie");
    runner.auto_resolve().expect("finish play");

    let played = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 1,
    };
    assert!(runner
        .game
        .modifiers
        .has(played, ModifierType::CannotDigivolve));
}

#[test]
fn bt23_037_compiles_cs_cost_reduction_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT23-037")
        .expect("BT23-037 loads")
        .start();
    let card = runner.compiled_card("BT23-037").expect("compiled card");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            digimon_dsl::compiled::CompiledClause::Declarative(
                digimon_dsl::compiled::CompiledDeclarativeClause::CostReduction { amount, .. }
            ) if *amount == Some(1)
        )),
        "Tentomon must reduce digivolving into CS by 1"
    );
}

fn first_non_pass(ids: &[u16]) -> u16 {
    ids.iter()
        .copied()
        .find(|id| *id != digimon_engine::action::space::PASS)
        .expect("non-pass action")
}

fn carrier() -> CardData {
    let mut card = make_test_card("CARRIER", "Carrier");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.colors = vec![CardColor::Yellow];
    card
}

fn hudie(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(1000);
    card.play_cost = 5;
    card.traits = vec!["Hudie".to_string()];
    card
}
