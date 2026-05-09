//! BT22-052 Leopardmon

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::replacement::ReplacementCause;

#[test]
fn bt22_052_loads_play_and_blocker_slice() {
    DebugRunner::builder()
        .dsl_card("BT22-052")
        .expect("BT22-052 must load from embedded DSL pack")
        .start();
}

#[test]
fn bt22_052_has_blast_digivolve_marker_and_keyword() {
    let runner = DebugRunner::builder()
        .dsl_card("BT22-052")
        .expect("BT22-052 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("BT22-052").expect("compiled card");

    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::BurstDigivolve
            && path.marker
            && path.cost == Some(CompiledCost::Literal(0))
    }));
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
            if keyword == "BlastDigivolve"
    )));
}

#[test]
fn bt22_052_other_digimon_would_leave_gains_memory_and_event_proceeds() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-052")
        .expect("BT22-052 must load from embedded DSL pack")
        .add_card(digimon("ALLY-A", "Ally A"))
        .add_card(digimon("ALLY-B", "Ally B"))
        .memory(0)
        .start();
    let _leopardmon = runner.place_on_field(0, "BT22-052", Some(0));
    let ally_a = runner.place_on_field(0, "ALLY-A", Some(0));
    let ally_b = runner.place_on_field(0, "ALLY-B", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ally_a, ReplacementCause::OpponentEffect);
    runner
        .auto_resolve()
        .expect("resolve non-cancelling would-leave subscriber");

    assert_eq!(
        runner.memory(), 2,
        "Leopardmon gains 2 memory when another Digimon would leave"
    );
    assert!(
        runner
            .game
            .players[0]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "ALLY-A"),
        "the original leave event still proceeds"
    );

    runner
        .game
        .delete_permanent_with_cause(ally_b, ReplacementCause::OpponentEffect);
    runner
        .auto_resolve()
        .expect("resolve second leave attempt");
    assert_eq!(
        runner.memory(), 2,
        "the once-per-turn subscriber does not gain memory twice"
    );
}

fn digimon(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card
}
