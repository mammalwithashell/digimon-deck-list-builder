//! BT23-064 Bakemon
//!
//! Implemented slice:
//! - [On Play] [When Digivolving] By deleting 1 of your Digimon, delete
//!   1 opponent level 4 or lower Digimon.
//! - Inherited [On Deletion] Gain 1 memory.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT23-064")
        .expect("BT23-064 must load from embedded DSL pack")
        .memory(5)
        .start()
}

#[test]
fn bt23_064_has_play_digivolve_and_inherited_deletion_clauses() {
    let runner = runner();
    let card = runner
        .compiled_card("BT23-064")
        .expect("BT23-064 must be compiled");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
        )),
        "BT23-064 must have a shared OnPlay/WhenDigivolving delete-cost clause"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnDeletion)
        )),
        "BT23-064 must have inherited OnDeletion gain-memory clause"
    );
}

#[test]
fn bt23_064_on_play_deletes_own_digimon_then_level4_opponent() {
    let mut ally = make_test_card("BT23-064-ALLY", "Ally");
    ally.level = Some(3);
    let mut opp = make_test_card("BT23-064-OPP", "Opponent");
    opp.level = Some(4);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-064")
        .expect("BT23-064 must load")
        .add_card(ally)
        .add_card(opp)
        .hand(0, &["BT23-064"])
        .memory(5)
        .start();

    runner.place_on_field(0, "BT23-064-ALLY", None);
    runner.place_on_field(1, "BT23-064-OPP", None);
    runner.play(0, 0);
    runner.auto_resolve();

    assert!(
        runner.game.players[1].battle_area.is_empty(),
        "BT23-064 should delete the selected level 4 or lower opponent Digimon"
    );
    assert_eq!(
        runner.game.players[0].trash.len(),
        1,
        "BT23-064 should pay by deleting one of your Digimon"
    );
}

#[test]
fn bt23_064_inherited_on_deletion_gains_memory() {
    let carrier = make_test_card("BT23-064-CARRIER", "Carrier");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-064")
        .expect("BT23-064 must load")
        .add_card(carrier)
        .memory(0)
        .start();

    let handle = runner.place_on_field(0, "BT23-064-CARRIER", None);
    runner.push_source(handle, "BT23-064");
    runner.game.delete_permanent_with_effects(handle);
    runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        1,
        "inherited OnDeletion should gain 1 memory"
    );
}
