//! `select_ordered_permutation` helper tests.
//!
//! `EffectContext::select_ordered_permutation` prompts a player to place N
//! items in a chosen order via sequential single-item picks. Each step
//! installs a `PendingSelection` with `kind = OrderedPermutation { remaining }`.
//! Action IDs reuse `SEL_REVEAL_START` (30–39), offset by index into the
//! remaining list — no new action range. Phase parks at
//! `GamePhase::SelectPermutation`.

use std::sync::{Arc, Mutex};

use digimon_engine::action::space::SEL_REVEAL_START;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
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
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// Push a card into a player's hand (for sourcing CardHandles from the game).
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

// ── three-item ordered permutation ──────────────────────────────────────────

#[test]
fn ordered_permutation_collects_picks_in_order() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("CARD-A"))
        .add_card(make_card("CARD-B"))
        .add_card(make_card("CARD-C"))
        .start();
    let tp = r.game.turn_player();
    let handle_a = push_to_hand(&mut r, tp, "CARD-A");
    let handle_b = push_to_hand(&mut r, tp, "CARD-B");
    let handle_c = push_to_hand(&mut r, tp, "CARD-C");

    let delivered: Arc<Mutex<Option<Vec<CardHandle>>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&delivered);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_ordered_permutation(
            vec![handle_a, handle_b, handle_c],
            "order the cards",
            move |_, picks| {
                *slot.lock().unwrap() = Some(picks);
            },
        );
    }

    // Step 1: remaining = 3, pick card_b (index 1 in original list)
    {
        let sel = r
            .game
            .pending_selection
            .as_ref()
            .expect("step 1 must park a selection");
        assert_eq!(
            sel.kind,
            SelectionKind::OrderedPermutation { remaining: 3 },
            "step 1 kind must be OrderedPermutation {{ remaining: 3 }}"
        );
        assert_eq!(
            sel.valid_action_ids.len(),
            3,
            "step 1 must offer 3 action IDs"
        );
        assert_eq!(r.game.current_phase, GamePhase::SelectPermutation);
        // card_b is index 1 in [A, B, C] → SEL_REVEAL_START + 1
        let pick_b_action = SEL_REVEAL_START + 1;
        assert!(
            sel.valid_action_ids.contains(&pick_b_action),
            "action for card_b must be in valid_action_ids"
        );
    }
    r.game.resolve_selection(tp, SEL_REVEAL_START + 1).unwrap();

    // Step 2: remaining = 2, pick card_c (index 1 in [A, C])
    {
        let sel = r
            .game
            .pending_selection
            .as_ref()
            .expect("step 2 must park a selection");
        assert_eq!(
            sel.kind,
            SelectionKind::OrderedPermutation { remaining: 2 },
            "step 2 kind must be OrderedPermutation {{ remaining: 2 }}"
        );
        assert_eq!(
            sel.valid_action_ids.len(),
            2,
            "step 2 must offer 2 action IDs"
        );
        assert_eq!(r.game.current_phase, GamePhase::SelectPermutation);
        // Remaining is [A, C]; card_c is at index 1 → SEL_REVEAL_START + 1
        let pick_c_action = SEL_REVEAL_START + 1;
        assert!(
            sel.valid_action_ids.contains(&pick_c_action),
            "action for card_c must be in valid_action_ids"
        );
    }
    r.game.resolve_selection(tp, SEL_REVEAL_START + 1).unwrap();

    // Step 3: remaining = 1, pick card_a (only one left)
    {
        let sel = r
            .game
            .pending_selection
            .as_ref()
            .expect("step 3 must park a selection");
        assert_eq!(
            sel.kind,
            SelectionKind::OrderedPermutation { remaining: 1 },
            "step 3 kind must be OrderedPermutation {{ remaining: 1 }}"
        );
        assert_eq!(
            sel.valid_action_ids.len(),
            1,
            "step 3 must offer exactly 1 action ID"
        );
        assert_eq!(r.game.current_phase, GamePhase::SelectPermutation);
        // Only card_a left → index 0
        assert!(
            sel.valid_action_ids.contains(&SEL_REVEAL_START),
            "only action must be SEL_REVEAL_START"
        );
    }
    r.game.resolve_selection(tp, SEL_REVEAL_START).unwrap();

    // Final: callback has fired with [B, C, A]; no pending selection remains
    assert!(
        r.game.pending_selection.is_none(),
        "pending_selection must be cleared after all picks"
    );
    assert_ne!(
        r.game.current_phase,
        GamePhase::SelectPermutation,
        "previous_phase must be restored after final pick"
    );
    let result = delivered
        .lock()
        .unwrap()
        .take()
        .expect("final callback must have fired");
    assert_eq!(
        result,
        vec![handle_b, handle_c, handle_a],
        "delivered order must be [B, C, A] (order of picks)"
    );
}

// ── empty items → immediate callback, no selection ──────────────────────────

#[test]
fn empty_items_invokes_final_callback_immediately() {
    let mut r = DebugRunner::builder().add_card(make_card("CARD-A")).start();
    let tp = r.game.turn_player();
    let phase_before = r.game.current_phase;

    let fired: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let flag = Arc::clone(&fired);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_ordered_permutation(vec![], "order nothing", move |_, picks| {
            assert!(picks.is_empty(), "empty items must deliver empty Vec");
            *flag.lock().unwrap() = true;
        });
    }

    assert!(
        *fired.lock().unwrap(),
        "final callback must fire immediately for empty items"
    );
    assert!(
        r.game.pending_selection.is_none(),
        "no pending selection must be installed for empty items"
    );
    assert_eq!(
        r.game.current_phase, phase_before,
        "phase must be unchanged for empty items"
    );
}

// ── singleton: must still install a selection (no-approximations policy) ────

#[test]
fn singleton_surfaces_as_one_choice_selection() {
    let mut r = DebugRunner::builder().add_card(make_card("CARD-A")).start();
    let tp = r.game.turn_player();
    let handle_a = push_to_hand(&mut r, tp, "CARD-A");

    let delivered: Arc<Mutex<Option<Vec<CardHandle>>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&delivered);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.select_ordered_permutation(vec![handle_a], "order single card", move |_, picks| {
            *slot.lock().unwrap() = Some(picks);
        });
    }

    // Selection must be installed even for 1 item (no-approximations policy)
    {
        let sel = r
            .game
            .pending_selection
            .as_ref()
            .expect("singleton must install a pending selection");
        assert_eq!(sel.kind, SelectionKind::OrderedPermutation { remaining: 1 },);
        assert_eq!(sel.valid_action_ids.len(), 1);
        assert_eq!(sel.valid_action_ids[0], SEL_REVEAL_START);
        assert_eq!(r.game.current_phase, GamePhase::SelectPermutation);
    }

    // Resolve the single choice
    r.game.resolve_selection(tp, SEL_REVEAL_START).unwrap();

    assert!(r.game.pending_selection.is_none());
    let result = delivered
        .lock()
        .unwrap()
        .take()
        .expect("callback must have fired");
    assert_eq!(result, vec![handle_a], "delivered must be [A]");
}
