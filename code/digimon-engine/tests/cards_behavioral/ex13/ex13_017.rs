//! EX13-017 Veemon — Digimon, Lv.3, Blue, DP 1000, Cost 3.
//! Traits: Mini Dragon, CS. Form: Rookie. Attribute: Free.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-017.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-017.md` agrees)
//!
//! ```text
//! Digivolve: Blue Lv.2 / cost 0.  [Digivolve] Lv.2 w/[CS] trait: Cost 0
//!
//! Effect:
//! [When Moving] [On Play] Reveal the top 3 cards of your deck. Add 1 card with
//! [Veedramon] in its text or the [Royal Knight] trait among them to the hand.
//! Return the rest to the bottom of the deck.
//!
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When this Digimon with [Veedramon] in its name
//! would leave the battle area by your opponent's effects, by suspending it, it
//! doesn't leave.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Blue/EX13_017.cs
//! - Alt digivolve `HasCSTraits`, level 2, cost 0 (requirements kept).
//! - Shared [When Moving]/[On Play] (not optional): `SimplifiedRevealDeckTopCardsAndSelect(3,
//!   HasText("Veedramon") || HasRoyalKnightTraits, AddHand, maxCount 1)`, rest → deck bottom.
//! - Inherited `WhenRemoveField` ActivateClass (OPT, optional): carrier top card
//!   `ContainsCardName("Veedramon")`, removal by an OPPONENT effect,
//!   `CanActivateSuspendCostEffect` → suspend it; on success it doesn't leave.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - Multi-timing [On Play]/[When Moving] reveal-3 add-1 (ST24-04 on_move idiom).
//! - Inherited would-leave replacement with a suspend cost (BT23-058 / ST24-10).
//! - Trait alt-digivolve (Lv.2 w/[CS] / cost 0).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::action::space::{encode_digivolve, PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-017";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn card(id: &str, name: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.colors = vec![CardColor::Blue];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-017 YAML parses, compiles and is in the embedded pack")
        .add_card({
            let mut c = card("VEED-TEXT", "Text Card", 4);
            c.effect_text = "Digivolve into [Veedramon].".to_string();
            c
        })
        .add_card({
            let mut c = card("RK-TRAIT", "Knight Card", 6);
            c.traits = vec!["Royal Knight".to_string()];
            c
        })
        .add_card(card("PLAIN", "Plain Card", 4))
        .add_card(card("VEEDRAMON-X", "Veedramon X", 4))
        .add_card(card("OTHER-L4", "Other Four", 4))
        .add_card({
            let mut c = card("CS-L2", "CS Baby", 2);
            c.colors = vec![CardColor::Yellow];
            c.traits = vec!["CS".to_string()];
            c
        })
        .add_card({
            let mut c = card("YELLOW-L2", "Yellow Baby", 2);
            c.colors = vec![CardColor::Yellow];
            c
        })
        .add_card(card("FILL", "Fill", 3))
}

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn deck_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn triggered(runner: &DebugRunner) -> Vec<CompiledTriggeredClause> {
    runner
        .compiled_card(CARD_ID)
        .expect("compiled")
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

/// Card ids the current reveal prompt offers.
fn offered_reveal_ids(runner: &DebugRunner) -> Vec<String> {
    let view = runner.pending_selection_view().expect("reveal prompt pending");
    assert_eq!(view.kind, SelectionKind::Reveal, "reveal prompt: {view:?}");
    view.valid_action_ids
        .iter()
        .filter_map(|&a| a.checked_sub(SEL_REVEAL_START))
        .filter_map(|i| runner.game.revealed_cards.get(i as usize))
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn pick_revealed(runner: &mut DebugRunner, card_id: &str) {
    let view = runner.pending_selection_view().expect("reveal prompt pending");
    let want = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| {
            a.checked_sub(SEL_REVEAL_START)
                .and_then(|i| runner.game.revealed_cards.get(i as usize))
                .is_some_and(|c| c.card_id(&runner.game.card_data) == card_id)
        })
        .unwrap_or_else(|| panic!("{card_id} must be a legal pick: {view:?}"));
    runner
        .execute_action(view.selecting_player, want)
        .expect("pick revealed card");
}

/// Deck listed BOTTOM-first: the revealed top 3 are the last three ids.
fn play_veemon(top3_bottom_first: &[&str]) -> DebugRunner {
    let mut deck = vec!["FILL", "FILL"];
    deck.extend_from_slice(top3_bottom_first);
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &deck)
        .memory(5)
        .start();
    runner.skip_mulligan();
    runner.play(0, 0).expect("Veemon plays");
    runner
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_017_has_the_blue_circle_and_the_cs_trait_alt_digivolve() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(paths, 2, "Blue Lv.2/0 + Lv.2 w/[CS] trait/0");
}

#[test]
fn ex13_017_clause_shape_matches_printed_text() {
    let runner = builder().start();
    let t = triggered(&runner);
    let reveal: Vec<_> = t
        .iter()
        .filter(|c| c.scope == CompiledScope::FaceUp)
        .collect();
    assert_eq!(reveal.len(), 1, "one shared [On Play]/[When Moving] clause");
    assert!(reveal[0].when.contains(&CompiledTiming::OnPlay));
    assert!(reveal[0].when.contains(&CompiledTiming::OnMove));
    assert!(!reveal[0].optional, "the reveal is mandatory");
    assert!(!reveal[0].once_per_turn);

    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let replacements = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                    scope: CompiledScope::Inherited,
                    once_per_turn: true,
                    optional: true,
                    ..
                })
            )
        })
        .count();
    assert_eq!(replacements, 1, "inherited [All Turns][OPT] optional leave-prevention");
}

#[test]
fn ex13_017_digivolves_from_a_cs_trait_lv2_for_0() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(3)
        .start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "CS-L2", Some(0));
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    let top = runner.game.players[0].battle_area[base.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(top, CARD_ID, "Lv.2 w/[CS] trait: Cost 0");
    assert_eq!(runner.memory(), 3, "cost 0");
}

#[test]
fn ex13_017_cannot_digivolve_from_a_non_cs_yellow_lv2() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(3)
        .start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "YELLOW-L2", Some(0));
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    let top = runner.game.players[0].battle_area[base.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(top, "YELLOW-L2", "neither the blue circle nor the [CS] route applies");
}

// ─── Section 2/3 — [On Play] reveal 3, add 1 ─────────────────────────────────

#[test]
fn ex13_017_on_play_offers_veedramon_text_and_royal_knight_cards_only() {
    let runner = play_veemon(&["PLAIN", "RK-TRAIT", "VEED-TEXT"]);
    let mut offered = offered_reveal_ids(&runner);
    offered.sort();
    assert_eq!(offered, vec!["RK-TRAIT", "VEED-TEXT"]);
}

#[test]
fn ex13_017_on_play_adds_the_pick_and_bottoms_the_rest() {
    let mut runner = play_veemon(&["PLAIN", "RK-TRAIT", "VEED-TEXT"]);
    pick_revealed(&mut runner, "RK-TRAIT");
    let _ = runner.auto_resolve();
    assert_eq!(hand_ids(&runner, 0), vec!["RK-TRAIT"]);
    let deck = deck_ids(&runner, 0);
    assert_eq!(deck.len(), 4, "2 padding + 2 returned");
    let bottom2: Vec<_> = deck[..2].to_vec();
    assert!(bottom2.contains(&"PLAIN".to_string()) && bottom2.contains(&"VEED-TEXT".to_string()),
        "the rest went to the bottom of the deck: {deck:?}");
    assert!(runner.game.revealed_cards.is_empty());
}

#[test]
fn ex13_017_on_play_with_no_eligible_card_adds_nothing() {
    let mut runner = play_veemon(&["PLAIN", "PLAIN", "OTHER-L4"]);
    let _ = runner.auto_resolve();
    assert!(hand_ids(&runner, 0).is_empty());
    let deck = deck_ids(&runner, 0);
    assert_eq!(deck.len(), 5, "all 3 returned");
    assert_eq!(&deck[3..], &["FILL", "FILL"], "padding stays on top; reveal went to the bottom");
}

// ─── Section 2/3 — [When Moving] ─────────────────────────────────────────────

#[test]
fn ex13_017_when_moving_from_breeding_reveals() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "PLAIN", "PLAIN", "VEED-TEXT"])
        .memory(3)
        .start();
    runner.skip_mulligan();
    runner.place_in_breeding(0, CARD_ID);
    assert!(runner.move_from_breeding(0), "move succeeds");
    pick_revealed(&mut runner, "VEED-TEXT");
    let _ = runner.auto_resolve();
    assert_eq!(hand_ids(&runner, 0), vec!["VEED-TEXT"]);
}

#[test]
fn ex13_017_another_digimon_moving_does_not_fire_veemons_when_moving() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "PLAIN", "PLAIN", "VEED-TEXT"])
        .memory(3)
        .start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_in_breeding(0, "OTHER-L4");
    assert!(runner.move_from_breeding(0));
    assert!(runner.pending_selection().is_none(), "[When Moving] is self-scoped");
    assert!(hand_ids(&runner, 0).is_empty());
    assert!(runner.game.revealed_cards.is_empty());
}

// ─── Section 2/3/5 — inherited leave-prevention ──────────────────────────────

fn carrier_setup(top: &str) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .memory(3)
        .start();
    runner.skip_mulligan();
    let carrier = runner.place_stack(0, &[CARD_ID, top]);
    (runner, carrier)
}

fn top_id(runner: &DebugRunner, index: usize) -> Option<String> {
    runner.game.players[0]
        .battle_area
        .get(index)
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
}

fn accept_replacement(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("leave-prevention prompt must be offered");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(view.is_optional, "the leave-prevention is optional");
    let accept = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("accept action");
    runner
        .execute_action(view.selecting_player, accept)
        .expect("accept");
    let _ = runner.auto_resolve();
}

#[test]
fn ex13_017_veedramon_carrier_survives_opponent_deletion_by_suspending() {
    let (mut runner, carrier) = carrier_setup("VEEDRAMON-X");
    runner
        .game
        .delete_permanents_batch(vec![carrier], ReplacementCause::OpponentEffect);
    accept_replacement(&mut runner);
    assert_eq!(top_id(&runner, carrier.index as usize).as_deref(), Some("VEEDRAMON-X"));
    assert!(
        runner.game.players[0].battle_area[carrier.index as usize].is_suspended,
        "the cost suspended it"
    );
    assert_eq!(runner.trash_size(0), 0);
}

#[test]
fn ex13_017_declining_the_leave_prevention_lets_it_leave() {
    let (mut runner, carrier) = carrier_setup("VEEDRAMON-X");
    runner
        .game
        .delete_permanents_batch(vec![carrier], ReplacementCause::OpponentEffect);
    let view = runner.pending_selection_view().expect("prompt offered");
    runner.execute_action(view.selecting_player, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(0), 0, "deleted");
}

#[test]
fn ex13_017_non_veedramon_carrier_gets_no_leave_prevention() {
    let (mut runner, carrier) = carrier_setup("OTHER-L4");
    runner
        .game
        .delete_permanents_batch(vec![carrier], ReplacementCause::OpponentEffect);
    assert!(runner.pending_selection().is_none(), "no [Veedramon] in its name");
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(0), 0);
}

#[test]
fn ex13_017_own_effect_removal_is_not_prevented() {
    let (mut runner, carrier) = carrier_setup("VEEDRAMON-X");
    runner
        .game
        .delete_permanents_batch(vec![carrier], ReplacementCause::OwnEffect);
    assert!(runner.pending_selection().is_none(), "only opponent's effects");
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(0), 0);
}

#[test]
fn ex13_017_already_suspended_carrier_cannot_pay_the_cost() {
    let (mut runner, carrier) = carrier_setup("VEEDRAMON-X");
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = true;
    runner
        .game
        .delete_permanents_batch(vec![carrier], ReplacementCause::OpponentEffect);
    assert!(runner.pending_selection().is_none(), "suspend cost unpayable");
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(0), 0);
}

#[test]
fn ex13_017_leave_prevention_is_once_per_turn() {
    let (mut runner, carrier) = carrier_setup("VEEDRAMON-X");
    runner
        .game
        .delete_permanents_batch(vec![carrier], ReplacementCause::OpponentEffect);
    accept_replacement(&mut runner);
    // Unsuspend it so only the OPT lock (not the cost) gates the second window.
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = false;
    runner
        .game
        .delete_permanents_batch(vec![carrier], ReplacementCause::OpponentEffect);
    assert!(runner.pending_selection().is_none(), "[Once Per Turn] already used");
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(0), 0);
}

#[test]
fn ex13_017_leave_prevention_lock_clears_next_turn() {
    let (mut runner, carrier) = carrier_setup("VEEDRAMON-X");
    runner
        .game
        .delete_permanents_batch(vec![carrier], ReplacementCause::OpponentEffect);
    accept_replacement(&mut runner);
    runner.end_turn();
    let _ = runner.auto_resolve();
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = false;
    runner
        .game
        .delete_permanents_batch(vec![carrier], ReplacementCause::OpponentEffect);
    accept_replacement(&mut runner);
    assert_eq!(top_id(&runner, carrier.index as usize).as_deref(), Some("VEEDRAMON-X"));
}
