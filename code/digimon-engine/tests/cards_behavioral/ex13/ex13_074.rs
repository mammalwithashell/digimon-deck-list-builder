//! EX13-074 Rie Kishibe — Tamer, Purple/Black, Cost 4. Trait: CS.
//!
//! # Card text (official Bandai DB, `data/card_bundles/EX13-074.md`)
//!
//! ```text
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [All Turns] [Once Per Turn] When any of your [Knightmon] text Digimon are
//! played or deleted, by placing 1 such card from your hand or trash under
//! this Tamer, <Draw 1>.
//! [Main] [Once Per Turn] If this Tamer has 3 or more [Knightmon] text cards
//! under it, it may digivolve into [LordKnightmon] in the hand or trash for a
//! digivolution cost of 3, ignoring digivolution requirements.
//!
//! Security Effect:
//! [Security] Play this card without paying the cost.
//! ```
//!
//! # DCGO C# reference
//! None — no `EX13_074.cs` at `b9a0638cd`. BT22_090.cs (older Rie Kishibe,
//! "this Tamer may digivolve into [LordKnightmon] in the hand") digivolves
//! onto `card.PermanentOfThisCard()` with the generic hand/trash digivolve —
//! the Tamer and its under-cards become digivolution cards, the digivolution
//! draw happens and [When Digivolving] fires.
//!
//! # Verdict — IMPLEMENTED (2026-10-01)
//! Clause 3 landed with `G-TAMER-DIGIVOLVE-INTO-DIGIMON`: an effect digivolve
//! that ignores digivolution requirements now accepts a level-less Tamer base.
//! Clause 2 uses the existing `select_union_zone` → `place_as_bottom_source`
//! path (a hand-or-trash card placed under this Tamer).
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - B Tamer [Start of Your Turn] memory floor.
//! - F [All Turns][OPT] played-OR-deleted observer with a placement cost.
//! - C Tamer [Main][OPT] effect digivolve of the Tamer itself.
//! - B Tamer [Security] play self free.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::action::space::{EFFECTS_PER_PERMANENT, FIELD_EFFECT_START, PASS};
use digimon_engine::action::{build_action_mask, encode_digivolve};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-074";
const LORDKNIGHTMON: &str = "EX13-064";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, level: u8, cost: u16) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = cost;
    c.colors = vec![CardColor::Black];
    c
}

/// A non-Knightmon-named card whose EFFECT text names [Knightmon]
/// ("[Knightmon] text" per the official Q&A).
fn text_knight(id: &str) -> CardData {
    let mut c = digimon(id, "Squire", 3, 3);
    c.effect_text = "[On Play] Reveal ... [Knightmon] ...".to_string();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-074 YAML parses, compiles and is in the embedded pack")
        .add_card(make_test_card("PAD", "PAD"))
        .add_card(digimon("KNIGHT", "Knightmon", 4, 3))
        .add_card(digimon("DKNIGHT", "DarkKnightmon", 5, 3))
        .add_card(text_knight("SQUIRE"))
        .add_card(digimon("PLAIN", "Plain", 4, 3))
        .add_card(digimon("LKM", "LordKnightmon", 6, 12))
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 8])
        .security(0, &["PAD"; 4])
        .security(1, &["PAD"; 4])
}

fn rie_on_field(r: &mut DebugRunner) -> PermanentHandle {
    let rie = r.place_on_field(0, CARD_ID, Some(0));
    r.game.current_phase = GamePhase::Main;
    rie
}

fn stack(r: &DebugRunner, h: PermanentHandle) -> Vec<(String, bool)> {
    r.game.players[0].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| (c.card_id(&r.game.card_data).to_string(), c.face_down))
        .collect()
}

fn under(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    let s = stack(r, h);
    s[..s.len() - 1].iter().map(|(id, _)| id.clone()).collect()
}

fn hand_index(r: &DebugRunner, id: &str) -> usize {
    r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} in hand"))
}

fn first_non_pass(r: &DebugRunner) -> u16 {
    *r.pending_selection_view()
        .expect("pending")
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("non-PASS")
}

fn main_offered(r: &DebugRunner, h: PermanentHandle) -> bool {
    let mask = build_action_mask(&r.game, 0);
    let base = FIELD_EFFECT_START + h.index as u16 * EFFECTS_PER_PERMANENT;
    (base..base + EFFECTS_PER_PERMANENT).any(|a| mask[a as usize] == 1.0)
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_074_is_purple_black_cs_tamer() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.cost, Some(4));
    assert!(card.traits.iter().any(|t| t == "CS"));
}

#[test]
fn ex13_074_clause_shape() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    let observer = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnAllyPlayed))
        .expect("played-or-deleted observer");
    assert!(
        observer.when.contains(&CompiledTiming::OnAnyDeletion),
        "played OR deleted share one clause (one [Once Per Turn] slot)"
    );
    assert!(observer.once_per_turn);
    let main = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::MainOnField))
        .expect("[Main] clause");
    assert!(main.once_per_turn);
    assert!(main.process.iter().any(|s| matches!(
        s,
        CompiledStep::EffectInitiatedDigivolve {
            ignore_requirements: true,
            ..
        }
    )));
    assert!(triggered
        .iter()
        .any(|t| t.when.contains(&CompiledTiming::StartOfYourTurn)));
    assert!(triggered
        .iter()
        .any(|t| t.when.contains(&CompiledTiming::OnSecurity)));
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [All Turns][OPT] played/deleted → place under this Tamer → Draw 1
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_074_knightmon_played_places_a_card_under_rie_and_draws() {
    let mut r = builder().hand(0, &["KNIGHT", "DKNIGHT"]).memory(10).start();
    let rie = rie_on_field(&mut r);
    r.play(0, hand_index(&r, "KNIGHT")).expect("play Knightmon");
    assert!(r.pending_is_optional(), "the placement cost is optional");
    r.accept_optional_trigger().expect("accept");
    // Only DarkKnightmon (hand) is a [Knightmon] text card left in hand/trash.
    let a = first_non_pass(&r);
    let hand_before = r.hand_size(0);
    r.execute_action(0, a).expect("place DarkKnightmon");
    r.auto_resolve().ok();
    assert_eq!(under(&r, rie), vec!["DKNIGHT".to_string()]);
    assert!(
        !stack(&r, rie)[0].1,
        "placed face up (no 'face down' in the text)"
    );
    assert_eq!(r.hand_size(0), hand_before - 1 + 1, "placed 1 from hand, drew 1");
}

#[test]
fn ex13_074_text_knightmon_card_counts_as_such_card() {
    let mut r = builder().hand(0, &["SQUIRE", "KNIGHT"]).memory(10).start();
    let rie = rie_on_field(&mut r);
    // SQUIRE has [Knightmon] only in its effect text — still a [Knightmon]
    // text Digimon, so playing it triggers.
    r.play(0, hand_index(&r, "SQUIRE")).expect("play");
    r.accept_optional_trigger().expect("accept");
    let a = first_non_pass(&r);
    r.execute_action(0, a).expect("place");
    r.auto_resolve().ok();
    assert_eq!(under(&r, rie), vec!["KNIGHT".to_string()]);
}

#[test]
fn ex13_074_decline_places_nothing_and_keeps_the_once_per_turn() {
    let mut r = builder()
        .hand(0, &["KNIGHT", "DKNIGHT", "KNIGHT"])
        .memory(10)
        .start();
    let rie = rie_on_field(&mut r);
    r.play(0, hand_index(&r, "KNIGHT")).expect("play");
    let hand_before = r.hand_size(0);
    r.decline_optional_trigger().expect("decline");
    r.auto_resolve().ok();
    assert!(under(&r, rie).is_empty());
    assert_eq!(r.hand_size(0), hand_before, "no draw");
    // A declined optional activation does not spend the [Once Per Turn].
    r.play(0, hand_index(&r, "DKNIGHT")).expect("play");
    assert!(r.pending_is_optional(), "offered again after a decline");
}

#[test]
fn ex13_074_once_per_turn() {
    let mut r = builder()
        .hand(0, &["KNIGHT", "DKNIGHT", "KNIGHT", "DKNIGHT"])
        .memory(10)
        .start();
    let rie = rie_on_field(&mut r);
    r.play(0, hand_index(&r, "KNIGHT")).expect("play");
    r.accept_optional_trigger().expect("accept");
    let a = first_non_pass(&r);
    r.execute_action(0, a).expect("place");
    r.auto_resolve().ok();
    assert_eq!(under(&r, rie).len(), 1);
    r.play(0, hand_index(&r, "DKNIGHT")).expect("play again");
    assert!(r.pending_selection().is_none(), "[Once Per Turn] already used");
}

#[test]
fn ex13_074_non_knightmon_digimon_does_not_trigger() {
    let mut r = builder().hand(0, &["PLAIN", "KNIGHT"]).memory(10).start();
    rie_on_field(&mut r);
    r.play(0, hand_index(&r, "PLAIN")).expect("play");
    assert!(r.pending_selection().is_none());
}

#[test]
fn ex13_074_not_offered_without_a_knightmon_card_in_hand_or_trash() {
    let mut r = builder().hand(0, &["KNIGHT"]).memory(10).start();
    rie_on_field(&mut r);
    r.play(0, 0).expect("play the only Knightmon");
    assert!(
        r.pending_selection().is_none(),
        "no [Knightmon] text card left to place → the cost can't be paid"
    );
}

#[test]
fn ex13_074_knightmon_deleted_can_place_itself_from_trash() {
    let mut r = builder().memory(5).start();
    let rie = rie_on_field(&mut r);
    let k = r.place_on_field(0, "KNIGHT", Some(0));
    r.game.delete_permanent_with_cause(k, ReplacementCause::OpponentEffect);
    assert!(r.pending_is_optional(), "deletion triggers the observer");
    r.accept_optional_trigger().expect("accept");
    let a = first_non_pass(&r);
    r.execute_action(0, a).expect("place the deleted Knightmon from trash");
    r.auto_resolve().ok();
    assert_eq!(under(&r, rie), vec!["KNIGHT".to_string()]);
    assert_eq!(r.trash_size(0), 0);
    assert_eq!(r.hand_size(0), 1, "drew 1");
}

#[test]
fn ex13_074_opponents_knightmon_does_not_trigger() {
    let mut r = builder().hand(0, &["KNIGHT"]).memory(5).start();
    rie_on_field(&mut r);
    let k = r.place_on_field(1, "KNIGHT", Some(0));
    r.game.delete_permanent_with_cause(k, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_none());
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — [Main][OPT] this Tamer digivolves into [LordKnightmon]
// ════════════════════════════════════════════════════════════════════════════

fn rie_with_knights(r: &mut DebugRunner, n: usize) -> PermanentHandle {
    let rie = rie_on_field(r);
    for i in 0..n {
        r.push_source(rie, if i % 2 == 0 { "KNIGHT" } else { "SQUIRE" });
    }
    rie
}

#[test]
fn ex13_074_main_digivolves_rie_into_lordknightmon_for_3() {
    let mut r = builder().hand(0, &["LKM"]).memory(5).start();
    let rie = rie_with_knights(&mut r, 3);
    assert!(main_offered(&r, rie));
    let hand0 = r.hand_size(0);
    assert!(r.game.activate_field_main(0, rie.index as usize));
    let v = r.pending_selection_view().expect("LordKnightmon pick");
    assert!(v.is_optional, "\"it may digivolve\"");
    let a = first_non_pass(&r);
    r.execute_action(0, a).expect("pick LordKnightmon");
    r.auto_resolve().ok();
    let ids: Vec<String> = stack(&r, rie).into_iter().map(|(id, _)| id).collect();
    assert_eq!(
        ids,
        vec!["KNIGHT", "SQUIRE", "KNIGHT", CARD_ID, "LKM"],
        "Rie and her under-cards become LordKnightmon's digivolution cards"
    );
    assert_eq!(r.memory(), 2, "fixed digivolution cost 3");
    assert_eq!(r.hand_size(0), hand0 - 1 + 1, "digivolution draw");
}

#[test]
fn ex13_074_main_digivolves_from_trash() {
    let mut r = builder().memory(5).start();
    r.inject_trash(0, "LKM");
    let rie = rie_with_knights(&mut r, 3);
    assert!(r.game.activate_field_main(0, rie.index as usize));
    let a = first_non_pass(&r);
    r.execute_action(0, a).expect("pick LordKnightmon from trash");
    r.auto_resolve().ok();
    assert_eq!(stack(&r, rie).last().map(|(id, _)| id.as_str()), Some("LKM"));
    assert_eq!(r.memory(), 2);
}

#[test]
fn ex13_074_main_needs_three_knightmon_text_cards_under_rie() {
    let mut r = builder().hand(0, &["LKM"]).memory(5).start();
    let rie = rie_with_knights(&mut r, 2);
    r.push_source(rie, "PLAIN");
    assert!(
        !main_offered(&r, rie),
        "2 [Knightmon] text cards + 1 other under Rie → condition fails"
    );
}

#[test]
fn ex13_074_main_may_decline() {
    let mut r = builder().hand(0, &["LKM"]).memory(5).start();
    let rie = rie_with_knights(&mut r, 3);
    assert!(r.game.activate_field_main(0, rie.index as usize));
    r.execute_action(0, PASS).expect("decline");
    r.auto_resolve().ok();
    assert_eq!(stack(&r, rie).last().map(|(id, _)| id.as_str()), Some(CARD_ID));
    assert_eq!(r.memory(), 5);
}

#[test]
fn ex13_074_main_only_offers_lordknightmon() {
    let mut r = builder().hand(0, &["DKNIGHT", "LKM"]).memory(5).start();
    let rie = rie_with_knights(&mut r, 3);
    assert!(r.game.activate_field_main(0, rie.index as usize));
    let non_pass = r
        .pending_selection_view()
        .expect("pick")
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .count();
    assert_eq!(non_pass, 1, "only the [LordKnightmon] card is a candidate");
}

#[test]
fn ex13_074_main_is_once_per_turn() {
    let mut r = builder().memory(5).start();
    let rie = rie_with_knights(&mut r, 3);
    assert!(r.game.activate_field_main(0, rie.index as usize));
    // No LordKnightmon anywhere → the pick self-skips; the activation is spent.
    r.auto_resolve().ok();
    assert!(!main_offered(&r, rie), "[Once Per Turn]");
}

#[test]
fn ex13_074_main_real_lordknightmon_fires_when_digivolving() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-074")
        .dsl_card(LORDKNIGHTMON)
        .expect("EX13-064")
        .add_card(make_test_card("PAD", "PAD"))
        .add_card(digimon("KNIGHT", "Knightmon", 4, 3))
        .hand(0, &[LORDKNIGHTMON, "KNIGHT"])
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 8])
        .memory(5)
        .start();
    let rie = r.place_on_field(0, CARD_ID, Some(0));
    for _ in 0..3 {
        r.push_source(rie, "KNIGHT");
    }
    r.game.current_phase = GamePhase::Main;
    assert!(r.game.activate_field_main(0, rie.index as usize));
    let a = first_non_pass(&r);
    r.execute_action(0, a).expect("pick EX13-064");
    // LordKnightmon's [When Digivolving] offers a cost-8-or-lower
    // [Knightmon] text card from hand or trash.
    let v = r.pending_selection_view().expect("[When Digivolving] fired");
    assert!(v.prompt.contains("Knightmon"), "{}", v.prompt);
    r.auto_resolve().ok();
    assert_eq!(
        r.game.players[0].battle_area[rie.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        LORDKNIGHTMON
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 4 — EX13-064 LordKnightmon's printed "[Rie Kishibe]: Cost 5" route
// with the REAL Rie Kishibe card (the alt-route is authored on EX13-064).
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_074_lordknightmon_digivolves_onto_real_rie_for_5_with_3_security() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-074")
        .dsl_card(LORDKNIGHTMON)
        .expect("EX13-064")
        .add_card(make_test_card("PAD", "PAD"))
        .hand(0, &[LORDKNIGHTMON])
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 8])
        .security(0, &["PAD"; 3])
        .memory(6)
        .start();
    let rie = r.place_on_field(0, CARD_ID, Some(0));
    r.game.current_phase = GamePhase::Main;
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[encode_digivolve(0, rie.index as u16) as usize], 1.0);
    assert!(r.game.digivolve_from_hand(0, 0, rie.index as usize, PlaySource::ByHand));
    r.auto_resolve().ok();
    assert_eq!(r.memory(), 1, "[Rie Kishibe]: Cost 5");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 5 — [Start of Your Turn] memory floor, [Security] play self
// ════════════════════════════════════════════════════════════════════════════

fn fire_start_of_turn(r: &mut DebugRunner, rie: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::StartOfYourTurn, TriggerSource::Permanent(rie));
    r.game.drain_effect_queue();
}

#[test]
fn ex13_074_start_of_your_turn_sets_memory_to_3() {
    let mut r = builder().memory(1).start();
    let rie = rie_on_field(&mut r);
    fire_start_of_turn(&mut r, rie);
    assert_eq!(r.memory(), 3, "1 memory → set to 3");
}

#[test]
fn ex13_074_start_of_your_turn_leaves_3_or_more_alone() {
    let mut r = builder().memory(4).start();
    let rie = rie_on_field(&mut r);
    fire_start_of_turn(&mut r, rie);
    assert_eq!(r.memory(), 4);
}

#[test]
fn ex13_074_security_plays_itself_free() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-074")
        .add_card(make_test_card("PAD", "PAD"))
        .add_card(digimon("ATTACKER", "Attacker", 5, 5))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(0, &[CARD_ID])
        .memory(5)
        .start();
    r.end_turn();
    assert_eq!(r.turn_player(), 1);
    let att = r.place_on_field(1, "ATTACKER", Some(0));
    r.attack_player(att, 0, false);
    r.auto_resolve().ok();
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID));
}
