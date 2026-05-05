//! BT9-033 Pillomon - Digimon, Lv.3, Yellow, DP 2000, Cost 3.
//! Traits: Mammal. Attribute: Vaccine.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [All Turns] Players can't play Digimon by effects.
//! ```
//!
//! No inherited or security text. Evolves from yellow Lv.2 for 0.
//!
//! # Patterns this test covers
//! - D6 player-scoped flood gate: `CannotPlayDigimonByEffect`
//! - `target_player: any` applies the restriction to both players while Pillomon
//!   is face up.
//! - Resolver source discrimination: effect-initiated Digimon plays are blocked,
//!   normal hand plays are still legal.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledPlayerRef,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CostDelta, ModifierType, PlaySource};

fn runner_with_pillomon() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT9-033")
        .expect("BT9-033 in embedded DSL pack")
        .add_card(make_test_card("DIG-A", "Effect Play Target A"))
        .add_card(make_test_card("DIG-B", "Effect Play Target B"))
        .hand(0, &["DIG-A"])
        .hand(1, &["DIG-B"])
        .memory(10)
        .start()
}

#[test]
fn bt9_033_metadata_and_evolution_match_printed_card() {
    let runner = runner_with_pillomon();
    let card = runner
        .compiled_card("BT9-033")
        .expect("BT9-033 in embedded pack");

    assert_eq!(card.name, "Pillomon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(2000));
    assert_eq!(card.traits, vec!["Mammal"]);
    assert_eq!(card.attribute.as_deref(), Some("Vaccine"));
    assert_eq!(card.alt_paths.len(), 1, "yellow Lv.2 digivolve path");
    assert_eq!(card.alt_paths[0].cost, Some(CompiledCost::Literal(0)));
}

#[test]
fn bt9_033_has_all_players_cannot_play_digimon_by_effect_floodgate() {
    let runner = runner_with_pillomon();
    let card = runner.compiled_card("BT9-033").expect("BT9-033 in pack");

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
        .expect("BT9-033 must compile one declarative flood_gate");

    assert_eq!(floodgate.0, "CannotPlayDigimonByEffect");
    assert_eq!(
        *floodgate.1,
        Some(CompiledPlayerRef::Any),
        "Pillomon says players, so both players must be targeted"
    );
}

#[test]
fn bt9_033_blocks_both_players_effect_playing_digimon_but_not_normal_play() {
    let mut runner = runner_with_pillomon();
    runner.place_on_field(0, "BT9-033", Some(0));
    runner.game.tick_declarative_effects();

    assert!(
        runner
            .game
            .modifiers
            .player_has(0, ModifierType::CannotPlayDigimonByEffect),
        "Pillomon must restrict its controller too"
    );
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotPlayDigimonByEffect),
        "Pillomon must restrict the opponent too"
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
