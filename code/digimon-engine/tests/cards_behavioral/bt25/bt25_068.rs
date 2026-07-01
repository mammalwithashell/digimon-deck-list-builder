//! BT25-068 Deltamon — Digimon, Lv.4, Black, DP 4000, Cost 4.
//! Traits: Composite, Titan, TS.
//! Evo: Lv.3 Black / cost 2 (printed; + DCGO alt on a [TS] Lv.3 base, cost 2).
//!
//! # Card text (cards.json — verbatim)
//!
//! <Collision> (During this Digimon's attack, all of your opponent's Digimon
//! gain <Blocker>, and must block if possible.)
//! [All Turns] [Once Per Turn] When this Digimon suspends, <De-Digivolve 1> 1
//! of your opponent's Digimon. (Trash the top card. You can't trash past level
//! 3 cards.)
//!
//! Inherited Effect:
//! [All Turns] This Digimon gets +1000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_068.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - A (keyword grant: Collision)
//! - B (on-suspend triggered clause, all-turns, OPT, self-source gated)
//! - C (mandatory opponent-permanent select → de_digivolve)
//! - G (inherited continuous +DP aura)
//! - H (alt digivolution path)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-068";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn lv3_base(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.traits = vec!["Composite".to_string()];
    card
}

/// A Lv.5 opponent Digimon stacked over a Lv.4 so De-Digivolve 1 can pop one
/// source (and never trashes below level 3).
fn opp_lv5(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(6000);
    card.traits = vec!["Beast".to_string()];
    card
}

fn opp_lv4(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.traits = vec!["Beast".to_string()];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-068 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(lv3_base("BLACK-LV3"))
        .add_card(opp_lv5("OPP-LV5"))
        .add_card(opp_lv4("OPP-LV4"))
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 8])
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_068_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Deltamon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(card.cost, Some(4));
    for t in ["Composite", "Titan", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "missing trait {t}");
    }
}

#[test]
fn bt25_068_has_collision_keyword() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "Collision"
        )
    });
    assert!(has, "BT25-068 must grant <Collision>");
}

#[test]
fn bt25_068_has_inherited_dp_aura() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
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
    assert!(aura, "BT25-068 must have an inherited +1000 DP aura");
}

#[test]
fn bt25_068_suspend_clause_is_opt_all_turns_self_source() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSuspend) => Some(t),
            _ => None,
        })
        .expect("must have an OnSuspend triggered clause");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(clause.once_per_turn, "[Once Per Turn] → once_per_turn");
    assert!(
        !clause.optional,
        "no printed 'may' on the suspend clause → mandatory once it fires"
    );
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::DeDigivolve { .. })),
        "suspend clause body must contain a de_digivolve step"
    );
}

#[test]
fn bt25_068_has_ts_lv3_cost2_alt_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let path = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Digivolve)
        .expect("must have a digivolve alt-path");
    let from = path.from.as_ref().expect("alt-path has a from predicate");
    assert_eq!(from.trait_has.as_deref(), Some("TS"));
    assert_eq!(path.cost, Some(CompiledCost::Literal(2)));
}

// ─── Section 2 — Behavior ────────────────────────────────────────────────────

/// Positive: this Digimon suspends with an opponent Digimon present → the
/// trigger fires (OPT) and De-Digivolves 1 source off the chosen opponent.
#[test]
fn bt25_068_self_suspend_de_digivolves_opponent() {
    let mut runner = base().start();
    let deltamon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_stack(1, &["OPP-LV4", "OPP-LV5"]);

    let opp_sources_before = runner.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();
    assert_eq!(
        opp_sources_before, 2,
        "precondition: opponent has 2 sources"
    );

    runner.game.suspend(deltamon);
    assert!(
        runner.pending_selection().is_some(),
        "self-suspend must install the De-Digivolve target select"
    );
    runner.auto_resolve().expect("resolve de_digivolve");

    let opp_sources_after = runner.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();
    assert_eq!(
        opp_sources_after,
        opp_sources_before - 1,
        "De-Digivolve 1 must remove one source from the opponent"
    );
}

/// OPT lockout: a second self-suspend in the same turn does not fire again.
#[test]
fn bt25_068_de_digivolve_is_once_per_turn() {
    let mut runner = base().start();
    let deltamon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_stack(1, &["OPP-LV4", "OPP-LV5"]);

    runner.game.suspend(deltamon);
    runner.auto_resolve().expect("first de_digivolve resolves");
    let after_first = runner.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();

    // Re-suspend in the same turn.
    runner.game.suspend(deltamon);
    assert!(
        runner.pending_selection().is_none(),
        "second self-suspend in the same turn must not fire (OPT)"
    );
    let after_second = runner.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();
    assert_eq!(
        after_first, after_second,
        "no further De-Digivolve on the second suspend"
    );
}

/// Negative: no opponent Digimon → the suspend clause is a no-op (no prompt).
#[test]
fn bt25_068_no_fire_without_opponent_digimon() {
    let mut runner = base().start();
    let deltamon = runner.place_on_field(0, CARD_ID, Some(0));

    runner.game.suspend(deltamon);
    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon → De-Digivolve select has no target → no-op"
    );
}

/// Inherited +1000 DP: as a digivolution source, Deltamon buffs its carrier.
#[test]
fn bt25_068_inherited_grants_plus_1000_dp() {
    let mut carrier = opp_lv5("CARRIER");
    carrier.dp = Some(6000);
    let mut runner = base().add_card(carrier).start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert_eq!(
        runner.effective_dp(stack),
        Some(7000),
        "carrier 6000 + inherited 1000 from Deltamon source = 7000"
    );
}
