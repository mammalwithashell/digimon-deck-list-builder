//! BT12-030 Imperialdramon: Dragon Mode — Digimon, Lv.6, Blue/Green, DP 12000,
//! Cost 12. Traits: Ancient Dragon. Form: Mega. Attribute: Free.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/BT12-030.md`)
//!
//! Digivolve: Blue Lv.5 / 4, Green Lv.5 / 4, Blue Lv.4 / 3, Green Lv.4 / 3.
//! [Digivolve] 3 from [Paildramon] or [Dinobeemon]
//!
//! [When Digivolving] If this Digimon has a blue digivolution card, unsuspend
//! this Digimon. If it has a green digivolution card, suspend 1 of your
//! opponent's Digimon. [End of Attack] This Digimon may digivolve into a
//! Digimon card with [Imperialdramon] in its name in your hand for the
//! digivolution cost. When this Digimon would digivolve by this effect, reduce
//! the digivolution cost by 2.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Blue/BT12_030.cs
//!
//! # Patterns
//! - A1: name-based alt digivolve (no color/level gate).
//! - C2: source-color conditional branches (self_source_count).
//! - S1: mandatory single-target opponent suspend (player choice).
//! - D2: [End of Attack] optional effect-initiated digivolve from hand, cost −2.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT12-030";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, color: CardColor, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.colors = vec![color];
    c.dp = Some(dp);
    c.play_cost = 3;
    c
}

fn with_evo(mut c: CardData, from_color: CardColor, from_level: u8, cost: u16) -> CardData {
    c.evo_costs.push(EvoCost {
        card_color: from_color as u8,
        level: from_level,
        memory_cost: cost,
    });
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-030 YAML loads")
        // Red bases so only the name route (not a color circle) applies.
        .add_card(digimon("PAIL", "Paildramon", CardColor::Red, 5, 8000))
        .add_card(digimon("DINO", "Dinobeemon", CardColor::Red, 5, 8000))
        .add_card(digimon("STING", "Stingmon", CardColor::Red, 5, 8000))
        .add_card(digimon("BLUE5", "Blue Lv5", CardColor::Blue, 5, 7000))
        .add_card(digimon("BLUE-SRC", "Veemon", CardColor::Blue, 4, 5000))
        .add_card(digimon("GREEN-SRC", "Wormmon", CardColor::Green, 4, 5000))
        .add_card(digimon("RED-SRC", "Agumon", CardColor::Red, 4, 5000))
        .add_card(with_evo(
            digimon("PALADIN", "Imperialdramon: Paladin Mode", CardColor::Blue, 7, 15000),
            CardColor::Blue,
            6,
            6,
        ))
        .add_card(with_evo(
            digimon("IMP-RED", "Imperialdramon: Red Mode", CardColor::Red, 7, 15000),
            CardColor::Red,
            6,
            6,
        ))
        .add_card(with_evo(
            digimon("OMNI", "Omnimon", CardColor::Blue, 7, 15000),
            CardColor::Blue,
            6,
            6,
        ))
        .add_card(digimon("OPP-A", "Opp A", CardColor::Red, 4, 5000))
        .add_card(digimon("OPP-B", "Opp B", CardColor::Red, 4, 5000))
        .add_card(digimon("FILL", "Filler", CardColor::Red, 3, 1000))
}

fn start(hand: &[&str]) -> DebugRunner {
    let mut r = builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(10)
        .start();
    r.game.current_phase = GamePhase::Main;
    r
}

fn ids(r: &DebugRunner, cards: &[digimon_engine::card_source::CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].hand)
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn handle_of(r: &DebugRunner, p: u8, id: &str) -> PermanentHandle {
    let i = field_ids(r, p)
        .iter()
        .position(|c| c == id)
        .unwrap_or_else(|| panic!("{id} on player {p}'s field: {:?}", field_ids(r, p)));
    PermanentHandle {
        player: p,
        index: i as u8,
    }
}

fn hand_index(r: &DebugRunner, p: u8, id: &str) -> usize {
    hand_ids(r, p)
        .iter()
        .position(|c| c == id)
        .unwrap_or_else(|| panic!("{id} in hand: {:?}", hand_ids(r, p)))
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn set_suspended(r: &mut DebugRunner, h: PermanentHandle, v: bool) {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = v;
}

fn offered_hand_ids(r: &DebugRunner) -> Vec<String> {
    let view = r.pending_selection_view().expect("hand prompt pending");
    let hand = hand_ids(r, view.selecting_player);
    let mut out: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| {
            [PLAY_HAND_START, HAND_EFFECT_START]
                .iter()
                .filter_map(|&s| a.checked_sub(s))
                .find(|&i| (i as usize) < hand.len() && i < 30)
        })
        .map(|i| hand[i as usize].clone())
        .collect();
    out.sort();
    out.dedup();
    out
}

fn pick_hand(r: &mut DebugRunner, id: &str) {
    let view = r.pending_selection_view().expect("hand prompt pending");
    let p = view.selecting_player;
    let slot = hand_index(r, p, id) as u16;
    let a = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} not selectable: {view:?}"));
    r.execute_action(p, a).expect("pick hand card");
}

fn decline(r: &mut DebugRunner) {
    let view = r.pending_selection_view().expect("optional prompt pending");
    r.execute_action(view.selecting_player, PASS).expect("decline");
}

fn digivolve(r: &mut DebugRunner, id: &str, target: PermanentHandle) -> bool {
    r.game.current_phase = GamePhase::Main;
    let idx = hand_index(r, 0, id);
    let ok = r
        .game
        .digivolve_from_hand(0, idx, target.index as usize, PlaySource::ByDigivolve);
    r.game.drain_effect_queue();
    ok
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt12_030_metadata_matches_print() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Imperialdramon: Dragon Mode");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(12000));
    assert_eq!(card.cost, Some(12));
    assert!(card.color.contains(&CompiledColor::Blue));
    assert!(card.color.contains(&CompiledColor::Green));
    assert!(card.traits.iter().any(|t| t == "Ancient Dragon"));
}

#[test]
fn bt12_030_alt_paths_four_circles_plus_name_route() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let digis: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digis.len(), 5);
    let lv5_4 = digis
        .iter()
        .filter(|p| {
            p.cost == Some(CompiledCost::Literal(4))
                && p.from.as_ref().is_some_and(|f| f.level_eq == Some(5))
        })
        .count();
    let lv4_3 = digis
        .iter()
        .filter(|p| {
            p.cost == Some(CompiledCost::Literal(3))
                && p.from.as_ref().is_some_and(|f| f.level_eq == Some(4))
        })
        .count();
    assert_eq!(lv5_4, 2, "blue/green Lv.5 cost 4");
    assert_eq!(lv4_3, 2, "blue/green Lv.4 cost 3");
}

#[test]
fn bt12_030_has_wd_and_end_of_attack_clauses() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let trig: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(card.effects.len(), 2);
    assert_eq!(trig.len(), 2);
    assert_eq!(trig[0].when, vec![CompiledTiming::WhenDigivolving]);
    assert_eq!(trig[1].when, vec![CompiledTiming::EndOfAttack]);
    assert!(!trig[1].once_per_turn);
    for t in &trig {
        assert_eq!(t.scope, CompiledScope::FaceUp);
    }
}

// ─── Section 2 — Digivolution requirements ───────────────────────────────────

#[test]
fn bt12_030_digivolves_from_paildramon_for_3() {
    let mut r = start(&[CARD_ID]);
    let base = r.place_on_field(0, "PAIL", Some(0));
    let m0 = r.memory();
    assert!(digivolve(&mut r, CARD_ID, base));
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
    assert_eq!(m0 - r.memory(), 3);
}

#[test]
fn bt12_030_digivolves_from_dinobeemon_for_3() {
    let mut r = start(&[CARD_ID]);
    let base = r.place_on_field(0, "DINO", Some(0));
    let m0 = r.memory();
    assert!(digivolve(&mut r, CARD_ID, base));
    assert_eq!(m0 - r.memory(), 3);
}

#[test]
fn bt12_030_digivolves_from_blue_lv5_for_4() {
    let mut r = start(&[CARD_ID]);
    let base = r.place_on_field(0, "BLUE5", Some(0));
    let m0 = r.memory();
    assert!(digivolve(&mut r, CARD_ID, base));
    assert_eq!(m0 - r.memory(), 4);
}

#[test]
fn bt12_030_cannot_digivolve_from_unrelated_red_lv5() {
    let mut r = start(&[CARD_ID]);
    let base = r.place_on_field(0, "STING", Some(0));
    let m0 = r.memory();
    assert!(!digivolve(&mut r, CARD_ID, base));
    assert_eq!(r.memory(), m0);
}

// ─── Section 3 — [When Digivolving] ──────────────────────────────────────────

#[test]
fn bt12_030_blue_source_unsuspends_this_digimon() {
    let mut r = start(&[CARD_ID]);
    let base = r.place_stack(0, &["BLUE-SRC", "PAIL"]);
    set_suspended(&mut r, base, true);
    let _opp = r.place_on_field(1, "OPP-A", Some(0));
    assert!(digivolve(&mut r, CARD_ID, base));
    assert!(
        r.pending_selection().is_none(),
        "no green digivolution card → no suspend prompt"
    );
    let h = handle_of(&r, 0, CARD_ID);
    assert!(!suspended(&r, h), "blue digivolution card → unsuspend");
}

#[test]
fn bt12_030_green_source_suspends_chosen_opponent_digimon() {
    let mut r = start(&[CARD_ID]);
    let base = r.place_stack(0, &["GREEN-SRC", "PAIL"]);
    set_suspended(&mut r, base, true);
    let a = r.place_on_field(1, "OPP-A", Some(0));
    let b = r.place_on_field(1, "OPP-B", Some(0));
    assert!(digivolve(&mut r, CARD_ID, base));
    let view = r.pending_selection_view().expect("suspend target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert_eq!(view.selecting_player, 0);
    assert!(!view.is_optional, "suspend is mandatory");
    assert_eq!(view.valid_action_ids.len(), 2, "player chooses among both");
    // Pick the second candidate to show it is a real choice.
    r.execute_action(0, view.valid_action_ids[1]).expect("pick");
    let _ = r.auto_resolve();
    let suspended_count = [a, b].iter().filter(|h| suspended(&r, **h)).count();
    assert_eq!(suspended_count, 1, "exactly 1 opponent Digimon suspended");
    let h = handle_of(&r, 0, CARD_ID);
    assert!(suspended(&r, h), "no blue digivolution card → stays suspended");
}

#[test]
fn bt12_030_blue_and_green_sources_do_both() {
    let mut r = start(&[CARD_ID]);
    let base = r.place_stack(0, &["BLUE-SRC", "GREEN-SRC", "PAIL"]);
    set_suspended(&mut r, base, true);
    let a = r.place_on_field(1, "OPP-A", Some(0));
    assert!(digivolve(&mut r, CARD_ID, base));
    let view = r.pending_selection_view().expect("suspend prompt");
    r.execute_action(0, view.valid_action_ids[0]).expect("pick");
    let _ = r.auto_resolve();
    assert!(suspended(&r, a));
    let h = handle_of(&r, 0, CARD_ID);
    assert!(!suspended(&r, h));
}

#[test]
fn bt12_030_no_blue_or_green_source_does_nothing() {
    let mut r = start(&[CARD_ID]);
    let base = r.place_stack(0, &["RED-SRC", "PAIL"]);
    set_suspended(&mut r, base, true);
    let a = r.place_on_field(1, "OPP-A", Some(0));
    assert!(digivolve(&mut r, CARD_ID, base));
    assert!(r.pending_selection().is_none());
    assert!(!suspended(&r, a));
    let h = handle_of(&r, 0, CARD_ID);
    assert!(suspended(&r, h));
}

#[test]
fn bt12_030_green_source_without_opponent_digimon_skips() {
    let mut r = start(&[CARD_ID]);
    let base = r.place_stack(0, &["GREEN-SRC", "PAIL"]);
    assert!(digivolve(&mut r, CARD_ID, base));
    assert!(r.pending_selection().is_none());
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
}

// ─── Section 4 — [End of Attack] ─────────────────────────────────────────────

#[test]
fn bt12_030_end_of_attack_offers_only_routable_imperialdramon() {
    let mut r = start(&["PALADIN", "IMP-RED", "OMNI"]);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.attack_player(h, 1, false);
    assert!(r.pending_is_optional(), "'may digivolve'");
    assert_eq!(offered_hand_ids(&r), vec!["PALADIN".to_string()]);
}

#[test]
fn bt12_030_end_of_attack_digivolves_with_cost_reduced_by_2() {
    let mut r = start(&["PALADIN"]);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let m0 = r.memory();
    r.attack_player(h, 1, false);
    pick_hand(&mut r, "PALADIN");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["PALADIN".to_string()]);
    assert_eq!(m0 - r.memory(), 4, "evo cost 6 reduced by 2");
}

#[test]
fn bt12_030_end_of_attack_decline_keeps_dragon_mode() {
    let mut r = start(&["PALADIN"]);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let m0 = r.memory();
    r.attack_player(h, 1, false);
    decline(&mut r);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
    assert_eq!(r.memory(), m0);
}

#[test]
fn bt12_030_end_of_attack_without_candidate_no_prompt() {
    let mut r = start(&["OMNI"]);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.attack_player(h, 1, false);
    assert!(r.pending_selection().is_none());
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
}
