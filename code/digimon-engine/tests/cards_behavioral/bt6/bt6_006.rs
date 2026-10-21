//! BT6-006 Tsunomon — Digi-Egg, Lv.2, Purple. Trait: Lesser.
//!
//! Printed text (official Bandai DB — data/card_bundles/BT6-006.md; card image
//! agrees):
//!   Inherited Effect: [Your Turn][Once Per Turn] When you trash a card in your
//!   hand using one of your effects, trigger <Draw 1>. (Draw 1 card from your
//!   deck.)
//!   Official Q&A: "Yes, it does." (the bundle keeps only the answer).
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/BT6/Purple/BT6_006.cs —
//! OnDiscardHand, inherited, maxCountPerTurn 1, mandatory (`optional: false`).
//! CanUseCondition: on the battle area, owner's turn, and
//! `CanTriggerOnTrashHand(hashtable, effect source owner == card owner,
//! trashed card owner == card owner)` — own effect AND own hand.
//!
//! Pattern rows: G4 (inherited trigger on a stacked egg), F-opt (OPT lockout).

#[path = "../bt26/support.rs"]
mod support;

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use support::*;

const CARD_ID: &str = "BT6-006";

/// Test-only [On Play] "your opponent trashes 1 card in their hand" — an
/// effect controlled by THIS card's controller that trashes the OTHER
/// player's hand.
const OPP_HAND_TRASHER_YAML: &str = r#"
card: T-OPPHANDTRASH
name: OppHandTrasher
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
traits: []
effects:
  - when: on_play
    summary: "[On Play] Trash 1 card in your opponent's hand"
    process:
      - select_hand:
          of: opponent
          bind_as: gone
          filter: {}
          prompt: "Trash 1 card in your opponent's hand"
      - trash_from_hand_by_index: { of: opponent, hand_index: gone }
"#;

fn setup(first: u8) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT6-006")
        .from_dsl_yaml(HAND_TRASHER_YAML)
        .expect("trasher")
        .from_dsl_yaml(OPP_HAND_TRASHER_YAML)
        .expect("opp trasher")
        .add_card(filler("FILLER"))
        .add_card(filler("DUD"))
        .add_card(digimon("HOST3", "Host", CardColor::Purple, 3, 3, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(first);
    r
}

/// Tsunomon under a Lv.3 host so its inherited text is live.
fn board(r: &mut DebugRunner) -> PermanentHandle {
    r.place_stack(0, &[CARD_ID, "HOST3"])
}

/// Player `p`'s own effect trashes `card_id` from `p`'s hand.
fn own_effect_trash(r: &mut DebugRunner, p: u8, card_id: &str) {
    let t = r.place_on_field(p, "T-HANDTRASH", Some(0));
    fire(r, EffectTiming::OnPlay, t);
    pick_hand(r, p, card_id);
    let _ = r.auto_resolve();
}

#[test]
fn bt6_006_structure_single_inherited_opt_mandatory_discard_observer() {
    use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
    let r = setup(0);
    let compiled = r.compiled_card(CARD_ID).expect("compiled BT6-006");
    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    let clause = triggered[0];
    assert_eq!(clause.scope, CompiledScope::Inherited);
    assert_eq!(clause.when, vec![CompiledTiming::OnDiscardHand]);
    assert!(clause.once_per_turn, "[Once Per Turn]");
    assert!(!clause.optional, "\"trigger <Draw 1>\" is mandatory");
}

#[test]
fn bt6_006_own_effect_trashing_own_hand_draws_one() {
    let mut r = setup(0);
    board(&mut r);
    push_hand(&mut r, 0, "DUD");
    let deck_before = r.deck_size(0);
    own_effect_trash(&mut r, 0, "DUD");
    assert!(r.pending_selection_view().is_none(), "the draw needs no choice");
    assert!(trash_ids(&r, 0).contains(&"DUD".to_string()));
    assert_eq!(r.hand_size(0), 1, "trashed 1, drew 1");
    assert_eq!(r.deck_size(0), deck_before - 1);
}

#[test]
fn bt6_006_no_draw_on_opponents_turn() {
    let mut r = setup(1);
    board(&mut r);
    push_hand(&mut r, 0, "DUD");
    let deck_before = r.deck_size(0);
    own_effect_trash(&mut r, 0, "DUD");
    assert_eq!(r.hand_size(0), 0, "[Your Turn]: no draw");
    assert_eq!(r.deck_size(0), deck_before);
}

#[test]
fn bt6_006_no_draw_when_opponents_effect_trashes_your_hand() {
    let mut r = setup(0);
    board(&mut r);
    push_hand(&mut r, 0, "DUD");
    let deck_before = r.deck_size(0);
    // Player 1's effect trashes player 0's hand, during player 0's turn.
    let t = r.place_on_field(1, "T-OPPHANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    drain_first(&mut r);
    assert!(trash_ids(&r, 0).contains(&"DUD".to_string()), "the hand was trashed");
    assert_eq!(r.hand_size(0), 0, "not \"one of YOUR effects\": no draw");
    assert_eq!(r.deck_size(0), deck_before);
}

#[test]
fn bt6_006_no_draw_when_your_effect_trashes_opponents_hand() {
    let mut r = setup(0);
    board(&mut r);
    push_hand(&mut r, 1, "DUD");
    let deck_before = r.deck_size(0);
    let t = r.place_on_field(0, "T-OPPHANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    drain_first(&mut r);
    assert!(trash_ids(&r, 1).contains(&"DUD".to_string()), "opponent's hand was trashed");
    assert_eq!(r.hand_size(0), 0, "not a card in YOUR hand: no draw");
    assert_eq!(r.deck_size(0), deck_before);
}

#[test]
fn bt6_006_tsunomon_alone_in_breeding_shape_does_not_draw() {
    // The text is inherited: with Tsunomon as the TOP card (not a source) it
    // is not live.
    let mut r = setup(0);
    r.place_stack(0, &[CARD_ID]);
    push_hand(&mut r, 0, "DUD");
    let deck_before = r.deck_size(0);
    own_effect_trash(&mut r, 0, "DUD");
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(r.deck_size(0), deck_before);
}

#[test]
fn bt6_006_once_per_turn_and_resets_next_turn() {
    let mut r = setup(0);
    board(&mut r);
    push_hand(&mut r, 0, "DUD");
    push_hand(&mut r, 0, "DUD");
    own_effect_trash(&mut r, 0, "DUD");
    assert_eq!(r.hand_size(0), 2, "1 left + 1 drawn");
    let deck_after_first = r.deck_size(0);

    // Same turn: a second own-effect hand trash does not draw again.
    own_effect_trash(&mut r, 0, "DUD");
    assert_eq!(r.deck_size(0), deck_after_first, "OPT spent this turn");

    // Next own turn: the lockout has cleared.
    r.end_turn();
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.game.turn_player(), 0);
    push_hand(&mut r, 0, "DUD");
    let deck_before = r.deck_size(0);
    own_effect_trash(&mut r, 0, "DUD");
    assert_eq!(r.deck_size(0), deck_before - 1, "draws again on the next turn");
}
