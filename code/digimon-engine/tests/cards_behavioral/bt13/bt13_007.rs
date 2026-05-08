//! BT13-007 King Drasil_7D6
//!
//! Implemented slice:
//! - Breeding floodgate that prevents your Digimon from digivolving.
//! - Breeding Royal Knight play cost reduction, via existing raw Rust gap stub.
//! - Start of main phase places the top Digi-Egg and Royal Knights under self.
//! - Inherited breeding observer gains memory when Royal Knight Options are placed.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

fn royal_knight(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.traits = vec!["Royal Knight".to_string()];
    card
}

fn digi_egg(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::DigiEgg;
    card.level = Some(2);
    card.dp = None;
    card
}

#[test]
fn bt13_007_loads_from_embedded_dsl_pack() {
    DebugRunner::builder()
        .dsl_card("BT13-007")
        .expect("BT13-007 must load from embedded DSL pack")
        .start();
}

#[test]
fn bt13_007_start_main_fires_from_breeding_and_tucks_digitama_plus_royal_knight() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-007")
        .expect("BT13-007 must load from embedded DSL pack")
        .add_card(digi_egg("NEXT-EGG"))
        .add_card(royal_knight("RK-DIGIMON"))
        .digitama(0, &["NEXT-EGG"])
        .start();

    runner.place_in_breeding(0, "BT13-007");
    runner.place_on_field(0, "RK-DIGIMON", Some(0));

    runner.game.enter_main_phase();
    runner
        .auto_resolve()
        .expect("BT13-007 start-main body should resolve");

    assert!(
        runner.game.player(0).battle_area.is_empty(),
        "the Royal Knight permanent should move from battle area under King Drasil"
    );
    assert!(
        runner.game.player(0).digitama_deck.is_empty(),
        "the revealed Digi-Egg should leave the digitama deck"
    );

    let breeding = runner
        .game
        .player(0)
        .breeding_area
        .as_ref()
        .expect("King Drasil remains in breeding");
    let stack_ids: Vec<_> = breeding
        .card_sources
        .iter()
        .map(|source| source.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(stack_ids.len(), 3);
    assert_eq!(
        stack_ids.last().map(String::as_str),
        Some("BT13-007"),
        "King Drasil should remain the top card in breeding"
    );
    assert!(
        stack_ids.iter().any(|id| id == "NEXT-EGG")
            && stack_ids.iter().any(|id| id == "RK-DIGIMON"),
        "revealed Digi-Egg and Royal Knight should both become sources under King Drasil"
    );
}

#[ignore = "pending: G-BREEDING-ROYAL-KNIGHT-COST-REDUCTION — existing example uses raw_rust amount_fn until formula can count this source's digivolution cards"]
#[test]
fn bt13_007_cost_reduction_counts_sources_under_king_drasil() {}
