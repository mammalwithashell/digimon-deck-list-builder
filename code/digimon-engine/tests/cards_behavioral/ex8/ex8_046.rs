//! EX8-046 Gotsumon
//!
//! Printed text:
//! - [On Deletion] By trashing 1 [Mineral]/[Rock] card in hand, Draw 2.
//! - Inherited: <Blocker>.

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX8-046")
        .expect("EX8-046 YAML parses and compiles")
        .build()
}

#[test]
fn ex8_046_has_on_deletion_and_inherited_blocker_clauses() {
    let runner = runner();
    let card = runner
        .compiled_card("EX8-046")
        .expect("EX8-046 compiled card present");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(triggered)
            if triggered.when == vec![CompiledTiming::OnDeletion]
    )));
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            scope: CompiledScope::Inherited,
            keyword,
            ..
        }) if keyword == "Blocker"
    )));
}

#[test]
fn ex8_046_inherited_blocker_is_available_from_source() {
    let mut host = make_test_card("HOST", "Host");
    host.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-046")
        .expect("EX8-046 YAML parses and compiles")
        .add_card(host)
        .build();
    let carrier = runner.place_stack(0, &["EX8-046", "HOST"]);

    assert!(
        runner.game.has_keyword(carrier, Keyword::Blocker),
        "EX8-046 inherited text grants Blocker to the carrier"
    );
}

#[test]
fn ex8_046_on_deletion_trashes_mineral_or_rock_from_hand_and_draws_two() {
    let mut mineral = make_test_card("MINERAL-HAND", "Mineral Hand");
    mineral.traits.push("Mineral".to_string());
    let filler = make_test_card("FILLER", "Filler");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-046")
        .expect("EX8-046 YAML parses and compiles")
        .add_card(mineral)
        .add_card(filler)
        .hand(0, &["MINERAL-HAND"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .build();
    let gotsu = runner.place_on_field(0, "EX8-046", None);
    let deck_before = runner.deck_size(0);

    runner.game.delete_permanent_with_effects(gotsu);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    assert!(runner.pending_is_optional());
    runner
        .auto_resolve()
        .expect("EX8-046 hand-trash selection resolves");

    assert_eq!(runner.deck_size(0), deck_before - 2);
    assert_eq!(runner.trash_size(0), 2, "deleted Gotsumon plus hand cost");
}
