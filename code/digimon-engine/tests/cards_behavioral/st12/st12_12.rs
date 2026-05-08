//! ST12-12 Sistermon Blanc
//!
//! Implemented slice:
//! - [On Play] By trashing 1 card in your hand, Draw 2.
//!
//! Gap-routed slice:
//! - Conditional Decoy (Red/Black) while you have a Huckmon/Royal Knight.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("ST12-12")
        .expect("ST12-12 must load from embedded DSL pack")
        .memory(5)
        .start()
}

#[test]
fn st12_12_has_on_play_discard_draw_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("ST12-12")
        .expect("ST12-12 must be compiled");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
        )),
        "ST12-12 must have an OnPlay discard/draw clause"
    );
}

#[test]
fn st12_12_on_play_trashes_one_card_and_draws_two() {
    let discard = make_test_card("ST12-12-DISCARD", "Discard");
    let filler = make_test_card("ST12-12-FILLER", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("ST12-12")
        .expect("ST12-12 must load")
        .add_card(discard)
        .add_card(filler)
        .hand(0, &["ST12-12", "ST12-12-DISCARD"])
        .deck(0, &["ST12-12-FILLER", "ST12-12-FILLER"])
        .memory(5)
        .start();

    runner.play(0, 0);
    runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].trash.len(),
        1,
        "one card should be trashed"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        2,
        "Draw 2 should replace the discarded card"
    );
}

#[ignore = "pending: G-CONDITIONAL-DECOY-AURA — conditional Decoy grant while Huckmon/Royal Knight is present needs keyword aura lowering with color-filtered Decoy"]
#[test]
fn st12_12_gains_conditional_decoy_for_red_black_allies() {}
