//! EX7-008 ToyAgumon — Digimon, Lv.3, Red, DP 1000, Cost 3.
//! Traits: Puppet. Form: Rookie. Attribute: Vaccine.
//!
//! # Card text (data/card_bundles/EX7-008.md — official Bandai DB, verbatim;
//! cross-checked against the card image EX7-008.webp)
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 card with [Three
//! Musketeers] in its text and 1 Option card with a use cost of 6 among them
//! to the hand. Return the rest to the bottom of the deck.
//!
//! Inherited Effect: [Your Turn] This Digimon gets +2000 DP.
//!
//! Official Q&A: "Yes, you must add as many cards to your hand as possible."
//!
//! Digivolution requirements (official Bandai DB):
//!   - Standard circle: Red Lv.2 / cost 0
//!   - xros_req: "[Digivolve] Lv.2 w/[Three Musketeers] in text: Cost 0"
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Red/EX7_008.cs
//!
//! # DCGO crosscheck
//! - Alt-digivolve: `TopCard.HasText("Three Musketeers") && Level == 2`,
//!   cost 0, ignoreDigivolutionRequirement: false → `in_text_contains`.
//! - [On Play]: ActivateClass(isOptional: false); CanActivate = deck >= 1.
//!   SimplifiedRevealDeckTopCardsAndSelect(revealCount 3, TWO conditions:
//!   `HasText("Three Musketeers")` (any card kind, despite the UI message
//!   saying "Digimon") and `IsOption && GetCostItself == 6`, each maxCount 1,
//!   Mode.AddHand; remainingCardsPlace: DeckBottom).
//! - ESS: ChangeSelfDPStaticEffect(+2000, isInherited) gated IsOwnerTurn.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - A2: two-bucket reveal (reveal 3, add ≤1 per bucket, bottom the rest)
//! - D1/D4: inherited [Your Turn] self DP aura
//! - G-DSL-IN-TEXT-CONTAINS alt-path

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX7-008";
/// EX7-070 Der Blitz — a REAL [Three Musketeers]-trait, cost-6 Option: it
/// satisfies BOTH buckets (trait scan ⊂ "in text"; use cost 6).
const DER_BLITZ: &str = "EX7-070";

fn digimon(id: &str, level: u8, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

/// A Digimon whose ONLY [Three Musketeers] reference is in its effect text
/// (no trait) — the "in its text" reading DCGO `HasText` implements.
fn tm_text_digimon(id: &str) -> CardData {
    let mut c = digimon(id, 4, CardColor::Red);
    c.effect_text = "[When Digivolving] Play 1 [Three Musketeers] trait card.".to_string();
    c
}

fn option(id: &str, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Red];
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Green];
    c
}

fn zone_ids(cards: &[digimon_engine::card_source::CardSource], data: &[CardData]) -> Vec<String> {
    cards.iter().map(|c| c.card_id(data).to_string()).collect()
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX7-008 YAML loads from the embedded pack")
        .dsl_card(DER_BLITZ)
        .expect("EX7-070 YAML loads from the embedded pack")
        .add_card(tm_text_digimon("TM-TEXT"))
        .add_card(option("OPT6", 6, &[]))
        .add_card(option("OPT5", 5, &[]))
        .add_card(filler("FILL"))
        .add_card(digimon("CARRIER", 4, CardColor::Red))
}

fn reveal_action(runner: &DebugRunner, card_id: &str) -> u16 {
    let idx = runner
        .game
        .revealed_cards
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not revealed"));
    SEL_REVEAL_START + idx as u16
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_008_has_mandatory_on_play_clause_and_inherited_your_turn_aura() {
    let runner = base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    assert_eq!(triggered[0].when, vec![CompiledTiming::OnPlay]);
    assert!(!triggered[0].optional, "the reveal is mandatory");

    let aura = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, active_when, dp_modifier, .. })
                if *scope == CompiledScope::Inherited && active_when.is_some() && *dp_modifier == Some(2000)
        )
    });
    assert!(aura, "inherited [Your Turn] +2000 DP aura must be present");

    let digivolve_paths = compiled
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .count();
    assert_eq!(digivolve_paths, 2, "standard Red Lv.2/0 + special TM-text Lv.2/0");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2/3 — [On Play] reveal behaviour
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_008_on_play_adds_one_tm_text_card_and_one_cost_six_option_bottoms_rest() {
    // Deck (last = top): FILL, FILL | PLAIN-FILL, OPT6, TM-TEXT ← top 3.
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "OPT6", "TM-TEXT"])
        .memory(10)
        .start();
    let deck_before = runner.deck_size(0);
    runner.play(0, 0).expect("play ToyAgumon");

    let view = runner.pending_selection_view().expect("bucket 0 prompt");
    assert!(matches!(view.kind, SelectionKind::RevealBucket { bucket_index: 0, .. }));
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "bucket 0 has a candidate → mandatory (Q&A: add as many as possible)"
    );
    runner
        .execute_action(0, reveal_action(&runner, "TM-TEXT"))
        .expect("pick TM-TEXT");
    let view = runner.pending_selection_view().expect("bucket 1 prompt");
    assert!(matches!(view.kind, SelectionKind::RevealBucket { bucket_index: 1, .. }));
    runner
        .execute_action(0, reveal_action(&runner, "OPT6"))
        .expect("pick OPT6");
    runner.auto_resolve().expect("finish");

    let hand = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand.contains(&"TM-TEXT".to_string()), "TM-text card added; hand={hand:?}");
    assert!(hand.contains(&"OPT6".to_string()), "cost-6 Option added; hand={hand:?}");
    assert_eq!(runner.hand_size(0), 2);
    assert_eq!(runner.deck_size(0), deck_before - 2, "1 of the 3 revealed returned to the deck");
    let bottom = runner.game.players[0].deck[0].card_id(&runner.game.card_data);
    assert_eq!(bottom, "FILL", "the rest go to the BOTTOM of the deck");
    assert!(runner.game.revealed_cards.is_empty());
}

#[test]
fn ex7_008_on_play_same_card_cannot_fill_both_buckets() {
    // Der Blitz matches both buckets; with no other candidate, bucket 1 is empty.
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", DER_BLITZ])
        .memory(10)
        .start();
    runner.play(0, 0).expect("play ToyAgumon");
    runner
        .execute_action(0, reveal_action(&runner, DER_BLITZ))
        .expect("pick Der Blitz for bucket 0");
    runner.auto_resolve().expect("finish");
    let hand = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert_eq!(hand, vec![DER_BLITZ.to_string()], "Der Blitz added exactly once");
}

#[test]
fn ex7_008_on_play_no_match_bottoms_all_three() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["OPT5", "FILL", "FILL", "FILL", "OPT5"])
        .memory(10)
        .start();
    let deck_before = runner.deck_size(0);
    runner.play(0, 0).expect("play ToyAgumon");
    runner.auto_resolve().expect("finish");
    assert_eq!(runner.hand_size(0), 0, "nothing qualifies (OPT5 costs 5, no TM text)");
    assert_eq!(runner.deck_size(0), deck_before, "all 3 revealed cards returned");
    assert!(runner.game.revealed_cards.is_empty());
}

#[test]
fn ex7_008_bucket_two_rejects_non_option_and_wrong_cost() {
    // Top 3: OPT5 (Option, cost 5), TM-TEXT (Digimon), FILL → bucket 0 takes
    // TM-TEXT, bucket 1 has NO legal pick (OPT5 costs 5).
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "TM-TEXT", "OPT5"])
        .memory(10)
        .start();
    runner.play(0, 0).expect("play ToyAgumon");
    runner
        .execute_action(0, reveal_action(&runner, "TM-TEXT"))
        .expect("pick TM-TEXT");
    runner.auto_resolve().expect("finish");
    let hand = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert_eq!(hand, vec!["TM-TEXT".to_string()], "OPT5 must not be added");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Inherited [Your Turn] +2000 DP
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_008_inherited_your_turn_dp_applies_on_own_turn_only() {
    let mut runner = base().start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(stack), Some(3000 + 2000), "+2000 on your turn");

    runner.end_turn();
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(stack),
        Some(3000),
        "no bonus on the opponent's turn"
    );
}

#[test]
fn ex7_008_face_up_toyagumon_gets_no_dp_from_its_inherited_effect() {
    let mut runner = base().start();
    let alone = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(alone), Some(1000), "printed DP only when face-up");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — Digivolution paths
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_008_digivolves_for_zero_from_lv2_with_tm_in_text_regardless_of_color() {
    let mut blk_tm_lv2 = digimon("BLK-TM-LV2", 2, CardColor::Black);
    blk_tm_lv2.inherited_text = "[Three Musketeers] ...".to_string();
    let mut runner = base()
        .add_card(blk_tm_lv2)
        .hand(0, &[CARD_ID])
        .memory(5)
        .start();
    let base_perm = runner.place_on_field(0, "BLK-TM-LV2", Some(0));
    let memory_before = runner.memory();
    assert!(runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand));
    assert_eq!(runner.memory(), memory_before, "cost 0");
}

#[test]
fn ex7_008_cannot_digivolve_from_plain_black_lv2() {
    let mut runner = base()
        .add_card(digimon("BLK-LV2", 2, CardColor::Black))
        .hand(0, &[CARD_ID])
        .memory(5)
        .start();
    let base_perm = runner.place_on_field(0, "BLK-LV2", Some(0));
    assert!(!runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand));
}
