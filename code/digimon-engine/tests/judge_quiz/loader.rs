//! Task 1.5 — confirm every judge-quiz card that is already implemented (DSL)
//! actually loads from the embedded DSL pack via `DebugRunner::dsl_card`.
//!
//! This gates the discovery wave: the scenario tests compose real cards by
//! chaining `.dsl_card(id)`, so if a card's YAML exists on disk but is not in
//! the embedded pack the scenario can't be built. Any failure here is a
//! test-harness/packaging gap to fix before writing the scenario, not a card
//! faithfulness gap.
//!
//! The 27 IDs below are the implemented set from
//! `card-resolution.md` §"Implementation status" (filesystem scan 2026-05-28),
//! re-derived against the AUTHORITATIVE printings (the quiz uses BT24-017 for
//! Medusamon, P-165 for ShoeShoemon, EX8-073 for Gallantmon X, etc. — not the
//! printings an earlier name-based inventory guessed).

use digimon_engine::debug_runner::DebugRunner;

/// Every quiz-referenced card with a DSL YAML on disk (per the 2026-05-28 scan).
const IMPLEMENTED_QUIZ_CARDS: &[&str] = &[
    "AD1-004",  // WarGreymon (Q25–27)
    "AD1-014",  // MetalGarurumon (Q25–27)
    "AD1-025",  // Omnimon [Assembly] (Q5, Q25–27)
    "BT1-090",  // Gravity Crush (Q10/Q11)
    "BT6-084",  // Sistermon Ciel (Q28)
    "BT7-107",  // Calling From the Darkness (Q19–21)  [NOTE: lacks a behavioral test]
    "BT9-033",  // Pillomon (Q6/Q7)
    "BT12-022", // ExVeemon (Q16)
    "BT12-050", // Stingmon (Q16)
    "BT16-025", // Paildramon (Q16)
    "BT17-077", // Imperialdramon: Paladin Mode ACE (Q18)
    "BT17-095", // Miraculous Mega Knight (Q25–27)
    "BT19-072", // LordKnightmon (Q15)
    "BT20-102", // Omnimon (X Antibody) (Q15)
    "BT22-042", // Nyabootmon (Q13/Q14)
    "BT23-057", // Gankoomon (Q28)
    "BT23-096", // Comet Hammer (Q8)
    "BT24-017", // Medusamon (Q1, Q2, Q22)
    "BT24-040", // Venusmon (Q12)
    "EX1-068",  // Ice Wall! (Q2)
    "EX4-006",  // Guilmon (Q15)
    "EX4-074",  // ShineGreymon: Ruin Mode (Q14)
    "EX8-005",  // Tumblemon (Q22/Q23)
    "EX8-051",  // Proganomon (Q22/Q23)
    "EX8-073",  // Gallantmon (X Antibody) (Q15)
    "EX8-074",  // MedievalGallantmon (Q30)
    "P-165",    // ShoeShoemon (Q13/Q14)
];

/// Every implemented quiz card must be loadable from the embedded DSL pack.
/// Reports the full set of failures in one shot rather than aborting on the
/// first, so a packaging gap surfaces completely.
#[test]
fn all_implemented_quiz_cards_load_from_embedded_pack() {
    let mut failures = Vec::new();
    for &id in IMPLEMENTED_QUIZ_CARDS {
        if let Err(e) = DebugRunner::builder().dsl_card(id) {
            failures.push(format!("{id}: {e}"));
        }
    }
    assert!(
        failures.is_empty(),
        "the following implemented quiz cards did not load from the embedded \
         DSL pack (fix packaging before building their scenarios):\n  {}",
        failures.join("\n  ")
    );
}
