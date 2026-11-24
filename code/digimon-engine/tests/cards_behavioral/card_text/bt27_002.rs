//! Source: `data/card_bundles/BT27-002.md` (printed text and official Q&A).
//!
//! Standard digivolve cost circles: (none — Digi-Egg)
//!
//! Inherited Effect:
//! [When Attacking] [Once Per Turn] You may play 1 level 4 or lower [Aquatic] or [DATA SQUAD] trait Digimon card from this Digimon's digivolution cards with the cost reduced by 1.
//!
//! Official Q&A:
//! Yes, you can play it.

use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::GamePhase;
use digimon_engine::selection::SelectionKind;

use super::common::{action, candidates};

const CARD: &str = "BT27-002";

fn mon(id: &str, level: u8, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = cost;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn runner() -> DebugRunner {
    let mut tamer = make_test_card("TEXT-TAMER-SQUAD", "Squad Tamer");
    tamer.card_kind = digimon_engine::enums::CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    tamer.traits = vec!["DATA SQUAD".into()];
    let mut r = DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(mon("TEXT-AQ4", 4, 5, &["Aquatic"]))
        .add_card(mon("TEXT-DS3", 3, 3, &["DATA SQUAD"]))
        .add_card(mon("TEXT-DS5", 5, 7, &["DATA SQUAD"]))
        .add_card(mon("TEXT-PLAIN4", 4, 5, &["Beast"]))
        .add_card(mon("TEXT-TOP", 5, 7, &[]))
        .add_card(tamer)
        .add_card(make_test_card("TEXT-FILLER", "Filler"))
        .deck(0, &["TEXT-FILLER"; 30])
        .deck(1, &["TEXT-FILLER"; 30])
        .security(1, &["TEXT-FILLER"; 5])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    r
}

fn attack_suspended(r: &mut DebugRunner, attacker: u8) {
    let target = r.place_on_field(1, "TEXT-FILLER", Some(0));
    r.game.players[1].battle_area[target.index as usize].is_suspended = true;
    action(r, 0, encode_attack(attacker as u16, target.index as u16));
}

fn stack(r: &mut DebugRunner) -> u8 {
    r.place_stack(
        0,
        &[CARD, "TEXT-AQ4", "TEXT-DS3", "TEXT-DS5", "TEXT-PLAIN4", "TEXT-TAMER-SQUAD", "TEXT-TOP"],
    )
    .index
}

fn on_field(r: &DebugRunner, id: &str) -> bool {
    r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == id)
}

#[test]
fn when_attacking_offers_only_level_four_or_lower_aquatic_or_data_squad_digimon_sources() {
    let mut r = runner();
    let attacker = stack(&mut r);
    attack_suspended(&mut r, attacker);
    r.assert_prompt(SelectionKind::Material, true, 0);
    assert_eq!(candidates(&r), vec!["TEXT-AQ4", "TEXT-DS3"]);
}

#[test]
fn when_attacking_plays_chosen_source_with_cost_reduced_by_one() {
    let mut r = runner();
    let attacker = stack(&mut r);
    attack_suspended(&mut r, attacker);
    let before = r.memory();
    r.choose_prompt_card("TEXT-AQ4", None).unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(before - r.memory(), 4);
    assert!(on_field(&r, "TEXT-AQ4"));
    let src = &r.game.players[0].battle_area[attacker as usize];
    assert!(!src
        .digivolution_cards()
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "TEXT-AQ4"));
}

#[test]
fn when_attacking_play_may_be_declined() {
    let mut r = runner();
    let attacker = stack(&mut r);
    attack_suspended(&mut r, attacker);
    let before = r.memory();
    r.decline_prompt().unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(before, r.memory());
    assert!(!on_field(&r, "TEXT-AQ4") && !on_field(&r, "TEXT-DS3"));
}

#[test]
fn when_attacking_without_eligible_sources_plays_nothing() {
    let mut r = runner();
    let attacker = r.place_stack(0, &[CARD, "TEXT-DS5", "TEXT-PLAIN4", "TEXT-TOP"]).index;
    attack_suspended(&mut r, attacker);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[0].battle_area.len(), 1);
}

#[test]
fn when_attacking_is_once_per_turn() {
    let mut r = runner();
    let attacker = stack(&mut r);
    attack_suspended(&mut r, attacker);
    r.choose_prompt_card("TEXT-DS3", None).unwrap();
    assert!(r.pending_selection().is_none());
    r.game.players[0].battle_area[attacker as usize].is_suspended = false;
    attack_suspended(&mut r, attacker);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.current_phase(), GamePhase::Main);
}
