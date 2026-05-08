//! BT23-058 Craniamon - Digimon, Lv.6, White.
//!
//! Supported slice:
//! - Printed metadata and digivolve routes.
//! - <Reboot> and <Blocker>.
//! - [All Turns] optional suspend-self replacement preventing one of your
//!   Digimon/Tamers from leaving by an opponent's effect.
//!
//! Gap-routed:
//! - [All Turns][OPT] when this Digimon suspends, delete all opponent Digimon
//!   with the lowest play cost needs a self-scoped on_suspend predicate plus
//!   aggregate lowest-play-cost deletion.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause,
};
use digimon_engine::debug_runner::DebugRunner;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT23-058")
        .expect("BT23-058 YAML loads")
        .memory(10)
        .start()
}

#[test]
fn bt23_058_has_printed_metadata_routes_reboot_and_blocker() {
    let runner = runner();
    let card = runner
        .compiled_card("BT23-058")
        .expect("BT23-058 compiled card present");

    assert_eq!(card.name, "Craniamon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(11));
    assert_eq!(card.dp, Some(11000));
    assert_eq!(card.color, vec![CompiledColor::White]);
    assert!(card.traits.iter().any(|name| name == "Royal Knight"));
    assert!(card.traits.iter().any(|name| name == "CS"));
    assert_eq!(card.attribute.as_deref(), Some("Data"));

    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(3))
            && path.from.as_ref().is_some_and(|from| {
                from.level_eq == Some(5) && from.color_is == Some(CompiledColor::White)
            })
    }));
    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(3))
            && path.from.as_ref().is_some_and(|from| {
                from.level_eq == Some(5) && from.trait_has.as_deref() == Some("CS")
            })
    }));

    for expected in ["Reboot", "Blocker"] {
        assert!(card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) if keyword == expected
        )));
    }
}

#[test]
fn bt23_058_has_optional_leave_prevention_replacement() {
    let runner = runner();
    let card = runner
        .compiled_card("BT23-058")
        .expect("BT23-058 compiled card present");

    let replacement = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                optional,
                summary,
                ..
            }) => Some((*optional, summary.as_deref().unwrap_or(""))),
            _ => None,
        })
        .expect("leave-prevention replacement exists");

    assert!(replacement.0, "printed 'by suspending' cost is opt-in");
    assert!(
        replacement.1.contains("doesn't leave"),
        "replacement summary should document cancellation"
    );
}

#[test]
#[ignore = "pending: G-SELF-ON-SUSPEND plus G-PLAY-COST-AGGREGATE — self-scoped on_suspend and lowest play-cost delete-all"]
fn bt23_058_when_this_suspends_deletes_all_opponent_lowest_play_cost_digimon() {
    panic!("requires self-only on_suspend predicate plus aggregate lowest play-cost deletion");
}
