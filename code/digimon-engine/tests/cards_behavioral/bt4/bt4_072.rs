//! BT4-072 Gogmamon
//!
//! This pass covers inherited [All Turns] +1000 DP. The face-up Digi-Burst
//! main effect is a later source-cost main-action pass.

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT4-072")
        .expect("BT4-072 YAML parses and compiles")
        .build()
}

#[test]
fn bt4_072_has_inherited_all_turns_dp_aura() {
    let runner = runner();
    let card = runner
        .compiled_card("BT4-072")
        .expect("BT4-072 compiled card present");

    let dp = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope: CompiledScope::Inherited,
            dp_modifier,
            ..
        }) => *dp_modifier,
        _ => None,
    });

    assert_eq!(dp, Some(1000));
}

#[test]
fn bt4_072_source_contributes_1000_dp_on_both_turns() {
    let mut host = make_test_card("HOST", "Host");
    host.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT4-072")
        .expect("BT4-072 YAML parses and compiles")
        .add_card(host)
        .build();
    let carrier = runner.place_stack(0, &["BT4-072", "HOST"]);

    assert_eq!(runner.game.source_dp_contribution(carrier, 0), 1000);
    runner.end_turn();
    assert_eq!(runner.game.source_dp_contribution(carrier, 0), 1000);
}
