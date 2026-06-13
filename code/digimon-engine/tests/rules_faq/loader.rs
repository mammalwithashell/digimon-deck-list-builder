//! Reused-card load gate (mirrors `judge_quiz/loader.rs`).
//!
//! The discovery-wave tests compose REAL implemented cards via
//! `DebugRunner::builder().dsl_card(id)`. If a card's YAML exists on disk but
//! is not in the embedded DSL pack, a scenario can't be built. Any failure
//! here is a test-harness/packaging gap to fix BEFORE writing the scenario —
//! not a card-faithfulness gap.
//!
//! The ids below are the reused-card vehicles pinned in `qa/qa-reports/rules-faq.md`
//! §"Reused-card picks". Authoring candidates (no-DP Digimon, dual-`evo_costs`
//! target, D-Reaper) are intentionally absent — they are `BLOCK-CARD` rows.

use digimon_engine::debug_runner::DebugRunner;

/// Reused implemented DSL vehicles for the rules-FAQ suite.
const REUSED_VEHICLES: &[&str] = &[
    "AD1-001",  // Greymon — vanilla Lv4 vehicle; name-substring source
    "AD1-004",  // WarGreymon — two-color [Red,Black]; "Greymon"-in-name
    "AD1-025",  // Omnimon — name-substring target
    "EX4-005",  // Agumon — two-color [Red,Yellow] Lv3
    "BT16-101", // Rapidmon (X Antibody) — two-color [Yellow,Green]; WhenAttacking+inherited
    "BT15-020", // Gabumon — conditional gained keyword (Blocker)
    "BT13-093", // Omekamon — [On Deletion] effect
    "BT10-042", // Venusmon — [When Digivolving]
    "BT23-072", // King Drasil_7D6 — no-Level Digimon (DP 9000)
    "BT21-072", // Arresterdramon: Superior Mode — "<Save> in its text" (icon-exact)
    "BT17-095", // Miraculous Mega Knight — two-color Option [Red,Blue]
];

/// Every reused vehicle must load from the embedded DSL pack. A failure names
/// the offending id so a typo or un-migrated card fails loudly here rather than
/// mid-scenario.
#[test]
fn reused_vehicles_load_from_embedded_dsl_pack() {
    for id in REUSED_VEHICLES {
        let built = DebugRunner::builder().dsl_card(id);
        assert!(
            built.is_ok(),
            "reused vehicle {id} must load from the embedded DSL pack: {:?}",
            built.err()
        );
    }
}
