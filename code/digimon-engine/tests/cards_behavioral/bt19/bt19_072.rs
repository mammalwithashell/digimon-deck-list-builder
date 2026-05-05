//! BT19-072 LordKnightmon - Digimon, Lv.6, Purple/White.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledPredicate, CompiledStep, CompiledTiming,
};
use digimon_engine::debug_runner::DebugRunner;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT19-072")
        .expect("BT19-072 YAML loads")
        .memory(10)
        .start()
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

fn predicate_has_level_lte(predicate: &CompiledPredicate, level: u8) -> bool {
    predicate.level_lte == Some(level)
        || predicate
            .any_of
            .iter()
            .any(|part| predicate_has_level_lte(part, level))
        || predicate
            .all_of
            .iter()
            .any(|part| predicate_has_level_lte(part, level))
}

#[test]
fn bt19_072_has_printed_metadata_and_route() {
    let runner = runner();
    let card = runner.compiled_card("BT19-072").expect("BT19-072 present");
    assert_eq!(card.name, "LordKnightmon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(11));
    assert_eq!(card.dp, Some(11000));
    assert_eq!(card.color, vec![CompiledColor::Purple, CompiledColor::White]);
    assert!(card.traits.iter().any(|name| name == "Royal Knight"));
    assert_eq!(card.attribute.as_deref(), Some("Virus"));
    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(3))
            && path
                .from
                .as_ref()
                .is_some_and(|from| from.level_eq == Some(5) && from.color_is == Some(CompiledColor::Purple))
    }));
}

#[test]
fn bt19_072_on_play_when_digivolving_selects_level_four_or_lower_digimon_from_trash() {
    let runner = runner();
    let card = runner.compiled_card("BT19-072").expect("BT19-072 present");
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
        .expect("trash play clause exists");
    assert!(clause.optional);
    assert!(clause.process.iter().any(|step| matches!(
        step,
        CompiledStep::SelectTrash { filter, .. }
            if predicate_contains_kind(filter, CompiledCardKind::Digimon)
                && predicate_has_level_lte(filter, 4)
    )));
    assert!(clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::PlayFromTrashFree { .. })));
}

#[test]
#[ignore = "pending: G-ATTACK-RETARGET — switch attack target to own Royal Knight"]
fn bt19_072_opponents_turn_switches_attack_target_to_royal_knight() {
    panic!("requires attack retarget pending-selection support");
}
