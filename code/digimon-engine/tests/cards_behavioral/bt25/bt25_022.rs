//! BT25-022 Lunamon — Digimon, Lv.3, Blue, DP 2000, Cost 3.
//! Traits: Mammal, Iliad, TS. Attribute: Data.
//!
//! Printed effect text (cards.json):
//!   [On Play] Reveal the top 3 cards of your deck. Add 1 [Iliad] trait card
//!   and 1 [TS] trait card among them to the hand. Return the rest to the
//!   bottom of the deck.
//!
//! Printed inherited text:
//!   <Jamming> (This Digimon can't be deleted in battles against Security
//!   Digimon.)
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/BT25/Blue/BT25_022.cs
//!
//! Pattern rows (RUST_DSL_TEST_API §4.3): reveal-N two-bucket pick (Group E),
//! mandatory pick (no "may"), inherited static keyword grant (Group G),
//! alt-digivolve recipe registration.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT25-022")
        .expect("BT25-022 YAML parses and compiles")
        .build()
}

/// Stack the given card IDs onto player 0's deck top. Last id in the slice
/// ends up on top of the deck (pushed last).
fn stack_deck_top(runner: &mut DebugRunner, ids: &[&str]) {
    for id in ids {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap_or_else(|| panic!("card {id} registered"));
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .deck
            .push(CardSource::new(data_idx, 0, card_index));
    }
}

// ── Section 1: Structural ──────────────────────────────────────────────────

#[test]
fn bt25_022_has_on_play_reveal_two_pick_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("BT25-022")
        .expect("BT25-022 compiled card present");

    let on_play = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("BT25-022 must have an [On Play] clause");

    let has_reveal = on_play
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::RevealTopDeck { .. }));
    let bucket_count = on_play
        .process
        .iter()
        .filter_map(|s| match s {
            CompiledStep::SelectRevealBuckets { buckets, .. } => Some(buckets.len()),
            _ => None,
        })
        .sum::<usize>();
    let add_to_hand_count = on_play
        .process
        .iter()
        .filter(|s| matches!(s, CompiledStep::AddToHandFromReveal { .. }))
        .count();
    let has_place_remainder = on_play
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlaceRemainderOnDeck { .. }));

    assert!(has_reveal, "must reveal top 3");
    assert_eq!(bucket_count, 2, "must have 2 buckets (Iliad, TS)");
    assert_eq!(add_to_hand_count, 2, "must add 2 cards to hand");
    assert!(has_place_remainder, "must place remainder on deck bottom");
}

#[test]
fn bt25_022_has_inherited_jamming() {
    let runner = runner();
    let card = runner
        .compiled_card("BT25-022")
        .expect("BT25-022 compiled card present");

    let jamming = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            scope,
            ..
        }) => *scope == CompiledScope::Inherited && keyword.eq_ignore_ascii_case("Jamming"),
        _ => false,
    });
    assert!(jamming, "BT25-022 must grant inherited <Jamming>");
}

#[test]
fn bt25_022_registers_ts_trait_alt_digivolve() {
    let runner = runner();
    let card = runner
        .compiled_card("BT25-022")
        .expect("BT25-022 compiled card present");
    // Two digivolve alt-paths: standard Blue Lv.2 + TS-trait Lv.2.
    let digivolve_paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(
        digivolve_paths, 2,
        "BT25-022 must register standard Blue and alt TS-trait digivolve paths"
    );
}

// ── Section 2/3: Behavioral — reveal + bucket picks ────────────────────────

#[test]
fn bt25_022_on_play_adds_iliad_and_ts_to_hand_rest_to_bottom() {
    let mut iliad = make_test_card("ILIAD-A", "Iliad A");
    iliad.traits.push("Iliad".to_string());
    let mut ts = make_test_card("TS-A", "TS A");
    ts.traits.push("TS".to_string());
    let filler = make_test_card("FILL-022", "Filler 022");
    let holder = make_test_card("BT25_022_HOLDER", "Holder");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-022")
        .expect("BT25-022 compiles")
        .add_card(iliad)
        .add_card(ts)
        .add_card(filler)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_022_HOLDER", None);
    let src = runner.top_card(holder_perm);

    let deck_before = runner.game.players[0].deck.len();
    // Top-3 (last pushed = top): FILL-022, TS-A, ILIAD-A.
    stack_deck_top(&mut runner, &["FILL-022", "TS-A", "ILIAD-A"]);

    let process = runner
        .compiled_card("BT25-022")
        .expect("compiled")
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => {
                Some(t.process.clone())
            }
            _ => None,
        })
        .expect("on_play process present");

    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }
    runner.auto_resolve().expect("bucket picks resolve");

    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(hand_ids.contains(&"ILIAD-A"), "Iliad pick to hand: {hand_ids:?}");
    assert!(hand_ids.contains(&"TS-A"), "TS pick to hand: {hand_ids:?}");
    assert!(
        !hand_ids.contains(&"FILL-022"),
        "non-matching filler must NOT be added to hand: {hand_ids:?}"
    );
    // The filler returned to the bottom; deck net loses the two picked cards.
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before + 1,
        "two picks left deck, one filler returned to bottom"
    );
}

/// Negative: a [TS] card can satisfy both buckets by filter, but
/// `no_duplicate_cards` forbids one card filling both. With only ONE TS card
/// and no Iliad card, only the TS bucket can be filled.
#[test]
fn bt25_022_no_duplicate_cards_blocks_one_card_filling_both_buckets() {
    let runner = runner();
    let card = runner.compiled_card("BT25-022").expect("compiled");
    let on_play = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("on_play clause");
    let no_dup = on_play.process.iter().any(|s| {
        matches!(
            s,
            CompiledStep::SelectRevealBuckets {
                no_duplicate_cards: true,
                ..
            }
        )
    });
    assert!(
        no_dup,
        "select_reveal_buckets must set no_duplicate_cards (DCGO mutualConditions: true)"
    );
}

/// Negative: with zero eligible candidates in the top-3, both buckets fizzle
/// silently (no pending selection) and all cards return to the deck bottom.
#[test]
fn bt25_022_on_play_fizzles_when_no_candidates() {
    let fa = make_test_card("F022-A", "f a");
    let fb = make_test_card("F022-B", "f b");
    let fc = make_test_card("F022-C", "f c");
    let holder = make_test_card("BT25_022_HOLDER2", "Holder2");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-022")
        .expect("BT25-022 compiles")
        .add_card(fa)
        .add_card(fb)
        .add_card(fc)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_022_HOLDER2", None);
    let src = runner.top_card(holder_perm);

    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.game.players[0].deck.len();
    stack_deck_top(&mut runner, &["F022-C", "F022-B", "F022-A"]);

    let process = runner
        .compiled_card("BT25-022")
        .expect("compiled")
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => {
                Some(t.process.clone())
            }
            _ => None,
        })
        .expect("on_play process present");
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }
    // Resolve any remaining permutation/placement steps if they surface.
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "fizzle adds nothing to hand"
    );
    // All 3 revealed cards return to the deck (placed on bottom).
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before + 3,
        "all 3 revealed cards return to deck on fizzle"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no pending selection remains after a fizzle path"
    );
    assert_eq!(
        runner.game.revealed_cards.len(),
        0,
        "reveal pool empty after fizzle"
    );
}
