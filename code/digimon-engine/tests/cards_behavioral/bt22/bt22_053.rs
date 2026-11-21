//! BT22-053 -- synthetic-board behavioural tests for the Keramon line
//! (dcgo-replay hunt). Fixtures: `keramon_line_support`.
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::keramon_line_support::*;
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, ModifierType};
use digimon_engine::permanent::PermanentHandle;

#[test]
fn bt22_053_on_play_adds_arata_and_an_unidentified_or_cs_card_rest_to_bottom() {
    let mut r = search_runner(&["INERT", "ARATA", "CS-1"]);
    let k = r.place_on_field(0, "BT22-053", None);
    r.fire_on_play(0, k.index as usize);
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["ARATA", "CS-1"]);
    assert!(trash_ids(&r, 0).is_empty(), "the rest returns to the deck");
    assert_eq!(r.deck_size(0), 1);
}

#[test]
fn bt22_053_on_play_accepts_unidentified_trait_too() {
    let mut r = search_runner(&["INERT", "ARATA", "UNID-1"]);
    let k = r.place_on_field(0, "BT22-053", None);
    r.fire_on_play(0, k.index as usize);
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["ARATA", "UNID-1"]);
}

#[test]
fn bt22_053_inherited_substitute_deletes_other_diaboromon_so_the_host_stays() {
    for seat in 0..=1u8 {
        let mut r = full_runner();
        r.game.turn_player_idx = seat as usize;
        r.game.memory_pair = (seat, 1 - seat);
        let host = r.place_stack(seat, &["BT22-053", "BT22-064"]);
        r.place_on_field(seat, "TOKEN_DIABOROMON", Some(0));
        r.game.delete_permanent_with_effects(host);
        r.game.drain_effect_queue();
        assert!(
            r.pending_is_optional(),
            "[Once Per Turn] ... by deleting 1 of your other [Diaboromon] (seat {seat})"
        );
        r.accept_optional_trigger().unwrap();
        let _ = r.auto_resolve();
        assert_eq!(field_top_ids(&r, seat), vec!["BT22-064"], "host stayed, token deleted");
    }
}

#[test]
fn bt22_053_substitute_declined_lets_the_host_leave() {
    let mut r = full_runner();
    let host = r.place_stack(0, &["BT22-053", "BT22-064"]);
    r.place_on_field(0, "TOKEN_DIABOROMON", Some(0));
    r.game.delete_permanent_with_effects(host);
    r.game.drain_effect_queue();
    r.decline_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(field_top_ids(&r, 0), vec!["TOKEN_DIABOROMON"]);
}

#[test]
fn bt22_053_substitute_needs_a_host_with_diaboromon_in_its_text() {
    let mut r = full_runner();
    let host = r.place_stack(0, &["BT22-053", "BLACK3"]);
    r.place_on_field(0, "TOKEN_DIABOROMON", Some(0));
    r.game.delete_permanent_with_effects(host);
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert_eq!(field_top_ids(&r, 0), vec!["TOKEN_DIABOROMON"], "host without [Diaboromon] text is deleted");
}

#[test]
fn bt22_053_substitute_host_with_diaboromon_in_effect_text_counts() {
    // HasText scans the printed effect text, not only the name.
    let mut r = full_runner();
    let host = r.place_stack(0, &["BT22-053", "DIA-TEXT"]);
    r.place_on_field(0, "TOKEN_DIABOROMON", Some(0));
    r.game.delete_permanent_with_effects(host);
    r.game.drain_effect_queue();
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(field_top_ids(&r, 0), vec!["DIA-TEXT"]);
}

#[test]
fn bt22_053_substitute_other_must_be_named_exactly_diaboromon() {
    let mut r = full_runner();
    let host = r.place_stack(0, &["BT22-053", "BT22-064"]);
    r.place_on_field(0, "DIA-X", Some(0)); // "Diaboromon X" is not [Diaboromon]
    r.game.delete_permanent_with_effects(host);
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert_eq!(field_top_ids(&r, 0), vec!["DIA-X"], "no legal delete cost: host leaves");
}

#[test]
fn bt22_053_substitute_is_once_per_turn() {
    let mut r = full_runner();
    let host = r.place_stack(0, &["BT22-053", "BT22-064"]);
    r.place_on_field(0, "TOKEN_DIABOROMON", Some(0));
    r.place_on_field(0, "TOKEN_DIABOROMON", Some(0));
    r.game.delete_permanent_with_effects(host);
    r.game.drain_effect_queue();
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(field_top_ids(&r, 0), vec!["BT22-064", "TOKEN_DIABOROMON"]);
    let host = r.perm_handle(0, 0);
    r.game.delete_permanent_with_effects(host);
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert_eq!(field_top_ids(&r, 0), vec!["TOKEN_DIABOROMON"], "second use the same turn is locked out");
}
