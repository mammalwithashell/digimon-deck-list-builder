//! Task 0 — `CountCappedZone::Material(PermanentHandle)` selects sources from
//! a permanent's digivolution stack.
//!
//! `card_sources` layout: index 0 = bottom card, `last()` = top card (confirmed
//! by `Permanent::top_card()` returning `card_sources.last()`). The Material
//! zone presents indices 0 through `len-2` as candidates — i.e., everything
//! EXCEPT the top card (the top card is the identity of the permanent and must
//! not be picked as a digivolution source to discard).
//!
//! Action IDs encode into the SOURCE_SELECT range:
//!   `SOURCE_SELECT_START + field_index * SOURCES_PER_FIELD + source_index`
//! so that the mask builder can expose them.

use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{PASS, SOURCES_PER_FIELD, SOURCE_SELECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::{CountCappedZone, EffectContext};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
use digimon_engine::selection::SelectionKind;

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
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// Append `card_id` on top of a field permanent's digivolution stack.
/// Since `top_card() = card_sources.last()`, calling this after `place_on_field`
/// displaces the previous top to a source position and makes `card_id` the new top.
fn push_source_card(r: &mut DebugRunner, field_index: usize, card_id: &str) {
    let tp = r.game.turn_player();
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_source_card: unknown card_id {}", card_id));
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, tp, card_index);
    r.game.players[tp as usize].battle_area[field_index]
        .card_sources
        .push(card);
}

// ── Test 1: collects permanent card sources (excluding top) ─────────────────

/// Stack layout: [SRC-A, SRC-B, SRC-C, TEST-MATZONE] (index 0=bottom, last=top).
/// `select_count_capped_multi` with `Material(handle)` should offer SRC-A,
/// SRC-B, SRC-C (indices 0, 1, 2) and NOT offer TEST-MATZONE (index 3 = top).
#[test]
fn material_zone_collects_permanent_card_sources() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("SRC-A"))
        .add_card(make_digimon("SRC-B"))
        .add_card(make_digimon("SRC-C"))
        .add_card(make_digimon("TEST-MATZONE"))
        .start();
    let tp = r.game.turn_player();

    // Place the top card on the field.
    let perm_handle = r.place_on_field(tp, "SRC-A", Some(0));
    let field_idx = perm_handle.index as usize;

    // Append SRC-B, SRC-C, then TEST-MATZONE on top (pushed last = top).
    push_source_card(&mut r, field_idx, "SRC-B");
    push_source_card(&mut r, field_idx, "SRC-C");
    push_source_card(&mut r, field_idx, "TEST-MATZONE");

    // Stack is now: [SRC-A, SRC-B, SRC-C, TEST-MATZONE]
    // top_card() == TEST-MATZONE (index 3); sources[0..=2] are the 3 picks.
    {
        let perm = &r.game.players[tp as usize].battle_area[field_idx];
        assert_eq!(perm.card_sources.len(), 4);
        assert_eq!(perm.top_card().card_id(& r.game.card_data), "TEST-MATZONE");
    }

    let delivered: Arc<Mutex<Option<Vec<CardHandle>>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&delivered);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), Some(perm_handle), tp);
        ctx.select_count_capped_multi(
            tp,
            CountCappedZone::Material(perm_handle),
            /*max=*/ 2,
            "select sources",
            /*is_optional_zero=*/ false,
            None,
            |_g, _src| true,
            move |_ctx, picks| {
                *slot.lock().unwrap() = Some(picks);
            },
        );
    }

    // Should park a selection (3 sources available, not the top).
    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("Material zone must park a selection when sources exist");

    assert_eq!(
        sel.kind,
        SelectionKind::CountCappedMultiSelect { max: 2, picked: 0 },
        "kind must be CountCappedMultiSelect {{ max: 2, picked: 0 }}"
    );
    assert_eq!(r.game.current_phase, GamePhase::SelectBudgeted);

    // The 3 source-index action IDs (0, 1, 2) should be offered.
    let base = SOURCE_SELECT_START + perm_handle.index as u16 * SOURCES_PER_FIELD;
    for src_idx in 0u16..3 {
        assert!(
            sel.valid_action_ids.contains(&(base + src_idx)),
            "source {} must be offered (action id {})",
            src_idx,
            base + src_idx
        );
    }
    // The top card (index 3) must NOT be offered.
    assert!(
        !sel.valid_action_ids.contains(&(base + 3)),
        "top card (index 3) must NOT be offered as a source candidate"
    );

    // Pick source at index 0 (SRC-A).
    r.game.resolve_selection(tp, base + 0).unwrap();

    // Now pick source at index 1 (SRC-B) — but SRC-A removed from candidates,
    // so its action id must not appear. SRC-C (index 2) stays.
    {
        let sel2 = r.game.pending_selection.as_ref().expect("step 2 selection");
        assert_eq!(
            sel2.kind,
            SelectionKind::CountCappedMultiSelect { max: 2, picked: 1 }
        );
        // SRC-A's action id must be gone.
        assert!(
            !sel2.valid_action_ids.contains(&(base + 0)),
            "SRC-A already picked, must not reappear"
        );
        // SRC-B and SRC-C must remain.
        assert!(
            sel2.valid_action_ids.contains(&(base + 1)),
            "SRC-B must still be available"
        );
        assert!(
            sel2.valid_action_ids.contains(&(base + 2)),
            "SRC-C must still be available"
        );
    }

    // Pick source at index 2 (SRC-C) — reaches max=2, auto-commits.
    r.game.resolve_selection(tp, base + 2).unwrap();

    assert!(
        r.game.pending_selection.is_none(),
        "no selection after auto-commit at max=2"
    );

    // Callback must have delivered [SRC-A handle, SRC-C handle].
    let result = delivered
        .lock()
        .unwrap()
        .take()
        .expect("callback must fire after auto-commit");
    assert_eq!(result.len(), 2, "callback must deliver exactly 2 handles");
}

// ── Test 2: zero-source permanent yields empty candidates (no selection) ─────

/// A permanent with only 1 card (no digivolution sources) should cause
/// `select_count_capped_multi` to fire the callback immediately with an
/// empty Vec (same no-op behavior as the Hand/Trash empty-filter path).
#[test]
fn material_zone_with_no_sources_yields_no_candidates() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("SOLO-TOP"))
        .start();
    let tp = r.game.turn_player();
    let perm_handle = r.place_on_field(tp, "SOLO-TOP", Some(0));

    // Only 1 card in the stack (the top), so 0 source candidates.
    {
        let perm = &r.game.players[tp as usize].battle_area[perm_handle.index as usize];
        assert_eq!(perm.card_sources.len(), 1);
    }

    let fired: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let flag = Arc::clone(&fired);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), Some(perm_handle), tp);
        ctx.select_count_capped_multi(
            tp,
            CountCappedZone::Material(perm_handle),
            /*max=*/ 2,
            "pick sources (should be empty)",
            /*is_optional_zero=*/ true,
            None,
            |_g, _src| true,
            move |_ctx, picks| {
                assert!(picks.is_empty(), "empty zone must deliver empty Vec");
                *flag.lock().unwrap() = true;
            },
        );
    }

    assert!(
        *fired.lock().unwrap(),
        "callback must fire immediately when no sources available"
    );
    assert!(
        r.game.pending_selection.is_none(),
        "no PendingSelection when source list is empty"
    );
    assert_ne!(
        r.game.current_phase,
        GamePhase::SelectBudgeted,
        "phase must be unchanged for empty-zone no-op"
    );
}

// ── Test 3: pass commits early with is_optional_zero=true ─────────────────

/// Stack with 3 sources + top; is_optional_zero=true so PASS available at
/// step 0. Submit PASS immediately → callback delivers empty Vec.
#[test]
fn material_zone_optional_zero_allows_pass_at_start() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("SRC-A"))
        .add_card(make_digimon("SRC-B"))
        .add_card(make_digimon("TOP-CARD"))
        .start();
    let tp = r.game.turn_player();
    let perm_handle = r.place_on_field(tp, "SRC-A", Some(0));
    push_source_card(&mut r, perm_handle.index as usize, "SRC-B");
    push_source_card(&mut r, perm_handle.index as usize, "TOP-CARD");
    // Stack: [SRC-A, SRC-B, TOP-CARD]; sources = [SRC-A, SRC-B]

    let delivered: Arc<Mutex<Option<Vec<CardHandle>>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&delivered);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), Some(perm_handle), tp);
        ctx.select_count_capped_multi(
            tp,
            CountCappedZone::Material(perm_handle),
            /*max=*/ 3,
            "pick up to 3 (optional)",
            /*is_optional_zero=*/ true,
            None,
            |_g, _src| true,
            move |_ctx, picks| {
                *slot.lock().unwrap() = Some(picks);
            },
        );
    }

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("selection must be installed");
    assert!(
        sel.is_optional,
        "is_optional must be true at picked=0 when is_optional_zero=true"
    );

    // Submit PASS → callback delivered empty Vec.
    r.game.resolve_selection(tp, PASS).unwrap();
    assert!(r.game.pending_selection.is_none());

    let result = delivered
        .lock()
        .unwrap()
        .take()
        .expect("callback must fire on PASS");
    assert!(result.is_empty(), "PASS at step 0 delivers empty Vec");
}
