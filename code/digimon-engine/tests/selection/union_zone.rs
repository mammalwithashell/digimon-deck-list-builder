//! `select_union_zone` helper tests.
//!
//! `EffectContext::select_union_zone` prompts a player to pick one card from
//! a union of zones (e.g. hand OR trash). Action IDs reuse the existing
//! `PLAY_HAND_START` (0-29) and `TRASH_EFFECT_START` (1150-1194) ranges —
//! no new action-space range is introduced. Phase parks at
//! `GamePhase::SelectUnion` with kind `SelectionKind::UnionZone { zones }`.

use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
use digimon_engine::selection::{SelectionError, SelectionKind, UnionZoneSet};

fn make_card(id: &str) -> CardData {
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
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// Helper: push a card directly into a player's trash without going through
/// normal play/deletion flow (mirrors the `append_source` pattern in material.rs).
fn push_to_trash(r: &mut DebugRunner, player: digimon_engine::enums::PlayerId, card_id: &str) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {}", card_id));
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    r.game.players[player as usize].trash.push(card);
}

// ── install / phase / kind assertions ───────────────────────────────────────

#[test]
fn install_sets_phase_and_kind() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("H1"))
        .add_card(make_card("T1"))
        .hand(0, &["H1"])
        .start();
    let tp = r.game.turn_player();
    push_to_trash(&mut r, tp, "T1");

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_union_zone(
            tp,
            UnionZoneSet::HAND | UnionZoneSet::TRASH,
            "pick one",
            false,
            |_, _| true,
            |_, _| {},
        );
    }

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("select_union_zone must install a pending selection");
    assert_eq!(
        sel.kind,
        SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::TRASH
        }
    );
    assert_eq!(r.game.current_phase, GamePhase::SelectUnion);
    assert_eq!(sel.selecting_player, tp);
}

// ── valid_action_ids coverage ────────────────────────────────────────────────

#[test]
fn valid_action_ids_covers_both_zones() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("H1"))
        .add_card(make_card("H2"))
        .add_card(make_card("T1"))
        .add_card(make_card("T2"))
        .hand(0, &["H1", "H2"])
        .start();
    let tp = r.game.turn_player();
    push_to_trash(&mut r, tp, "T1");
    push_to_trash(&mut r, tp, "T2");

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_union_zone(
            tp,
            UnionZoneSet::HAND | UnionZoneSet::TRASH,
            "pick",
            false,
            |_, _| true,
            |_, _| {},
        );
    }

    let sel = r.game.pending_selection.as_ref().unwrap();
    assert_eq!(sel.valid_action_ids.len(), 4, "2 hand + 2 trash = 4 IDs");

    // Hand IDs in PLAY_HAND range
    let hand_ids: Vec<u16> = sel
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id < PLAY_HAND_START + 30)
        .collect();
    assert_eq!(hand_ids.len(), 2);
    assert!(hand_ids.contains(&(PLAY_HAND_START)));
    assert!(hand_ids.contains(&(PLAY_HAND_START + 1)));

    // Trash IDs in TRASH_EFFECT range
    let trash_ids: Vec<u16> = sel
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id >= TRASH_EFFECT_START)
        .collect();
    assert_eq!(trash_ids.len(), 2);
    assert!(trash_ids.contains(&TRASH_EFFECT_START));
    assert!(trash_ids.contains(&(TRASH_EFFECT_START + 1)));
}

// ── filter ───────────────────────────────────────────────────────────────────

#[test]
fn filter_restricts_valid_action_ids() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("H1"))
        .add_card(make_card("H2"))
        .add_card(make_card("T1"))
        .add_card(make_card("T2"))
        .hand(0, &["H1", "H2"])
        .start();
    let tp = r.game.turn_player();
    push_to_trash(&mut r, tp, "T1");
    push_to_trash(&mut r, tp, "T2");

    // Only allow cards whose card_id starts with "T" (i.e. T1 / T2 in trash).
    // The filter receives a &CardSource — look up the card_id via game.card_data.
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_union_zone(
            tp,
            UnionZoneSet::HAND | UnionZoneSet::TRASH,
            "trash only",
            false,
            |game, card| game.card_data[card.data_index].card_id.starts_with('T'),
            |_, _| {},
        );
    }

    let sel = r.game.pending_selection.as_ref().unwrap();
    assert_eq!(
        sel.valid_action_ids.len(),
        2,
        "only 2 trash cards pass the filter"
    );
    assert!(sel
        .valid_action_ids
        .iter()
        .all(|&id| id >= TRASH_EFFECT_START));
}

// ── empty filter → no-op ─────────────────────────────────────────────────────

#[test]
fn empty_filter_is_noop() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("H1"))
        .add_card(make_card("T1"))
        .hand(0, &["H1"])
        .start();
    let tp = r.game.turn_player();
    push_to_trash(&mut r, tp, "T1");
    let phase_before = r.game.current_phase;

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_union_zone(
            tp,
            UnionZoneSet::HAND | UnionZoneSet::TRASH,
            "none eligible",
            true,
            |_, _| false,
            |_, _| {},
        );
    }

    assert!(
        r.game.pending_selection.is_none(),
        "empty filter produces no prompt"
    );
    assert_eq!(
        r.game.current_phase, phase_before,
        "phase must be unchanged when no-op"
    );
}

// ── hand-only / trash-only zones ─────────────────────────────────────────────

#[test]
fn hand_only_zone_excludes_trash() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("H1"))
        .add_card(make_card("T1"))
        .hand(0, &["H1"])
        .start();
    let tp = r.game.turn_player();
    push_to_trash(&mut r, tp, "T1");

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_union_zone(
            tp,
            UnionZoneSet::HAND,
            "hand only",
            false,
            |_, _| true,
            |_, _| {},
        );
    }

    let sel = r.game.pending_selection.as_ref().unwrap();
    assert_eq!(sel.valid_action_ids.len(), 1);
    assert_eq!(sel.valid_action_ids[0], PLAY_HAND_START);
}

#[test]
fn trash_only_zone_excludes_hand() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("H1"))
        .add_card(make_card("T1"))
        .hand(0, &["H1"])
        .start();
    let tp = r.game.turn_player();
    push_to_trash(&mut r, tp, "T1");

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_union_zone(
            tp,
            UnionZoneSet::TRASH,
            "trash only",
            false,
            |_, _| true,
            |_, _| {},
        );
    }

    let sel = r.game.pending_selection.as_ref().unwrap();
    assert_eq!(sel.valid_action_ids.len(), 1);
    assert_eq!(sel.valid_action_ids[0], TRASH_EFFECT_START);
}

// ── callback resolution ───────────────────────────────────────────────────────

#[test]
fn callback_receives_correct_handle_for_trash_pick() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("H1"))
        .add_card(make_card("H2"))
        .add_card(make_card("T1"))
        .add_card(make_card("T2"))
        .hand(0, &["H1", "H2"])
        .start();
    let tp = r.game.turn_player();
    push_to_trash(&mut r, tp, "T1");
    push_to_trash(&mut r, tp, "T2");

    // Record the card_index of the second trash card (index 1 = T2).
    let expected_handle = r.game.player(tp).trash[1].handle();

    let observed: Arc<Mutex<Option<CardHandle>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&observed);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_union_zone(
            tp,
            UnionZoneSet::HAND | UnionZoneSet::TRASH,
            "pick",
            false,
            |_, _| true,
            move |_, handle| {
                *slot.lock().unwrap() = Some(handle);
            },
        );
    }

    // Resolve with trash index 1 → TRASH_EFFECT_START + 1
    let action_id = TRASH_EFFECT_START + 1;
    r.game.resolve_selection(tp, action_id).unwrap();

    assert_eq!(
        *observed.lock().unwrap(),
        Some(expected_handle),
        "callback must receive the handle of the second trash card"
    );
    assert_ne!(
        r.game.current_phase,
        GamePhase::SelectUnion,
        "previous_phase must be restored after resolution"
    );
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn callback_receives_correct_handle_for_hand_pick() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("H1"))
        .add_card(make_card("H2"))
        .add_card(make_card("T1"))
        .hand(0, &["H1", "H2"])
        .start();
    let tp = r.game.turn_player();
    push_to_trash(&mut r, tp, "T1");

    let expected_handle = r.game.player(tp).hand[0].handle();

    let observed: Arc<Mutex<Option<CardHandle>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&observed);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_union_zone(
            tp,
            UnionZoneSet::HAND | UnionZoneSet::TRASH,
            "pick",
            false,
            |_, _| true,
            move |_, handle| {
                *slot.lock().unwrap() = Some(handle);
            },
        );
    }

    // Resolve with hand index 0 → PLAY_HAND_START + 0
    r.game.resolve_selection(tp, PLAY_HAND_START).unwrap();

    assert_eq!(
        *observed.lock().unwrap(),
        Some(expected_handle),
        "callback must receive the handle of the first hand card"
    );
    assert!(r.game.pending_selection.is_none());
}

// ── optional / PASS ───────────────────────────────────────────────────────────

#[test]
fn mandatory_rejects_pass() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("H1"))
        .hand(0, &["H1"])
        .start();
    let tp = r.game.turn_player();

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_union_zone(
            tp,
            UnionZoneSet::HAND,
            "must pick",
            false,
            |_, _| true,
            |_, _| {},
        );
    }

    let err = r
        .game
        .resolve_selection(tp, PASS)
        .expect_err("mandatory selection must reject PASS");
    assert_eq!(err, SelectionError::InvalidAction);
    assert!(r.game.pending_selection.is_some());
}

#[test]
fn optional_accepts_pass() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("H1"))
        .hand(0, &["H1"])
        .start();
    let tp = r.game.turn_player();

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_union_zone(
            tp,
            UnionZoneSet::HAND,
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
        .expect("optional selection must accept PASS");
    assert!(r.game.pending_selection.is_none());
}
