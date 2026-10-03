//! EX13-027 Chuumon — Digimon, Lv.3, Yellow, DP 1000, Cost 3.
//! Traits: Beast. Form: Rookie. Attribute: Virus.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-027.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-027.md` agrees)
//!
//! ```text
//! Digivolve: Yellow Lv.2 / cost 0;  Black Lv.2 / cost 0
//!
//! [When Moving] [On Play] Reveal the top 3 cards of your deck. Among them, add
//! 1 card with [Sukamon] or [Etemon] in its name to the hand and trash 1 such
//! card. Return the rest to the bottom of the deck.
//!
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When this Digimon would leave the battle area
//! other than by your effects, by deleting 1 other Digimon with [Sukamon] in
//! its name, it doesn't leave.
//! ```
//! Official Q&A: with only 1 qualifying card revealed, that card is added to
//! the hand (the hand pick is resolved first).
//!
//! # DCGO C# reference
//! None — DCGO has no `EX13_027.cs` at `b9a0638cd`. The inherited clause is
//! EX11-022 Karakurumon's inherited leave-prevention with a [Sukamon]-name
//! cost; the reveal clause is the EX13-026 Kudamon [When Moving][On Play]
//! shape with a hand bucket + a trash bucket (`reveal_search`).
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - A2/A3 reveal-3 two-bucket pick (add to hand + trash), remainder bottom.
//! - Shared [When Moving] + [On Play] trigger.
//! - F3 inherited leave-prevention replacement, OPT, select-and-delete cost,
//!   "other than by your effects" cause gate.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-027";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = 3;
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-027 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("SUKA", "Sukamon", 4))
        .add_card(digimon("SUKA2", "PlatinumSukamon", 5))
        .add_card(digimon("ETE", "Etemon", 5))
        .add_card(digimon("ETE2", "MetalEtemon", 6))
        .add_card(digimon("PLAIN", "Plainmon", 4))
        .add_card(digimon("FILL", "Filler", 3))
        .add_card(digimon("CARRIER", "Carrier", 4))
        .add_card(digimon("OTHER-SUKA", "Sukamon Ally", 4))
        .add_card(digimon("OTHER-PLAIN", "Plain Ally", 4))
}

/// Chuumon in hand, `deck_top` stacked on the deck (index 0 = first revealed).
fn setup(deck_top: &[&str]) -> DebugRunner {
    let mut deck: Vec<&str> = vec!["FILL"; 4];
    for c in deck_top.iter().rev() {
        deck.push(c);
    }
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &deck)
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.skip_mulligan();
    r
}

fn hand_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn trash_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn field_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn handle_of(r: &DebugRunner, id: &str) -> PermanentHandle {
    let i = field_ids(r)
        .iter()
        .position(|c| c == id)
        .unwrap_or_else(|| panic!("{id} on field"));
    PermanentHandle {
        player: 0,
        index: i as u8,
    }
}

fn offered_reveal_ids(r: &DebugRunner) -> Vec<String> {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let mut ids: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| a.checked_sub(SEL_REVEAL_START))
        .filter_map(|i| r.game.revealed_cards.get(i as usize))
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    ids.sort();
    ids
}

fn pick_revealed(r: &mut DebugRunner, card_id: &str) {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let want = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .find(|&a| {
            a.checked_sub(SEL_REVEAL_START)
                .and_then(|i| r.game.revealed_cards.get(i as usize))
                .is_some_and(|c| c.card_id(&r.game.card_data) == card_id)
        })
        .unwrap_or_else(|| panic!("{card_id} must be a legal pick: {view:?}"));
    r.execute_action(view.selecting_player, want)
        .expect("pick revealed card");
}

fn deck_bottom_ids(r: &DebugRunner, n: usize) -> Vec<String> {
    r.game.players[0].deck[..n]
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_027_printed_metadata_and_digivolve_paths() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(3), Some(1000), Some(3)));
    assert_eq!(c.color, vec![CompiledColor::Yellow]);
    assert_eq!(c.traits, vec!["Beast".to_string()]);
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 2, "Yellow Lv.2 and Black Lv.2 circles");
    for p in digi {
        assert_eq!(p.cost, Some(CompiledCost::Literal(0)));
    }
}

#[test]
fn ex13_027_clause_shape() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    let t = triggered[0];
    assert!(t.when.contains(&CompiledTiming::OnPlay));
    assert!(t.when.contains(&CompiledTiming::OnMove));
    assert!(!t.optional, "reveal is mandatory");
    assert_eq!(t.scope, CompiledScope::FaceUp);

    let replacements: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                scope,
                optional,
                once_per_turn,
                ..
            }) => Some((*scope, *optional, *once_per_turn)),
            _ => None,
        })
        .collect();
    assert_eq!(
        replacements,
        vec![(CompiledScope::Inherited, true, true)],
        "one inherited, optional (by deleting…), once-per-turn replacement"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2/3 — [On Play] reveal: add 1 + trash 1 [Sukamon]/[Etemon]-named
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_027_on_play_offers_only_sukamon_or_etemon_named_cards() {
    let mut r = setup(&["SUKA", "ETE", "PLAIN"]);
    r.play(0, 0).expect("Chuumon played");
    assert_eq!(
        offered_reveal_ids(&r),
        vec!["ETE".to_string(), "SUKA".to_string()],
        "only [Sukamon]/[Etemon]-named cards are eligible"
    );
}

#[test]
fn ex13_027_on_play_adds_one_trashes_one_and_bottoms_the_rest() {
    let mut r = setup(&["SUKA", "ETE", "PLAIN"]);
    r.play(0, 0).expect("Chuumon played");
    pick_revealed(&mut r, "ETE"); // hand bucket
    pick_revealed(&mut r, "SUKA"); // trash bucket
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r), vec!["ETE".to_string()]);
    assert_eq!(trash_ids(&r), vec!["SUKA".to_string()]);
    assert_eq!(deck_bottom_ids(&r, 1), vec!["PLAIN".to_string()]);
    assert!(r.pending_selection().is_none());
}

#[test]
fn ex13_027_name_contains_matches_compound_names() {
    let mut r = setup(&["SUKA2", "ETE2", "PLAIN"]);
    r.play(0, 0).expect("Chuumon played");
    assert_eq!(
        offered_reveal_ids(&r),
        vec!["ETE2".to_string(), "SUKA2".to_string()],
        "[Sukamon]/[Etemon] *in its name* includes PlatinumSukamon / MetalEtemon"
    );
}

#[test]
fn ex13_027_single_match_goes_to_hand_not_trash() {
    let mut r = setup(&["PLAIN", "SUKA", "FILL"]);
    r.play(0, 0).expect("Chuumon played");
    let _ = r.auto_resolve();
    assert_eq!(
        hand_ids(&r),
        vec!["SUKA".to_string()],
        "Official Q&A: the lone match is added to the hand"
    );
    assert!(trash_ids(&r).is_empty(), "nothing left to trash");
    assert_eq!(r.deck_size(0), 6, "the 2 others go to the deck bottom");
}

#[test]
fn ex13_027_no_match_returns_all_to_bottom() {
    let mut r = setup(&["PLAIN", "FILL", "PLAIN"]);
    r.play(0, 0).expect("Chuumon played");
    let _ = r.auto_resolve();
    assert!(hand_ids(&r).is_empty());
    assert!(trash_ids(&r).is_empty());
    assert_eq!(r.deck_size(0), 7);
}

#[test]
fn ex13_027_when_moving_from_breeding_fires_reveal() {
    let mut r = setup(&["SUKA", "PLAIN", "PLAIN"]);
    r.game.players[0].hand.clear();
    r.place_in_breeding(0, CARD_ID);
    assert!(r.move_from_breeding(0), "move succeeds");
    assert!(
        r.pending_selection().is_some(),
        "[When Moving] fires the reveal clause"
    );
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r), vec!["SUKA".to_string()]);
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3/5 — Inherited [All Turns][OPT] leave prevention
// ════════════════════════════════════════════════════════════════════════════

fn inherited_setup() -> (DebugRunner, PermanentHandle) {
    let mut r = builder().deck(0, &["FILL"; 6]).deck(1, &["FILL"; 6]).start();
    r.skip_mulligan();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    (r, carrier)
}

#[test]
fn ex13_027_inherited_deletes_other_sukamon_and_carrier_stays() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "OTHER-SUKA", Some(0));
    r.place_on_field(0, "OTHER-PLAIN", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let accept = r.pending_selection_view().expect("replacement accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    assert!(accept.is_optional);
    r.execute_action(0, accept.valid_action_ids[0])
        .expect("accept replacement");

    let cost = r.pending_selection_view().expect("select cost Digimon");
    assert_eq!(cost.kind, SelectionKind::OwnField);
    let offered: Vec<_> = cost
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .collect();
    assert_eq!(offered.len(), 1, "only the other [Sukamon]-named Digimon");
    r.execute_action(0, *offered[0]).expect("pay cost");
    let _ = r.auto_resolve();

    let field = field_ids(&r);
    assert!(field.contains(&"CARRIER".to_string()), "carrier doesn't leave");
    assert!(!field.contains(&"OTHER-SUKA".to_string()), "cost Digimon deleted");
    assert!(field.contains(&"OTHER-PLAIN".to_string()));
}

#[test]
fn ex13_027_inherited_not_offered_without_other_sukamon() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "OTHER-PLAIN", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(
        r.pending_selection_view().is_none(),
        "no payable cost → no prevention offer"
    );
    assert!(!field_ids(&r).contains(&"CARRIER".to_string()));
}

#[test]
fn ex13_027_inherited_carrier_itself_named_sukamon_is_not_other() {
    // The carrier is the only [Sukamon]: "other" excludes it.
    let mut r = builder().deck(0, &["FILL"; 6]).start();
    r.skip_mulligan();
    let carrier = r.place_stack(0, &[CARD_ID, "SUKA"]);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none());
    assert!(!field_ids(&r).contains(&"SUKA".to_string()));
}

#[test]
fn ex13_027_inherited_not_offered_for_own_effect_removal() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "OTHER-SUKA", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OwnEffect);
    assert!(
        r.pending_selection_view().is_none(),
        "\"other than by your effects\""
    );
    assert!(!field_ids(&r).contains(&"CARRIER".to_string()));
}

#[test]
fn ex13_027_inherited_decline_lets_carrier_leave() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "OTHER-SUKA", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    let accept = r.pending_selection_view().expect("accept prompt");
    assert!(accept.is_optional, "\"by deleting…\" is a may-pay");
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    let field = field_ids(&r);
    assert!(!field.contains(&"CARRIER".to_string()));
    assert!(field.contains(&"OTHER-SUKA".to_string()));
}

#[test]
fn ex13_027_inherited_protects_only_this_digimon() {
    // Another of your Digimon leaving is not protected by Chuumon's inherited.
    let (mut r, _carrier) = inherited_setup();
    let suka = r.place_on_field(0, "OTHER-SUKA", Some(0));
    let plain = r.place_on_field(0, "OTHER-PLAIN", Some(0));
    let _ = suka;
    r.game
        .delete_permanent_with_cause(plain, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none());
    assert!(!field_ids(&r).contains(&"OTHER-PLAIN".to_string()));
}

#[test]
fn ex13_027_inherited_once_per_turn_and_resets() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "OTHER-SUKA", Some(0));
    r.place_on_field(0, "SUKA", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    let accept = r.pending_selection_view().expect("accept prompt");
    r.execute_action(0, accept.valid_action_ids[0]).expect("accept");
    let cost = r.pending_selection_view().expect("cost prompt");
    let pick = *cost
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("a cost");
    r.execute_action(0, pick).expect("pay");
    let _ = r.auto_resolve();
    assert!(field_ids(&r).contains(&"CARRIER".to_string()));

    // Second removal in the same turn is not prevented.
    let carrier = handle_of(&r, "CARRIER");
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(
        r.pending_selection_view().is_none(),
        "once per turn: not offered again this turn"
    );
    assert!(!field_ids(&r).contains(&"CARRIER".to_string()));
}

#[test]
fn ex13_027_inherited_lockout_clears_next_turn() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "OTHER-SUKA", Some(0));
    r.place_on_field(0, "SUKA", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    let accept = r.pending_selection_view().expect("accept prompt");
    r.execute_action(0, accept.valid_action_ids[0]).expect("accept");
    let cost = r.pending_selection_view().expect("cost prompt");
    let pick = *cost
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("a cost");
    r.execute_action(0, pick).expect("pay");
    let _ = r.auto_resolve();

    r.end_turn();
    let _ = r.auto_resolve();
    let carrier = handle_of(&r, "CARRIER");
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(
        r.pending_selection_view().is_some(),
        "OPT resets on the next turn"
    );
}
