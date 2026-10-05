//! BT9-047 Pomumon - Digimon, Lv.3, Green, DP 2000, Cost 3.
//! Traits: Vegetation. Attribute: Data.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT9-047.md)
//!
//! ```text
//! [All Turns] Players can't play Digimon by effects.
//! ```
//!
//! No inherited or security text. Evolves from green Lv.2 for 0. Reprint of the BT9-047 / BT14-009 text; no per-card DCGO file.
//!
//! # Patterns this test covers
//! - D6 player-scoped flood gate: `CannotPlayDigimonByEffect`
//! - `target_player: any` applies the restriction to both players while Pomumon
//!   is face up.
//! - Resolver source discrimination: effect-initiated Digimon plays are blocked,
//!   normal hand plays are still legal.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledPlayerRef,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CostDelta, ModifierType, PlaySource};

fn runner_with_pomumon() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT9-047")
        .expect("BT9-047 in embedded DSL pack")
        .add_card(make_test_card("DIG-A", "Effect Play Target A"))
        .add_card(make_test_card("DIG-B", "Effect Play Target B"))
        .hand(0, &["DIG-A"])
        .hand(1, &["DIG-B"])
        .memory(10)
        .start()
}

#[test]
fn bt9_047_metadata_and_evolution_match_printed_card() {
    let runner = runner_with_pomumon();
    let card = runner
        .compiled_card("BT9-047")
        .expect("BT9-047 in embedded pack");

    assert_eq!(card.name, "Pomumon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(2000));
    assert_eq!(card.traits, vec!["Vegetation"]);
    assert_eq!(card.attribute.as_deref(), Some("Data"));
    assert_eq!(card.alt_paths.len(), 1, "green Lv.2 digivolve path");
    assert_eq!(card.alt_paths[0].cost, Some(CompiledCost::Literal(0)));
}

#[test]
fn bt9_047_has_all_players_cannot_play_digimon_by_effect_floodgate() {
    let runner = runner_with_pomumon();
    let card = runner.compiled_card("BT9-047").expect("BT9-047 in pack");

    let floodgate = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
                modifier,
                target_player,
                ..
            }) => Some((modifier, target_player)),
            _ => None,
        })
        .expect("BT9-047 must compile one declarative flood_gate");

    assert_eq!(floodgate.0, "CannotPlayDigimonByEffect");
    assert_eq!(
        *floodgate.1,
        Some(CompiledPlayerRef::Any),
        "Pomumon says players, so both players must be targeted"
    );
}

#[test]
fn bt9_047_blocks_both_players_effect_playing_digimon_but_not_normal_play() {
    let mut runner = runner_with_pomumon();
    runner.place_on_field(0, "BT9-047", Some(0));
    runner.game.tick_declarative_effects();

    assert!(
        runner
            .game
            .modifiers
            .player_has(0, ModifierType::CannotPlayDigimonByEffect),
        "Pomumon must restrict its controller too"
    );
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotPlayDigimonByEffect),
        "Pomumon must restrict the opponent too"
    );

    let p0_effect_play =
        runner
            .game
            .play_from_hand_with_cost(0, 0, CostDelta::Free, PlaySource::ByEffect);
    assert!(
        p0_effect_play.is_none(),
        "player 0 effect-initiated Digimon play must be blocked"
    );
    assert_eq!(runner.game.player(0).hand.len(), 1);

    let p1_effect_play =
        runner
            .game
            .play_from_hand_with_cost(1, 0, CostDelta::Free, PlaySource::ByEffect);
    assert!(
        p1_effect_play.is_none(),
        "player 1 effect-initiated Digimon play must be blocked"
    );
    assert_eq!(runner.game.player(1).hand.len(), 1);

    let normal_play =
        runner
            .game
            .play_from_hand_with_cost(0, 0, CostDelta::Free, PlaySource::ByHand);
    assert!(
        normal_play.is_some(),
        "normal hand play must remain legal under CannotPlayDigimonByEffect"
    );
    assert_eq!(runner.game.player(0).hand.len(), 0);
}
