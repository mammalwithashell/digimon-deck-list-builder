//! EX11-038 Sunarizamon
//!
//! # Printed card text (authoritative — cards.json `effect_description_eng`)
//!
//! "[When Moving] [On Play] By trashing 1 [Mineral] or [Rock] trait card from
//!  your hand or your Digimon's digivolution cards, <Draw 1>."
//!
//! "[Inherited] When effects trash this card from a [Mineral] or [Rock] trait
//!  Digimon's digivolution cards, <Draw 1>."
//!
//! # Implementation status
//!
//! Clause 0 ([When Moving] / [On Play]):
//!   BLOCKED — G-DSL-SELECT-OWN-SOURCES-FILTER (qa/dsl-vocab-gaps.md).
//!   The cost requires trashing 1 Mineral/Rock from hand OR own digivolution
//!   cards as a single unified selection. `select_union_zone` supports
//!   {hand, trash} but not digivolution sources; `select_own_sources` (filter
//!   now supported) covers sources only. No DSL verb spans hand ∪ own
//!   digivolution sources as one player-visible pool. Clause omitted; all
//!   tests for it carry the ignore tag below (matching EX11-065 convention).
//!   A hand-only proxy is not acceptable per the no-approximations policy.
//!
//! Clause 1 ([Inherited] source-trash draw):
//!   IMPLEMENTED — `on_digivolution_card_trashed` with host-trait and
//!   trashed-source-id conditions, `draw: { of: you, count: 1 }` body.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-038")
        .expect("EX11-038 YAML parses and compiles")
        .build()
}

#[test]
fn ex11_038_has_inherited_source_trash_draw_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("EX11-038")
        .expect("EX11-038 compiled card present");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(triggered)
            if triggered.scope == CompiledScope::Inherited
                && triggered.when == vec![CompiledTiming::OnDigivolutionCardTrashed]
    )));
}

#[test]
fn ex11_038_source_trash_draws_one() {
    let mut host = make_test_card("ROCK-HOST", "Rock Host");
    host.traits.push("Rock".to_string());
    let filler = make_test_card("FILLER", "Filler");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-038")
        .expect("EX11-038 YAML parses and compiles")
        .add_card(host)
        .add_card(filler)
        .deck(0, &["FILLER", "FILLER"])
        .build();

    let host = runner.place_on_field(0, "ROCK-HOST", None);
    let source = runner.push_source(host, "EX11-038");
    let hand_before = runner.hand_size(0);

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert_eq!(runner.hand_size(0), hand_before + 1);
}

// ─── Clause 0: [When Moving] / [On Play] — BLOCKED ───────────────────────────
//
// The printed cost is "by trashing 1 [Mineral] or [Rock] trait card from your
// hand OR your Digimon's digivolution cards." No DSL verb exists for a single
// player-visible selection spanning hand ∪ own digivolution sources (see
// qa/dsl-vocab-gaps.md § "Rocks pool pass residual" and EX11-065 Clause 0).
// All tests for this clause are ignored with the same tag used by EX11-065.

#[test]
#[ignore = "pending: G-DSL-SELECT-OWN-SOURCES-FILTER from qa/dsl-vocab-gaps.md — when_moving/on_play trash-hand-or-source cost"]
fn ex11_038_has_on_play_when_moving_draw_clause() {
    // When G-DSL-SELECT-OWN-SOURCES-FILTER closes and the hand-or-source union
    // trash cost is expressible, EX11-038 must gain an OnPlay + OnMove clause
    // with optional: false (printed "By trashing …" mandates the cost) and
    // draw: { of: you, count: 1 } in its process body.
    let runner = runner();
    let card = runner
        .compiled_card("EX11-038")
        .expect("EX11-038 compiled card present");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert!(
        triggered
            .iter()
            .any(|t| t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::OnMove)),
        "EX11-038 must have an [OnPlay, OnMove] clause once the gap closes; got: {triggered:?}"
    );
}

#[test]
#[ignore = "pending: G-DSL-SELECT-OWN-SOURCES-FILTER from qa/dsl-vocab-gaps.md — when_moving/on_play trash hand Mineral draws 1 (positive)"]
fn ex11_038_on_play_trash_hand_mineral_draws_one() {
    // Positive: have a Mineral card in hand, trash it via on-play trigger → Draw 1.
    todo!()
}

#[test]
#[ignore = "pending: G-DSL-SELECT-OWN-SOURCES-FILTER from qa/dsl-vocab-gaps.md — when_moving/on_play trash own source Rock draws 1 (positive)"]
fn ex11_038_on_play_trash_own_source_rock_draws_one() {
    // Positive: have a Rock source in own Digimon's digivolution stack, trash it
    // via on-play trigger → Draw 1.
    todo!()
}

#[test]
#[ignore = "pending: G-DSL-SELECT-OWN-SOURCES-FILTER from qa/dsl-vocab-gaps.md — when_moving/on_play no eligible card skips draw (negative)"]
fn ex11_038_on_play_no_eligible_card_no_draw() {
    // Negative: no Mineral/Rock in hand and no Mineral/Rock in own sources →
    // no pending selection and hand count unchanged.
    todo!()
}
