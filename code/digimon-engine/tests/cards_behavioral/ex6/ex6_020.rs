//! EX6-020 Gatomon.

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::SEL_REVEAL_START;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/ex6/EX6-020.yaml");

#[test]
fn ex6_020_reveals_angel_trait_and_mirei_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX6-020 YAML loads")
        .add_card(trait_digimon("ANGEL", "Angel"))
        .add_card(tamer("MIREI", "Mirei Mikagura"))
        .add_card(trait_digimon("HOLY-BEAST", "Holy Beast"))
        .deck(0, &["ANGEL", "MIREI", "HOLY-BEAST"])
        .hand(0, &["EX6-020"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play EX6-020");
    pick_revealed(&mut runner, "ANGEL");
    pick_revealed(&mut runner, "MIREI");
    runner.auto_resolve().expect("bottom reveal remainder");

    let hand = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand.contains(&"ANGEL".to_string()));
    assert!(hand.contains(&"MIREI".to_string()));
    assert!(!hand.contains(&"HOLY-BEAST".to_string()));
}

#[test]
fn ex6_020_inherited_when_attacking_gives_minus_2000_dp() {
    let mut opponent = make_test_card("OPP", "Opponent");
    opponent.colors = vec![CardColor::Purple];
    opponent.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX6-020 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(opponent)
        .memory(0)
        .start();
    let carrier = runner.place_stack(0, &["EX6-020", "CARRIER"]);
    let target = runner.place_on_field(1, "OPP", Some(0));

    fire(&mut runner, EffectTiming::WhenAttacking, carrier);
    let view = runner
        .pending_selection_view()
        .expect("opponent Digimon prompt");
    assert_eq!(view.valid_action_ids.len(), 1);
    assert_mask_allows(&runner, view.selecting_player, view.valid_action_ids[0]);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("select target");

    assert_eq!(runner.effective_dp(target), Some(3000));
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

fn trait_digimon(id: &str, trait_name: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.traits = vec![trait_name.to_string()];
    card
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card
}

fn pick_revealed(runner: &mut DebugRunner, id: &str) {
    let action = runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
        .expect("revealed card exists");
    let view = runner.pending_selection_view().expect("reveal selection");
    assert!(view.valid_action_ids.contains(&action));
    assert_mask_allows(runner, view.selecting_player, action);
    runner
        .execute_action(0, action)
        .expect("pick revealed card");
}

fn assert_mask_allows(runner: &DebugRunner, player: u8, action: u16) {
    assert_eq!(
        build_action_mask(&runner.game, player)[action as usize],
        1.0,
        "pending action {action} must be legal in the action mask"
    );
}

fn zone_ids(cards: &[digimon_engine::card_source::CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}
