//! §4.6d-residual SelectMaterial helper tests.
//!
//! `EffectContext::select_material` prompts the controller to pick a source
//! (digivolution-stack card) from a target permanent. Action IDs are encoded
//! in the `SOURCE_SELECT` range (2000-2168); phase parks at
//! `GamePhase::SelectMaterial` with kind `SelectionKind::Material`.

use digimon_engine::action::space::{
    decode_source_select, BREEDING_SOURCE_SELECT_END, BREEDING_SOURCE_SELECT_START,
    BREEDING_TARGET, PASS, SOURCES_PER_FIELD, SOURCE_SELECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
use digimon_engine::permanent::PermanentHandle;
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
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
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
fn install_emits_source_select_action_ids_excluding_top() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .add_card(make_digimon("MID"))
        .add_card(make_digimon("TOP"))
        .start();
    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "BASE", Some(0));
    append_source(&mut r, oc.index as usize, "MID");
    append_source(&mut r, oc.index as usize, "TOP");
    // Three sources now: BASE (idx 0), MID (idx 1), TOP (idx 2).
    // The top card is the active Digimon and must NOT be offered as a
    // material — mirrors CountCappedZone::Material's contract.

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

    // Indices 0 and 1 only — index 2 (TOP) is excluded.
    let expected: Vec<u16> = (0..2)
        .map(|i| SOURCE_SELECT_START + (oc.index as u16) * SOURCES_PER_FIELD + i as u16)
        .collect();
    assert_eq!(sel.valid_action_ids, expected);
    assert_eq!(sel.valid_action_ids.len(), 2);
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

// ── Breeding-carrier single-pick select_material (S1.3) ──────────────────────

/// `select_material` against a breeding-area carrier (King Drasil) must
/// install a real `SelectMaterial` prompt with breeding-source action IDs —
/// NOT no-op (the pre-S1.3 bug, where `battle_area.get(14)` returned `None`).
#[test]
fn breeding_carrier_select_material_emits_breeding_source_ids() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("KD-BASE"))
        .add_card(make_digimon("KD-MID"))
        .add_card(make_digimon("KD-TOP"))
        .start();
    let tp = r.game.turn_player();
    // Build a 3-card breeding stack: KD-BASE, KD-MID, KD-TOP.
    let handle = r.place_stack(tp, &["KD-BASE", "KD-MID", "KD-TOP"]);
    let perm = r.game.players[tp as usize]
        .battle_area
        .remove(handle.index as usize);
    r.game.players[tp as usize].breeding_area = Some(perm);
    let carrier = PermanentHandle {
        player: tp,
        index: BREEDING_TARGET as u8,
    };

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_material(
            carrier,
            "pick a breeding material",
            true,
            |_, _| true,
            |_, _| {},
        );
    }

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("breeding-carrier select_material must install a SelectMaterial prompt");
    assert_eq!(sel.kind, SelectionKind::Material);
    assert_eq!(r.game.current_phase, GamePhase::SelectMaterial);
    // Two sources (KD-BASE, KD-MID); KD-TOP excluded as the active card.
    assert_eq!(sel.valid_action_ids.len(), 2);
    for &aid in &sel.valid_action_ids {
        assert!(
            (BREEDING_SOURCE_SELECT_START..BREEDING_SOURCE_SELECT_END).contains(&aid),
            "action id {aid} must be a breeding-source ID"
        );
    }
    // tp == 0 in DebugRunner default → 2168 + 0*12 + i.
    assert_eq!(sel.valid_action_ids[0], BREEDING_SOURCE_SELECT_START);
    assert_eq!(sel.valid_action_ids[1], BREEDING_SOURCE_SELECT_START + 1);
}

/// Resolving a breeding-carrier `select_material` pick delivers the correct
/// source index to the callback — proves the callback recovers the index via
/// `range_start`, not `decode_source_select` (which would mis-decode a
/// breeding-source action ID).
#[test]
fn breeding_carrier_select_material_callback_receives_source_index() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("KD-BASE"))
        .add_card(make_digimon("KD-MID"))
        .add_card(make_digimon("KD-TOP"))
        .start();
    let tp = r.game.turn_player();
    let handle = r.place_stack(tp, &["KD-BASE", "KD-MID", "KD-TOP"]);
    let perm = r.game.players[tp as usize]
        .battle_area
        .remove(handle.index as usize);
    r.game.players[tp as usize].breeding_area = Some(perm);
    let carrier = PermanentHandle {
        player: tp,
        index: BREEDING_TARGET as u8,
    };

    let received: std::sync::Arc<std::sync::Mutex<Option<usize>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let sink = std::sync::Arc::clone(&received);
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_material(
            carrier,
            "pick a breeding material",
            false,
            |_, _| true,
            move |_, src_idx| {
                *sink.lock().unwrap() = Some(src_idx);
            },
        );
    }

    // Pick the second source (KD-MID, source index 1).
    let action = BREEDING_SOURCE_SELECT_START + 1;
    r.game.resolve_selection(tp, action).expect("resolve pick");
    assert_eq!(
        *received.lock().unwrap(),
        Some(1),
        "callback must receive source index 1, recovered via range_start"
    );
}

#[test]
fn callback_receives_decoded_source_index() {
    use std::sync::{Arc, Mutex};

    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .add_card(make_digimon("MID"))
        .add_card(make_digimon("MID2"))
        .add_card(make_digimon("TOP"))
        .start();
    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "BASE", Some(0));
    append_source(&mut r, oc.index as usize, "MID");
    append_source(&mut r, oc.index as usize, "MID2");
    append_source(&mut r, oc.index as usize, "TOP");
    // Four sources: BASE (0), MID (1), MID2 (2), TOP (3).
    // Index 3 (TOP) is excluded; indices 0..=2 are selectable.

    let observed: Arc<Mutex<Option<usize>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&observed);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), Some(oc), tp);
        ctx.select_material(
            oc,
            "pick any",
            false,
            |_, _| true,
            move |_, source_index| {
                *slot.lock().unwrap() = Some(source_index);
            },
        );
    }

    // Resolve with the action ID for source index 2 (MID2 — a material, not the top).
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
        ctx.select_material(
            oc,
            "optional",
            true,
            |_, _| true,
            |_, _| {
                panic!("callback must NOT fire on decline");
            },
        );
    }

    r.game
        .resolve_selection(tp, PASS)
        .expect("optional selection accepts PASS");
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn single_source_permanent_offers_no_materials() {
    use std::sync::{Arc, Mutex};

    // A permanent with only its top card (1 source) has no materials —
    // the top is excluded by contract, so no prompt is installed.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("SOLO"))
        .start();
    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "SOLO", Some(0));

    // Track callback invocations: with no candidates, the callback must
    // never fire — regression for the no-approximations contract (the
    // engine cannot silently auto-pick the top card to satisfy a script).
    let callback_count: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let counter = Arc::clone(&callback_count);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), Some(oc), tp);
        ctx.select_material(
            oc,
            "no materials",
            true,
            |_, _| true,
            move |_, _| {
                *counter.lock().unwrap() += 1;
            },
        );
    }

    assert!(
        r.game.pending_selection.is_none(),
        "single-source permanent has no materials — top is never offered"
    );
    assert_ne!(r.game.current_phase, GamePhase::SelectMaterial);
    assert_eq!(
        *callback_count.lock().unwrap(),
        0,
        "callback must not fire when no material candidates exist"
    );
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
