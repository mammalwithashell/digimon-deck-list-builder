//! ST10-12 LadyDevimon.
//!
//! [When Digivolving] You may trash 1 card in your hand to reveal the top 3
//! cards of your deck. Add 1 yellow and 1 purple Digimon with the [Angel],
//! [Archangel] or [Fallen Angel] trait among them to your hand. Place the
//! remaining cards at the bottom of your deck in any order.
//!
//! Inherited: [Your Turn] All of your yellow Digimon gain <Retaliation>.

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{PLAY_HAND_START, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

#[test]
fn st10_12_when_digivolving_discards_then_adds_yellow_and_purple_angel_line() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST10-12")
        .expect("ST10-12 YAML loads")
        .add_card(digimon("BASE", CardColor::Purple, "Filler"))
        .add_card(digimon("DISCARD", CardColor::Purple, "Filler"))
        .add_card(digimon("YELLOW-ANGEL", CardColor::Yellow, "Angel"))
        .add_card(digimon("PURPLE-FALLEN", CardColor::Purple, "Fallen Angel"))
        .add_card(digimon("REMAINDER", CardColor::Blue, "Beast"))
        .hand(0, &["DISCARD"])
        .deck(0, &["YELLOW-ANGEL", "PURPLE-FALLEN", "REMAINDER"])
        .start();

    let lady = runner.place_stack(0, &["BASE", "ST10-12"]);
    fire(&mut runner, EffectTiming::WhenDigivolving, lady);

    let discard_action = PLAY_HAND_START;
    let view = runner.pending_selection_view().expect("discard prompt");
    assert!(view.valid_action_ids.contains(&discard_action));
    assert_mask_allows(&runner, view.selecting_player, discard_action);
    runner
        .execute_action(0, discard_action)
        .expect("choose hand card to trash");

    pick_revealed(&mut runner, "YELLOW-ANGEL");
    pick_revealed(&mut runner, "PURPLE-FALLEN");
    runner.auto_resolve().expect("bottom the remainder");

    let hand = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    let trash = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(hand.contains(&"YELLOW-ANGEL".to_string()));
    assert!(hand.contains(&"PURPLE-FALLEN".to_string()));
    assert!(!hand.contains(&"REMAINDER".to_string()));
    assert!(trash.contains(&"DISCARD".to_string()));
}

#[test]
fn st10_12_inherited_grants_retaliation_to_own_yellow_digimon_on_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST10-12")
        .expect("ST10-12 YAML loads")
        .add_card(digimon("YELLOW-CARRIER", CardColor::Yellow, "Angel"))
        .add_card(digimon("PURPLE-CARRIER", CardColor::Purple, "Fallen Angel"))
        .start();

    let yellow = runner.place_stack(0, &["ST10-12", "YELLOW-CARRIER"]);
    let purple = runner.place_stack(0, &["ST10-12", "PURPLE-CARRIER"]);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(yellow, Keyword::Retaliation));
    assert!(!runner.game.has_keyword(purple, Keyword::Retaliation));

    runner.end_turn();
    runner.game.tick_declarative_effects();
    assert!(
        !runner.game.has_keyword(yellow, Keyword::Retaliation),
        "[Your Turn] inherited aura must switch off on the opponent's turn"
    );
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

fn digimon(id: &str, color: CardColor, trait_name: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.traits = vec![trait_name.to_string()];
    card.level = Some(4);
    card.dp = Some(4000);
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
    let view = runner
        .pending_selection_view()
        .expect("reveal bucket prompt");
    assert!(view.valid_action_ids.contains(&action));
    assert_mask_allows(runner, view.selecting_player, action);
    runner
        .execute_action(0, action)
        .expect("pick revealed card");
}

fn assert_mask_allows(runner: &DebugRunner, player: u8, action: u16) {
    assert_eq!(
        build_action_mask(&runner.game, player)[action as usize],
        1.0
    );
}

fn zone_ids(cards: &[digimon_engine::card_source::CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}
