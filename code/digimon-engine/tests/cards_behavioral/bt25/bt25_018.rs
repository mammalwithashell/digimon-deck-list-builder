//! BT25-018 Apollomon — Digimon, Lv.6, Red/Yellow, DP 12000, Cost 12.
//! Traits: Shaman, Olympos XII, Iliad, TS.
//!
//! # Card text (cards.json — verbatim)
//!
//! When this card would be played, if your opponent has a Digimon with 12000 DP
//! or more, reduce the cost by 5.
//! [On Play] [When Digivolving] For the turn, all of your opponent's Digimon get
//! -2000 DP for each of your Digimon. Then, delete 1 of their Digimon with as
//! much DP as this Digimon or less.
//! [End of Your Turn] 2 of your Digimon may DNA digivolve into [GraceNovamon] in
//! the hand. Then, 1 of your Digimon may attack.
//!
//! Inherited: [When Attacking] [Once Per Turn] Delete 1 of your opponent's
//! Digimon with as much DP as this Digimon or less.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_018.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - D2 cost reduction (board-condition gated)
//! - D1 per-own-Digimon-count DP debuff to all opponent Digimon (turn-scoped)
//! - F-delete with DP ≤ source (source_dp formula)
//! - G2 DNA digivolve (EOT, optional) + may attack
//! - G4-adjacent inherited When Attacking delete (OPT)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardKind;

const CARD_ID: &str = "BT25-018";

fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(dp);
    c.play_cost = 6;
    c
}

fn own_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-018 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(opp_digimon("OPP-BIG", 13000))
        .add_card(opp_digimon("OPP-SMALL", 5000))
        .add_card(own_digimon("OWN-DIGI"))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_018_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    assert_eq!(card.name, "Apollomon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(12000));
}

#[test]
fn bt25_018_has_cost_reduction_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                amount: Some(5),
                ..
            })
        )
    });
    assert!(has, "cost reduction of 5");
}

#[test]
fn bt25_018_has_eot_dna_clause_and_inherited_wa() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let eot = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::EndOfYourTurn)
        )
    });
    let inh_wa = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::WhenAttacking)
                    && t.once_per_turn
        )
    });
    assert!(eot, "EOT DNA clause present");
    assert!(inh_wa, "inherited When Attacking OPT clause present");
}

// ─── Section 2 — Behavior: OnPlay debuff + delete ────────────────────────────

/// All opponent Digimon get -2000 DP for each of YOUR Digimon. With 1 own
/// Digimon already present + Apollomon on play = 2 own Digimon → -4000 to each
/// opp Digimon. We read the debuff on a 13000-DP opponent BEFORE auto-resolving
/// the mandatory delete (the big one is read directly so no index shift from a
/// later placement). 13000 - 4000 = 9000.
#[test]
fn bt25_018_on_play_debuffs_all_opponent_digimon_per_own_count() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["DECK-PAD"])
        .memory(13)
        .start();
    // One own Digimon already on the field; Apollomon becomes the 2nd after play.
    runner.place_on_field(0, "OWN-DIGI", Some(0));
    let opp_big = runner.place_on_field(1, "OPP-BIG", Some(0));
    let big_before = runner.effective_dp(opp_big).expect("dp"); // 13000

    let _perm = runner.play(0, 0).expect("play Apollomon");
    // Own Digimon count is now 2 (OWN-DIGI + Apollomon) → -4000 to each opp.
    // Read the debuff immediately, before resolving the mandatory delete.
    assert_eq!(
        runner.effective_dp(opp_big).expect("dp"),
        big_before - 4000,
        "each opp Digimon gets -2000 per your Digimon (2 own → -4000)"
    );
    runner.auto_resolve().ok();
}

/// The delete sub-clause only targets opp Digimon with DP ≤ this Digimon's DP.
/// With a 13000-DP opponent (> Apollomon's 12000) it cannot be the delete target
/// after the debuff is applied; but a small one (≤12000) can.
#[test]
fn bt25_018_on_play_installs_delete_prompt_for_weaker_digimon() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["DECK-PAD"])
        .memory(13)
        .start();
    runner.place_on_field(1, "OPP-SMALL", Some(0));

    let _perm = runner.play(0, 0).expect("play Apollomon");
    assert!(
        runner.pending_selection().is_some(),
        "the delete prompt installs when an opp Digimon with DP ≤ this exists"
    );
}

/// Deleting removes the chosen weaker opp Digimon from the battle area.
#[test]
fn bt25_018_on_play_deletes_weaker_opponent_digimon() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["DECK-PAD"])
        .memory(13)
        .start();
    runner.place_on_field(1, "OPP-SMALL", Some(0));
    let opp_before = runner.battle_area_size(1);

    let _perm = runner.play(0, 0).expect("play Apollomon");
    runner.auto_resolve().ok();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "the weaker opp Digimon is deleted"
    );
}
