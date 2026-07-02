//! BT25-031 Patamon — Digimon, Lv.3, Yellow, DP 2000, Cost 3.
//! Traits: Mammal, Iliad, ADAMAS, TS. Attribute: Data.
//!
//! Printed effect text (cards.json):
//!   [On Play] Reveal the top 3 cards of your deck. Add 1 [Angel], [Archangel],
//!   [Three Great Angels] or [Four Great Dragons] trait card and 1 [TS] trait
//!   card among them to the hand. Return the rest to the bottom of the deck.
//!
//! Printed inherited text:
//!   <Barrier> (When this Digimon would be deleted in battle, by trashing your
//!   top security card, it isn't deleted.)
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_031.cs
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
        .dsl_card("BT25-031")
        .expect("BT25-031 YAML parses and compiles")
        .build()
}

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
fn bt25_031_has_on_play_reveal_two_pick_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("BT25-031")
        .expect("BT25-031 compiled card present");

    let on_play = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("BT25-031 must have an [On Play] clause");

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
    assert_eq!(bucket_count, 2, "must have 2 buckets (Angel-group, TS)");
    assert_eq!(add_to_hand_count, 2, "must add 2 cards to hand");
    assert!(has_place_remainder, "must place remainder on deck bottom");
}

#[test]
fn bt25_031_has_inherited_barrier() {
    let runner = runner();
    let card = runner
        .compiled_card("BT25-031")
        .expect("BT25-031 compiled card present");

    let barrier = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            scope,
            ..
        }) => *scope == CompiledScope::Inherited && keyword.eq_ignore_ascii_case("Barrier"),
        _ => false,
    });
    assert!(barrier, "BT25-031 must grant inherited <Barrier>");
}

#[test]
fn bt25_031_registers_ts_trait_alt_digivolve() {
    let runner = runner();
    let card = runner
        .compiled_card("BT25-031")
        .expect("BT25-031 compiled card present");
    let digivolve_paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(
        digivolve_paths, 2,
        "BT25-031 must register standard Yellow and alt TS-trait digivolve paths"
    );
}

// ── Section 2/3: Behavioral — reveal + bucket picks ────────────────────────

#[test]
fn bt25_031_on_play_adds_angel_group_and_ts_rest_to_bottom() {
    let mut angel = make_test_card("ANGEL-A", "Angel A");
    angel.traits.push("Archangel".to_string());
    let mut ts = make_test_card("TS-031", "TS 031");
    ts.traits.push("TS".to_string());
    let filler = make_test_card("FILL-031", "Filler 031");
    let holder = make_test_card("BT25_031_HOLDER", "Holder");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-031")
        .expect("BT25-031 compiles")
        .add_card(angel)
        .add_card(ts)
        .add_card(filler)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_031_HOLDER", None);
    let src = runner.top_card(holder_perm);

    let deck_before = runner.game.players[0].deck.len();
    // Top-3 (last pushed = top): FILL-031, TS-031, ANGEL-A.
    stack_deck_top(&mut runner, &["FILL-031", "TS-031", "ANGEL-A"]);

    let process = runner
        .compiled_card("BT25-031")
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
    assert!(
        hand_ids.contains(&"ANGEL-A"),
        "Archangel pick to hand: {hand_ids:?}"
    );
    assert!(
        hand_ids.contains(&"TS-031"),
        "TS pick to hand: {hand_ids:?}"
    );
    assert!(
        !hand_ids.contains(&"FILL-031"),
        "non-matching filler must NOT be added to hand: {hand_ids:?}"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before + 1,
        "two picks left deck, one filler returned to bottom"
    );
}

/// Negative: the Angel-group bucket accepts Angel/Archangel/Three Great
/// Angels/Four Great Dragons traits — and rejects a card with none of them.
/// A plain Dragon (not Four Great Dragons) must NOT satisfy the Angel bucket.
#[test]
fn bt25_031_angel_bucket_rejects_non_angel_traits() {
    let mut ts = make_test_card("TS-031B", "TS 031B");
    ts.traits.push("TS".to_string());
    let mut dragon = make_test_card("DRG-031", "Dragon 031");
    dragon.traits.push("Dragon".to_string());
    let filler = make_test_card("FILL-031B", "Filler 031B");
    let holder = make_test_card("BT25_031_HOLDER2", "Holder2");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-031")
        .expect("BT25-031 compiles")
        .add_card(ts)
        .add_card(dragon)
        .add_card(filler)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_031_HOLDER2", None);
    let src = runner.top_card(holder_perm);
    stack_deck_top(&mut runner, &["FILL-031B", "DRG-031", "TS-031B"]);

    let process = runner
        .compiled_card("BT25-031")
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
    runner
        .auto_resolve()
        .expect("resolves with only TS candidate");

    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        hand_ids.contains(&"TS-031B"),
        "TS pick to hand: {hand_ids:?}"
    );
    assert!(
        !hand_ids.contains(&"DRG-031"),
        "plain Dragon trait must NOT satisfy the Angel/4-Great-Dragons bucket: {hand_ids:?}"
    );
}

/// Negative: zero eligible candidates → both buckets fizzle silently and all
/// 3 revealed cards return to the deck bottom (no pending selection).
#[test]
fn bt25_031_on_play_fizzles_when_no_candidates() {
    let fa = make_test_card("F031-A", "f a");
    let fb = make_test_card("F031-B", "f b");
    let fc = make_test_card("F031-C", "f c");
    let holder = make_test_card("BT25_031_HOLDER3", "Holder3");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-031")
        .expect("BT25-031 compiles")
        .add_card(fa)
        .add_card(fb)
        .add_card(fc)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_031_HOLDER3", None);
    let src = runner.top_card(holder_perm);

    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.game.players[0].deck.len();
    stack_deck_top(&mut runner, &["F031-C", "F031-B", "F031-A"]);

    let process = runner
        .compiled_card("BT25-031")
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
