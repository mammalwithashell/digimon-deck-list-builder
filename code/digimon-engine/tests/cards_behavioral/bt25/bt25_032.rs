//! BT25-032 Liollmon — Digimon, Lv.3, Yellow, DP 2000, Cost 3.
//! Traits: Holy Beast, Glowing Dawn, BEATBREAK. Attribute: Vaccine.
//!
//! # Printed effect text (data/cards.json — confirmed vs DCGO)
//!   [On Play] Reveal the top 3 cards of your deck. Add 1 [Glowing Dawn] trait
//!   card and 1 yellow [BEATBREAK] trait card among them to the hand. Return
//!   the rest to the bottom of the deck.
//!
//! # Printed inherited text
//!   <Barrier> (When this Digimon would be deleted in battle, by trashing your
//!   top security card, it isn't deleted.)
//!
//! # Alt-digivolve (cards.json xros_req)
//!   [Digivolve] Lv.2 w/[Glowing Dawn] trait: Cost 0
//!
//! # DCGO C# reference
//!   DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_032.cs
//!   - AddSelfDigivolutionRequirementStaticEffect(level 2, TopCard
//!     HasGlowingDawnTraits, cost 0).
//!   - OnEnterFieldAnyone: SimplifiedRevealDeckTopCardsAndSelect(revealCount 3,
//!     bucket0 EqualsTraits("Glowing Dawn"), bucket1 EqualsTraits("BEATBREAK") &&
//!     HasCardColor(Yellow), maxCount 1 each, mode AddHand,
//!     remainingCardsPlace DeckBottom, mutualConditions: true).
//!   - WhenPermanentWouldBeDeleted: BarrierSelfEffect(isInheritedEffect: true).
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//!   - Group E: reveal-N two-bucket pick (no "may" → mandatory)
//!   - no_duplicate_cards (DCGO mutualConditions: true)
//!   - Group G: inherited static keyword grant (<Barrier>)
//!   - alt-digivolve recipe registration

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardColor;

const CARD_ID: &str = "BT25-032";

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-032 YAML parses and compiles")
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
        .compiled_card(CARD_ID)
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
fn bt25_032_is_yellow_glowing_dawn_digimon() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Liollmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(2000));
    assert!(card.traits.iter().any(|t| t == "Glowing Dawn"));
    assert!(card.traits.iter().any(|t| t == "BEATBREAK"));
}

#[test]
fn bt25_032_has_on_play_reveal_two_pick_clause() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");

    let on_play = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("BT25-032 must have an [On Play] clause");

    let has_reveal = on_play
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::RevealTopDeck { count: 3, .. }));
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
        "must have 2 buckets (Glowing Dawn, yellow BEATBREAK)"
    );
    assert_eq!(add_to_hand_count, 2, "must add 2 cards to hand");
    assert!(has_place_remainder, "must place remainder on deck bottom");
}

#[test]
fn bt25_032_no_duplicate_cards_set() {
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
fn bt25_032_has_inherited_barrier() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let barrier = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            scope,
            ..
        }) => *scope == CompiledScope::Inherited && keyword.eq_ignore_ascii_case("Barrier"),
        _ => false,
    });
    assert!(barrier, "BT25-032 must grant inherited <Barrier>");
}

#[test]
fn bt25_032_registers_glowing_dawn_alt_digivolve() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let digivolve_paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(
        digivolve_paths, 1,
        "BT25-032 must register the Glowing-Dawn-trait Lv.2 alt digivolve path"
    );
}

// ── Section 2/3: Behavioral — reveal + bucket picks ────────────────────────

#[test]
fn bt25_032_on_play_adds_glowing_dawn_and_yellow_beatbreak_rest_to_bottom() {
    let mut gd = make_test_card("GD-A", "GD A");
    gd.traits.push("Glowing Dawn".to_string());
    let mut bb = make_test_card("YBB-032", "Yellow BB 032");
    bb.traits.push("BEATBREAK".to_string());
    bb.colors = vec![CardColor::Yellow];
    let filler = make_test_card("FILL-032", "Filler 032");
    let holder = make_test_card("BT25_032_HOLDER", "Holder");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-032 compiles")
        .add_card(gd)
        .add_card(bb)
        .add_card(filler)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_032_HOLDER", None);
    let src = runner.top_card(holder_perm);

    let deck_before = runner.game.players[0].deck.len();
    // Top-3 (last pushed = top): FILL-032, YBB-032, GD-A.
    stack_deck_top(&mut runner, &["FILL-032", "YBB-032", "GD-A"]);

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
        hand_ids.contains(&"GD-A"),
        "Glowing Dawn pick to hand: {hand_ids:?}"
    );
    assert!(
        hand_ids.contains(&"YBB-032"),
        "yellow BEATBREAK pick to hand: {hand_ids:?}"
    );
    assert!(
        !hand_ids.contains(&"FILL-032"),
        "non-matching filler must NOT be added to hand: {hand_ids:?}"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before + 1,
        "two picks left deck, one filler returned to bottom"
    );
}

/// Negative (color filter): a GREEN [BEATBREAK] card does NOT satisfy the
/// "yellow [BEATBREAK]" bucket — only the Glowing Dawn card is added.
#[test]
fn bt25_032_beatbreak_bucket_rejects_non_yellow_color() {
    let mut gd = make_test_card("GD-032B", "GD 032B");
    gd.traits.push("Glowing Dawn".to_string());
    let mut green_bb = make_test_card("GBB-032", "Green BB 032");
    green_bb.traits.push("BEATBREAK".to_string());
    green_bb.colors = vec![CardColor::Green];
    let filler = make_test_card("FILL-032B", "Filler 032B");
    let holder = make_test_card("BT25_032_HOLDER2", "Holder2");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-032 compiles")
        .add_card(gd)
        .add_card(green_bb)
        .add_card(filler)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_032_HOLDER2", None);
    let src = runner.top_card(holder_perm);
    stack_deck_top(&mut runner, &["FILL-032B", "GBB-032", "GD-032B"]);

    let process = on_play_process(&runner);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }
    runner
        .auto_resolve()
        .expect("resolves with only the Glowing Dawn candidate");

    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        hand_ids.contains(&"GD-032B"),
        "Glowing Dawn pick to hand: {hand_ids:?}"
    );
    assert!(
        !hand_ids.contains(&"GBB-032"),
        "a GREEN [BEATBREAK] must NOT satisfy the yellow BEATBREAK bucket: {hand_ids:?}"
    );
}

/// Negative: zero eligible candidates → both buckets fizzle silently and all
/// 3 revealed cards return to the deck bottom (no pending selection).
#[test]
fn bt25_032_on_play_fizzles_when_no_candidates() {
    let fa = make_test_card("F032-A", "f a");
    let fb = make_test_card("F032-B", "f b");
    let fc = make_test_card("F032-C", "f c");
    let holder = make_test_card("BT25_032_HOLDER3", "Holder3");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-032 compiles")
        .add_card(fa)
        .add_card(fb)
        .add_card(fc)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_032_HOLDER3", None);
    let src = runner.top_card(holder_perm);

    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.game.players[0].deck.len();
    stack_deck_top(&mut runner, &["F032-C", "F032-B", "F032-A"]);

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
