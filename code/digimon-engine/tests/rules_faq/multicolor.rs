//! Cards with multiple Colours — surface M. Ledger rows MC-1, MC-2, MC-8.
//!
//! "Can a blue + green digimon be a target of an effect that targets blue
//! digimon? Yes, a digimon with 2 or more colours are treated as digimon of all
//! those colours." / "My Digimon has multiple traits or colours, can I for my
//! effect treat it as just 1 of those colours? No." (General Rules/FAQ.)
//!
//! Pinned via the engine's own colour-identity predicate `CardData::is_color`
//! on the REAL in-game card data (post-load), so a colour-filter that checks
//! membership matches the card for each of its colours and for none it lacks.
//! Runtime gating rows (MC-3..MC-7) are TBD (tasks.md §7.1).

#![allow(unused_imports)]

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::CardColor;

fn card<'a>(r: &'a DebugRunner, id: &str) -> &'a CardData {
    r.game
        .card_data
        .iter()
        .find(|c| c.card_id == id)
        .unwrap_or_else(|| panic!("{id} present in loaded card data"))
}

/// MC-1 / MC-8 — a two-colour Digimon is treated as EACH of its colours (so a
/// "target a Red Digimon" and a "target a Yellow Digimon" effect both match it),
/// and is NOT any colour it lacks (can't be treated as just one).
#[test]
fn mc1_two_color_digimon_is_each_of_its_colors() {
    // EX4-005 Agumon — [Red, Yellow].
    let r = DebugRunner::builder()
        .dsl_card("EX4-005")
        .expect("EX4-005 (Agumon) in embedded DSL pack")
        .start();
    let c = card(&r, "EX4-005");

    assert!(c.is_color(CardColor::Red), "treated as Red");
    assert!(c.is_color(CardColor::Yellow), "treated as Yellow");
    assert!(
        !c.is_color(CardColor::Blue),
        "FAQ: a multi-colour card cannot be treated as a colour it does not have"
    );
}

/// MC-1 generalises to a different colour pair — BT16-101 Rapidmon (X Antibody)
/// is [Yellow, Green].
#[test]
fn mc1_holds_for_a_second_color_pair() {
    let r = DebugRunner::builder()
        .dsl_card("BT16-101")
        .expect("BT16-101 (Rapidmon X) in embedded DSL pack")
        .start();
    let c = card(&r, "BT16-101");

    assert!(c.is_color(CardColor::Yellow), "treated as Yellow");
    assert!(c.is_color(CardColor::Green), "treated as Green");
    assert!(!c.is_color(CardColor::Red), "not Red");
}

/// A two-colour Option carries both colours too (BT17-095 Miraculous Mega
/// Knight — [Red, Blue]) — the data basis for MC-4/MC-6 ("return/use a blue
/// option" can take a blue+green option).
#[test]
fn mc_two_color_option_carries_both_colors() {
    let r = DebugRunner::builder()
        .dsl_card("BT17-095")
        .expect("BT17-095 (Miraculous Mega Knight) in embedded DSL pack")
        .start();
    let c = card(&r, "BT17-095");

    assert!(c.is_color(CardColor::Red), "Option is Red");
    assert!(c.is_color(CardColor::Blue), "Option is Blue");
}
