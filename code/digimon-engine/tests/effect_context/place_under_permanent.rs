//! Task 1 — `EffectContext::place_card_under_permanent_bottom` moves a card
//! from any zone (hand, trash, permanent stack) to the bottom of a target
//! permanent's digivolution stack.
//!
//! Stack layout reminder: `card_sources[0]` = bottom card,
//! `card_sources.last()` = top card (the visible Digimon).
//!
//! Callers: `<Save>` (tucks the card being-deleted under a Tamer),
//!          `<Material Save N>` (moves own digivolution sources under a Tamer).

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};

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

fn make_tamer(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Tamer,
        level: None,
        dp: None,
        play_cost: 3,
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

/// Helper: push a card into a player's hand directly (bypasses play_from_hand).
fn push_to_hand(r: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
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

/// Helper: push a card into a player's trash directly.
fn push_to_trash(r: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {}", card_id));
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    let handle = card.handle();
    r.game.players[player as usize].trash.push(card);
    handle
}

/// Helper: push a source card onto an existing permanent's stack
/// (makes it the new top card).
fn push_source_on_top(
    r: &mut DebugRunner,
    perm_player: u8,
    perm_idx: usize,
    card_id: &str,
) -> CardHandle {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_source_on_top: unknown card_id {}", card_id));
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, perm_player, card_index);
    let handle = card.handle();
    r.game.players[perm_player as usize].battle_area[perm_idx]
        .card_sources
        .push(card);
    handle
}

// ── Test 1: From hand ────────────────────────────────────────────────────────

/// Place a card from hand to the bottom of a target permanent's stack.
/// Before: target has 1 source (its own top card).
/// After: target has 2 sources; TUCK-CARD is at index 0 (bottom),
///        TARGET-TOP remains at index 1 (top, still visible).
#[test]
fn place_from_hand_lands_at_bottom_of_stack() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("TARGET-TOP"))
        .add_card(make_digimon("TUCK-CARD"))
        .start();

    let tp = r.game.turn_player();
    let target_handle = r.place_on_field(tp, "TARGET-TOP", Some(0));

    // Put TUCK-CARD in player's hand, capture its handle.
    let tuck_handle = push_to_hand(&mut r, tp, "TUCK-CARD");

    // Pre-condition checks.
    assert_eq!(
        r.game.players[tp as usize].hand.len(),
        1,
        "hand has 1 card before"
    );
    assert_eq!(
        r.game.players[tp as usize].battle_area[target_handle.index as usize]
            .card_sources
            .len(),
        1,
        "target has 1 card (its own top) before"
    );

    {
        let mut ctx = EffectContext::new(&mut r.game, tuck_handle, None, tp);
        ctx.place_card_under_permanent_bottom(tuck_handle, target_handle);
    }

    // Post-condition: hand emptied.
    assert_eq!(
        r.game.players[tp as usize].hand.len(),
        0,
        "hand emptied after tuck"
    );

    // Post-condition: target has 2 sources, TUCK-CARD at index 0 (bottom).
    let target_perm = &r.game.players[tp as usize].battle_area[target_handle.index as usize];
    assert_eq!(
        target_perm.card_sources.len(),
        2,
        "target permanent has 2 sources after tuck"
    );
    assert_eq!(
        target_perm.card_sources[0].card_id(&r.game.card_data),
        "TUCK-CARD",
        "tucked card lands at index 0 (bottom of stack)"
    );
    assert_eq!(
        target_perm.top_card().card_id(&r.game.card_data),
        "TARGET-TOP",
        "original top stays at the visible top"
    );
}

// ── Test 2: From trash (Save's typical use case) ─────────────────────────────

/// Place a card from trash to the bottom of a target permanent's stack.
/// DCGO Save places `self` (the card being deleted, which lands in trash)
/// under a Tamer. This test simulates that by pre-populating trash.
#[test]
fn place_from_trash_lands_at_bottom_of_stack() {
    let mut r = DebugRunner::builder()
        .add_card(make_tamer("TARGET-TAMER"))
        .add_card(make_digimon("SAVE-CARD"))
        .start();

    let tp = r.game.turn_player();
    let target_handle = r.place_on_field(tp, "TARGET-TAMER", Some(0));

    // Pre-populate trash (mimics Save: card was just deleted → moved to trash).
    let save_handle = push_to_trash(&mut r, tp, "SAVE-CARD");

    assert_eq!(
        r.game.players[tp as usize].trash.len(),
        1,
        "trash has 1 card before"
    );
    assert_eq!(
        r.game.players[tp as usize].battle_area[target_handle.index as usize]
            .card_sources
            .len(),
        1,
        "tamer has 1 source before"
    );

    {
        let mut ctx = EffectContext::new(&mut r.game, save_handle, None, tp);
        ctx.place_card_under_permanent_bottom(save_handle, target_handle);
    }

    // Trash emptied.
    assert_eq!(
        r.game.players[tp as usize].trash.len(),
        0,
        "trash emptied after tuck"
    );

    // Target gained a bottom source.
    let tamer_perm = &r.game.players[tp as usize].battle_area[target_handle.index as usize];
    assert_eq!(
        tamer_perm.card_sources.len(),
        2,
        "tamer has 2 sources after tuck"
    );
    assert_eq!(
        tamer_perm.card_sources[0].card_id(&r.game.card_data),
        "SAVE-CARD",
        "SAVE-CARD is at index 0 (bottom of tamer stack)"
    );
    assert_eq!(
        tamer_perm.top_card().card_id(&r.game.card_data),
        "TARGET-TAMER",
        "tamer itself stays at top"
    );
}

// ── Test 3: Multiple successive calls (MaterialSave's use case) ───────────────

/// MaterialSave(N): call `place_card_under_permanent_bottom` N times, each
/// time moving a card from one permanent's sources to a target permanent.
///
/// Ordering invariant (critical for MaterialSave correctness):
///   - First call: card lands at index 0 of target.
///   - Second call: new card inserted at index 0, pushing previous card to index 1.
///   - Third call: new card at index 0, previous cards shift up.
///
/// So the last card placed is closest to the bottom, and the first card
/// placed is deepest (highest index among the moved cards, just below
/// the target's original top).
#[test]
fn multiple_successive_placements_preserve_insertion_order() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("SOURCE-PERM"))
        .add_card(make_digimon("SRC-A"))
        .add_card(make_digimon("SRC-B"))
        .add_card(make_digimon("SRC-C"))
        .add_card(make_tamer("TARGET-TAMER"))
        .start();

    let tp = r.game.turn_player();
    let source_perm_handle = r.place_on_field(tp, "SOURCE-PERM", Some(0));
    let source_perm_idx = source_perm_handle.index as usize;

    // Build a stack: [SRC-A, SRC-B, SRC-C, SOURCE-PERM]
    // (SOURCE-PERM is the top card placed by place_on_field; we push sources on top)
    // Actually push SRC-A, SRC-B, SRC-C as sources UNDER SOURCE-PERM:
    // We want the stack to be [SOURCE-PERM, SRC-A, SRC-B, SRC-C] where SOURCE-PERM
    // is at index 0 and SRC-C is at the top. But for MaterialSave, the card being
    // operated on is SOURCE-PERM's own sources. Let's set it up cleanly.
    //
    // Realistic setup: SOURCE-PERM was played, then SRC-A, SRC-B, SRC-C digivolved
    // on top of it in sequence. So the stack is:
    //   index 0 = SOURCE-PERM (bottom / base)
    //   index 1 = SRC-A
    //   index 2 = SRC-B
    //   index 3 = SRC-C  (top)

    let src_a_handle = push_source_on_top(&mut r, tp, source_perm_idx, "SRC-A");
    let src_b_handle = push_source_on_top(&mut r, tp, source_perm_idx, "SRC-B");
    let src_c_handle = push_source_on_top(&mut r, tp, source_perm_idx, "SRC-C");

    {
        let perm = &r.game.players[tp as usize].battle_area[source_perm_idx];
        assert_eq!(perm.card_sources.len(), 4);
        assert_eq!(perm.top_card().card_id(&r.game.card_data), "SRC-C");
    }

    let target_handle = r.place_on_field(tp, "TARGET-TAMER", Some(0));

    // Now MaterialSave: move SRC-A, SRC-B, SRC-C (sources of SOURCE-PERM)
    // under TARGET-TAMER, one at a time.
    // Note: after each removal the stack shrinks. We move from the cards
    // currently in the stack (by handle, not by position).
    {
        let mut ctx = EffectContext::new(&mut r.game, src_a_handle, None, tp);
        ctx.place_card_under_permanent_bottom(src_a_handle, target_handle);
    }
    {
        let mut ctx = EffectContext::new(&mut r.game, src_b_handle, None, tp);
        ctx.place_card_under_permanent_bottom(src_b_handle, target_handle);
    }
    {
        let mut ctx = EffectContext::new(&mut r.game, src_c_handle, None, tp);
        ctx.place_card_under_permanent_bottom(src_c_handle, target_handle);
    }

    // SOURCE-PERM should now have only 1 card left (itself, the base).
    let source_perm_after = &r.game.players[tp as usize].battle_area[source_perm_idx];
    assert_eq!(
        source_perm_after.card_sources.len(),
        1,
        "all 3 sources removed from SOURCE-PERM"
    );
    assert_eq!(
        source_perm_after.top_card().card_id(&r.game.card_data),
        "SOURCE-PERM",
        "SOURCE-PERM's own top card remains"
    );

    // TARGET-TAMER should now have 4 cards: TAMER at top + 3 tucked sources.
    // Each successive insert(0) pushes previous card deeper:
    //   After SRC-A: [SRC-A, TAMER]
    //   After SRC-B: [SRC-B, SRC-A, TAMER]
    //   After SRC-C: [SRC-C, SRC-B, SRC-A, TAMER]
    let target_after_idx = target_handle.index as usize;
    let target_perm = &r.game.players[tp as usize].battle_area[target_after_idx];
    assert_eq!(
        target_perm.card_sources.len(),
        4,
        "target has 4 sources after 3 tucks"
    );
    // Last placed (SRC-C) is at the very bottom (index 0).
    assert_eq!(
        target_perm.card_sources[0].card_id(&r.game.card_data),
        "SRC-C",
        "last placed card (SRC-C) is at index 0 (bottom)"
    );
    assert_eq!(
        target_perm.card_sources[1].card_id(&r.game.card_data),
        "SRC-B",
        "SRC-B at index 1"
    );
    assert_eq!(
        target_perm.card_sources[2].card_id(&r.game.card_data),
        "SRC-A",
        "first placed card (SRC-A) is at index 2 (just below top)"
    );
    // TARGET-TAMER's own card is at the top (last index = 3).
    assert_eq!(
        target_perm.top_card().card_id(&r.game.card_data),
        "TARGET-TAMER",
        "TARGET-TAMER stays at top"
    );
}
