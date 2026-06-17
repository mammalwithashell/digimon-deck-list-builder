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

/// Insert a digivolution source under the breeding permanent (King Drasil),
/// so `per: material_count` in the cost-reduction formula has something to count.
fn add_breeding_source(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    let source = digimon_engine::card_source::CardSource::new(data_idx, player, card_index);
    runner.game.players[player as usize]
        .breeding_area
        .as_mut()
        .expect("King Drasil in breeding")
        .card_sources
        .insert(0, source);
}

fn playable_royal_knight(id: &str, play_cost: u16) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(7);
    card.dp = Some(15000);
    card.play_cost = play_cost;
    card.traits = vec!["Royal Knight".to_string()];
    card
}

/// The SHIPPED (embedded-pack) BT13-007 reduces a Royal Knight's play cost by
/// 4 + 1 per digivolution card under King Drasil, via a structured formula
/// (`amount_fn: { base: 4, per: material_count, delta: 1 }`) — no raw_rust.
/// With 2 sources under King Drasil, a cost-15 Royal Knight is reduced by 6.
/// (Behavioral parity vs the former raw_rust amount_fn is cross-checked in
/// cost_hooks::drasil_production_structured_formula_matches_raw_rust.)
#[test]
fn bt13_007_cost_reduction_counts_sources_under_king_drasil() {
    let mut rules = digimon_engine::rules::Rules::standard();
    rules.memory_range = (-20, 20);
    let mut runner = DebugRunner::builder()
        .with_rules(rules)
        .dsl_card("BT13-007")
        .expect("BT13-007 must load from embedded DSL pack")
        .add_card(playable_royal_knight("RK-PLAY", 15))
        .add_card(digi_egg("SRC-A"))
        .add_card(digi_egg("SRC-B"))
        .hand(0, &["RK-PLAY"])
        .memory(20)
        .start();

    runner.place_in_breeding(0, "BT13-007");
    add_breeding_source(&mut runner, 0, "SRC-A");
    add_breeding_source(&mut runner, 0, "SRC-B");
    runner.game.enter_main_phase();

    // The optional Royal-Knight reducer parks for an accept/decline choice.
    assert!(
        runner.play(0, 0).is_none(),
        "optional cost reducer should park instead of resolving immediately"
    );
    assert!(
        runner.pending_selection().is_some(),
        "accept/decline of the reduction is exposed as a selection"
    );
    runner.execute_branch(0).expect("accept the cost reduction");

    // 15 - (4 base + 2 digivolution cards) = 9 paid; 20 - 9 = 11.
    assert_eq!(
        runner.memory(),
        11,
        "Royal Knight cost reduced by 4 + 2 sources under King Drasil"
    );
}
