//! BT13-110 Royal Knights of the Purge - Option, White, Royal Knight.
//!
//! Supported slice:
//! - [Main] Draw 1, then place this card in the battle area.
//! - [Security] Place this card in the battle area.
//!
//! Gap-routed:
//! - Hand-card placement under [King Drasil_7D6] in breeding.
//! - Delay play from breeding sources with On Play suppression and Rush.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledStep, CompiledTiming,
};
use digimon_engine::debug_runner::DebugRunner;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT13-110")
        .expect("BT13-110 YAML loads")
        .memory(10)
        .start()
}

#[test]
fn bt13_110_has_printed_metadata() {
    let runner = runner();
    let card = runner.compiled_card("BT13-110").expect("BT13-110 present");
    assert_eq!(card.name, "Royal Knights of the Purge");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.color, vec![CompiledColor::White]);
    assert!(card.traits.iter().any(|name| name == "Royal Knight"));
}

#[test]
fn bt13_110_main_draws_one_and_places_self() {
    let runner = runner();
    let card = runner.compiled_card("BT13-110").expect("BT13-110 present");
    let main = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::MainFromHand) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("[Main] clause exists");
    assert!(matches!(
        main.process.first(),
        Some(CompiledStep::Draw { count: 1, .. })
    ));
    assert!(main
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::PlaceSelfAsDelayOption)));
}

#[test]
fn bt13_110_security_places_self() {
    let runner = runner();
    let card = runner.compiled_card("BT13-110").expect("BT13-110 present");
    assert!(card.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(triggered) => {
            triggered.when.contains(&CompiledTiming::OnSecurity)
                && triggered
                    .process
                    .iter()
                    .any(|step| matches!(step, CompiledStep::PlaceSelfAsDelayOption))
        }
        _ => false,
    }));
}

#[test]
#[ignore = "pending: RK-G001 — place selected hand Digimon under King Drasil_7D6 in breeding"]
fn bt13_110_main_may_place_hand_digimon_under_king_drasil() {
    panic!("requires filtered breeding target and hand-card source placement");
}

#[test]
#[ignore = "pending: RK-G001 — play Royal Knight from breeding sources with On Play suppression and Rush"]
fn bt13_110_delay_plays_royal_knight_from_breeding_sources_with_rush() {
    panic!("requires breeding source selection/play plumbing");
}
