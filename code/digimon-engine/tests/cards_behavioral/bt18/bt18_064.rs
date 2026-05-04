//! BT18-064 Mercurymon
//!
//! Printed text:
//! - [On Play] [When Digivolving] Until the end of your opponent's turn, this
//!   Digimon can't be returned to the hand or deck by your opponent's effects.
//! - Inherited: [Opponent's Turn] This Digimon gets +2000 DP.

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::ModifierType;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT18-064")
        .expect("BT18-064 YAML parses and compiles")
        .build()
}

#[test]
fn bt18_064_has_inherited_opponent_turn_dp_aura() {
    let runner = runner();
    let card = runner
        .compiled_card("BT18-064")
        .expect("BT18-064 compiled card present");

    let dp_modifier = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope: CompiledScope::Inherited,
            dp_modifier,
            ..
        }) => *dp_modifier,
        _ => None,
    });

    assert_eq!(dp_modifier, Some(2000));
}

#[test]
fn bt18_064_on_play_grants_return_to_hand_and_deck_immunity_only() {
    let mut runner = runner();
    let handle = runner.place_on_field(0, "BT18-064", None);
    runner.fire_on_play(0, handle.index as usize);

    assert!(runner
        .game
        .modifiers
        .has(handle, ModifierType::CannotBeReturnedToHand));
    assert!(runner
        .game
        .modifiers
        .has(handle, ModifierType::CannotBeReturnedToDeck));
    assert!(
        !runner
            .game
            .modifiers
            .has(handle, ModifierType::CannotBeDeDigivolved),
        "Mercurymon protects against hand/deck return, not De-Digivolve"
    );
}

#[test]
fn bt18_064_inherited_dp_applies_on_opponents_turn_only() {
    let mut host = make_test_card("BT18-HOST", "Mercurymon Host");
    host.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-064")
        .expect("BT18-064 YAML parses and compiles")
        .add_card(host)
        .build();
    let carrier = runner.place_stack(0, &["BT18-064", "BT18-HOST"]);

    assert_eq!(
        runner.game.source_dp_contribution(carrier, 0),
        0,
        "inherited [Opponent's Turn] DP bonus must not apply on your turn"
    );

    runner.end_turn();
    assert_eq!(
        runner.game.source_dp_contribution(carrier, 0),
        2000,
        "inherited [Opponent's Turn] DP bonus applies during opponent's turn"
    );
}
