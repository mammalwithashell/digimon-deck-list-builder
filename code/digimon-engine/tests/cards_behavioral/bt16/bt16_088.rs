//! BT16-088 Cody Hida & T.K. Takaishi.

use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, PlaySource};
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn bt16_088_start_of_main_plays_patamon_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-088")
        .expect("BT16-088 loads")
        .add_card(digimon_named("PATAMON", "Patamon", CardColor::Yellow))
        .hand(0, &["PATAMON"])
        .memory(5)
        .start();
    let tamer = runner.place_on_field(0, "BT16-088", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(tamer));
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("Patamon hand selection");
    assert_eq!(view.kind, SelectionKind::Hand);
    let pick = first_non_pass(&view.valid_action_ids);
    runner
        .execute_action(view.selecting_player, pick)
        .expect("select Patamon");
    runner.auto_resolve().expect("finish free play");

    assert_eq!(runner.game.players[0].hand.len(), 0);
    assert_eq!(runner.game.players[0].battle_area.len(), 2);
    assert_eq!(runner.game.memory, 5, "free play should not spend memory");
}

#[test]
fn bt16_088_black_yellow_digivolve_observer_suspends_for_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-088")
        .expect("BT16-088 loads")
        .add_card(digimon_named("BASE", "Base", CardColor::Yellow))
        .add_card(level4("YELLOW-EVO", CardColor::Yellow))
        .hand(0, &["YELLOW-EVO"])
        .memory(3)
        .start();
    let tamer = runner.place_on_field(0, "BT16-088", Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.game.current_phase = GamePhase::Main;

    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByDigivolve),
        "digivolve into yellow"
    );
    let view = runner
        .pending_selection_view()
        .expect("optional tamer observer");
    runner
        .execute_action(view.selecting_player, first_non_pass(&view.valid_action_ids))
        .expect("accept observer");
    runner.auto_resolve().expect("finish observer");

    assert!(runner.game.players[0].battle_area[tamer.index as usize].is_suspended);
    assert_eq!(runner.game.memory, 4);
}

fn first_non_pass(ids: &[u16]) -> u16 {
    ids.iter()
        .copied()
        .find(|id| *id != digimon_engine::action::space::PASS)
        .expect("non-pass action")
}

fn digimon_named(id: &str, name: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(1000);
    card.play_cost = 3;
    card.colors = vec![color];
    card
}

fn level4(id: &str, color: CardColor) -> CardData {
    let mut card = digimon_named(id, id, color);
    card.level = Some(4);
    card.dp = Some(4000);
    card.evo_costs = vec![EvoCost {
        card_color: color as u8,
        level: 3,
        memory_cost: 0,
    }];
    card
}
