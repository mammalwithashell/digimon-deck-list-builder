//! ST12-15 From Master to Disciple
//!
//! Implemented slice:
//! - [Main]/[Security] reveal 3; add 1 Huckmon/Sistermon/Royal Knight; trash
//!   the rest; place this card as a Delay option.
//!
//! Gap-routed slice:
//! - Delay body that reduces the next digivolution cost by 1.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn st12_15_has_main_search_and_security_search() {
    let runner = DebugRunner::builder()
        .dsl_card("ST12-15")
        .expect("ST12-15 must load from embedded DSL pack")
        .memory(5)
        .start();
    let card = runner.compiled_card("ST12-15").expect("compiled card");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand)
        )),
        "ST12-15 must have Main search"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "ST12-15 must have Security search"
    );
}

#[ignore = "pending: G-DELAY-NEXT-DIGIVOLVE-COST-REDUCTION — Delay activation must create a one-shot next-digivolve cost reduction"]
#[test]
fn st12_15_delay_reduces_next_digivolution_cost_by_1() {}
