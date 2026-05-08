use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt13_019_has_blocker_keyword() {
    let runner = DebugRunner::builder()
        .dsl_card("BT13-019")
        .expect("load")
        .start();
    let card = runner.compiled_card("BT13-019").expect("compiled card");
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { .. })
    )));
}

#[ignore = "pending: G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY — play Sistermon from trash or Royal Knight from breeding sources, excluding names"]
#[test]
fn bt13_019_plays_sistermon_or_royal_knight_from_breeding_sources() {}
