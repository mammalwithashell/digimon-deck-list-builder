//! BT23-047 Examon

use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt23_047_loads_keyword_slice() {
    DebugRunner::builder()
        .dsl_card("BT23-047")
        .expect("BT23-047 must load from embedded DSL pack")
        .start();
}

#[test]
fn bt23_047_has_partition_green_lv5_and_blue_lv5() {
    let runner = DebugRunner::builder()
        .dsl_card("BT23-047")
        .expect("BT23-047 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("BT23-047").expect("compiled card");

    let partition = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::Partition {
            scope: CompiledScope::FaceUp,
            sources,
            exclude_cause,
            ..
        }) => Some((sources, exclude_cause)),
        _ => None,
    });
    let (sources, exclude_cause) = partition.expect("BT23-047 Partition clause");

    assert_eq!(sources.len(), 2);
    assert!(sources
        .iter()
        .any(|p| p.level_eq == Some(5) && p.color_is == Some(CompiledColor::Green)));
    assert!(sources
        .iter()
        .any(|p| p.level_eq == Some(5) && p.color_is == Some(CompiledColor::Blue)));
    assert!(exclude_cause.iter().any(|cause| cause == "own_effect"));
    assert!(exclude_cause.iter().any(|cause| cause == "battle"));
}

#[ignore = "pending: G-N-TARGET-SUSPEND-UNSUSPEND-LOCK and G-SECURITY-REMOVED-OPTION-TRASH — suspend 5, next unsuspend lock, may attack, and security-removed option trash/delete"]
#[test]
fn bt23_047_suspend_lock_and_security_removed_tail() {}
