//! BT25-064 ToyAgumon — Digimon, Lv.3, Black, DP 2000, Cost 3.
//! Traits: Puppet, Iliad, TS. Attribute: Vaccine.
//!
//! Printed effect text (cards.json):
//!   [On Play] Reveal the top 3 cards of your deck. Add 1 Option card and 1
//!   [TS] trait card among them to the hand. Return the rest to the bottom of
//!   the deck.
//!
//! Printed inherited text:
//!   <Reboot> (This Digimon also unsuspends in your opponent's unsuspend phase.)
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_064.cs
//!
//! Pattern rows (RUST_DSL_TEST_API §4.3): A1 reveal-N two-bucket pick (Group E,
//! one bucket is Option-kind), H7 inherited Reboot keyword grant (Group H),
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
        .dsl_card("BT25-064")
        .expect("BT25-064 YAML parses and compiles")
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

fn on_play_process(runner: &DebugRunner) -> Vec<CompiledStep> {
    runner
        .compiled_card("BT25-064")
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
fn bt25_064_has_on_play_reveal_two_pick_clause() {
    let runner = runner();
    let card = runner.compiled_card("BT25-064").expect("compiled");

    let on_play = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("BT25-064 must have an [On Play] clause");

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
    assert_eq!(bucket_count, 2, "must have 2 buckets (Option, TS)");
    assert_eq!(add_to_hand_count, 2, "must add 2 cards to hand");
    assert!(has_place_remainder, "must place remainder on deck bottom");
}

#[test]
fn bt25_064_no_duplicate_cards_set() {
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
    assert!(no_dup, "select_reveal_buckets must set no_duplicate_cards");
}

#[test]
fn bt25_064_has_inherited_reboot() {
    let runner = runner();
    let card = runner.compiled_card("BT25-064").expect("compiled");
    let reboot = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            scope,
            ..
        }) => *scope == CompiledScope::Inherited && keyword.eq_ignore_ascii_case("Reboot"),
        _ => false,
    });
    assert!(reboot, "BT25-064 must grant inherited <Reboot>");
}

#[test]
fn bt25_064_registers_ts_trait_alt_digivolve() {
    let runner = runner();
    let card = runner.compiled_card("BT25-064").expect("compiled");
    let digivolve_paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(
        digivolve_paths, 1,
        "BT25-064 must register the TS-trait alt digivolve path"
    );
}

// ── Section 2/3: Behavioral ────────────────────────────────────────────────

#[test]
fn bt25_064_on_play_adds_option_and_ts_to_hand_rest_to_bottom() {
    // Use a real Option card so the `kind: option` filter matches a genuine
    // Option, not a synthesized stand-in.
    let mut ts = make_test_card("TS-064", "TS 064");
    ts.traits.push("TS".to_string());
    let filler = make_test_card("FILL-064", "Filler 064");
    let holder = make_test_card("BT25_064_HOLDER", "Holder");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-064")
        .expect("BT25-064 compiles")
        .dsl_card("BT25-100") // Iron Slash — a real Option card
        .expect("BT25-100 compiles")
        .add_card(ts)
        .add_card(filler)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_064_HOLDER", None);
    let src = runner.top_card(holder_perm);

    let deck_before = runner.game.players[0].deck.len();
    stack_deck_top(&mut runner, &["FILL-064", "TS-064", "BT25-100"]);

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
        hand_ids.contains(&"BT25-100"),
        "Option pick to hand: {hand_ids:?}"
    );
    assert!(
        hand_ids.contains(&"TS-064"),
        "TS pick to hand: {hand_ids:?}"
    );
    assert!(
        !hand_ids.contains(&"FILL-064"),
        "non-matching filler must NOT be added to hand: {hand_ids:?}"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before + 1,
        "two picks left deck, one filler returned to bottom"
    );
}

#[test]
fn bt25_064_on_play_fizzles_when_no_candidates() {
    let fa = make_test_card("F064-A", "f a");
    let fb = make_test_card("F064-B", "f b");
    let fc = make_test_card("F064-C", "f c");
    let holder = make_test_card("BT25_064_HOLDER2", "Holder2");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-064")
        .expect("BT25-064 compiles")
        .add_card(fa)
        .add_card(fb)
        .add_card(fc)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_064_HOLDER2", None);
    let src = runner.top_card(holder_perm);

    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.game.players[0].deck.len();
    stack_deck_top(&mut runner, &["F064-C", "F064-B", "F064-A"]);

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
        "no pending selection after fizzle"
    );
    assert_eq!(
        runner.game.revealed_cards.len(),
        0,
        "reveal pool empty after fizzle"
    );
}
