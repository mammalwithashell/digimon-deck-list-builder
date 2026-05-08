//! BT20-060 Alphamon: Ouryuken - Digimon, Lv.7, Black/Purple/Red.
//!
//! Supported slice:
//! - Printed metadata, ACE Overflow, standard and DNA digivolve routes.
//! - [On Play][When Digivolving] select 1 opponent Digimon and give -15000 DP
//!   until the end of the opponent's turn.
//!
//! Gap-routed:
//! - [Hand][Counter] Blast DNA Digivolve needs counter-window Blast DNA action
//!   support for named materials.
//! - DNA-gated security trash + Recovery and the security-removed memory
//!   observer need faithful DNA-origin and security-removed dispatch coverage.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledModifierValue, CompiledStep, CompiledTiming,
};
use digimon_engine::debug_runner::DebugRunner;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT20-060")
        .expect("BT20-060 YAML loads")
        .memory(10)
        .start()
}

#[test]
fn bt20_060_has_printed_metadata_ace_overflow_and_routes() {
    let runner = runner();
    let card = runner
        .compiled_card("BT20-060")
        .expect("BT20-060 compiled card present");

    assert_eq!(card.name, "Alphamon: Ouryuken");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(7));
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(16000));
    assert_eq!(
        card.color,
        vec![
            CompiledColor::Black,
            CompiledColor::Purple,
            CompiledColor::Red
        ]
    );
    assert!(card.traits.iter().any(|name| name == "X Antibody"));
    assert!(card.traits.iter().any(|name| name == "Royal Knight"));
    assert!(card.traits.iter().any(|name| name == "Chronicle"));
    assert_eq!(card.attribute.as_deref(), Some("Vaccine"));
    assert_eq!(card.ace_overflow, Some(-5));

    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(6))
            && path.from.as_ref().is_some_and(|from| {
                from.level_eq == Some(6) && from.color_is == Some(CompiledColor::Black)
            })
    }));
    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::DnaDigivolve
            && path.cost == Some(CompiledCost::Literal(0))
    }));
}

#[test]
fn bt20_060_on_play_when_digivolving_selects_opponent_digimon_for_minus_15000() {
    let runner = runner();
    let card = runner
        .compiled_card("BT20-060")
        .expect("BT20-060 compiled card present");

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
        .expect("On Play/When Digivolving DP reduction clause exists");

    assert!(!clause.optional, "printed DP reduction has no 'may'");
    assert!(matches!(
        clause.process.first(),
        Some(CompiledStep::SelectOpponentPermanent { filter, .. })
            if filter.kind == Some(CompiledCardKind::Digimon)
    ));
    assert!(clause.process.iter().any(|step| matches!(
        step,
        CompiledStep::AddDpModifier {
            value: CompiledModifierValue::Literal(-15000),
            ..
        }
    )));
}

#[test]
#[ignore = "pending: G-BLAST-DNA-COUNTER — no counter-window Blast DNA action path for named hand/field materials"]
fn bt20_060_hand_counter_blast_dna_uses_alphamon_and_ouryumon() {
    panic!("requires Blast DNA Digivolve action-mask support for [Counter]");
}

#[test]
#[ignore = "pending: DNA-origin gated tail plus security-trash/recovery sequence for BT20-060"]
fn bt20_060_dna_origin_trashes_security_and_recovers() {
    panic!("requires faithful DNA-origin tail sequencing");
}

#[test]
#[ignore = "pending: G-SECURITY-REMOVED-OBSERVER — all-turns security stack removed observer"]
fn bt20_060_security_removed_gain_three_memory_once_per_turn() {
    panic!("requires security-removed event dispatch and OPT handling");
}
