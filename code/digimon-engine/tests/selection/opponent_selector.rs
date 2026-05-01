//! Tests for `EffectContext::as_selecting_player` — the opponent-as-selector
//! builder (Task 5 / Phase 4).
//!
//! Covers:
//! - `select_own_permanent` routed to the opponent as selector
//! - `select_effect_choice` routed to the opponent as selector
//! - `select_hand` routed to the opponent as selector
//! - The override does NOT apply when calling helpers directly (no scope)
//! - The override is cleared after the scope method returns (no leak to a second install)

use std::sync::{Arc, Mutex};

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

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
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn push_to_hand(r: &mut DebugRunner, player: PlayerId, card_id: &str) -> CardHandle {
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

// ── Test 1 ───────────────────────────────────────────────────────────────────

/// `as_selecting_player(1).select_own_permanent(...)` installs a selection
/// that routes to player 1 (the opponent) as the selector, even though player 0
/// is the effect controller.
///
/// - `pending_selection.selecting_player == 1`
/// - Player 0's mask has no selection actions set
/// - Player 1's mask has the expected action IDs set
/// - Resolving from player 1 fires the callback with player 0's permanent
#[test]
fn as_selecting_player_routes_to_opponent_for_own_permanent() {
    // Controller is player 0 (turn player). Place two permanents for player 0.
    let mut r = DebugRunner::builder().add_card(make_card("ALLY")).start();
    let p0 = r.game.turn_player(); // 0
    let p1: PlayerId = 1 - p0;

    // Place two field permanents for player 0.
    r.place_on_field(p0, "ALLY", Some(0));
    r.place_on_field(p0, "ALLY", Some(0));

    // Install selection: player 1 chooses one of player 0's permanents.
    let fired: Arc<Mutex<Option<PermanentHandle>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&fired);
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, p0);
        ctx.as_selecting_player(p1).select_own_permanent(
            "Opponent: choose one of controller's Digimon",
            false,
            |_g, _h| true,
            move |_ctx, h| {
                *slot.lock().unwrap() = Some(h);
            },
        );
    }

    // Pending selection must exist and route to player 1.
    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("selection must be installed");
    assert_eq!(
        sel.selecting_player, p1,
        "selecting_player must be the opponent (p1)"
    );
    assert_eq!(sel.kind, SelectionKind::OwnField);

    // Player 0 (controller) must see NO valid action bits.
    let mask_p0 = build_action_mask(&r.game, p0);
    let legal_p0: Vec<usize> = mask_p0
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| (v > 0.0).then_some(i))
        .collect();
    assert!(
        legal_p0.is_empty(),
        "controller (p0) must see an empty mask, got {:?}",
        legal_p0
    );

    // Player 1 (selector) must see the two field-slot action IDs.
    let mask_p1 = build_action_mask(&r.game, p1);
    let legal_p1: Vec<usize> = mask_p1
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| (v > 0.0).then_some(i))
        .collect();
    assert!(
        legal_p1.contains(&(encode_attack(0, 0) as usize)),
        "p1 mask must expose slot 0 of p0's field"
    );
    assert!(
        legal_p1.contains(&(encode_attack(0, 1) as usize)),
        "p1 mask must expose slot 1 of p0's field"
    );

    // Resolve from player 1 — pick slot 0.
    r.game
        .resolve_selection(p1, encode_attack(0, 0))
        .expect("p1 resolving is valid");

    assert!(
        r.game.pending_selection.is_none(),
        "selection cleared after resolution"
    );

    let handle = fired
        .lock()
        .unwrap()
        .take()
        .expect("callback must fire on resolution");
    assert_eq!(
        handle.player, p0,
        "resolved handle must point at player 0's permanent"
    );
    assert_eq!(handle.index, 0);
}

// ── Test 2 ───────────────────────────────────────────────────────────────────

/// `as_selecting_player(opp).select_effect_choice(...)` installs a choice
/// selection where the opponent must pick the branch.
///
/// - `selecting_player == opp`
/// - Resolving from opp fires the chosen branch
#[test]
fn as_selecting_player_routes_to_opponent_for_effect_choice() {
    let mut r = DebugRunner::builder().add_card(make_card("X")).start();
    let p0 = r.game.turn_player();
    let opp: PlayerId = 1 - p0;

    let chosen: Arc<Mutex<Option<usize>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&chosen);
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, p0);
        ctx.as_selecting_player(opp).select_effect_choice(
            "Opponent chooses",
            vec!["Branch A".into(), "Branch B".into()],
            move |_ctx, idx| {
                *slot.lock().unwrap() = Some(idx);
            },
        );
    }

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("selection must be installed");
    assert_eq!(
        sel.selecting_player, opp,
        "selecting_player must be the opponent"
    );
    assert_eq!(sel.kind, SelectionKind::EffectChoice);

    // Resolve: opponent picks branch index 1.
    use digimon_engine::action::space::HAND_EFFECT_START;
    r.game
        .resolve_selection(opp, HAND_EFFECT_START + 1)
        .expect("valid branch resolution");

    assert!(r.game.pending_selection.is_none());
    assert_eq!(
        *chosen.lock().unwrap(),
        Some(1),
        "callback must fire with chosen index 1"
    );
}

// ── Test 3 ───────────────────────────────────────────────────────────────────

/// `as_selecting_player(1).select_hand(0, ...)` — player 1 chooses a card
/// from player 0's hand.
///
/// - `selecting_player == 1`
/// - `of_player == 0` (the hand being iterated belongs to p0)
/// - Resolving from player 1 fires the callback with the correct zone index
#[test]
fn as_selecting_player_routes_to_opponent_for_hand() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("H0"))
        .add_card(make_card("H1"))
        .start();
    let p0 = r.game.turn_player();
    let p1: PlayerId = 1 - p0;

    // Put two cards in p0's hand.
    push_to_hand(&mut r, p0, "H0");
    push_to_hand(&mut r, p0, "H1");

    let hand_idx_cb: Arc<Mutex<Option<usize>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&hand_idx_cb);
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, p0);
        ctx.as_selecting_player(p1).select_hand(
            p0, // of_player — whose hand is being picked from
            "Opponent: choose a card from p0's hand",
            false,
            |_g, _i| true,
            move |_ctx, hand_index| {
                *slot.lock().unwrap() = Some(hand_index);
            },
        );
    }

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("selection must be installed");
    assert_eq!(
        sel.selecting_player, p1,
        "selecting_player must be p1 (the opponent)"
    );
    assert_eq!(sel.kind, SelectionKind::Hand);

    // Both p0 hand slots (0 and 1) must be valid.
    assert!(
        sel.valid_action_ids.contains(&(PLAY_HAND_START)),
        "slot 0 must be valid"
    );
    assert!(
        sel.valid_action_ids.contains(&(PLAY_HAND_START + 1)),
        "slot 1 must be valid"
    );

    // Resolve from p1 — pick hand index 0.
    r.game
        .resolve_selection(p1, PLAY_HAND_START)
        .expect("p1 resolves hand pick");

    assert!(r.game.pending_selection.is_none());
    assert_eq!(
        *hand_idx_cb.lock().unwrap(),
        Some(0),
        "callback must receive hand index 0"
    );
}

// ── Test 4 ───────────────────────────────────────────────────────────────────

/// Calling `ctx.select_own_permanent(...)` WITHOUT the scope still uses
/// `self.player` (the controller) as `selecting_player`. The override field
/// must default to None and not affect direct calls.
#[test]
fn non_override_helper_still_uses_self_player() {
    let mut r = DebugRunner::builder().add_card(make_card("A")).start();
    let p0 = r.game.turn_player();

    // Place a permanent for p0 to select.
    r.place_on_field(p0, "A", Some(0));

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, p0);
        ctx.select_own_permanent(
            "Controller picks own permanent",
            false,
            |_g, _h| true,
            |_ctx, _h| {},
        );
    }

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("selection must be installed");
    assert_eq!(
        sel.selecting_player, p0,
        "without scope, selecting_player must be self.player (p0)"
    );
}

// ── Test 5 ───────────────────────────────────────────────────────────────────

/// The override is cleared after the scope method returns. A second
/// `select_own_permanent` call (without scope) that follows must see
/// `selecting_player == self.player`, not the previously-set override value.
///
/// Flow:
/// 1. Install via `ctx.as_selecting_player(p1).select_own_permanent(...)` — resolve it.
/// 2. Install via `ctx.select_own_permanent(...)` (no scope).
/// 3. Assert second selection's `selecting_player == p0`.
#[test]
fn nested_scopes_do_not_leak() {
    let mut r = DebugRunner::builder().add_card(make_card("B")).start();
    let p0 = r.game.turn_player();
    let p1: PlayerId = 1 - p0;

    // Place a permanent for p0.
    r.place_on_field(p0, "B", Some(0));

    // First install: scope routes to p1.
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, p0);
        ctx.as_selecting_player(p1).select_own_permanent(
            "p1 chooses",
            false,
            |_g, _h| true,
            |_ctx, _h| {},
        );
    }

    // Confirm first selection routes to p1.
    {
        let sel = r
            .game
            .pending_selection
            .as_ref()
            .expect("first selection must be installed");
        assert_eq!(sel.selecting_player, p1);
    }

    // Resolve the first selection from p1.
    r.game
        .resolve_selection(p1, encode_attack(0, 0))
        .expect("first resolution must succeed");

    assert!(
        r.game.pending_selection.is_none(),
        "first selection cleared"
    );

    // Second install: direct call — no scope.
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, p0);
        ctx.select_own_permanent("p0 chooses", false, |_g, _h| true, |_ctx, _h| {});
    }

    // The second selection must use p0 (self.player), not the leaked p1.
    let sel2 = r
        .game
        .pending_selection
        .as_ref()
        .expect("second selection must be installed");
    assert_eq!(
        sel2.selecting_player, p0,
        "second selection must not be contaminated by the previous scope; expected p0, got {}",
        sel2.selecting_player
    );
}

// ── Test 6 (edge-case) ───────────────────────────────────────────────────────

/// When the scope method is called but the underlying helper is a no-op (empty
/// filter → no PendingSelection installed), the override must still not leak to
/// a subsequent direct call.
#[test]
fn empty_filter_scope_does_not_leak_override() {
    let mut r = DebugRunner::builder().add_card(make_card("C")).start();
    let p0 = r.game.turn_player();
    let p1: PlayerId = 1 - p0;

    // Place a permanent for p0 so the second (direct) call has something to pick.
    r.place_on_field(p0, "C", Some(0));
    let phase_before = r.game.current_phase;

    // Scope call with a filter that rejects everything → no-op.
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, p0);
        ctx.as_selecting_player(p1).select_own_permanent(
            "reject all",
            false,
            |_g, _h| false,
            |_ctx, _h| {},
        );
    }

    // No selection installed (empty filter).
    assert!(
        r.game.pending_selection.is_none(),
        "empty-filter scope must not install a selection"
    );
    assert_eq!(
        r.game.current_phase, phase_before,
        "phase must be unchanged after empty-filter scope call"
    );

    // Now a direct (no-scope) call; its selecting_player must be p0.
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, p0);
        ctx.select_own_permanent("p0 picks", false, |_g, _h| true, |_ctx, _h| {});
    }

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("direct call must install a selection");
    assert_eq!(
        sel.selecting_player, p0,
        "direct call after no-op scope must use self.player (p0), not leaked override"
    );
    assert_eq!(r.game.current_phase, GamePhase::SelectTarget);
}
