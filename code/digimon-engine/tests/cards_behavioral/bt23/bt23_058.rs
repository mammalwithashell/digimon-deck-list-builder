//! BT23-058 Craniamon - Digimon, Lv.6, Black (official Bandai DB /
//! data/card_bundles/BT23-058.md).
//!
//! Supported slice:
//! - Printed metadata and digivolve routes.
//! - <Reboot> and <Blocker>.
//! - [All Turns] optional suspend-self replacement preventing one of your
//!   Digimon/Tamers from leaving by an opponent's effect.
//! - [All Turns][OPT] when this Digimon suspends, delete all opponent Digimon
//!   with the lowest play cost (self-scoped on_suspend + aggregate lowest
//!   play-cost delete-all — the BT9-112 DeathXmon Clause C precedent gated on
//!   the BT23-077 Sistermon Ciel `event_permanent_is_source` self-scope).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - F3 leave-prevention replacement (suspend-self cost)
//! - G3 self-scoped OnSuspend observer (`event_permanent_is_source`)
//! - lowest-play-cost delete-all via play_cost_lte aggregate (lowest_play_cost,
//!   scope: opponent)

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledPredicate, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT23-058")
        .expect("BT23-058 YAML loads")
        .memory(10)
        .start()
}

fn make_digimon(id: &str, play_cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = play_cost;
    card
}

fn predicate_has_event_permanent_is_source(predicate: &CompiledPredicate) -> bool {
    predicate.event_permanent_is_source == Some(true)
        || predicate
            .all_of
            .iter()
            .any(predicate_has_event_permanent_is_source)
        || predicate
            .any_of
            .iter()
            .any(predicate_has_event_permanent_is_source)
        || predicate
            .none_of
            .iter()
            .any(predicate_has_event_permanent_is_source)
        || predicate
            .not
            .as_ref()
            .is_some_and(|p| predicate_has_event_permanent_is_source(p))
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
    assert_eq!(card.color, vec![CompiledColor::Black]);
    assert!(card.traits.iter().any(|name| name == "Royal Knight"));
    assert!(card.traits.iter().any(|name| name == "CS"));
    assert_eq!(card.attribute.as_deref(), Some("Data"));

    // Printed circle: Black Lv.5 / cost 3 (official Bandai DB — the card was
    // previously mis-authored White).
    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(3))
            && path.from.as_ref().is_some_and(|from| {
                from.level_eq == Some(5) && from.color_is == Some(CompiledColor::Black)
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
fn bt23_058_has_self_scoped_on_suspend_delete_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("BT23-058")
        .expect("BT23-058 compiled card present");

    let on_suspend = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when == vec![CompiledTiming::OnSuspend] =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("self-suspend lowest-play-cost delete clause should be authored");
    assert!(
        on_suspend
            .condition
            .as_ref()
            .is_some_and(predicate_has_event_permanent_is_source),
        "OnSuspend must be gated to the suspending event permanent being this Craniamon"
    );
}

#[test]
fn bt23_058_self_suspend_deletes_all_lowest_play_cost_opponent_digimon() {
    // Opponent board: two cost-3 Digimon (the lowest) and one cost-5 Digimon.
    // Suspending THIS Craniamon deletes every cost-3 Digimon and leaves the
    // cost-5 one. No selection installs — "delete ALL with the lowest play
    // cost" is a deterministic sweep.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-058")
        .expect("BT23-058 YAML loads")
        .add_card(make_digimon("CHEAP-A", 3))
        .add_card(make_digimon("CHEAP-B", 3))
        .add_card(make_digimon("PRICEY", 5))
        .memory(10)
        .start();
    let craniamon = runner.place_on_field(0, "BT23-058", Some(0));
    runner.place_on_field(1, "CHEAP-A", Some(0));
    runner.place_on_field(1, "CHEAP-B", Some(0));
    runner.place_on_field(1, "PRICEY", Some(0));
    assert_eq!(runner.battle_area_size(1), 3);

    runner.game.suspend(craniamon);
    runner
        .auto_resolve()
        .expect("resolve lowest-play-cost sweep");

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "both cost-3 (lowest play cost) opponent Digimon are deleted; cost-5 remains"
    );
    assert_eq!(
        runner.game.players[1].trash.len(),
        2,
        "the two deleted Digimon move to their owner's trash"
    );
}

#[test]
fn bt23_058_other_permanent_suspending_does_not_fire_delete_clause() {
    // NEGATIVE: a different own permanent suspending must NOT trigger
    // Craniamon's self-scoped OnSuspend delete-all.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-058")
        .expect("BT23-058 YAML loads")
        .add_card(make_digimon("OWN-OTHER", 4))
        .add_card(make_digimon("CHEAP-A", 3))
        .memory(10)
        .start();
    let _craniamon = runner.place_on_field(0, "BT23-058", Some(0));
    let other = runner.place_on_field(0, "OWN-OTHER", Some(0));
    runner.place_on_field(1, "CHEAP-A", Some(0));

    runner.game.suspend(other);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "another permanent suspending must not delete the opponent's Digimon"
    );
}

#[test]
fn bt23_058_self_suspend_with_no_opponent_digimon_is_a_noop() {
    // NEGATIVE: Craniamon suspends but the opponent controls no Digimon, so the
    // lowest-play-cost sweep has nothing to delete and installs no selection.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-058")
        .expect("BT23-058 YAML loads")
        .memory(10)
        .start();
    let craniamon = runner.place_on_field(0, "BT23-058", Some(0));

    runner.game.suspend(craniamon);

    assert!(
        runner.pending_selection().is_none(),
        "self-suspend with no opponent Digimon installs no prompt"
    );
    assert_eq!(runner.battle_area_size(1), 0);
}
