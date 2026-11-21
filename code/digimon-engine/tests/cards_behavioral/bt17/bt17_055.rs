//! BT17-055 -- synthetic-board behavioural tests for the Keramon line
//! (dcgo-replay hunt). Fixtures: `keramon_line_support`.
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::keramon_line_support::*;
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, ModifierType};
use digimon_engine::permanent::PermanentHandle;

// --------------------------------------------------------------- BT17-055
#[test]
fn bt17_055_when_digivolving_de_digivolves_then_restricts_a_cost_8_or_less_digimon() {
    let mut r = full_runner();
    let host = r.place_on_field(0, "BT5-063", Some(0)); // black Lv.4
    let victim = r.place_stack(1, &["OPP-C4", "OPP-C6"]);
    r.place_on_field(1, "OPP-C9", Some(0));
    digivolve_via_action(&mut r, 0, "BT17-055", host);
    let _ = r.auto_resolve();
    // <De-Digivolve 1>: the stack's top card (Lv.6 OPP-C6) is trashed.
    assert_eq!(field_top_ids(&r, 1)[0], "OPP-C4");
    assert_eq!(r.trash_size(1), 1);
    // `auto_resolve` takes the first legal pick each time, i.e. a cost <= 8 Digimon.
    let restricted: Vec<_> = (0..r.battle_area_size(1))
        .filter(|&i| {
            r.modifiers()
                .has(PermanentHandle { player: 1, index: i as u8 }, ModifierType::CannotAttackPlayer)
        })
        .collect();
    assert_eq!(restricted.len(), 1);
    assert_ne!(field_top_ids(&r, 1)[restricted[0]], "OPP-C9", "cost 9 is not eligible");
}

#[test]
fn bt17_055_cost_9_digimon_cannot_be_restricted() {
    let mut r = full_runner();
    let host = r.place_on_field(0, "BT5-063", Some(0));
    let nine = r.place_on_field(1, "OPP-C9", Some(0));
    digivolve_via_action(&mut r, 0, "BT17-055", host);
    let _ = r.auto_resolve();
    assert!(!r.modifiers().has(nine, ModifierType::CannotAttackPlayer));
}

#[test]
fn bt17_055_inherited_de_digivolves_when_you_play_another_diaboromon_named_digimon() {
    let mut r = full_runner();
    r.place_stack(0, &["BT17-055", "BLACK3"]);
    r.place_stack(1, &["OPP-C4", "OPP-C6"]);
    let tok = r.place_on_field(0, "TOKEN_DIABOROMON", None);
    r.fire_play_event_triggers(0, tok.index as usize, false, false);
    let _ = r.auto_resolve();
    assert_eq!(field_top_ids(&r, 1), vec!["OPP-C4"], "<De-Digivolve 1> trashed the top card");

    // [Once Per Turn]
    let tok2 = r.place_on_field(0, "TOKEN_DIABOROMON", None);
    r.fire_play_event_triggers(0, tok2.index as usize, false, false);
    let _ = r.auto_resolve();
    assert_eq!(r.trash_size(1), 1);
}

#[test]
fn bt17_055_inherited_ignores_non_diaboromon_plays_and_opponents() {
    let mut r = full_runner();
    r.place_stack(0, &["BT17-055", "BLACK3"]);
    r.place_stack(1, &["OPP-C4", "OPP-C6"]);
    let plain = r.place_on_field(0, "PLAIN-LV3", None);
    r.fire_play_event_triggers(0, plain.index as usize, false, false);
    let _ = r.auto_resolve();
    assert_eq!(r.trash_size(1), 0);
    let theirs = r.place_on_field(1, "TOKEN_DIABOROMON", None);
    r.fire_play_event_triggers(1, theirs.index as usize, false, false);
    let _ = r.auto_resolve();
    assert_eq!(r.trash_size(1), 0);
}
