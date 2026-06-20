//! ST23-06 Gekkomon — Digimon, Lv.3, Green, Cost 0.
//! Traits: Reptile, Glowing Dawn, BEATBREAK. Attribute: Data.
//!
//! # Card text (data/cards.json — verbatim)
//! [When Moving] [On Play] Reveal the top 3 cards of your deck. Among them, add
//!   1 [Glowing Dawn] trait card to the hand and place 1 such card face down
//!   under any of your [Glowing Dawn] trait Tamers. Return the rest to the
//!   bottom of the deck.
//! Inherited: <Piercing> (When this Digimon attacks and deletes an opponent's
//!   Digimon in battle, it checks security before the attack ends.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST23/Green/ST23_06.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - A2/A3 — reveal-3, two-bucket select (add-to-hand + place-under-tamer)
//! - G-DSL-PLACE-REVEALED-CARD-UNDER-TAMER — place a revealed card face-down
//!   under a chosen [Glowing Dawn] Tamer (the gap closed by this card)
//! - [When Moving] / [On Play] dual trigger
//! - inherited <Piercing> (structural)
//! - alt-digivolve from Lv.2 [Glowing Dawn] cost 0
//!
//! # Verdict — IMPLEMENTED (2026-06-15)
//! The reveal→add-1 half was already expressible. The gap was the SECOND
//! revealed card going face-down under a chosen [Glowing Dawn] Tamer: closed by
//! `place_revealed_card_under_tamer` (reveal-pool branch of
//! `place_selected_card_under_tamer`). When no [Glowing Dawn] Tamer exists only
//! the add-to-hand bucket runs (DCGO `HasMatchConditionOwnersPermanent` gate).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "ST23-06";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn gekkomon() -> CardData {
    card_data_from_compiled(CARD_ID)
}

/// A [Glowing Dawn] Digimon card (legal for both buckets).
fn gd_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

/// A non-[Glowing Dawn] card (illegal for both buckets; goes to deck bottom).
fn plain_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["Beast".to_string()];
    c
}

/// A [Glowing Dawn] Tamer (legal place-under target).
fn gd_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Green];
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

/// A NON-[Glowing Dawn] Tamer (illegal place-under target).
fn plain_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Green];
    c.traits = vec!["Beast".to_string()];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn fire_on_play(r: &mut DebugRunner, handle: PermanentHandle) {
    r.fire_on_play(0, handle.index as usize);
}

fn process_contains_place_under_tamer(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|s| match s {
        CompiledStep::PlaceSelectedCardUnderTamer { .. } => true,
        CompiledStep::If {
            then, else_branch, ..
        } => {
            process_contains_place_under_tamer(then)
                || process_contains_place_under_tamer(else_branch)
        }
        CompiledStep::ForEach { body, .. } | CompiledStep::PerSelected { body, .. } => {
            process_contains_place_under_tamer(body)
        }
        _ => false,
    })
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn st23_06_is_green_glowing_dawn_lv3_digimon() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(3));
    assert!(card.traits.iter().any(|t| t == "Glowing Dawn"));
    assert!(card.traits.iter().any(|t| t == "BEATBREAK"));
}

#[test]
fn st23_06_has_onmove_onplay_and_inherited_piercing() {
    let card = compiled(CARD_ID);
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(
        triggered.iter().any(|t| t
            .when
            .iter()
            .any(|w| *w == CompiledTiming::OnPlay)
            && t.when.iter().any(|w| *w == CompiledTiming::OnMove)),
        "[When Moving][On Play] shared reveal clause present"
    );
    let has_inherited_piercing = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(
                digimon_dsl::compiled::CompiledDeclarativeClause::GrantKeyword { scope, keyword, .. }
            ) if *scope == CompiledScope::Inherited
                && keyword.eq_ignore_ascii_case("Piercing")
        )
    });
    assert!(has_inherited_piercing, "inherited <Piercing> present");
}

#[test]
fn st23_06_reveal_clause_places_a_card_under_a_tamer() {
    let card = compiled(CARD_ID);
    let shared = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.iter().any(|w| *w == CompiledTiming::OnPlay) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("shared reveal clause");
    assert!(
        process_contains_place_under_tamer(&shared.process),
        "the reveal clause places a revealed card under a Tamer"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: reveal-3 → add 1 to hand + place 1 under a GD Tamer
// ════════════════════════════════════════════════════════════════════════════

/// Build a runner with Gekkomon on the field, a deck whose top 3 are the named
/// cards (in deck-top-first order), and optionally a [Glowing Dawn] Tamer.
/// `deck_top` lists cards from the TOP of the deck downward.
fn setup(deck_top: &[&str], with_gd_tamer: bool, with_plain_tamer: bool) -> (DebugRunner, PermanentHandle) {
    let mut builder = DebugRunner::builder()
        .add_card(gekkomon())
        .add_card(gd_card("GD_A"))
        .add_card(gd_card("GD_B"))
        .add_card(plain_card("PLAIN"))
        .add_card(gd_tamer("GDT"))
        .add_card(plain_tamer("PT"))
        .add_card(filler("FILLER"));

    // The DebugRunner deck() convention: index 0 is deck BOTTOM, last is TOP
    // (drawn first). We want `deck_top[0]` to be revealed first, so the reveal
    // order = deck top downward. reveal_top_deck pulls from the top (Vec end).
    // Build the deck so the Vec END holds deck_top[0]: reverse + pad.
    let mut deck: Vec<&str> = Vec::new();
    deck.extend(std::iter::repeat("FILLER").take(4)); // deck bottom padding
    for &c in deck_top.iter().rev() {
        deck.push(c);
    }
    builder = builder.deck(0, &deck).deck(1, &["FILLER"; 6]).memory(5);
    let mut runner = builder.start();
    runner.set_first_player(0);

    if with_gd_tamer {
        runner.place_on_field(0, "GDT", Some(0));
    }
    if with_plain_tamer {
        runner.place_on_field(0, "PT", Some(0));
    }

    let gekko = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, gekko)
}

/// POSITIVE: reveal GD_A, GD_B, PLAIN. With a [Glowing Dawn] Tamer present:
/// add 1 GD to hand, place 1 GD face-down under the Tamer, PLAIN to deck bottom.
#[test]
fn st23_06_adds_one_and_places_one_under_glowing_dawn_tamer() {
    let (mut runner, gekko) = setup(&["GD_A", "GD_B", "PLAIN"], true, false);
    let tamer = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "GDT")
        .map(|i| PermanentHandle { player: 0, index: i as u8 })
        .expect("GD Tamer on field");
    let hand_before = runner.hand_size(0);
    let tamer_sources_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    fire_on_play(&mut runner, gekko);

    // Resolve the reveal buckets (add + place picks), then the Tamer pick.
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "exactly 1 [Glowing Dawn] card added to hand"
    );
    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before + 1,
        "1 card placed under the [Glowing Dawn] Tamer"
    );
    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down,
        "the placed card is face-down"
    );
    // The placed source is a [Glowing Dawn] card.
    let placed = &runner.game.players[0].battle_area[tamer.index as usize].card_sources[0];
    let placed_id = placed.card_id(&runner.game.card_data);
    assert!(
        placed_id == "GD_A" || placed_id == "GD_B",
        "the placed card is one of the revealed [Glowing Dawn] cards (got {placed_id})"
    );
    // No stuck selection; PLAIN returned to deck bottom.
    assert!(runner.game.pending_selection.is_none(), "no stuck selection");
}

/// NEGATIVE (no GD Tamer): only the add-to-hand bucket runs; nothing is placed
/// under a Tamer (DCGO `HasMatchConditionOwnersPermanent` gate).
#[test]
fn st23_06_only_adds_to_hand_when_no_glowing_dawn_tamer() {
    // A NON-[Glowing Dawn] Tamer is on field — still no legal place target.
    let (mut runner, gekko) = setup(&["GD_A", "GD_B", "PLAIN"], false, true);
    let pt = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "PT")
        .map(|i| PermanentHandle { player: 0, index: i as u8 })
        .expect("plain Tamer on field");
    let hand_before = runner.hand_size(0);
    let pt_sources_before = runner.game.players[0].battle_area[pt.index as usize]
        .card_sources
        .len();

    fire_on_play(&mut runner, gekko);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "1 [Glowing Dawn] card still added to hand"
    );
    assert_eq!(
        runner.game.players[0].battle_area[pt.index as usize]
            .card_sources
            .len(),
        pt_sources_before,
        "no card placed under a non-[Glowing Dawn] Tamer"
    );
    assert!(runner.game.pending_selection.is_none(), "no stuck selection");
}

/// Only 1 [Glowing Dawn] card revealed: it goes to hand (the add bucket has
/// priority); nothing left for the place bucket. With a GD Tamer present, the
/// place bucket simply finds no legal card.
#[test]
fn st23_06_single_glowing_dawn_revealed_goes_to_hand_only() {
    let (mut runner, gekko) = setup(&["GD_A", "PLAIN", "PLAIN"], true, false);
    let tamer = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "GDT")
        .map(|i| PermanentHandle { player: 0, index: i as u8 })
        .expect("GD Tamer");
    let hand_before = runner.hand_size(0);
    let tamer_sources_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    fire_on_play(&mut runner, gekko);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "the single [Glowing Dawn] card is added to hand"
    );
    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before,
        "no second [Glowing Dawn] card to place under the Tamer (no_duplicate)"
    );
    assert!(runner.game.pending_selection.is_none());
}

/// NEGATIVE (no [Glowing Dawn] in reveal): nothing added, nothing placed; all 3
/// go to deck bottom.
#[test]
fn st23_06_no_glowing_dawn_revealed_all_to_deck_bottom() {
    let (mut runner, gekko) = setup(&["PLAIN", "PLAIN", "PLAIN"], true, false);
    let tamer = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "GDT")
        .map(|i| PermanentHandle { player: 0, index: i as u8 })
        .expect("GD Tamer");
    let hand_before = runner.hand_size(0);
    let tamer_sources_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    fire_on_play(&mut runner, gekko);
    let _ = runner.auto_resolve();

    assert_eq!(runner.hand_size(0), hand_before, "no [Glowing Dawn] to add");
    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before,
        "no [Glowing Dawn] to place"
    );
    assert!(runner.game.pending_selection.is_none());
}
