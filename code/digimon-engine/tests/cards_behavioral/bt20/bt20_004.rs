//! BT20-004 Pinamon — Digi-Egg, Lv.2, Green, Cost 0 (Bird, ACCEL).
//!
//! Printed (official Bandai DB bundle + DCGO BT20_004.cs):
//! Inherited Effect [Your Turn] [Once Per Turn] When any of your Digimon with
//! the [ACCEL] trait are played, this Digimon may digivolve into a Digimon card
//! with the [ACCEL] trait in the hand with the digivolution cost reduced by 2.
//!
//! The card had only a JSON stub before (hunt cluster
//! alt-digivolve-xros-req-cards); synthetic boards only.

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT20-004";

fn digimon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = 0;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn lv4(id: &str, traits: &[&str]) -> CardData {
    let mut c = digimon(id, 4, traits);
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Green as u8,
        level: 3,
        memory_cost: 3,
    }];
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT20-004 YAML loads from the embedded pack")
        .add_card(digimon("HOST", 3, &["Beast"]))
        .add_card(digimon("ACCEL-PLAYED", 3, &["ACCEL"]))
        .add_card(digimon("PLAIN-PLAYED", 3, &["Beast"]))
        .add_card(lv4("ACCEL4", &["ACCEL"]))
        .add_card(lv4("ACCEL4B", &["ACCEL"]))
        .add_card(lv4("PLAIN4", &["Beast"]))
        .add_card(digimon("F", 3, &["Beast"]))
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(5)
}

fn host_with_pinamon(r: &mut DebugRunner) -> PermanentHandle {
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    r.place_stack(0, &[ID, "HOST"])
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

fn accept_outer(r: &mut DebugRunner) {
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement), "outer yes/no");
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().expect("accept 'may digivolve'");
}

#[test]
fn bt20_004_playing_an_accel_digimon_offers_a_cost_minus_2_digivolve_into_an_accel_digimon() {
    let mut r = base().hand(0, &["ACCEL-PLAYED", "ACCEL4", "ACCEL4B", "PLAIN4"]).start();
    let host = host_with_pinamon(&mut r);
    r.play(0, 0).expect("play the ACCEL Digimon");
    accept_outer(&mut r);

    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    assert!(r.pending_is_optional(), "the pick is declinable");
    let view = r.pending_selection_view().unwrap();
    let picks: Vec<u16> = view.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(picks.len(), 2, "only [ACCEL] trait Digimon cards: {picks:?}");
    let before = r.memory();
    r.execute_action(0, picks[0]).unwrap();
    assert!(top_id(&r, host).starts_with("ACCEL4"), "got {}", top_id(&r, host));
    assert_eq!(before - r.memory(), 1, "printed cost 3 reduced by 2");
}

#[test]
fn bt20_004_a_non_accel_play_does_not_trigger() {
    let mut r = base().hand(0, &["PLAIN-PLAYED", "ACCEL4"]).start();
    let _host = host_with_pinamon(&mut r);
    r.play(0, 0).expect("play a non-ACCEL Digimon");
    assert!(r.pending_selection().is_none(), "needs an [ACCEL] trait Digimon");
}

#[test]
fn bt20_004_the_opponents_accel_play_does_not_trigger() {
    let mut r = base().hand(0, &["ACCEL4"]).start();
    let _host = host_with_pinamon(&mut r);
    // The opponent's ACCEL Digimon enters play (bundle fired as the real play
    // flow does): "any of YOUR Digimon" — it is not.
    let h = r.place_on_field(1, "ACCEL-PLAYED", Some(0));
    r.fire_play_event_triggers(1, h.index as usize, false, false);
    assert!(r.pending_selection().is_none());
}

#[test]
fn bt20_004_is_inactive_on_the_opponents_turn() {
    let mut r = base().hand(0, &["ACCEL4"]).start();
    let _host = host_with_pinamon(&mut r);
    r.game.turn_player_idx = 1;
    let h = r.place_on_field(0, "ACCEL-PLAYED", Some(0));
    r.fire_play_event_triggers(0, h.index as usize, false, false);
    assert!(r.pending_selection().is_none(), "[Your Turn] only");
}

#[test]
fn bt20_004_declining_the_yes_no_refunds_the_once_per_turn() {
    let mut r = base()
        .hand(0, &["ACCEL-PLAYED", "ACCEL-PLAYED", "ACCEL4"])
        .start();
    let _host = host_with_pinamon(&mut r);
    r.play(0, 0).expect("first ACCEL play");
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
    r.decline_optional_trigger().expect("decline");
    assert!(r.pending_selection().is_none());
    r.play(0, 0).expect("second ACCEL play");
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "declining the activation does not spend the [Once Per Turn]"
    );
}

#[test]
fn bt20_004_no_legal_digivolve_in_hand_means_no_trigger() {
    let mut r = base().hand(0, &["ACCEL-PLAYED", "PLAIN4"]).start();
    let _host = host_with_pinamon(&mut r);
    r.play(0, 0).expect("play");
    assert!(
        r.pending_selection().is_none(),
        "no [ACCEL] Digimon card in hand to digivolve into: no dead yes/no"
    );
}
