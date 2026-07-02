//! BT25-048 Bearmon — Digimon, Lv.3, Green, DP 2000, Cost 3.
//! Traits: Beast, Iliad, TS. Attribute: Vaccine.
//!
//! # Card text (cards.json — verbatim)
//!
//! [Your Turn] When this Digimon would digivolve into a [TS] trait Digimon
//! card, reduce the cost by 1.
//!
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When this Digimon wins a battle, ＜Draw 1＞.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_048.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - D2 (digivolution-cost-reduction declarative, restricted to this base)
//! - F-win-battle (source_deleted_battle_opponent → draw, OPT)
//! - H (alt digivolution path)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlaySource};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-048";

fn named_lv4(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.evo_costs = vec![EvoCost {
        card_color: 3,
        level: 3,
        memory_cost: 2,
    }];
    c
}

fn weak_opp(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(1000);
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-048 YAML parses and compiles")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(weak_opp("OPP-WEAK"))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt25_048_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    assert_eq!(card.name, "Bearmon");
    assert_eq!(card.dp, Some(2000));
    assert!(card
        .color
        .iter()
        .any(|c| format!("{c:?}").contains("Green")));
}

#[test]
fn bt25_048_has_ts_alt_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let path = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Digivolve)
        .expect("digivolve alt-path");
    assert_eq!(path.from.as_ref().unwrap().trait_has.as_deref(), Some("TS"));
    assert_eq!(path.cost, Some(CompiledCost::Literal(0)));
}

#[test]
fn bt25_048_has_cost_reduction_declarative() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has_cr = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )
    });
    assert!(
        has_cr,
        "[Your Turn] digivolve-cost reduction must compile to a CostReduction declarative"
    );
}

#[test]
fn bt25_048_has_inherited_win_battle_opt_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let win = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && (t.when.contains(&CompiledTiming::OnAnyDeletion)
                        || t.when.contains(&CompiledTiming::OnDeletion)
                        || t.when.contains(&CompiledTiming::EndOfBattle)) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited win-battle clause present");
    assert!(win.once_per_turn, "win-battle draw is [Once Per Turn]");
}

// ─── Section 2 — Behavior: digivolve cost reduction (-1 into a [TS] Digimon) ──

#[test]
fn bt25_048_reduces_ts_digivolve_cost_by_one() {
    // Bearmon (Lv.3 green) on field, a [TS] Lv.4 in hand whose printed evo cost
    // from Lv.3 green is 2. With Bearmon's [Your Turn] reduction the digivolve
    // should cost 1 instead of 2.
    let mut ts_target = named_lv4("TS-LV4", "TsTarget");
    ts_target.traits = vec!["TS".to_string()];

    let mut runner = base()
        .add_card(ts_target)
        .hand(0, &["TS-LV4"])
        .memory(10)
        .start();
    runner.game.turn_player_idx = 0;
    let bear = runner.place_on_field(0, CARD_ID, Some(0));

    let memory_before = runner.memory();
    // Digivolve Bearmon into the TS Lv.4 from hand (hand index 0). The card
    // reducer (`source_is_cost_target_permanent`) applies automatically during
    // cost calculation — no interactive player-reducer prompt is installed.
    let ok = runner
        .game
        .digivolve_from_hand(0, 0, bear.index as usize, PlaySource::ByDigivolve);
    assert!(ok, "digivolve must succeed");
    // Printed cost 2, reduced by 1 → 1 memory spent.
    assert_eq!(
        runner.memory(),
        memory_before - 1,
        "[TS] digivolve cost reduced from 2 to 1 by Bearmon's [Your Turn] clause"
    );
}

#[test]
fn bt25_048_no_reduction_for_non_ts_digivolve() {
    // A non-[TS] Lv.4 in hand: printed evo cost 2, NO reduction → 2 spent.
    let plain = named_lv4("PLAIN-LV4", "PlainTarget"); // no TS trait
    let mut runner = base()
        .add_card(plain)
        .hand(0, &["PLAIN-LV4"])
        .memory(10)
        .start();
    runner.game.turn_player_idx = 0;
    let bear = runner.place_on_field(0, CARD_ID, Some(0));

    let memory_before = runner.memory();
    let ok = runner
        .game
        .digivolve_from_hand(0, 0, bear.index as usize, PlaySource::ByDigivolve);
    assert!(ok, "digivolve must succeed");
    assert_eq!(
        runner.memory(),
        memory_before - 2,
        "non-[TS] digivolve pays the full printed cost (no reduction)"
    );
}

// ─── Section 3 — Behavior: inherited win-battle draw ─────────────────────────

#[test]
fn bt25_048_inherited_draws_on_battle_win() {
    let mut carrier = named_lv4("CARRIER", "Carrier");
    carrier.dp = Some(8000);
    let mut runner = base().add_card(carrier).memory(10).start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]); // Bearmon is a source
    let weak = runner.place_on_field(1, "OPP-WEAK", Some(0)); // 1000 DP → deleted

    let deck_before = runner.deck_size(0);
    runner.attack_digimon(stack, weak, false);
    runner.auto_resolve().expect("resolve inherited draw");

    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "winning a battle draws 1 via Bearmon's inherited effect"
    );
}

#[test]
fn bt25_048_inherited_does_not_draw_without_battle_win() {
    let mut carrier = named_lv4("CARRIER", "Carrier");
    carrier.dp = Some(8000);
    let mut runner = base().add_card(carrier).memory(10).start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    // No opponent Digimon: attack the player (security check, not a battle win).
    let deck_before = runner.deck_size(0);
    runner.attack_player(stack, 1, false);
    runner.auto_resolve().ok();

    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "a player attack / security check is not a battle win → no inherited draw"
    );
}
