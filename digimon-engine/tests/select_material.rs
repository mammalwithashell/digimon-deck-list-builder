//! §4.6d-residual SelectMaterial helper tests.
//!
//! `EffectContext::select_material` prompts the controller to pick a source
//! (digivolution-stack card) from a target permanent. Action IDs are encoded
//! in the `SOURCE_SELECT` range (2000-2168); phase parks at
//! `GamePhase::SelectMaterial` with kind `SelectionKind::Material`.

use digimon_engine::action::space::{
    decode_source_select, PASS, SOURCES_PER_FIELD, SOURCE_SELECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
use digimon_engine::selection::{SelectionError, SelectionKind};

fn make_digimon(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// Push additional `CardSource` entries onto a permanent's `card_sources`
/// vec — simulates a 2- or 3-card digivolution stack without running
/// `Game::digivolve_onto`.
fn append_source(r: &mut DebugRunner, field_index: usize, card_id: &str) {
    let tp = r.game.turn_player();
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card id");
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, tp, card_index);
    r.game.players[tp as usize].battle_area[field_index]
        .card_sources
        .push(card);
}

#[test]
fn install_emits_source_select_action_ids_for_each_source() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .add_card(make_digimon("MID"))
        .add_card(make_digimon("TOP"))
        .start();
    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "BASE", Some(0));
    append_source(&mut r, oc.index as usize, "MID");
    append_source(&mut r, oc.index as usize, "TOP");
    // Three sources now: BASE, MID, TOP.

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), Some(oc), tp);
        ctx.select_material(oc, "pick a material", true, |_, _| true, |_, _| {});
    }

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("select_material installs a selection");
    assert_eq!(sel.kind, SelectionKind::Material);
    assert!(sel.is_optional);
    assert_eq!(r.game.current_phase, GamePhase::SelectMaterial);

    let expected: Vec<u16> = (0..3)
        .map(|i| SOURCE_SELECT_START + (oc.index as u16) * SOURCES_PER_FIELD + i as u16)
        .collect();
    assert_eq!(sel.valid_action_ids, expected);
}

#[test]
fn filter_excludes_specific_source_positions() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .add_card(make_digimon("MID"))
        .add_card(make_digimon("TOP"))
        .start();
    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "BASE", Some(0));
    append_source(&mut r, oc.index as usize, "MID");
    append_source(&mut r, oc.index as usize, "TOP");

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), Some(oc), tp);
        // Only the middle source (index 1) is legal.
        ctx.select_material(
            oc,
            "pick the middle material",
            false,
            |_, source_index| source_index == 1,
            |_, _| {},
        );
    }

    let sel = r.game.pending_selection.as_ref().unwrap();
    assert_eq!(sel.valid_action_ids.len(), 1);
    assert_eq!(
        sel.valid_action_ids[0],
        SOURCE_SELECT_START + (oc.index as u16) * SOURCES_PER_FIELD + 1
    );
    assert!(!sel.is_optional);
}

#[test]
fn empty_after_filter_does_not_park() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .start();
    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "BASE", Some(0));

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), Some(oc), tp);
        ctx.select_material(oc, "none eligible", true, |_, _| false, |_, _| {});
    }

    assert!(
        r.game.pending_selection.is_none(),
        "empty filter produces no prompt — select_material is a no-op"
    );
    // Phase untouched.
    assert_ne!(r.game.current_phase, GamePhase::SelectMaterial);
}

#[test]
fn callback_receives_decoded_source_index() {
    use std::sync::{Arc, Mutex};

    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .add_card(make_digimon("MID"))
        .add_card(make_digimon("TOP"))
        .start();
    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "BASE", Some(0));
    append_source(&mut r, oc.index as usize, "MID");
    append_source(&mut r, oc.index as usize, "TOP");

    let observed: Arc<Mutex<Option<usize>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&observed);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), Some(oc), tp);
        ctx.select_material(oc, "pick any", false, |_, _| true, move |_, source_index| {
            *slot.lock().unwrap() = Some(source_index);
        });
    }

    // Resolve with the action ID for source index 2.
    let action = SOURCE_SELECT_START + (oc.index as u16) * SOURCES_PER_FIELD + 2;
    let (decoded_field, decoded_source) = decode_source_select(action);
    assert_eq!(decoded_field, oc.index as u16);
    assert_eq!(decoded_source, 2);

    r.game.resolve_selection(tp, action).unwrap();
    assert_eq!(*observed.lock().unwrap(), Some(2));
    assert_ne!(
        r.game.current_phase,
        GamePhase::SelectMaterial,
        "previous_phase restored after resolution"
    );
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn mandatory_select_material_rejects_pass() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .add_card(make_digimon("MID"))
        .start();
    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "BASE", Some(0));
    append_source(&mut r, oc.index as usize, "MID");

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), Some(oc), tp);
        ctx.select_material(oc, "must pick", false, |_, _| true, |_, _| {});
    }

    let err = r
        .game
        .resolve_selection(tp, PASS)
        .expect_err("mandatory selection rejects PASS");
    assert_eq!(err, SelectionError::InvalidAction);
    assert!(r.game.pending_selection.is_some(), "prompt still parked");
}

#[test]
fn optional_select_material_accepts_pass() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .add_card(make_digimon("MID"))
        .start();
    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "BASE", Some(0));
    append_source(&mut r, oc.index as usize, "MID");

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), Some(oc), tp);
        ctx.select_material(oc, "optional", true, |_, _| true, |_, _| {
            panic!("callback must NOT fire on decline");
        });
    }

    r.game
        .resolve_selection(tp, PASS)
        .expect("optional selection accepts PASS");
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn empty_permanent_is_a_noop() {
    // Guard against off-by-one: an out-of-range of_permanent should not
    // panic or install an empty selection.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .start();
    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "BASE", Some(0));
    // Craft a handle pointing at a nonexistent slot.
    let fake = digimon_engine::permanent::PermanentHandle {
        player: tp,
        index: 99,
    };

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), Some(oc), tp);
        ctx.select_material(fake, "out of range", true, |_, _| true, |_, _| {});
    }

    assert!(r.game.pending_selection.is_none());
}
