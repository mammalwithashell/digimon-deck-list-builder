//! Phase F Task 6 — `Keyword::Training` auto-install behavioral tests.
//!
//! `<Training>` is a `[Main]` active skill (RULES_CONTEXT 16-40 / DCGO
//! `Training.cs`):
//!   - Activatable from BATTLE area OR BREEDING area.
//!   - Cost: suspend self (must be unsuspended; gate
//!     `if (thisPermanent.IsSuspended || !thisPermanent.CanSuspend)`).
//!   - Effect: place the controller's deck-top card at the BOTTOM of self's
//!     digivolution stack, FACE-DOWN (DCGO line 30:
//!     `AddDigivolutionCardsBottom(..., isFacedown: true)`).
//!
//! ## Auto-install shape (Phase F §F4)
//!
//! `EffectTiming::MainOnField` declarative skill on the carrier:
//!   - Activation gate: `!perm.is_suspended` (DCGO line 23).
//!   - Body: suspend self (cost), then move the deck-top CardSource into
//!     the carrier's `card_sources[0]` slot with `face_down=true`.
//!
//! ## Empty-deck handling
//!
//! DCGO `Training.cs:30` indexes `card.Owner.LibraryCards[0]` — a synchronous
//! list lookup. With an empty deck this would throw at runtime; in practice
//! DCGO never reaches this line because the activation gate is implicit
//! through the `SetUpActivateClass` framework which (per parity with the
//! `<Save>` / `<MaterialSave>` precedents) does not pre-check deck size.
//!
//! The Rust port chooses the safer behavior: the gate (`condition`) does NOT
//! check deck size; activation always pays the suspend cost; if the deck is
//! empty when the body runs, the move is a silent no-op (deck stays empty,
//! stack size unchanged). This mirrors the documented "no-op on empty
//! source" pattern used elsewhere in `EffectContext` primitives (e.g.
//! `Player::draw` no-ops on empty deck).
//!
//! ## Battle-area vs. breeding-area dispatch
//!
//! Battle-area Training is dispatched by the existing `activate_field_main`
//! over `MainOnField` effects — no substrate change needed.
//!
//! Breeding-area Training requires a parallel dispatch path: the
//! breeding-area permanent is `Player::breeding_area: Option<Permanent>`, NOT
//! a slot in `battle_area`. Phase F Task 6 adds a Training-only emitter and
//! activator under field index `BREEDING_TARGET (=14)` so the action mask can
//! surface it and the decoder can route to a dedicated activator. We restrict
//! to Training-bearing carriers only — RULES_CONTEXT 16-40 specifies that
//! ONLY `<Training>` activates from breeding; surfacing all `MainOnField`
//! effects from breeding would inadvertently expose Save/MaterialSave/MindLink
//! from breeding too (which is wrong).

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    BREEDING_TARGET, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;
use digimon_engine::permanent::Permanent;

use super::helpers::{digimon_with_keywords, plain_digimon};

fn training_digimon(id: &str) -> CardData {
    digimon_with_keywords(id, 3, 3000, vec![Keyword::Training])
}

fn field_main_bit(field_index: usize) -> usize {
    (FIELD_EFFECT_START
        + field_index as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN) as usize
}

fn breeding_main_bit() -> usize {
    field_main_bit(BREEDING_TARGET as usize)
}

/// Plant `card_id` at the top of `player`'s deck (deck top = last element of
/// the deck Vec, see `Player::draw`'s `deck.pop()`).
fn push_deck_top(r: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_deck_top: unknown card_id {}", card_id));
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    r.game.players[player as usize].deck.push(card);
}

// ─── Test 1: battle-area happy path ─────────────────────────────────────────

/// P0 has a Training Digimon on battle area. A known card sits on top of
/// P0's deck. Activate Training. Expected:
///   - The carrier is now suspended.
///   - The carrier's stack grew by exactly one source.
///   - The new source is at index 0 (bottom of stack) and is face-down.
///   - The deck top card was the one moved.
///   - The deck shrunk by one.
#[test]
fn training_suspends_self_and_places_deck_top_face_down() {
    let mut r = DebugRunner::builder()
        .add_card(training_digimon("TR"))
        .add_card(plain_digimon("DECKTOP"))
        .start();

    let tr = r.place_on_field(0, "TR", None);
    let tr_idx = tr.index as usize;

    // Plant DECKTOP at the top of P0's deck.
    push_deck_top(&mut r, 0, "DECKTOP");
    let deck_size_before = r.game.players[0].deck.len();
    let stack_before = r.game.players[0].battle_area[tr_idx].card_sources.len();
    assert_eq!(stack_before, 1, "TR starts with just its top card");

    r.game.enter_main_phase();
    r.game.set_memory(0);

    // Mask must offer the [Main] activation.
    {
        let mask = build_action_mask(&r.game, 0);
        assert_eq!(
            mask[field_main_bit(tr_idx)],
            1.0,
            "Training [Main] activation must appear in the mask when carrier is unsuspended"
        );
    }

    let fired = r.game.activate_field_main(0, tr_idx);
    assert!(fired, "Training activation must succeed from battle area");

    // Carrier suspended.
    assert!(
        r.game.players[0].battle_area[tr_idx].is_suspended,
        "carrier must be suspended after Training (cost paid)"
    );

    // Stack grew by one; new source at index 0 is face-down DECKTOP.
    let perm = &r.game.players[0].battle_area[tr_idx];
    assert_eq!(
        perm.card_sources.len(),
        stack_before + 1,
        "carrier's stack must have grown by exactly one source"
    );
    assert_eq!(
        perm.card_sources[0].card_id(&r.game.card_data),
        "DECKTOP",
        "deck-top card must sit at the very bottom (index 0) of the stack"
    );
    assert!(
        perm.card_sources[0].face_down,
        "the placed source must be face-down (DCGO `isFacedown: true`)"
    );
    // Top is unchanged.
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "TR",
        "TR remains the visible top after Training"
    );

    // Deck shrunk by one.
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_size_before - 1,
        "deck must shrink by exactly one after Training"
    );
}

// ─── Test 2: suspended carrier blocks Training ──────────────────────────────

/// P0's Training carrier is already suspended. The mask must NOT offer the
/// activation, and direct activation must fail. RULES_CONTEXT 16-40 +
/// DCGO `Training.cs:23` gate: must be unsuspended to activate.
#[test]
fn training_blocked_when_self_already_suspended() {
    let mut r = DebugRunner::builder()
        .add_card(training_digimon("TR"))
        .add_card(plain_digimon("DECKTOP"))
        .start();

    let tr = r.place_on_field(0, "TR", None);
    let tr_idx = tr.index as usize;
    push_deck_top(&mut r, 0, "DECKTOP");

    // Suspend the carrier.
    r.game.players[0].battle_area[tr_idx].is_suspended = true;

    r.game.enter_main_phase();
    r.game.set_memory(0);

    // Mask must NOT offer Training.
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[field_main_bit(tr_idx)],
        0.0,
        "Training activation must NOT appear in the mask when carrier is suspended"
    );

    // Direct activation must fail.
    let fired = r.game.activate_field_main(0, tr_idx);
    assert!(
        !fired,
        "Training activation must fail when carrier is suspended"
    );
    assert!(
        r.game.pending_selection.is_none(),
        "no selection parked when activation gate fails"
    );
    // Stack unchanged.
    assert_eq!(
        r.game.players[0].battle_area[tr_idx].card_sources.len(),
        1,
        "carrier's stack must be unchanged when activation is blocked"
    );
}

// ─── Test 3: breeding-area Training ─────────────────────────────────────────

/// P0 has a Training Digimon in the breeding area (no battle-area copy).
/// A known card sits on top of P0's deck. Activate breeding-area Training.
/// Expected:
///   - The breeding-area carrier is suspended.
///   - The breeding carrier's stack grew by one face-down DECKTOP at index 0.
///   - The deck shrunk by one.
#[test]
fn training_works_from_breeding_area() {
    let mut r = DebugRunner::builder()
        .add_card(training_digimon("TR"))
        .add_card(plain_digimon("DECKTOP"))
        .start();

    // Manually plant a Training Digimon in the breeding area.
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TR")
        .unwrap();
    let card_index = r.game.next_card_index();
    let source = CardSource::new(data_idx, 0, card_index);
    r.game.players[0].breeding_area = Some(Permanent::new(source, 0));

    push_deck_top(&mut r, 0, "DECKTOP");
    let deck_size_before = r.game.players[0].deck.len();

    r.game.enter_main_phase();
    r.game.set_memory(0);

    // Sanity: carrier exists in breeding and is unsuspended.
    {
        let breeding = r.game.players[0]
            .breeding_area
            .as_ref()
            .expect("TR must be in breeding area for this test");
        assert!(
            !breeding.is_suspended,
            "breeding carrier must start unsuspended"
        );
        assert_eq!(breeding.card_sources.len(), 1);
    }

    // Mask must offer the breeding-area Training activation at the
    // BREEDING_TARGET-indexed FIELD_EFFECT bit.
    {
        let mask = build_action_mask(&r.game, 0);
        assert_eq!(
            mask[breeding_main_bit()],
            1.0,
            "Training [Main] activation from breeding must appear at \
             FIELD_EFFECT_START + BREEDING_TARGET*EFFECTS_PER_PERMANENT + 2"
        );
    }

    // Activate via the breeding-area dispatch path.
    let fired = r.game.activate_field_main(0, BREEDING_TARGET as usize);
    assert!(
        fired,
        "Training activation must succeed from breeding area"
    );

    // Carrier suspended.
    let breeding = r.game.players[0]
        .breeding_area
        .as_ref()
        .expect("TR must still be in breeding after Training");
    assert!(
        breeding.is_suspended,
        "breeding carrier must be suspended after Training"
    );

    // Stack grew by one; new source at index 0 is face-down DECKTOP.
    assert_eq!(breeding.card_sources.len(), 2);
    assert_eq!(
        breeding.card_sources[0].card_id(&r.game.card_data),
        "DECKTOP",
        "deck-top card must sit at the bottom (index 0) of the breeding stack"
    );
    assert!(
        breeding.card_sources[0].face_down,
        "placed source must be face-down"
    );
    assert_eq!(
        breeding.top_card().card_id(&r.game.card_data),
        "TR",
        "TR remains the visible top of the breeding-area permanent"
    );

    // Deck shrunk by one.
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_size_before - 1,
        "deck must shrink by exactly one"
    );
}

// ─── Test 4: empty-deck behavior ─────────────────────────────────────────────

/// P0's Training carrier is unsuspended; P0's deck is empty. Activation
/// must still succeed and pay the suspend cost (the gate doesn't pre-check
/// deck size — DCGO `CanActivateTraining` does not gate on deck contents
/// either). The body silently no-ops on the move; stack size is unchanged.
#[test]
fn training_empty_deck_no_op() {
    let mut r = DebugRunner::builder()
        .add_card(training_digimon("TR"))
        .start();

    let tr = r.place_on_field(0, "TR", None);
    let tr_idx = tr.index as usize;

    // Empty the deck explicitly.
    r.game.players[0].deck.clear();
    let stack_before = r.game.players[0].battle_area[tr_idx].card_sources.len();

    r.game.enter_main_phase();
    r.game.set_memory(0);

    // Mask must STILL offer Training (no deck-size gate).
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[field_main_bit(tr_idx)],
        1.0,
        "Training activation must appear even when the deck is empty (gate is \
         only on suspension, not deck size)"
    );

    let fired = r.game.activate_field_main(0, tr_idx);
    assert!(
        fired,
        "Training activation must succeed even with an empty deck (cost paid)"
    );

    // Carrier suspended, stack unchanged, deck still empty.
    let perm = &r.game.players[0].battle_area[tr_idx];
    assert!(
        perm.is_suspended,
        "carrier must be suspended (cost paid) even on empty-deck no-op"
    );
    assert_eq!(
        perm.card_sources.len(),
        stack_before,
        "carrier's stack must be unchanged on empty-deck no-op"
    );
    assert!(
        r.game.players[0].deck.is_empty(),
        "deck must remain empty"
    );
}
