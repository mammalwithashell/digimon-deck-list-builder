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
//! (Lv '-', DP 9000). NV-1 (a no-DP Digimon can't gain DP) uses a Lv.2 Digi-Egg:
//! no Digimon card in the pool prints DP '-'.

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
/// having no DP value." (General Rules/FAQ.) Vehicle: BT10-003 Pickmons, a Lv.2
/// Digi-Egg — the FAQ's own subject; no Digimon card in the pool prints DP "-".
/// Until 2026-10-03 the vehicle was BT24-068 DemiDevimon, read off a pre-release
/// cards.json stub with no DP; that card prints 1000.
#[test]
fn nv1_no_dp_digimon_cannot_gain_dp() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT10-003")
        .expect("BT10-003 (Pickmons) in embedded DSL pack")
        .start();
    let c = card(&r, "BT10-003");
    assert_eq!(c.level, Some(2), "precondition: a Level 2 Digimon");
    assert!(c.dp.is_none(), "precondition: Pickmons prints no DP value");

    let p = r.turn_player();
    let h = r.place_on_field(p, "BT10-003", Some(0));
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
