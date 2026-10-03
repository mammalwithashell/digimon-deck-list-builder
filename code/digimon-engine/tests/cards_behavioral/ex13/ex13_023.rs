//! EX13-023 UlforceVeedramon — Digimon, Lv.6, Blue, DP 12000, Cost 12.
//! Traits: Holy Warrior / Royal Knight / CS. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-023.md`)
//!
//! ```text
//! Digivolve: Blue Lv.5 / 3; [Digivolve] Lv.5 w/[CS] trait: Cost 3
//! Assembly -5: Lv.5 × Lv.4 × Lv.3, all w/[Veemon]/[Veedramon] in name
//! <Blocker> <Evade>
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] 1 of your
//! Digimon may change orientation.
//! [On Play] [When Digivolving] You may return all of your opponent's Digimon
//! with the fewest digivolution cards to the bottom of the deck.
//! [All Turns] Your opponent's effects can't reduce this unsuspended Digimon's
//! DP, return its stacked cards to the hand or deck, or trash them.
//! ```
//! Official Q&A: prevents "(perform XX on) stacked cards" (incl. the top
//! stacked cards trashed by <De-Digivolve>, "trash a digivolution card"), but
//! XX on the Digimon itself is still allowed.
//!
//! # DCGO C# reference
//! `EX13_023.cs` — change orientation (canNoSelect, `RemoveUse()` on no pick),
//! optional `DeckBottomBounceClass` of `IsMinDigivolutionCards` Digimon, and
//! three `IsOpponentEffect` + `!IsSuspended` statics (ImmuneFromDPMinus,
//! CanNotBeTrashedBySkill, CanNotBeReturnedToLibraryBySkill).
//!
//! # Verdict — IMPLEMENTED
//! [All Turns] via G-ENGINE-STACKED-CARD-RETURN-PROTECTION (RESOLVED
//! 2026-10-01): aura `modifier_from: opponent` + `ImmuneFromStackReturn` +
//! controller-scoped `ImmuneFromStackTrashing` (incl. <De-Digivolve>).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledDeclarativeClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, Expiry};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "EX13-023";

fn digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = 3;
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-023 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("L3", 3))
        .add_card(digimon("L4", 4))
        .add_card(digimon("L5", 5))
        .add_card(digimon("ALLY", 4))
        .add_card(digimon("OPP", 5))
        .add_card(digimon("FILL", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start()
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn set_suspended(r: &mut DebugRunner, h: PermanentHandle, v: bool) {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = v;
    r.game.tick_declarative_effects();
}

fn pick(r: &mut DebugRunner, idx: usize) {
    let v = r.pending_selection_view().expect("pending");
    r.execute_action(v.selecting_player, v.valid_action_ids[idx])
        .expect("pick");
}

/// Both [On Play] clauses queue together when an opponent Digimon exists:
/// resolve the order prompt (first entry = the orientation clause).
fn order_first(r: &mut DebugRunner) {
    while r.pending_kind() == Some(digimon_engine::selection::SelectionKind::TriggerOrder) {
        let v = r.pending_selection_view().unwrap();
        r.execute_action(v.selecting_player, v.valid_action_ids[0]).unwrap();
    }
}

fn decline(r: &mut DebugRunner) {
    r.decline_optional_trigger().expect("decline");
}

/// An opponent-controlled (player 1) effect context.
fn opp_ctx(r: &mut DebugRunner) -> EffectContext<'_> {
    let card = r.game.players[1].deck[0].handle();
    EffectContext::new(&mut r.game, card, None, 1)
}

/// An own (player 0) effect context.
fn own_ctx(r: &mut DebugRunner) -> EffectContext<'_> {
    let card = r.game.players[0].deck[0].handle();
    EffectContext::new(&mut r.game, card, None, 0)
}

fn sources(r: &DebugRunner, h: PermanentHandle) -> usize {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .len()
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_023_identity_alt_paths_and_keywords() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    let count = |k: CompiledAltPathKind| card.alt_paths.iter().filter(|p| p.kind == k).count();
    assert_eq!(count(CompiledAltPathKind::Digivolve), 2);
    assert_eq!(count(CompiledAltPathKind::Assembly), 1);
    let keywords: Vec<&str> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword, ..
            }) => Some(keyword.as_str()),
            _ => None,
        })
        .collect();
    assert!(keywords.contains(&"Blocker") && keywords.contains(&"Evade"), "{keywords:?}");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [OP][WD][WA][OPT] change orientation
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_023_on_play_unsuspends_a_suspended_digimon() {
    let mut r = runner();
    let ally = r.place_on_field(0, "ALLY", Some(0));
    set_suspended(&mut r, ally, true);
    let uv = r.place_stack(0, &["L5", CARD_ID]);
    r.fire_on_play(0, uv.index as usize);
    let v = r.pending_selection_view().expect("orientation pick");
    assert!(v.is_optional, "'may' — PASS is legal");
    assert_eq!(v.valid_action_ids.len(), 2, "both own Digimon can change orientation");
    // Pick the ally (placed first → lower field index → first action).
    pick(&mut r, 0);
    // The deck-bounce offer has no opponent Digimon: nothing else pending.
    assert!(!suspended(&r, ally), "the suspended Digimon unsuspends");
    assert!(!suspended(&r, uv));
}

#[test]
fn ex13_023_on_play_suspends_an_unsuspended_digimon() {
    let mut r = runner();
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let uv = r.place_stack(0, &["L5", CARD_ID]);
    r.fire_on_play(0, uv.index as usize);
    pick(&mut r, 0);
    assert!(suspended(&r, ally), "the unsuspended Digimon suspends");
}

#[test]
fn ex13_023_declining_the_orientation_refunds_the_once_per_turn() {
    let mut r = runner();
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let uv = r.place_stack(0, &["L5", CARD_ID]);
    r.fire_on_play(0, uv.index as usize);
    decline(&mut r);
    assert!(!suspended(&r, ally));
    // DCGO RemoveUse(): the [When Attacking] can still offer it this turn.
    r.attack_player(uv, 1, false);
    let v = r.pending_selection_view().expect("[When Attacking] offers it again");
    assert!(v.is_optional);
    // Only the ally can change orientation (UlforceVeedramon is suspended by
    // attacking and has no CannotUnsuspend — it is offered too).
    assert!(!v.valid_action_ids.is_empty());
}

#[test]
fn ex13_023_used_orientation_is_once_per_turn() {
    let mut r = runner();
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let uv = r.place_stack(0, &["L5", CARD_ID]);
    r.fire_on_play(0, uv.index as usize);
    pick(&mut r, 0);
    assert!(suspended(&r, ally));
    r.attack_player(uv, 1, false);
    assert!(
        r.pending_selection_view().is_none(),
        "[Once Per Turn]: no second orientation change this turn"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — [OP][WD] may return all fewest-source opponent Digimon
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_023_returns_every_fewest_source_opponent_digimon_to_deck_bottom() {
    let mut r = runner();
    let _a = r.place_stack(1, &["FILL", "OPP"]);
    let _b = r.place_stack(1, &["FILL", "FILL", "OPP"]);
    let _c = r.place_stack(1, &["FILL", "OPP"]);
    let uv = r.place_stack(0, &["L5", CARD_ID]);
    let deck_before = r.game.players[1].deck.len();
    r.fire_on_play(0, uv.index as usize);
    order_first(&mut r);
    // Orientation pick first (only UlforceVeedramon itself): decline it.
    decline(&mut r);
    assert!(r.pending_is_optional(), "'You may return…' is offered");
    r.accept_optional_trigger().expect("accept bounce");
    let _ = r.auto_resolve();
    let left: Vec<usize> = r.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.card_sources.len())
        .collect();
    assert_eq!(left, vec![3], "only the 2-source Digimon stays");
    assert_eq!(
        r.game.players[1].deck.len(),
        deck_before + 4,
        "both 1-source Digimon returned with their cards to the deck"
    );
}

#[test]
fn ex13_023_bounce_can_be_declined() {
    let mut r = runner();
    let _a = r.place_stack(1, &["FILL", "OPP"]);
    let uv = r.place_stack(0, &["L5", CARD_ID]);
    r.fire_on_play(0, uv.index as usize);
    order_first(&mut r);
    decline(&mut r);
    decline(&mut r);
    assert_eq!(r.game.players[1].battle_area.len(), 1);
}

// ════════════════════════════════════════════════════════════════════════════
// Section 4 — [All Turns] opponent-effect protections while unsuspended
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_023_unsuspended_ignores_opponent_dp_reduction_only() {
    let mut r = runner();
    let uv = r.place_stack(0, &["L5", CARD_ID]);
    r.game.tick_declarative_effects();
    opp_ctx(&mut r).add_dp_modifier(uv, -5000, Expiry::EndOfTurn);
    assert_eq!(r.effective_dp(uv), Some(12000), "opponent's -5000 doesn't reduce it");
    own_ctx(&mut r).add_dp_modifier(uv, -2000, Expiry::EndOfTurn);
    assert_eq!(r.effective_dp(uv), Some(10000), "own DP reduction still applies");
    set_suspended(&mut r, uv, true);
    assert_eq!(r.effective_dp(uv), Some(5000), "suspended: the opponent's -5000 applies");
}

#[test]
fn ex13_023_unsuspended_stacked_cards_cant_be_returned_by_opponent() {
    let mut r = runner();
    let uv = r.place_stack(0, &["L3", "L4", "L5", CARD_ID]);
    r.game.tick_declarative_effects();
    let l3 = r.game.players[0].battle_area[uv.index as usize].card_sources[0].handle();
    let l4 = r.game.players[0].battle_area[uv.index as usize].card_sources[1].handle();
    assert!(!opp_ctx(&mut r).return_card_source_to_hand(uv, l3), "→ hand blocked");
    assert!(!opp_ctx(&mut r).return_card_source_to_deck(uv, l3, true), "→ deck blocked");
    assert_eq!(sources(&r, uv), 4);
    assert!(own_ctx(&mut r).return_card_source_to_hand(uv, l4), "own effect may");
    set_suspended(&mut r, uv, true);
    assert!(opp_ctx(&mut r).return_card_source_to_deck(uv, l3, true), "suspended: allowed");
}

#[test]
fn ex13_023_unsuspended_stacked_cards_cant_be_trashed_or_de_digivolved_by_opponent() {
    let mut r = runner();
    let uv = r.place_stack(0, &["L3", "L4", "L5", CARD_ID]);
    r.game.tick_declarative_effects();
    let l3 = r.game.players[0].battle_area[uv.index as usize].card_sources[0].handle();
    assert!(!opp_ctx(&mut r).trash_card_source(uv, l3), "trash a digivolution card blocked");
    assert_eq!(opp_ctx(&mut r).trash_bottom_sources(uv, 1), 0);
    assert_eq!(
        opp_ctx(&mut r).de_digivolve(uv, Some(3), Some(1)),
        0,
        "<De-Digivolve> trashes the top stacked card — blocked"
    );
    assert_eq!(sources(&r, uv), 4);
    set_suspended(&mut r, uv, true);
    assert_eq!(opp_ctx(&mut r).de_digivolve(uv, Some(3), Some(1)), 1, "suspended: allowed");
}

#[test]
fn ex13_023_protection_does_not_stop_effects_on_the_digimon_itself() {
    // Official Q&A: XX on the Digimon itself is still permitted — returning
    // the whole Digimon to the hand is not "returning its stacked cards".
    let mut r = runner();
    let uv = r.place_stack(0, &["L5", CARD_ID]);
    r.game.tick_declarative_effects();
    let hand_before = r.game.players[0].hand.len();
    let mut ctx = opp_ctx(&mut r);
    let _ = ctx.return_to_hand(uv);
    r.game.drain_effect_queue();
    assert!(r.game.players[0].battle_area.is_empty(), "bounced");
    assert_eq!(r.game.players[0].hand.len(), hand_before + 1, "top card → hand");
}
