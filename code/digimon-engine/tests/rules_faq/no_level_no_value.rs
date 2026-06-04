//! Digimon with no Level / no Value — surface M + R. Ledger rows NL-1, NL-2, NV-1.
//!
//! "What is the treatment of Digimon whose Lv. is '-'? A Digimon whose Lv. is
//! '-' becomes a 'Digimon without Lv.'. Since it does not have Lv., it cannot
//! evolve from/into that Digimon under normal evolution conditions. Also, it is
//! not the target of effects that refer to Lv." / "When a 'Digimon without Lv.'
//! is in the breeding area, can it move to the battle area during the breeding
//! phase? If the Digimon has DP, it can." (General Rules/FAQ.)
//!
//! Vehicle: BT23-072 King Drasil_7D6 — the only implemented no-Level Digimon
//! (Lv '-', DP 9000). NV-1 (no-DP Digimon can't gain DP) is BLOCK-CARD: zero
//! implemented no-DP Digimon (authoring candidate, tasks.md §4).

#![allow(unused_imports)]

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::ModifierEntry;

fn card<'a>(r: &'a DebugRunner, id: &str) -> &'a CardData {
    r.game
        .card_data
        .iter()
        .find(|c| c.card_id == id)
        .unwrap_or_else(|| panic!("{id} present in loaded card data"))
}

/// NL-1 — a no-Level Digimon has `level == None`, so any effect or digivolution
/// condition that reads "Lv.X or less/more" finds no level to compare against.
#[test]
fn nl1_no_level_digimon_has_no_level_value() {
    let r = DebugRunner::builder()
        .dsl_card("BT23-072")
        .expect("BT23-072 (King Drasil_7D6) in embedded DSL pack")
        .start();
    let c = card(&r, "BT23-072");

    assert_eq!(c.card_kind, CardKind::Digimon, "it is a Digimon");
    assert!(
        c.level.is_none(),
        "FAQ: a Digimon whose Lv. is '-' is a 'Digimon without Lv.' — no level value"
    );
}

/// NL-2 — a no-Level Digimon that HAS a DP value is breeding-eligible (it can be
/// promoted from the breeding area during the breeding phase).
#[test]
fn nl2_no_level_digimon_with_dp_is_breeding_eligible() {
    let r = DebugRunner::builder()
        .dsl_card("BT23-072")
        .expect("BT23-072 (King Drasil_7D6) in embedded DSL pack")
        .start();
    let c = card(&r, "BT23-072");

    assert!(c.level.is_none(), "no Level");
    assert!(
        c.dp.is_some_and(|dp| dp > 0),
        "FAQ: a no-Level Digimon with DP can move from breeding to battle"
    );
}

/// NV-1 — "My Level 2 Digimon has no DP value. Can it gain DP from other
/// effects? No, even if an effect is applying +X000 DP the Digimon is treated as
/// having no DP value." (General Rules/FAQ.) Vehicle: BT24-068 DemiDevimon (a
/// Lv3 Digimon whose printed DP is "-").
#[test]
fn nv1_no_dp_digimon_cannot_gain_dp() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT24-068")
        .expect("BT24-068 (DemiDevimon) in embedded DSL pack")
        .start();
    let c = card(&r, "BT24-068");
    assert!(c.dp.is_none(), "precondition: DemiDevimon has no printed DP value");

    let p = r.turn_player();
    let h = r.place_on_field(p, "BT24-068", Some(0));
    assert_eq!(r.effective_dp(h), None, "a no-DP Digimon has no DP value");

    // Apply a +3000 DP modifier — the Digimon still has NO DP value.
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeDp, 3000, Expiry::Permanent, p as u8),
    );
    assert_eq!(
        r.effective_dp(h),
        None,
        "FAQ NV-1: a no-DP Digimon cannot gain DP — it remains a no-value Digimon"
    );
}
