//! End-to-end behavioral test combining `place_remainder_on_deck` (Phase 4
//! follow-up helper) with `as_selecting_player(opp).select_union_zone(...)`
//! (Tasks 2 + 5).
//!
//! **Scenario** (synthesized from meta patterns — not a real printed card):
//! > "Reveal top 3 cards of your deck. Place them on top of your deck in any
//! >  order. Then your opponent chooses one card from your hand OR trash to trash."
//!
//! **Migration note**: the original test used `select_ordered_permutation` with
//! a hand-rolled `for handle in ordered_vec.iter().rev()` loop inside the
//! callback. This version replaces that with `place_remainder_on_deck` for the
//! placement phase, then installs the opponent union-zone as a separate step
//! after the permutation resolves. All original assertions are preserved.
//!
//! This test walks every resolution step, checks selection-kind and
//! `selecting_player` routing, verifies mask-level action visibility for
//! each player, and confirms the observable game-state mutations at the end.

use std::sync::{Arc, Mutex};

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{PLAY_HAND_START, SEL_REVEAL_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlayerId, StackPosition};
use digimon_engine::selection::{SelectionKind, UnionZoneSet};

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

/// Push a card directly into a player's hand (mirrors the helper in opponent_selector.rs).
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

/// Push a card directly into a player's trash (mirrors the helper in union_zone.rs).
fn push_to_trash(r: &mut DebugRunner, player: PlayerId, card_id: &str) {
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

// ─── Main test ───────────────────────────────────────────────────────────────

/// Exercises the full two-phase flow:
///
/// **Phase 1 — ordered permutation (3 steps)**
/// Player 0 picks the order for 3 revealed deck cards. At each step the
/// selection kind, remaining count, and `SelectPermutation` phase are
/// checked. After all 3 picks the permutation callback runs, places the
/// cards back onto the deck in the chosen order using
/// `return_to_deck_from_reveal(_, _, StackPosition::Top)`, and immediately
/// installs the union-zone selection.
///
/// **Phase 2 — opponent union-zone**
/// Player 1 (opponent) must pick one card from player 0's HAND or TRASH.
/// Both the `selecting_player` routing and the action-mask visibility are
/// verified:
/// - Player 0 mask: zero selection-action bits (not their turn to select)
/// - Player 1 mask: exactly the 4 expected bits (2 hand + 2 trash)
///
/// After player 1 resolves by picking hand index 0, the chosen card is
/// confirmed removed from player 0's hand.
#[test]
fn permutation_then_opponent_union_zone_tech_flow() {
    // ─── 1. Build game ────────────────────────────────────────────────────────
    //
    // Deck (player 0): top 3 = [DECK-A (top), DECK-B, DECK-C]
    //   In Vec terms (last = top): deck = [DECK-C, DECK-B, DECK-A]
    // Hand (player 0): HAND-0, HAND-1
    // Trash (player 0): TRASH-0, TRASH-1
    // Player 1 plays no cards (just resolves the opponent selection).

    let mut r = DebugRunner::builder()
        .add_card(make_card("DECK-A"))
        .add_card(make_card("DECK-B"))
        .add_card(make_card("DECK-C"))
        .add_card(make_card("HAND-0"))
        .add_card(make_card("HAND-1"))
        .add_card(make_card("TRASH-0"))
        .add_card(make_card("TRASH-1"))
        // Deck order in builder: last entry = top-of-deck. We want top = DECK-A.
        .deck(0, &["DECK-C", "DECK-B", "DECK-A"])
        .start();

    let p0: PlayerId = r.game.turn_player();
    let p1: PlayerId = 1 - p0;

    // Populate hand and trash for player 0 via helpers (avoids builder coupling).
    let hand0_handle = push_to_hand(&mut r, p0, "HAND-0");
    let _hand1_handle = push_to_hand(&mut r, p0, "HAND-1");
    push_to_trash(&mut r, p0, "TRASH-0");
    push_to_trash(&mut r, p0, "TRASH-1");

    let phase_before = r.game.current_phase;

    // ─── 2. Reveal top 3 cards ────────────────────────────────────────────────
    //
    // `reveal_top_deck` pops from the end of the Vec (= top-of-deck) and
    // moves them into `game.revealed_cards`. After this:
    //   revealed_cards = [DECK-A (popped first), DECK-B, DECK-C]
    //   player 0 deck is empty.
    let top3 = r.game.reveal_top_deck(p0, 3);
    assert_eq!(top3.len(), 3, "deck must have had 3 cards to reveal");
    assert!(r.game.players[p0 as usize].deck.is_empty(), "deck must be empty after reveal");

    // top3[0] = DECK-A (was top), top3[1] = DECK-B, top3[2] = DECK-C.
    // These handles are not passed directly to place_remainder_on_deck (it reads
    // game.revealed_cards automatically), but are kept here for documentation.
    let _handle_a = top3[0]; // DECK-A
    let _handle_b = top3[1]; // DECK-B
    let _handle_c = top3[2]; // DECK-C

    // ─── 3. Install the permutation selection via place_remainder_on_deck ───────
    //
    // `place_remainder_on_deck` reads whatever remains in `game.revealed_cards`
    // (all 3 cards from step 2) and installs an ordered-permutation selection.
    // The callback places each card back on top with the correct iteration
    // direction so ordered_vec[0] is drawn first.
    //
    // The opponent union-zone is installed as a separate step after the
    // permutation resolves (see step 5 below).
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, p0);
        ctx.place_remainder_on_deck(p0, StackPosition::Top);
    }

    // We capture whether the union-zone was eventually resolved by recording
    // the handle the union-zone callback fires with.
    let union_zone_cb_fired: Arc<Mutex<Option<CardHandle>>> = Arc::new(Mutex::new(None));
    let union_zone_slot = Arc::clone(&union_zone_cb_fired);

    // ─── 4. Walk through permutation steps ───────────────────────────────────
    //
    // Desired order: [DECK-C (top), DECK-B, DECK-A (bottom)] — i.e. pick C first.
    // Initial remaining list: [A, B, C] at indices [0, 1, 2].

    // Step 1 — remaining = 3, pick DECK-C (index 2 in [A, B, C])
    {
        let sel = r.game.pending_selection.as_ref()
            .expect("step 1: selection must be installed");
        assert_eq!(
            sel.kind,
            SelectionKind::OrderedPermutation { remaining: 3 },
            "step 1 kind must be OrderedPermutation {{ remaining: 3 }}"
        );
        assert_eq!(sel.valid_action_ids.len(), 3, "step 1 must offer 3 choices");
        assert_eq!(r.game.current_phase, GamePhase::SelectPermutation, "phase must be SelectPermutation");
        assert_eq!(sel.selecting_player, p0, "permutation selects player must be p0 (the controller)");
        // DECK-C is at index 2 → SEL_REVEAL_START + 2
        assert!(sel.valid_action_ids.contains(&(SEL_REVEAL_START + 2)));
    }
    r.game.resolve_selection(p0, SEL_REVEAL_START + 2).expect("step 1 resolve");

    // Step 2 — remaining = 2, pick DECK-B (index 1 in [A, B])
    {
        let sel = r.game.pending_selection.as_ref()
            .expect("step 2: selection must be installed");
        assert_eq!(
            sel.kind,
            SelectionKind::OrderedPermutation { remaining: 2 },
            "step 2 kind must be OrderedPermutation {{ remaining: 2 }}"
        );
        assert_eq!(sel.valid_action_ids.len(), 2, "step 2 must offer 2 choices");
        assert_eq!(r.game.current_phase, GamePhase::SelectPermutation);
        // Remaining is [A, B]; DECK-B is at index 1 → SEL_REVEAL_START + 1
        assert!(sel.valid_action_ids.contains(&(SEL_REVEAL_START + 1)));
    }
    r.game.resolve_selection(p0, SEL_REVEAL_START + 1).expect("step 2 resolve");

    // Step 3 — remaining = 1, only DECK-A left
    {
        let sel = r.game.pending_selection.as_ref()
            .expect("step 3: selection must be installed");
        assert_eq!(
            sel.kind,
            SelectionKind::OrderedPermutation { remaining: 1 },
            "step 3 kind must be OrderedPermutation {{ remaining: 1 }}"
        );
        assert_eq!(sel.valid_action_ids.len(), 1, "step 3 must offer exactly 1 choice");
        assert_eq!(r.game.current_phase, GamePhase::SelectPermutation);
        assert!(sel.valid_action_ids.contains(&SEL_REVEAL_START));
    }
    r.game.resolve_selection(p0, SEL_REVEAL_START).expect("step 3 resolve");

    // ─── 4b. Install opponent union-zone (separate step after permutation) ────
    //
    // `place_remainder_on_deck` only handles placement. The opponent union-zone
    // is a separate effect that follows the placement in the card-text scenario.
    // We install it here, after the permutation has fully resolved, using the
    // same `as_selecting_player(p1).select_union_zone(...)` call from the
    // original test.
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, p0);
        ctx.as_selecting_player(p1)
            .select_union_zone(
                p0,
                UnionZoneSet::HAND | UnionZoneSet::TRASH,
                "Opponent: choose a card from your opponent's hand or trash",
                false,
                |_, _| true,
                move |resolve_ctx, chosen_handle| {
                    let player_ref = resolve_ctx.game.player(p0);
                    if let Some(idx) = player_ref.hand.iter().position(|c| c.handle() == chosen_handle) {
                        resolve_ctx.game.trash_from_hand_by_index(p0, idx);
                    }
                    *union_zone_slot.lock().unwrap() = Some(chosen_handle);
                },
            );
    }

    // ─── 5. Post-permutation assertions ──────────────────────────────────────
    //
    // After the final pick the place_remainder_on_deck callback fires:
    //  - ordered_vec = [DECK-C, DECK-B, DECK-A] (order of picks)
    //  - The callback (StackPosition::Top, rev iteration) places cards:
    //      push DECK-A (rev[0]) → deck = [DECK-A]
    //      push DECK-B (rev[1]) → deck = [DECK-A, DECK-B]
    //      push DECK-C (rev[2]) → deck = [DECK-A, DECK-B, DECK-C]
    //    Pop from Vec end gives DECK-C first — confirming top = DECK-C.
    //  - The union-zone selection is now installed (opponent picks from p0's hand/trash).

    let deck = &r.game.players[p0 as usize].deck;
    assert_eq!(deck.len(), 3, "all 3 revealed cards must be back on deck");
    // Top of deck = last element = DECK-C (chosen first = desired top).
    let top_id = &r.game.card_data[deck.last().unwrap().data_index].card_id;
    assert_eq!(top_id, "DECK-C", "deck top must be DECK-C (first chosen)");
    let second_id = &r.game.card_data[deck[deck.len() - 2].data_index].card_id;
    assert_eq!(second_id, "DECK-B", "deck second must be DECK-B");
    let third_id = &r.game.card_data[deck[0].data_index].card_id;
    assert_eq!(third_id, "DECK-A", "deck bottom-of-3 must be DECK-A");

    // Union-zone selection must be installed.
    {
        let sel = r.game.pending_selection.as_ref()
            .expect("union-zone selection must be installed after permutation");
        assert_eq!(
            sel.kind,
            SelectionKind::UnionZone {
                zones: UnionZoneSet::HAND | UnionZoneSet::TRASH
            },
            "kind must be UnionZone {{ HAND | TRASH }}"
        );
        assert_eq!(sel.selecting_player, p1, "opponent (p1) must be the selector");
        assert_eq!(r.game.current_phase, GamePhase::SelectUnion);
        // 2 hand cards + 2 trash cards = 4 valid actions.
        assert_eq!(
            sel.valid_action_ids.len(),
            4,
            "valid_action_ids must cover 2 hand + 2 trash = 4 total; got {:?}",
            sel.valid_action_ids
        );
        assert!(sel.valid_action_ids.contains(&PLAY_HAND_START),       "hand slot 0 must be valid");
        assert!(sel.valid_action_ids.contains(&(PLAY_HAND_START + 1)), "hand slot 1 must be valid");
        assert!(sel.valid_action_ids.contains(&TRASH_EFFECT_START),          "trash slot 0 must be valid");
        assert!(sel.valid_action_ids.contains(&(TRASH_EFFECT_START + 1)),   "trash slot 1 must be valid");
    }

    // ─── 6. Mask-level routing check ─────────────────────────────────────────
    //
    // While the union-zone selection is pending and routed to p1:
    // - p0 mask must have zero selection-action bits
    // - p1 mask must expose exactly the 4 union-zone action IDs

    let mask_p0 = build_action_mask(&r.game, p0);
    let legal_p0: Vec<usize> = mask_p0
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| (v > 0.0).then_some(i))
        .collect();
    assert!(
        legal_p0.is_empty(),
        "p0 (controller) must see an empty mask while opponent selects; got {:?}",
        legal_p0
    );

    let mask_p1 = build_action_mask(&r.game, p1);
    let legal_p1: Vec<usize> = mask_p1
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| (v > 0.0).then_some(i))
        .collect();
    assert_eq!(
        legal_p1.len(),
        4,
        "p1 must see exactly 4 legal actions (2 hand + 2 trash); got {:?}",
        legal_p1
    );
    assert!(legal_p1.contains(&(PLAY_HAND_START as usize)),       "p1 mask: hand slot 0");
    assert!(legal_p1.contains(&(PLAY_HAND_START as usize + 1)),   "p1 mask: hand slot 1");
    assert!(legal_p1.contains(&(TRASH_EFFECT_START as usize)),          "p1 mask: trash slot 0");
    assert!(legal_p1.contains(&(TRASH_EFFECT_START as usize + 1)),     "p1 mask: trash slot 1");

    // ─── 7. Opponent resolves — picks hand index 0 (HAND-0) ──────────────────
    //
    // We pick the hand card so the state change (removal from hand) is clearly
    // observable. Trash-from-trash would be a no-op in terms of position.
    let hand_size_before = r.game.players[p0 as usize].hand.len();
    r.game
        .resolve_selection(p1, PLAY_HAND_START)
        .expect("p1 resolving union-zone with hand slot 0 must succeed");

    // ─── 8. Final assertions ──────────────────────────────────────────────────

    assert!(
        r.game.pending_selection.is_none(),
        "pending_selection must be None after all selections resolved"
    );
    assert_eq!(
        r.game.current_phase,
        phase_before,
        "phase must be restored to what it was before the whole flow"
    );

    // HAND-0 was removed from p0's hand (trashed by the callback).
    assert_eq!(
        r.game.players[p0 as usize].hand.len(),
        hand_size_before - 1,
        "p0 hand must have shrunk by 1 after opponent's union-zone pick"
    );
    let hand0_still_in_hand = r.game.players[p0 as usize]
        .hand
        .iter()
        .any(|c| c.handle() == hand0_handle);
    assert!(
        !hand0_still_in_hand,
        "HAND-0 must no longer be in p0's hand after being chosen and trashed"
    );

    // Union-zone callback must have fired with HAND-0's handle.
    let fired_handle = union_zone_cb_fired
        .lock()
        .unwrap()
        .take()
        .expect("union-zone callback must have fired");
    assert_eq!(
        fired_handle, hand0_handle,
        "union-zone callback must have received HAND-0's handle"
    );
}
