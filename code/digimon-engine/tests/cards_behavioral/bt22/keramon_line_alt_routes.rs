//! Keramon line alt-digivolve routes -- synthetic-board behavioural tests for the Keramon line
//! (dcgo-replay hunt). Fixtures: `keramon_line_support`.
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::keramon_line_support::*;
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, ModifierType};
use digimon_engine::permanent::PermanentHandle;

#[test]
fn alt_digivolve_routes_cs_trait_bases() {
    let mut r = full_runner();
    let cs2 = r.place_on_field(0, "CS-LV2", Some(0));
    let plain2 = r.place_on_field(0, "PLAIN-LV2", Some(0));
    assert!(can_digivolve(&mut r, 0, "BT22-053", cs2), "Lv.2 w/[CS] -> BT22-053 (cost 0)");
    assert!(!can_digivolve(&mut r, 0, "BT22-053", plain2), "a non-CS red Lv.2 is neither route");

    let cs3 = r.place_on_field(0, "CS-LV3", Some(0));
    let plain3 = r.place_on_field(0, "PLAIN-LV3", Some(0));
    assert!(can_digivolve(&mut r, 0, "BT22-057", cs3), "Lv.3 w/[CS] -> BT22-057");
    assert!(!can_digivolve(&mut r, 0, "BT22-057", plain3));

    let cs4 = r.place_on_field(0, "CS-LV4", Some(0));
    assert!(can_digivolve(&mut r, 0, "BT22-059", cs4), "Lv.4 w/[CS] -> BT22-059");

    let cs5 = r.place_on_field(0, "CS-LV5", Some(0));
    let inf = r.place_on_field(0, "INF-NAMED", Some(0));
    let plain5 = r.place_on_field(0, "PLAIN-LV5", Some(0));
    assert!(can_digivolve(&mut r, 0, "BT22-064", cs5), "Lv.5 w/[CS] -> BT22-064");
    assert!(can_digivolve(&mut r, 0, "BT22-064", inf), "[Infermon] -> BT22-064 (cost 3)");
    assert!(!can_digivolve(&mut r, 0, "BT22-064", plain5));
}
