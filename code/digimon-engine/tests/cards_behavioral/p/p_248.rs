//! P-248 Veemon: checked P-248.webp and DCGO P/Blue/P_248.cs.
//! [Start of Your Main Phase] By trashing 1 card with [Veedramon] in its text or the [Armor Form] or [Free] trait
//! from your hand, <Draw 1> and gain 1 memory. (DCGO: SelectHandEffect, canNoSelect: true; the reward only
//! follows a trashed card.)
//! Inherited: [Your Turn] This Digimon gets +2000 DP.
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "P-248";

fn digimon(id: &str, traits: &[&str], text: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c.effect_text = text.into();
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-248 YAML loads")
        .add_card(digimon("FREE", &["Free"], ""))
        .add_card(digimon("ARMOR", &["Armor Form"], ""))
        .add_card(digimon("VTEXT", &[], "Digivolve from [Veedramon]: Cost 3"))
        .add_card(digimon("PLAIN", &["Dragon"], ""))
        .add_card(digimon("DRAW", &[], ""))
        .add_card(digimon("CARRIER", &[], ""))
        .add_card(digimon("SEC", &[], ""))
}

fn hand_has(runner: &DebugRunner, id: &str) -> bool {
    runner.game.players[0].hand.iter().any(|c| c.card_id(&runner.game.card_data) == id)
}

fn trash_has(runner: &DebugRunner, id: &str) -> bool {
    runner.game.players[0].trash.iter().any(|c| c.card_id(&runner.game.card_data) == id)
}

fn pick_hand(runner: &mut DebugRunner, id: &str) {
    let slot = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .expect("card in hand");
    runner.game.decode_action(digimon_engine::action::space::HAND_EFFECT_START + slot as u16, 0);
}

fn start_main_trashes(cost_card: &str) {
    let mut runner = base().hand(0, &[cost_card, "PLAIN"]).deck(0, &["DRAW"]).memory(2).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let memory_before = runner.memory();
    let deck_before = runner.deck_size(0);

    runner.game.enter_main_phase();
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand), "an eligible hand card opens the trash prompt");
    pick_hand(&mut runner, cost_card);
    runner.auto_resolve().expect("finish the start-of-main effect");

    assert!(trash_has(&runner, cost_card), "{cost_card} is trashed");
    assert!(hand_has(&runner, "PLAIN"));
    assert!(hand_has(&runner, "DRAW"), "draw 1");
    assert_eq!(runner.deck_size(0), deck_before - 1);
    assert_eq!(runner.memory(), memory_before + 1, "gain 1 memory");
}

#[test]
fn p_248_start_main_trashes_free_trait_card_to_draw_and_gain_memory() {
    start_main_trashes("FREE");
}

#[test]
fn p_248_start_main_accepts_armor_form_trait() {
    start_main_trashes("ARMOR");
}

#[test]
fn p_248_start_main_accepts_veedramon_in_text() {
    start_main_trashes("VTEXT");
}

#[test]
fn p_248_start_main_declining_gives_nothing() {
    let mut runner = base().hand(0, &["FREE"]).deck(0, &["DRAW"]).memory(2).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let memory_before = runner.memory();
    runner.game.enter_main_phase();
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    runner.game.decode_action(digimon_engine::action::space::PASS, 0);
    runner.auto_resolve().expect("finish after declining");
    assert!(hand_has(&runner, "FREE"));
    assert!(!hand_has(&runner, "DRAW"));
    assert_eq!(runner.memory(), memory_before);
}

#[test]
fn p_248_start_main_no_prompt_without_an_eligible_card() {
    let mut runner = base().hand(0, &["PLAIN"]).deck(0, &["DRAW"]).memory(2).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();
    assert_ne!(runner.pending_kind(), Some(SelectionKind::Hand), "no eligible card, no prompt");
    assert!(hand_has(&runner, "PLAIN"));
}

#[test]
fn p_248_inherited_gives_2000_dp_on_your_turn_only() {
    let mut runner = base().memory(2).start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert_eq!(runner.effective_dp(carrier).unwrap(), 4000, "your turn: 2000 + 2000");
    runner.pass_turn();
    assert_eq!(runner.effective_dp(carrier).unwrap(), 2000, "opponent's turn: no bonus");
}
