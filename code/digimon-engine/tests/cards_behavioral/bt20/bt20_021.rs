//! BT20-021 Jesmon GX — Digimon, Lv.7, Red/Purple, DP 16000, Cost 9.
//! Traits: Holy Warrior / X Antibody / Royal Knight. ACE (Overflow -5).
//!
//! # Card text (cards.json / card image)
//!
//! ```text
//! [Hand] [Counter] <Blast Digivolve> (Your Digimon may digivolve into this
//!   card without paying the cost.)
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] By placing 1
//!   [Royal Knight] trait card from your hand or trash as this Digimon's
//!   bottom digivolution card, delete 1 of your opponent's Digimon with as
//!   much or less DP as this Digimon.
//! [When Attacking] [Once Per Turn] This Digimon unsuspends. Then, for every
//!   2 [Royal Knight] trait cards in this Digimon's digivolution cards, trash
//!   your opponent's top security card.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT20/Red/BT20_021.cs
//!
//! # Implemented slice
//! - [Hand][Counter] <Blast Digivolve>: `burst_digivolve` alt-path marker
//!   (cost 0) + `grant_keyword: BlastDigivolve`. PRECEDENT: BT20-060.
//!
//! # Gap-routed (NOT implemented — see YAML stubs)
//! - Clause "place a [Royal Knight] from hand OR trash as bottom source, then
//!   delete by this Digimon's DP" — G-UNION-HAND-TRASH-SOURCE-COST (union
//!   hand/trash → bottom-source placement) + source-DP-compare predicate.
//! - Clause "unsuspend, then trash opp top security per 2 [Royal Knight]
//!   sources" — G-SOURCE-COUNT-SECURITY-TRASH (trait-count-in-sources formula
//!   driving a per-bucket security trash).

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt20_021_loads_ace_stub() {
    DebugRunner::builder()
        .dsl_card("BT20-021")
        .expect("BT20-021 must load from embedded DSL pack")
        .start();
}

#[test]
fn bt20_021_has_blast_digivolve_marker_and_keyword() {
    let runner = DebugRunner::builder()
        .dsl_card("BT20-021")
        .expect("BT20-021 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("BT20-021").expect("compiled card");

    assert!(
        card.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::BurstDigivolve
                && path.marker
                && path.cost == Some(CompiledCost::Literal(0))
        }),
        "BT20-021 must register a <Blast Digivolve> alt-path marker at cost 0"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "BlastDigivolve"
        )),
        "BT20-021 must grant the BlastDigivolve keyword"
    );
}

#[ignore = "pending: G-UNION-HAND-TRASH-SOURCE-COST and G-SOURCE-COUNT-SECURITY-TRASH — Royal Knight source cost, DP compare, unsuspend, and source-count security trash; tracker: qa/dsl-vocab-gaps.md"]
#[test]
fn bt20_021_source_cost_delete_and_source_count_security_trash() {}
