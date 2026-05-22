//! BT13-110 Royal Knights of the Purge - Option, White, Royal Knight.
//!
//! Supported slice:
//! - [Main] Draw 1, then place this card in the battle area.
//! - [Security] Place this card in the battle area.
//!
//! Gap-routed:
//! - Hand-card placement under [King Drasil_7D6] in breeding — RK-G001
//!   filter shipped (Phase 2 Track J PR 1), but the printed "you may"
//!   optionality needs `optional: bool` on `select_own_breeding_permanent`
//!   (`G-OPTIONAL-BREEDING-SELECTION`).
//! - Delay play from breeding sources with On Play suppression and Rush
//!   needs source-stack selection/play (G-BREEDING-SOURCE-PLAY family).

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
#[ignore = "pending: G-OPTIONAL-BREEDING-SELECTION — RK-G001 filter shipped (Phase 2 Track J PR 1) but `select_own_breeding_permanent` is hardcoded `is_optional: false`, so the printed 'you may' clause can't surface a decline path"]
fn bt13_110_main_may_place_hand_digimon_under_king_drasil() {
    panic!("requires optional select_own_breeding_permanent before the printed 'you may' clause can be authored faithfully");
}

#[test]
#[ignore = "pending: G-BREEDING-SOURCE-PLAY — play 1 [Royal Knight] from breeding digivolution sources with On Play suppression and Rush grant (out of Track J scope)"]
fn bt13_110_delay_plays_royal_knight_from_breeding_sources_with_rush() {
    panic!("requires breeding-source selection/play substrate (out of Track J scope)");
}
