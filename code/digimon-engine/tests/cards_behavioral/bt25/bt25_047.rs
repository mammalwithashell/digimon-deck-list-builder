//! BT25-047 Floramon — Digimon, Lv.3, Green, DP 2000, Cost 3.
//! Traits: Vegetation, Iliad, TS. Attribute: Data.
//!
//! Printed effect text (cards.json):
//!   [On Play] Reveal the top 3 cards of your deck. Add 1 [Vegetation] or
//!   [Shaman] trait card and 1 [TS] trait card among them to the hand. Return
//!   the rest to the bottom of the deck.
//!
//! Printed inherited text:
//!   [Your Turn] All of your Digimon get +1000 DP.
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_047.cs
//!
//! Pattern rows (RUST_DSL_TEST_API §4.3): A1 reveal-N two-bucket pick (Group E),
//! mandatory pick (no "may"), D4 inherited your-turn DP aura (Group D),
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
        .dsl_card("BT25-047")
        .expect("BT25-047 YAML parses and compiles")
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

fn on_play_process(runner: &DebugRunner) -> Vec<CompiledStep> {
    runner
        .compiled_card("BT25-047")
        .expect("compiled")
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => {
                Some(t.process.clone())
            }
            _ => None,
        })
        .expect("on_play process present")
}

// ── Section 1: Structural ──────────────────────────────────────────────────

#[test]
fn bt25_047_has_on_play_reveal_two_pick_clause() {
    let runner = runner();
    let card = runner.compiled_card("BT25-047").expect("compiled");

    let on_play = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("BT25-047 must have an [On Play] clause");

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
    assert_eq!(
        bucket_count, 2,
        "must have 2 buckets (Vegetation/Shaman, TS)"
    );
    assert_eq!(add_to_hand_count, 2, "must add 2 cards to hand");
    assert!(has_place_remainder, "must place remainder on deck bottom");
}

#[test]
fn bt25_047_no_duplicate_cards_set() {
    let runner = runner();
    let on_play = on_play_process(&runner);
    let no_dup = on_play.iter().any(|s| {
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

#[test]
fn bt25_047_has_inherited_your_turn_dp_aura() {
    let runner = runner();
    let card = runner.compiled_card("BT25-047").expect("compiled");
    let aura = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Inherited,
                dp_modifier: Some(1000),
                ..
            })
        )
    });
    assert!(aura, "BT25-047 must declare inherited +1000 DP aura");
}

#[test]
fn bt25_047_registers_ts_trait_alt_digivolve() {
    let runner = runner();
    let card = runner.compiled_card("BT25-047").expect("compiled");
    let digivolve_paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(
        digivolve_paths, 1,
        "BT25-047 must register the TS-trait alt digivolve path"
    );
}

// ── Section 2/3: Behavioral — reveal + bucket picks ────────────────────────

#[test]
fn bt25_047_on_play_adds_vegetation_and_ts_to_hand_rest_to_bottom() {
    let mut veg = make_test_card("VEG-A", "Veg A");
    veg.traits.push("Vegetation".to_string());
    let mut ts = make_test_card("TS-047", "TS 047");
    ts.traits.push("TS".to_string());
    let filler = make_test_card("FILL-047", "Filler 047");
    let holder = make_test_card("BT25_047_HOLDER", "Holder");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-047")
        .expect("BT25-047 compiles")
        .add_card(veg)
        .add_card(ts)
        .add_card(filler)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_047_HOLDER", None);
    let src = runner.top_card(holder_perm);

    let deck_before = runner.game.players[0].deck.len();
    stack_deck_top(&mut runner, &["FILL-047", "TS-047", "VEG-A"]);

    let process = on_play_process(&runner);
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
    assert!(
        hand_ids.contains(&"VEG-A"),
        "Vegetation pick to hand: {hand_ids:?}"
    );
    assert!(
        hand_ids.contains(&"TS-047"),
        "TS pick to hand: {hand_ids:?}"
    );
    assert!(
        !hand_ids.contains(&"FILL-047"),
        "non-matching filler must NOT be added to hand: {hand_ids:?}"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before + 1,
        "two picks left deck, one filler returned to bottom"
    );
}

/// Negative: with zero eligible candidates in the top-3, both buckets fizzle
/// silently (no pending selection) and all cards return to the deck bottom.
#[test]
fn bt25_047_on_play_fizzles_when_no_candidates() {
    let fa = make_test_card("F047-A", "f a");
    let fb = make_test_card("F047-B", "f b");
    let fc = make_test_card("F047-C", "f c");
    let holder = make_test_card("BT25_047_HOLDER2", "Holder2");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-047")
        .expect("BT25-047 compiles")
        .add_card(fa)
        .add_card(fb)
        .add_card(fc)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_047_HOLDER2", None);
    let src = runner.top_card(holder_perm);

    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.game.players[0].deck.len();
    stack_deck_top(&mut runner, &["F047-C", "F047-B", "F047-A"]);

    let process = on_play_process(&runner);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "fizzle adds nothing to hand"
    );
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

/// Positive (Shaman branch): a [Shaman] card satisfies the first bucket too.
#[test]
fn bt25_047_shaman_satisfies_first_bucket() {
    let mut shaman = make_test_card("SHAMAN-A", "Shaman A");
    shaman.traits.push("Shaman".to_string());
    let mut ts = make_test_card("TS-047B", "TS 047 B");
    ts.traits.push("TS".to_string());
    let filler = make_test_card("FILL-047B", "Filler 047 B");
    let holder = make_test_card("BT25_047_HOLDER3", "Holder3");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-047")
        .expect("BT25-047 compiles")
        .add_card(shaman)
        .add_card(ts)
        .add_card(filler)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_047_HOLDER3", None);
    let src = runner.top_card(holder_perm);

    stack_deck_top(&mut runner, &["FILL-047B", "TS-047B", "SHAMAN-A"]);

    let process = on_play_process(&runner);
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
    assert!(
        hand_ids.contains(&"SHAMAN-A"),
        "Shaman pick to hand: {hand_ids:?}"
    );
    assert!(
        hand_ids.contains(&"TS-047B"),
        "TS pick to hand: {hand_ids:?}"
    );
}
