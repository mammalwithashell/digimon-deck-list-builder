//! Tests for `EffectContext::place_remainder_on_deck` — the "scry-and-return"
//! helper that installs an `OrderedPermutation` over whatever remains in
//! `game.revealed_cards` and places the result at `StackPosition::{Top,Bottom,Random}`.
//!
//! Contract: `ordered_vec[0]` is drawn first among the placed cards.
//! - `Top`    → ordered_vec[0] lands at the last Vec index (end = top of deck)
//! - `Bottom` → ordered_vec[0] lands at the highest index among the placed group
//!              (closest to the top among the inserted-at-bottom cards)
//! - `Random` → permutation still fires; each card inserted at random position

use digimon_engine::action::space::SEL_REVEAL_START;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, PlayerId, StackPosition};
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
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

/// Push a card directly into the reveal pool.
fn push_to_reveal(r: &mut DebugRunner, card_id: &str) -> CardHandle {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_reveal: unknown card_id {}", card_id));
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, 0, card_index);
    let handle = card.handle();
    r.game.revealed_cards.push(card);
    handle
}

// ─── Test 1: empty reveal pool is a no-op ────────────────────────────────────

#[test]
fn empty_reveal_pool_is_noop() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("CARD-A"))
        .deck(0, &["CARD-A"])
        .start();

    let p0: PlayerId = r.game.turn_player();
    let deck_len_before = r.game.players[p0 as usize].deck.len();

    assert!(
        r.game.revealed_cards.is_empty(),
        "revealed_cards must be empty at the start of this test"
    );

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, p0);
        ctx.place_remainder_on_deck(p0, StackPosition::Top);
    }

    assert!(
        r.game.pending_selection.is_none(),
        "no PendingSelection must be installed when revealed_cards is empty"
    );
    assert_eq!(
        r.game.players[p0 as usize].deck.len(),
        deck_len_before,
        "deck must be unchanged"
    );
    assert!(
        r.game.revealed_cards.is_empty(),
        "revealed_cards must still be empty"
    );
}

// ─── Test 2: single revealed card — singleton permutation ────────────────────

#[test]
fn single_revealed_card_surfaces_one_choice_selection() {
    let mut r = DebugRunner::builder()
        .add_card(make_card("CARD-A"))
        .deck(0, &[])
        .start();

    let p0: PlayerId = r.game.turn_player();

    let _handle_a = push_to_reveal(&mut r, "CARD-A");

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, p0);
        ctx.place_remainder_on_deck(p0, StackPosition::Bottom);
    }

    // Selection must be installed.
    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("a 1-card remainder must install a PendingSelection");

    assert_eq!(
        sel.kind,
        SelectionKind::OrderedPermutation { remaining: 1 },
        "kind must be OrderedPermutation {{ remaining: 1 }}"
    );
    assert_eq!(
        sel.valid_action_ids.len(),
        1,
        "exactly 1 choice must be offered"
    );
    assert!(
        sel.valid_action_ids.contains(&SEL_REVEAL_START),
        "the single action ID must be SEL_REVEAL_START ({})",
        SEL_REVEAL_START
    );

    // Resolve the only choice.
    r.game
        .resolve_selection(p0, SEL_REVEAL_START)
        .expect("resolving single-card permutation must succeed");

    assert!(
        r.game.pending_selection.is_none(),
        "no further selection must be pending"
    );
    assert!(
        r.game.revealed_cards.is_empty(),
        "revealed_cards must be empty after placement"
    );

    // The card must be at the bottom of the deck (index 0 = bottom).
    let deck = &r.game.players[p0 as usize].deck;
    assert_eq!(deck.len(), 1, "deck must have exactly 1 card");
    let placed_id = &r.game.card_data[deck[0].data_index].card_id;
    assert_eq!(
        placed_id, "CARD-A",
        "CARD-A must be at library_cards[0] (bottom)"
    );
}

// ─── Test 3: three cards to Top — ordered_vec[0] drawn first ─────────────────

#[test]
fn three_cards_to_top_preserves_chosen_order() {
    // Reveal pool: [A, B, C] at indices [0, 1, 2].
    // Player chooses [B, C, A] → B drawn first, then C, then A.
    // With StackPosition::Top the deck (empty to start) ends up as:
    //   push A (rev[0]) → [A]
    //   push C (rev[1]) → [A, C]
    //   push B (rev[2]) → [A, C, B]
    // End = top of deck, so draw order = B (last), C, A (first = deepest).
    // library_cards: [A, C, B]  (index 2 = B = drawn first)

    let mut r = DebugRunner::builder()
        .add_card(make_card("CARD-A"))
        .add_card(make_card("CARD-B"))
        .add_card(make_card("CARD-C"))
        .deck(0, &[])
        .start();

    let p0: PlayerId = r.game.turn_player();

    let _handle_a = push_to_reveal(&mut r, "CARD-A"); // index 0
    let _handle_b = push_to_reveal(&mut r, "CARD-B"); // index 1
    let _handle_c = push_to_reveal(&mut r, "CARD-C"); // index 2

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, p0);
        ctx.place_remainder_on_deck(p0, StackPosition::Top);
    }

    // Step 1 — pick B (index 1 → SEL_REVEAL_START + 1).
    r.game
        .resolve_selection(p0, SEL_REVEAL_START + 1)
        .expect("step 1: pick B");

    // Step 2 — remaining is [A, C]; pick C (index 1 → SEL_REVEAL_START + 1).
    r.game
        .resolve_selection(p0, SEL_REVEAL_START + 1)
        .expect("step 2: pick C");

    // Step 3 — remaining is [A]; only choice.
    r.game
        .resolve_selection(p0, SEL_REVEAL_START)
        .expect("step 3: pick A");

    assert!(r.game.pending_selection.is_none(), "no further selection");
    assert!(
        r.game.revealed_cards.is_empty(),
        "reveal pool must be empty"
    );

    let deck = &r.game.players[p0 as usize].deck;
    assert_eq!(deck.len(), 3, "all 3 cards must be on the deck");

    // deck Vec: last index = top = drawn first = B.
    let top_id = &r.game.card_data[deck[2].data_index].card_id;
    let mid_id = &r.game.card_data[deck[1].data_index].card_id;
    let bot_id = &r.game.card_data[deck[0].data_index].card_id;
    assert_eq!(
        top_id, "CARD-B",
        "deck top (index 2) must be CARD-B (chosen 1st)"
    );
    assert_eq!(
        mid_id, "CARD-C",
        "deck middle (index 1) must be CARD-C (chosen 2nd)"
    );
    assert_eq!(
        bot_id, "CARD-A",
        "deck bottom (index 0) must be CARD-A (chosen 3rd)"
    );
}

// ─── Test 4: three cards to Bottom — ordered_vec[0] drawn first ──────────────

#[test]
fn three_cards_to_bottom_preserves_chosen_order() {
    // Reveal pool: [A, B, C].
    // Player chooses [B, C, A] → B drawn first among the placed group.
    //
    // With StackPosition::Bottom the target state is:
    //   deck[0] = A  (deepest — drawn last among the 3)
    //   deck[1] = C
    //   deck[2] = B  (closest to top among the 3 — drawn first)
    //
    // This is achieved by iterating ordered_vec forward and calling insert(0)
    // for each card:
    //   insert B at 0 → [B]
    //   insert C at 0 → [C, B]
    //   insert A at 0 → [A, C, B]
    // deck[0]=A, deck[1]=C, deck[2]=B — index 2 is closest to top in the group,
    // so B is drawn first. ✓

    let mut r = DebugRunner::builder()
        .add_card(make_card("CARD-A"))
        .add_card(make_card("CARD-B"))
        .add_card(make_card("CARD-C"))
        .deck(0, &[])
        .start();

    let p0: PlayerId = r.game.turn_player();

    let _handle_a = push_to_reveal(&mut r, "CARD-A"); // index 0
    let _handle_b = push_to_reveal(&mut r, "CARD-B"); // index 1
    let _handle_c = push_to_reveal(&mut r, "CARD-C"); // index 2

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, p0);
        ctx.place_remainder_on_deck(p0, StackPosition::Bottom);
    }

    // Step 1 — pick B (index 1 → SEL_REVEAL_START + 1).
    r.game
        .resolve_selection(p0, SEL_REVEAL_START + 1)
        .expect("step 1: pick B");

    // Step 2 — remaining is [A, C]; pick C (index 1 → SEL_REVEAL_START + 1).
    r.game
        .resolve_selection(p0, SEL_REVEAL_START + 1)
        .expect("step 2: pick C");

    // Step 3 — remaining is [A]; only choice.
    r.game
        .resolve_selection(p0, SEL_REVEAL_START)
        .expect("step 3: pick A");

    assert!(r.game.pending_selection.is_none(), "no further selection");
    assert!(
        r.game.revealed_cards.is_empty(),
        "reveal pool must be empty"
    );

    let deck = &r.game.players[p0 as usize].deck;
    assert_eq!(deck.len(), 3, "all 3 cards must be on the deck");

    // deck[0] = deepest/absolute bottom; deck[2] = closest-to-top among placed.
    let closest_top_id = &r.game.card_data[deck[2].data_index].card_id;
    let mid_id = &r.game.card_data[deck[1].data_index].card_id;
    let deepest_id = &r.game.card_data[deck[0].data_index].card_id;
    assert_eq!(
        closest_top_id, "CARD-B",
        "deck[2] (closest-to-top) must be CARD-B (drawn first)"
    );
    assert_eq!(mid_id, "CARD-C", "deck[1] (middle) must be CARD-C");
    assert_eq!(
        deepest_id, "CARD-A",
        "deck[0] (deepest) must be CARD-A (drawn last)"
    );
}
