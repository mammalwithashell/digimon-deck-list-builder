//! `select_count_capped_multi` helper tests.
//!
//! `EffectContext::select_count_capped_multi` prompts a player to pick *up to*
//! N cards from a zone. Each pick reuses the existing zone action range (hand:
//! `PLAY_HAND_START + i`, trash: `TRASH_EFFECT_START + i`). `PASS` (action 62)
//! commits early; reaching `picked == max` auto-commits. Phase parks at
//! `GamePhase::SelectBudgeted` with `kind = CountCappedMultiSelect { max, picked }`.

use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::{CountCappedZone, EffectContext};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
use digimon_engine::selection::{SelectionError, SelectionKind};

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

/// Push a card into a player's hand and return its handle.
fn push_to_hand(
    r: &mut DebugRunner,
    player: digimon_engine::enums::PlayerId,
    card_id: &str,
) -> CardHandle {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card_id {}", card_id));
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    let handle = card.handle();
    r.game.players[player as usize].hand.push(card);
    handle
}

// ── Test 1: auto_commits_at_max ──────────────────────────────────────────────

/// 3 hand cards, max=2, is_optional_zero=false. Pick card at index 0 (picked=1),
/// then pick card at index 2 (picked==max → auto-commit). Assert callback
/// delivered [card_0, card_2].
#[test]
fn auto_commits_at_max() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("C0"))
        .add_card(make_card("C1"))
        .add_card(make_card("C2"))
        .start();
    let tp = r.game.turn_player();
    let h0 = push_to_hand(&mut r, tp, "C0");
    let _h1 = push_to_hand(&mut r, tp, "C1");
    let h2 = push_to_hand(&mut r, tp, "C2");

    let delivered: Arc<Mutex<Option<Vec<CardHandle>>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&delivered);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_count_capped_multi(
            tp,
            CountCappedZone::Hand,
            2,
            "pick up to 2",
            false,
            None,
            |_, _| true,
            move |_, picks| {
                *slot.lock().unwrap() = Some(picks);
            },
        );
    }

    // Step 1: picked=0 — 3 hand picks available, PASS not available yet
    {
        let sel = r.game.pending_selection.as_ref().expect("step 1 must park selection");
        assert_eq!(
            sel.kind,
            SelectionKind::CountCappedMultiSelect { max: 2, picked: 0 },
            "step 1: kind must be CountCappedMultiSelect {{ max: 2, picked: 0 }}"
        );
        assert_eq!(r.game.current_phase, GamePhase::SelectBudgeted);
        assert!(
            sel.valid_action_ids.contains(&(PLAY_HAND_START)),
            "card 0 must be pickable"
        );
        assert!(
            sel.valid_action_ids.contains(&(PLAY_HAND_START + 2)),
            "card 2 must be pickable"
        );
        // PASS not available at picked=0 when not optional (is_optional == false)
        assert!(
            !sel.is_optional,
            "is_optional must be false at picked=0 (PASS not available)"
        );
    }

    // Pick card 0 (PLAY_HAND_START + 0)
    r.game.resolve_selection(tp, PLAY_HAND_START).unwrap();

    // Step 2: picked=1 — card 0 excluded, PASS now available (via is_optional)
    {
        let sel = r.game.pending_selection.as_ref().expect("step 2 must park selection");
        assert_eq!(
            sel.kind,
            SelectionKind::CountCappedMultiSelect { max: 2, picked: 1 },
            "step 2: kind must be CountCappedMultiSelect {{ max: 2, picked: 1 }}"
        );
        assert_eq!(r.game.current_phase, GamePhase::SelectBudgeted);
        // card 0 was picked — must not appear in valid_action_ids
        assert!(
            !sel.valid_action_ids.contains(&PLAY_HAND_START),
            "card 0 must be excluded from step 2"
        );
        // card 2 (index 2) is still available
        assert!(
            sel.valid_action_ids.contains(&(PLAY_HAND_START + 2)),
            "card 2 must remain available"
        );
        // PASS is now available (picked >= 1) — gated via is_optional
        assert!(
            sel.is_optional,
            "is_optional must be true at picked=1 (PASS available)"
        );
    }

    // Pick card 2 (PLAY_HAND_START + 2) — reaches max, auto-commits
    r.game.resolve_selection(tp, PLAY_HAND_START + 2).unwrap();

    // No further selection installed (auto-committed at max=2)
    assert!(
        r.game.pending_selection.is_none(),
        "no selection must remain after auto-commit at max"
    );
    assert_ne!(
        r.game.current_phase,
        GamePhase::SelectBudgeted,
        "phase must be restored after auto-commit"
    );

    // Callback delivered [card_0, card_2]
    let result = delivered.lock().unwrap().take().expect("callback must fire on auto-commit");
    assert_eq!(
        result,
        vec![h0, h2],
        "callback must deliver [card_0, card_2] in pick order"
    );
}

// ── Test 2: pass_commits_early_when_picked_ge_1 ──────────────────────────────

/// 3 cards, max=3, is_optional_zero=false. Pick card 1, then PASS → callback
/// delivered [card_1].
#[test]
fn pass_commits_early_when_picked_ge_1() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("C0"))
        .add_card(make_card("C1"))
        .add_card(make_card("C2"))
        .start();
    let tp = r.game.turn_player();
    let _h0 = push_to_hand(&mut r, tp, "C0");
    let h1 = push_to_hand(&mut r, tp, "C1");
    let _h2 = push_to_hand(&mut r, tp, "C2");

    let delivered: Arc<Mutex<Option<Vec<CardHandle>>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&delivered);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_count_capped_multi(
            tp,
            CountCappedZone::Hand,
            3,
            "pick up to 3",
            false,
            None,
            |_, _| true,
            move |_, picks| {
                *slot.lock().unwrap() = Some(picks);
            },
        );
    }

    // Pick card 1 (PLAY_HAND_START + 1)
    r.game.resolve_selection(tp, PLAY_HAND_START + 1).unwrap();

    // Now PASS is available (picked=1 >= 1) — gated via is_optional
    {
        let sel = r.game.pending_selection.as_ref().expect("step 2 must park selection");
        assert_eq!(
            sel.kind,
            SelectionKind::CountCappedMultiSelect { max: 3, picked: 1 }
        );
        assert!(
            sel.is_optional,
            "is_optional must be true at picked=1 (PASS available)"
        );
    }

    // Submit PASS → early commit
    r.game.resolve_selection(tp, PASS).unwrap();

    assert!(r.game.pending_selection.is_none(), "no selection after PASS commit");
    assert_ne!(r.game.current_phase, GamePhase::SelectBudgeted);

    let result = delivered.lock().unwrap().take().expect("callback must fire on PASS commit");
    assert_eq!(result, vec![h1], "callback must deliver [card_1]");
}

// ── Test 3: pass_rejected_when_picked_zero_and_not_optional ─────────────────

/// 3 cards, max=2, is_optional_zero=false. Assert PASS not in valid_action_ids
/// at step 0, and resolve_selection with PASS returns InvalidAction.
#[test]
fn pass_rejected_when_picked_zero_and_not_optional() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("C0"))
        .add_card(make_card("C1"))
        .add_card(make_card("C2"))
        .start();
    let tp = r.game.turn_player();
    push_to_hand(&mut r, tp, "C0");
    push_to_hand(&mut r, tp, "C1");
    push_to_hand(&mut r, tp, "C2");

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_count_capped_multi(
            tp,
            CountCappedZone::Hand,
            2,
            "pick up to 2",
            false,
            None,
            |_, _| true,
            |_, _| {},
        );
    }

    // PASS must not be available at picked=0 with is_optional_zero=false
    // is_optional == false means PASS is not gated
    {
        let sel = r.game.pending_selection.as_ref().expect("selection must be installed");
        assert_eq!(
            sel.is_optional, false,
            "is_optional must be false at picked=0 (not optional)"
        );
        // PASS must not be in valid_action_ids either
        assert!(
            !sel.valid_action_ids.contains(&PASS),
            "PASS must not appear in valid_action_ids at step 0 (not optional)"
        );
    }

    // Attempting to resolve with PASS must return InvalidAction
    let err = r
        .game
        .resolve_selection(tp, PASS)
        .expect_err("PASS must be rejected when not optional and picked=0");
    assert_eq!(
        err,
        SelectionError::InvalidAction,
        "error must be InvalidAction"
    );

    // Selection must still be pending
    assert!(r.game.pending_selection.is_some(), "selection must remain after rejected PASS");
}

// ── Test 4: optional_zero_allows_pass_at_start ───────────────────────────────

/// 3 cards, max=2, is_optional_zero=true. Assert PASS is in valid_action_ids
/// at step 0. Resolve PASS → callback delivered empty Vec.
#[test]
fn optional_zero_allows_pass_at_start() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("C0"))
        .add_card(make_card("C1"))
        .add_card(make_card("C2"))
        .start();
    let tp = r.game.turn_player();
    push_to_hand(&mut r, tp, "C0");
    push_to_hand(&mut r, tp, "C1");
    push_to_hand(&mut r, tp, "C2");

    let delivered: Arc<Mutex<Option<Vec<CardHandle>>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&delivered);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_count_capped_multi(
            tp,
            CountCappedZone::Hand,
            2,
            "pick up to 2 (optional)",
            true, // is_optional_zero = true
            None,
            |_, _| true,
            move |_, picks| {
                *slot.lock().unwrap() = Some(picks);
            },
        );
    }

    // PASS must be available at picked=0 when is_optional_zero=true — gated via is_optional
    {
        let sel = r.game.pending_selection.as_ref().expect("selection must be installed");
        assert!(
            sel.is_optional,
            "is_optional must be true at picked=0 when is_optional_zero=true"
        );
        assert_eq!(
            sel.kind,
            SelectionKind::CountCappedMultiSelect { max: 2, picked: 0 }
        );
    }

    // Resolve PASS → callback delivered empty Vec
    r.game.resolve_selection(tp, PASS).unwrap();

    assert!(r.game.pending_selection.is_none(), "no selection after PASS at step 0");
    assert_ne!(r.game.current_phase, GamePhase::SelectBudgeted);

    let result = delivered
        .lock()
        .unwrap()
        .take()
        .expect("callback must fire on PASS at step 0 (optional)");
    assert!(result.is_empty(), "callback must deliver empty Vec on optional PASS at start");
}

// ── Test 5: picked_items_excluded_from_next_step ─────────────────────────────

/// 3 cards, max=3. Pick card 0 → next step's valid_action_ids excludes
/// card 0's action id (PLAY_HAND_START + 0).
#[test]
fn picked_items_excluded_from_next_step() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("C0"))
        .add_card(make_card("C1"))
        .add_card(make_card("C2"))
        .start();
    let tp = r.game.turn_player();
    push_to_hand(&mut r, tp, "C0");
    push_to_hand(&mut r, tp, "C1");
    push_to_hand(&mut r, tp, "C2");

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_count_capped_multi(
            tp,
            CountCappedZone::Hand,
            3,
            "pick up to 3",
            false,
            None,
            |_, _| true,
            |_, _| {},
        );
    }

    // Step 1: all 3 cards available (no PASS yet — is_optional == false)
    {
        let sel = r.game.pending_selection.as_ref().expect("step 1 selection");
        assert_eq!(
            sel.valid_action_ids.len(),
            3,
            "all 3 hand cards must be offered at step 1"
        );
        assert!(!sel.is_optional, "is_optional must be false at step 1 (PASS not available)");
    }

    // Pick card 0
    r.game.resolve_selection(tp, PLAY_HAND_START).unwrap();

    // Step 2: card 0 must be excluded
    {
        let sel = r.game.pending_selection.as_ref().expect("step 2 selection");
        assert!(
            !sel.valid_action_ids.contains(&PLAY_HAND_START),
            "PLAY_HAND_START (card 0) must be excluded after picking it"
        );
        assert!(
            sel.valid_action_ids.contains(&(PLAY_HAND_START + 1)),
            "card 1 must still be available"
        );
        assert!(
            sel.valid_action_ids.contains(&(PLAY_HAND_START + 2)),
            "card 2 must still be available"
        );
    }
}

// ── Test 6: kind_reflects_picked_counter ─────────────────────────────────────

/// At each step, `kind == CountCappedMultiSelect { max: 2, picked: N }` where
/// N increments from 0 → 1 after each pick.
#[test]
fn kind_reflects_picked_counter() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("C0"))
        .add_card(make_card("C1"))
        .add_card(make_card("C2"))
        .start();
    let tp = r.game.turn_player();
    push_to_hand(&mut r, tp, "C0");
    push_to_hand(&mut r, tp, "C1");
    push_to_hand(&mut r, tp, "C2");

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_count_capped_multi(
            tp,
            CountCappedZone::Hand,
            2,
            "pick up to 2",
            false,
            None,
            |_, _| true,
            |_, _| {},
        );
    }

    // Step 1: picked=0
    {
        let sel = r.game.pending_selection.as_ref().expect("step 1");
        assert_eq!(
            sel.kind,
            SelectionKind::CountCappedMultiSelect { max: 2, picked: 0 },
            "step 1: kind must have picked=0"
        );
    }

    // Pick card 0
    r.game.resolve_selection(tp, PLAY_HAND_START).unwrap();

    // Step 2: picked=1
    {
        let sel = r.game.pending_selection.as_ref().expect("step 2");
        assert_eq!(
            sel.kind,
            SelectionKind::CountCappedMultiSelect { max: 2, picked: 1 },
            "step 2: kind must have picked=1"
        );
    }
}

// ── Test 7: trash zone variant ────────────────────────────────────────────────

/// Verify CountCappedZone::Trash uses TRASH_EFFECT_START range.
#[test]
fn trash_zone_uses_correct_range() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("T0"))
        .add_card(make_card("T1"))
        .start();
    let tp = r.game.turn_player();

    // Push cards directly into trash
    {
        let data_idx0 = r.game.card_data.iter().position(|c| c.card_id == "T0").unwrap();
        let data_idx1 = r.game.card_data.iter().position(|c| c.card_id == "T1").unwrap();
        let ci0 = r.game.next_card_index();
        r.game.players[tp as usize].trash.push(CardSource::new(data_idx0, tp, ci0));
        let ci1 = r.game.next_card_index();
        r.game.players[tp as usize].trash.push(CardSource::new(data_idx1, tp, ci1));
    }

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_count_capped_multi(
            tp,
            CountCappedZone::Trash,
            2,
            "pick up to 2 from trash",
            false,
            None,
            |_, _| true,
            |_, _| {},
        );
    }

    let sel = r.game.pending_selection.as_ref().expect("selection must be installed");
    assert_eq!(r.game.current_phase, GamePhase::SelectBudgeted);

    // All valid_action_ids must be in TRASH_EFFECT range
    for &aid in &sel.valid_action_ids {
        if aid == PASS {
            continue;
        }
        assert!(
            aid >= TRASH_EFFECT_START && aid < TRASH_EFFECT_START + 45,
            "action id {} must be in TRASH_EFFECT range",
            aid
        );
    }
    assert!(sel.valid_action_ids.contains(&TRASH_EFFECT_START));
    assert!(sel.valid_action_ids.contains(&(TRASH_EFFECT_START + 1)));
}

// ── Test 8: empty_filter_is_noop ─────────────────────────────────────────────

/// When no cards pass the filter, callback fires immediately with empty Vec;
/// no PendingSelection installed.
#[test]
fn empty_filter_is_noop() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("C0"))
        .add_card(make_card("C1"))
        .start();
    let tp = r.game.turn_player();
    push_to_hand(&mut r, tp, "C0");
    push_to_hand(&mut r, tp, "C1");
    let phase_before = r.game.current_phase;

    let fired: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let flag = Arc::clone(&fired);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_count_capped_multi(
            tp,
            CountCappedZone::Hand,
            2,
            "pick (none pass)",
            false,
            None,
            |_, _| false, // filter rejects all
            move |_, picks| {
                assert!(picks.is_empty(), "no-op must deliver empty Vec");
                *flag.lock().unwrap() = true;
            },
        );
    }

    assert!(
        *fired.lock().unwrap(),
        "callback must fire immediately when no cards pass filter"
    );
    assert!(
        r.game.pending_selection.is_none(),
        "no pending selection when filter passes nothing"
    );
    assert_eq!(
        r.game.current_phase, phase_before,
        "phase must be unchanged for empty-filter no-op"
    );
}
