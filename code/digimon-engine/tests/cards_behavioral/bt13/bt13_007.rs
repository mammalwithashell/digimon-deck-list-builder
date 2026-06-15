//! BT13-007 King Drasil_7D6
//!
//! Implemented slice:
//! - Breeding floodgate that prevents your Digimon from digivolving (gated to
//!   your own turn — opponent-turn Blast/Counter digivolves are not blocked).
//! - Breeding Royal Knight play cost reduction: 4 + this Digimon's digivolution
//!   card count, via a native `base_per_delta` formula.
//! - Start of main phase places the top Digi-Egg and Royal Knights under self.
//! - Inherited breeding observer gains memory when Royal Knight Options are placed.

use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, ModifierType};

fn royal_knight(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.traits = vec!["Royal Knight".to_string()];
    card
}

/// A playable Royal Knight Digimon used as the cost-reduction target. Carries a
/// non-trivial play cost so the reduction is observable as a memory delta.
fn royal_knight_digimon(id: &str, play_cost: u16) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(6);
    card.dp = Some(11000);
    card.play_cost = play_cost;
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

/// Insert a card directly as a bottom digivolution source under the breeding-area
/// King Drasil. Mirrors `cost_hooks::stacked_would_play_reducers::add_breeding_source`.
fn add_breeding_source(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("add_breeding_source: unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    let source = CardSource::new(data_idx, player, card_index);
    let breeding = runner.game.players[player as usize]
        .breeding_area
        .as_mut()
        .expect("breeding permanent exists");
    // Insert beneath King Drasil's top card so it counts as a digivolution card.
    breeding.card_sources.insert(0, source);
}

/// Build a `CardSource` for `card_id` owned by `player` (for direct placement).
fn make_source(runner: &mut DebugRunner, player: u8, card_id: &str) -> CardSource {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("make_source: unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    CardSource::new(data_idx, player, card_index)
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

/// Build a runner with King Drasil in breeding carrying `n_sources` digivolution
/// cards, plus one playable Royal Knight (`RK-PLAY`, cost 11) in player 0's hand.
fn cost_reduction_runner(n_sources: usize) -> DebugRunner {
    let mut builder = DebugRunner::builder()
        .dsl_card("BT13-007")
        .expect("BT13-007 must load from embedded DSL pack")
        .add_card(royal_knight_digimon("RK-PLAY", 11));
    for i in 0..n_sources {
        builder = builder.add_card(royal_knight(&format!("SRC-{i}")));
    }
    let mut runner = builder.hand(0, &["RK-PLAY"]).memory(20).start();

    runner.place_in_breeding(0, "BT13-007");
    for i in 0..n_sources {
        add_breeding_source(&mut runner, 0, &format!("SRC-{i}"));
    }
    runner.game.enter_main_phase();
    runner
}

/// [Breeding][Your Turn][OPT] "Royal Knight play cost −4, and −1 more per this
/// Digimon's digivolution card."
///
/// With N digivolution cards under King Drasil and a cost-11 Royal Knight in hand,
/// accepting the optional reducer should pay `11 - (4 + N)` from memory.
///
/// Sources: BT13-007 card text; DCGO `BT13_007.cs` (reduction = DigivolutionCards.Count + 4).
fn assert_cost_reduction_for_sources(n_sources: usize) {
    let expected_reduction = 4 + n_sources as i16;
    let mut runner = cost_reduction_runner(n_sources);

    let start_memory = runner.memory();
    assert_eq!(start_memory, 20, "start at memory 20");

    // Playing the Royal Knight should park behind the optional reducer choice
    // (the [OPT] "you may reduce") before any memory is paid.
    let played = runner.play(0, 0);
    assert!(
        played.is_none(),
        "play should park behind the optional cost-reduction choice ({n_sources} sources)"
    );
    assert_eq!(
        runner.memory(),
        20,
        "no memory paid before the reducer choice is resolved"
    );
    let pending = runner
        .pending_selection()
        .expect("optional cost reducer must surface a pending selection");
    let choices = pending
        .effect_choices
        .as_ref()
        .expect("optional reducer exposes accept/decline choices");
    assert!(
        choices.len() >= 2,
        "optional reducer must expose an accept and a decline (PASS) branch"
    );

    // Accept the reduction (branch 0).
    runner.execute_branch(0).expect("accept King Drasil reducer");
    runner.auto_resolve().ok();

    let paid = start_memory - runner.memory();
    let expected_paid = 11 - expected_reduction;
    assert_eq!(
        paid, expected_paid,
        "with {n_sources} sources the reduction must be 4 + {n_sources} = {expected_reduction}; \
         expected to pay {expected_paid} of an 11-cost Royal Knight, paid {paid}"
    );
    assert!(
        runner
            .game
            .players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "RK-PLAY"),
        "the Royal Knight should resolve into the battle area"
    );
}

#[test]
fn bt13_007_cost_reduction_counts_sources_under_king_drasil_two_sources() {
    // 2 digivolution cards → reduction of 4 + 2 = 6.
    assert_cost_reduction_for_sources(2);
}

#[test]
fn bt13_007_cost_reduction_counts_sources_under_king_drasil_four_sources() {
    // 4 digivolution cards → reduction of 4 + 4 = 8.
    assert_cost_reduction_for_sources(4);
}

/// [Breeding][Your Turn] "All of your Digimon can't digivolve."
///
/// The floodgate must be gated on the owner's turn as well as in_breeding — on the
/// opponent's turn (e.g. a Blast/Counter digivolve) it must NOT block.
///
/// Sources: BT13-007 card text ("[Your Turn]"); DCGO `BT13_007.cs` gates the
/// CannotDigivolve floodgate on the owner's turn.
#[test]
fn bt13_007_floodgate_blocks_own_digivolve_only_on_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-007")
        .expect("BT13-007 must load from embedded DSL pack")
        .add_card(royal_knight("OWN-DIGIMON"))
        .start();

    runner.place_in_breeding(0, "BT13-007");
    let own = runner.place_on_field(0, "OWN-DIGIMON", Some(0));

    // On player 0's turn the floodgate is active → CannotDigivolve installed.
    runner.game.turn_player_idx = 0;
    runner.game.tick_declarative_effects();
    assert!(
        runner.game.modifiers.has(own, ModifierType::CannotDigivolve),
        "on your turn, King Drasil's floodgate must block your Digimon from digivolving"
    );

    // On the opponent's turn the floodgate is inactive → modifier cleared, so a
    // Blast/Counter digivolve is not blocked.
    runner.game.turn_player_idx = 1;
    runner.game.tick_declarative_effects();
    assert!(
        !runner.game.modifiers.has(own, ModifierType::CannotDigivolve),
        "on the opponent's turn the [Your Turn] floodgate must NOT block your Digimon \
         (Blast/Counter digivolves remain legal)"
    );
}

/// [Breeding][Inherited][OPT] "When a [Royal Knight] trait Option card is placed in
/// the battle area, gain 1 memory."
///
/// King Drasil must have a Royal Knight Option source beneath it so the inherited
/// observer is live. Placing a Royal Knight Option grants 1 memory; placing a
/// non-Royal-Knight Option does not.
///
/// Sources: BT13-007 inherited text; DCGO `BT13_007.cs`.
#[test]
fn bt13_007_inherited_gains_memory_when_royal_knight_option_placed() {
    let mut rk_option = make_test_card("RK-OPTION", "RK-OPTION");
    rk_option.card_kind = CardKind::Option;
    rk_option.traits = vec!["Royal Knight".to_string()];

    let mut plain_option = make_test_card("PLAIN-OPTION", "PLAIN-OPTION");
    plain_option.card_kind = CardKind::Option;

    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-007")
        .expect("BT13-007 must load from embedded DSL pack")
        // The inherited effect lives on a source UNDER King Drasil, so put a
        // Royal Knight beneath it to source the observer.
        .add_card(royal_knight("RK-SRC"))
        .add_card(rk_option)
        .add_card(plain_option)
        .memory(5)
        .start();

    runner.place_in_breeding(0, "BT13-007");
    add_breeding_source(&mut runner, 0, "RK-SRC");
    runner.game.turn_player_idx = 0;
    runner.game.enter_main_phase();

    // Placing a non-Royal-Knight Option must NOT gain memory.
    let before_plain = runner.memory();
    let plain = make_source(&mut runner, 0, "PLAIN-OPTION");
    runner.game.install_field_option_as_ordinary(plain, 0);
    runner.auto_resolve().ok();
    assert_eq!(
        runner.memory(),
        before_plain,
        "placing a non-Royal-Knight Option must not gain memory"
    );

    // Placing a Royal Knight Option must gain exactly 1 memory.
    let before_rk = runner.memory();
    let rk = make_source(&mut runner, 0, "RK-OPTION");
    runner.game.install_field_option_as_ordinary(rk, 0);
    runner.auto_resolve().ok();
    assert_eq!(
        runner.memory(),
        before_rk + 1,
        "placing a Royal Knight Option must gain 1 memory via the inherited observer"
    );
}
