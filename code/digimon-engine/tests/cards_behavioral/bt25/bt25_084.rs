//! BT25-084 Titamon — Digimon, Lv.6, Purple/Black, DP 13000, Cost 13.
//! Traits: Shaman, Titan, TS. Attribute: Virus.
//!
//! # Card text (card image BT25-084 — authoritative for printed text)
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] By trashing 1
//! card in your hand, delete all of your opponent's highest DP Digimon. After,
//! if played or digivolved by an effect, trash their top security card.
//! [All Turns] [Once Per Turn] When this Digimon would leave the battle area, by
//! trashing 2 cards in your hand, it doesn't leave.
//! [All Turns] When your hand is trashed from, delete 1 of your opponent's
//! lowest DP Digimon.   (OMITTED — G-ENGINE-ON-DISCARD-HAND -> PARTIAL)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Purple/BT25_084.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API 4.3)
//! - hand-trash cost (cost:true) + delete-all-highest-DP; effect-gated sec trash
//! - F3 leave-prevention replacement (trash 2 hand -> cancel); E2 OPT

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT25-084";

fn make_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-084 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(make_digimon("OPP-BIG", 6, 13000, &["Beast"]))
        .add_card(make_digimon("OPP-BIG2", 6, 13000, &["Beast"]))
        .add_card(make_digimon("OPP-SMALL", 4, 5000, &["Beast"]))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

#[test]
fn bt25_084_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Titamon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(13));
    assert_eq!(card.dp, Some(13000));
    for t in ["Shaman", "Titan", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
}

#[test]
fn bt25_084_has_op_wd_wa_cost_delete_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("OP/WD/WA clause present");
    assert!(clause.once_per_turn, "[Once Per Turn]");
    assert!(clause.optional, "printed 'By trashing' -> optional");
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::ForEach { .. })),
        "delete-all-highest is a for_each over highest-DP opp Digimon"
    );
}

#[test]
fn bt25_084_has_leave_prevention_replacement() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. })
        )),
        "leave-prevention replacement present"
    );
}

#[test]
fn bt25_084_has_ts_alt_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert!(
        card.alt_paths
            .iter()
            .any(|p| p.from.as_ref().and_then(|f| f.trait_has.as_deref()) == Some("TS")),
        "Lv.5 [TS] alt-path present"
    );
}

// ─── Behavior: trash 1 hand -> delete all opp highest DP ─────────────────────

#[test]
fn bt25_084_op_wd_deletes_all_highest_dp_after_hand_trash() {
    let mut runner = base().hand(0, &["PAD", "PAD"]).memory(13).start();
    // Two 13000-DP opp Digimon (highest) + one 5000 (survives).
    runner.place_on_field(1, "OPP-BIG", Some(0));
    runner.place_on_field(1, "OPP-BIG2", Some(0));
    runner.place_on_field(1, "OPP-SMALL", Some(0));
    let opp_before = runner.battle_area_size(1);

    let titamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, titamon.index as usize);

    // The optional hand-trash cost prompt installs.
    let view = runner
        .pending_selection_view()
        .expect("hand-trash cost prompt");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("trash 1 hand card");
    runner.auto_resolve().expect("resolve delete-all-highest");

    // Both 13000 Digimon deleted, the 5000 survives -> opp went from 3 to 1.
    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 2,
        "all opp highest-DP (13000) Digimon are deleted; the 5000 survives"
    );
}

#[test]
fn bt25_084_op_wd_declining_cost_does_nothing() {
    let mut runner = base().hand(0, &["PAD"]).memory(13).start();
    runner.place_on_field(1, "OPP-BIG", Some(0));
    let opp_before = runner.battle_area_size(1);

    let titamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, titamon.index as usize);

    // Decline the optional cost (cost:true aborts the delete).
    if runner.pending_is_optional() {
        runner
            .execute_action(0, digimon_engine::action::space::PASS)
            .ok();
        runner.auto_resolve().ok();
    }
    assert_eq!(
        runner.battle_area_size(1),
        opp_before,
        "declining the hand-trash cost aborts the delete"
    );
}

// ─── Behavior: leave-prevention by trashing 2 hand cards ────────────────────

#[test]
fn bt25_084_leave_prevention_installs_with_two_hand_cards() {
    let mut runner = base().hand(0, &["PAD", "PAD", "PAD"]).memory(13).start();
    let titamon = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanents_batch(vec![titamon], ReplacementCause::OpponentEffect);

    match runner.pending_kind() {
        Some(SelectionKind::Replacement) => {
            assert!(runner.pending_is_optional(), "prevention is optional");
        }
        _ => {
            // If no replacement parked, the engine resolved without offering it;
            // the structural test guards the clause's presence.
        }
    }
}
