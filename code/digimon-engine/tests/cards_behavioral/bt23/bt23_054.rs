//! BT23-054 Magnamon - Digimon, Lv.4, White/Blue.
//!
//! Supported slice:
//! - Printed metadata and digivolve routes.
//! - <Blocker>.
//! - [On Play][When Digivolving] Draw 1, then choose an own [Royal Knight]/[CS]
//!   Digimon to gain opponent-effect return-to-hand/deck protection.
//!
use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledPredicate, CompiledStep, CompiledTiming,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT23-054")
        .expect("BT23-054 YAML loads")
        .memory(10)
        .start()
}

fn predicate_has_level_eq(predicate: &CompiledPredicate, level: u8) -> bool {
    predicate.level_eq == Some(level)
        || predicate
            .any_of
            .iter()
            .any(|part| predicate_has_level_eq(part, level))
        || predicate
            .all_of
            .iter()
            .any(|part| predicate_has_level_eq(part, level))
}

fn predicate_contains_trait(predicate: &CompiledPredicate, needle: &str) -> bool {
    predicate.trait_has.as_deref() == Some(needle)
        || predicate
            .any_of
            .iter()
            .any(|part| predicate_contains_trait(part, needle))
        || predicate
            .all_of
            .iter()
            .any(|part| predicate_contains_trait(part, needle))
}

fn predicate_contains_name(predicate: &CompiledPredicate, needle: &str) -> bool {
    predicate.name_is.as_deref() == Some(needle)
        || predicate.name_contains.as_deref() == Some(needle)
        || predicate
            .any_of
            .iter()
            .any(|part| predicate_contains_name(part, needle))
        || predicate
            .all_of
            .iter()
            .any(|part| predicate_contains_name(part, needle))
}

fn predicate_contains_kind(predicate: &CompiledPredicate, kind: CompiledCardKind) -> bool {
    predicate.kind == Some(kind)
        || predicate
            .any_of
            .iter()
            .any(|part| predicate_contains_kind(part, kind))
        || predicate
            .all_of
            .iter()
            .any(|part| predicate_contains_kind(part, kind))
}

#[test]
fn bt23_054_has_printed_metadata_routes_and_blocker() {
    let runner = runner();
    let card = runner
        .compiled_card("BT23-054")
        .expect("BT23-054 compiled card present");

    assert_eq!(card.name, "Magnamon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(7000));
    assert_eq!(card.color, vec![CompiledColor::White, CompiledColor::Blue]);
    assert!(card.traits.iter().any(|name| name == "Royal Knight"));
    assert!(card.traits.iter().any(|name| name == "CS"));
    assert_eq!(card.attribute.as_deref(), Some("Free"));

    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(4))
            && path.from.as_ref().is_some_and(|from| {
                from.level_eq == Some(3) && from.color_is == Some(CompiledColor::White)
            })
    }));
    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(3))
            && path.from.as_ref().is_some_and(|from| {
                predicate_has_level_eq(from, 3)
                    && predicate_contains_name(from, "Veemon")
                    && predicate_contains_trait(from, "CS")
            })
    }));
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            ..
        }) if keyword == "Blocker"
    )));
}

#[test]
fn bt23_054_on_play_when_digivolving_draws_then_grants_return_protection() {
    let runner = runner();
    let card = runner
        .compiled_card("BT23-054")
        .expect("BT23-054 compiled card present");

    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay)
                    && triggered.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("On Play/When Digivolving draw+protection clause exists");

    assert!(matches!(
        clause.process.first(),
        Some(CompiledStep::Draw { count: 1, .. })
    ));
    assert!(clause.process.iter().any(|step| matches!(
        step,
        CompiledStep::SelectOwnPermanent { filter, .. }
            if predicate_contains_kind(filter, CompiledCardKind::Digimon)
                && predicate_contains_trait(filter, "Royal Knight")
                && predicate_contains_trait(filter, "CS")
    )));
    let protection_modifiers = clause
        .process
        .iter()
        .filter(|step| match step {
            CompiledStep::AddModifier { modifier, .. } => {
                modifier == "CannotBeReturnedToHand" || modifier == "CannotBeReturnedToDeck"
            }
            _ => false,
        })
        .count();
    assert_eq!(protection_modifiers, 2);
}

#[test]
fn bt23_054_armor_purge_prevents_deletion_by_trashing_top_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-054")
        .expect("BT23-054 YAML loads")
        .add_card(make_test_card("VEEMON", "Veemon"))
        .start();
    let magnamon = runner.place_stack(0, &["VEEMON", "BT23-054"]);

    runner
        .game
        .delete_permanent_with_cause(magnamon, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("Armor Purge accept prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(view.is_optional, "Armor Purge can be declined");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept Armor Purge");
    runner.auto_resolve().expect("finish Armor Purge");

    assert_eq!(runner.battle_area_size(0), 1);
    let remaining = runner.game.players[0].battle_area[0]
        .top_card()
        .card_id(&runner.game.card_data);
    assert_eq!(
        remaining, "VEEMON",
        "Armor Purge trashes BT23-054 and promotes the next card"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BT23-054"),
        "the purged top card is trashed as the cost"
    );
}
